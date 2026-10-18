# Frontend integration: Top Headlines

Prepared for the separate My-Newsfeed frontend. The production WebScraper database
already contains the new category and six enabled RSS sources. This document
requests frontend implementation; no frontend changes or first-collection
verification were performed as part of adding the sources.

## Requested behavior

Expose **Top Headlines** alongside the site's existing news categories. Reuse the
existing news listing, article cards, source selector, pagination, saved-article
behavior, and mobile layout. Selecting this category shows articles belonging to
its enabled sources. If categories are discovered dynamically, verify that the
new category appears without adding a separate page. If navigation uses an
explicit list, register it using the site's existing category routing convention.

These are publisher headline feeds, not a separate ranking of stories. Use the
normal publication-time ordering; do not imply an editorial importance score.

## Production sources

The exact category name is `Top Headlines`. Resolve its ID from
`dbo.SourceCategories` rather than assuming a numeric ID. The source IDs below
are production reference values, not a list to hardcode into category filtering.

| Source ID | Label | RSS URL |
| --- | --- | --- |
| 8903 | NBC News | https://feeds.nbcnews.com/nbcnews/public/news |
| 8904 | CBS News | https://www.cbsnews.com/latest/rss/main |
| 8905 | The New York Times | https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml |
| 8906 | The Guardian | https://www.theguardian.com/international/rss |
| 8907 | Al Jazeera | https://www.aljazeera.com/xml/rss/all.xml |
| 8908 | Sky News | https://feeds.skynews.com/feeds/rss/home.xml |

BBC News (4084) and NPR (7093) remain in `General`; they were not reassigned.
All six new sources were verified enabled and assigned to Top Headlines after
commit. Their articles arrive through the normal scheduled collector. Source
registration alone does not establish successful ingestion.

## Data contract

Use the existing server-side news data access path:

- `dbo.SourceCategories.ID` joins `dbo.Sources.SourceCategoryId`.
- `dbo.Sources.ID` joins `dbo.Pages.SourceId`.
- Only include sources with `Enabled = 1`.
- Use `Pages.ID` as the existing article identity and saved-item key.
- Use `Pages.Url` for the article destination; `Sources.Url` is the RSS feed.
- Use `Sources.Label` for publisher attribution.
- Article fields are `Title`, `Description`, `ImageUrl`, `Published`, and
  `ScrapedAt`. Description, image, and publication time can be absent.

The following is a reference query, not a new API endpoint. Adapt it to the
application's existing repository and response model. Bind all parameters;
validate `@Offset >= 0` and bound the positive `@PageSize` in application code.
`@SourceId` is nullable and filters within this category.

```sql
SELECT p.ID AS ArticleId,
       p.Title, p.Description, p.Url AS ArticleUrl, p.ImageUrl,
       p.Published, p.ScrapedAt,
       s.ID AS SourceId, s.Label AS SourceName,
       c.ID AS CategoryId, c.Category_Name AS CategoryName
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID = p.SourceId
JOIN dbo.SourceCategories c ON c.ID = s.SourceCategoryId
WHERE s.Enabled = 1
  AND c.Category_Name = @CategoryName -- N'Top Headlines'
  AND (@SourceId IS NULL OR s.ID = @SourceId)
ORDER BY COALESCE(p.Published, p.ScrapedAt) DESC, p.ID DESC
OFFSET @Offset ROWS FETCH NEXT @PageSize ROWS ONLY;
```

Retain the site's established retention/date-window predicates if it has them,
and apply the same predicates to pagination counts. Populate source options
from enabled `Sources` in this category, even before they have article rows.
Discover category navigation from `SourceCategories` and enabled `Sources`,
rather than requiring an article row for the category to appear.

No new tables, scraper deployment, browser RSS fetching, or database writes are
required. Reuse the frontend's existing read access to these three tables. No
new credentials belong in browser code.

## Rendering and empty states

Use existing card styles, publisher attribution, article links, image fallback,
and timezone formatting. Treat stored timestamps consistently with the existing
UTC-to-display-time handling. Render titles and summaries with the application's
normal escaping/sanitization, and retain its outbound URL validation. Feeds may
supply only a summary; do not promise full article text or paywall bypass.

If the category exists but has no collected articles, show a normal empty state
such as “No headlines available yet. Please check back soon.” Keep the category
and source options visible. Database failures should use the site's normal error
handling, not be presented as an empty feed.

Keep the existing article URL identity and `ArticleIdRedirects` recovery behavior
for saved articles; see [article URL identity](article-url-identity.md).

## Acceptance checks

1. Top Headlines appears in category navigation and works on direct navigation
   and reload using the frontend's existing routing convention.
2. Its source selector lists the six enabled sources above. BBC and NPR remain
   under General.
3. Collected articles appear with their publisher, correct article destination,
   and normal newest-first ordering. Filtering by one source excludes the others.
4. Pagination and counts use the same category/source/date filters. Other news
   categories retain their existing behavior.
5. Missing images, summaries, or publication dates render cleanly; an empty
   category remains visible and shows an appropriate message.
6. Disabled sources are excluded. Category membership comes from the database,
   so future additions do not require hardcoded source-ID changes.
7. Verify production article availability, then verify actual rendered desktop
   and mobile cards after frontend deployment. Report ingestion problems
   separately from frontend rendering problems.

The repeatable backend registration script is
[`sql/top_headlines_seed.sql`](../sql/top_headlines_seed.sql). It has already been
applied to production; frontend implementation does not require running it again.
