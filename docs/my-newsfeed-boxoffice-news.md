# My-Newsfeed box-office news

The user chose free Deadline/Variety **news feeds**, including Sunday estimates
and Monday/Tuesday follow-ups. This uses the existing Sources/SourceCategories/
Pages news pipeline, not structured box-office tables or a licensed API.

Feeds:

- https://deadline.com/v/box-office/feed/
- https://variety.com/v/film/box-office/feed/

Both were verified as active RSS on 2026-10-12. Deadline's /tag/box-office/feed/
was sparsely updated, while the selected /v/box-office/feed/ is current. Avoid
Box Office Mojo's old news feed (latest entries January 2023) and The Numbers'
removed /rss-feed endpoint.

## Display

Expose the **Box Office** category through the existing news category filter.
Show headlines, bounded excerpts, images when supplied, publication dates in
Central time, provider labels and links to the original articles. Use ordinary
HTML encoding/sanitization and the site's established safe link/image behavior.

Coverage includes domestic, overseas-market and worldwide reporting, plus related
box-office analysis. The publishers choose their article dates and can update an
existing article. Do not claim every article contains both domestic and worldwide
figures or that every Sunday/Monday/Tuesday guarantees a new story.

These are news articles. Do not infer an Estimate/Actual status from weekday,
create dollar totals by parsing headlines, or combine weekend/cumulative figures.
If adding a status badge later, require explicit verified article evidence; a
Sunday article can already contain some actual figures and a Monday one can
still use estimates. Publisher attribution remains visible.

## Collection and configuration

Existing hourly --from-db collection runs all seven days, including Sundays,
Mondays and Tuesdays; no extra schedule flag or container restart is required.
The deployed command caps each source at its ten newest articles per collection.
Existing URL-based upserts update repeated articles rather than duplicating them.
Published dates retain the original earliest known publication time; LastScrapedAt
means last checked, not necessarily when a publisher revised an article.
An article correction must be present in the publisher's RSS to be observed.

`sql/boxoffice_news_seed.sql` creates the Box Office category if missing and
registers the two feeds **disabled**, preserving existing settings on repeat setup.
Apply with migration/admin credentials, then enable explicitly:

```sql
UPDATE dbo.Sources SET Enabled=1
WHERE Url IN (
 'https://deadline.com/v/box-office/feed/',
 'https://variety.com/v/film/box-office/feed/'
);
```

No schema, credentials, runtime grants, sports/movie imports or web-app rebuild
are required for ingestion. A database-driven news selector can expose the new
category automatically; frontend changes are separate if it hardcodes categories.
Read-only preview:

```bash
web_scraper --dry-run --per-source 10 https://deadline.com/v/box-office/feed/ https://variety.com/v/film/box-office/feed/ -o /tmp/boxoffice-preview.xml
```

Import with runtime credentials using the same URLs and --db, omitting --dry-run.
Use a temporary output file for targeted imports so the main combined RSS remains
intact. The normal hourly run updates the combined feed with all enabled sources.

## Verification

Check both source records are enabled, categorized as Box Office, and have NULL
LastError after collection. Confirm publication dates, provider URLs, excerpts and
repeat-import identity with the existing news importer. No historical/news rows
should be deleted as part of this configuration change.

## Frontend build handoff

### Request to give the My-Newsfeed builder

“Integrate the deployed Box Office news category into the existing news page and
category selector. Reuse the site's data-access, article-card, authorization and
pagination patterns. Read existing WebScraper news tables; do not fetch publisher
RSS from the browser. Display Deadline/Variety headlines, excerpts, publication
dates, attribution and article links. Preserve category/source selection in URLs,
handle unavailable images and refresh failures, and do not present the articles
as structured box-office totals. Follow the queries and acceptance checks below.”

### Production status and identity

Deployed 2026-10-12: category ID 5615, Deadline source ID 7094, Variety source ID
7095. Initial collection imported 20 articles, with stable IDs on repeat import.
These IDs identify this deployment; discover categories/sources from the database
rather than hardcoding those numbers in application logic.

The frontend needs SELECT on dbo.SourceCategories, dbo.Sources and dbo.Pages,
using its existing server-side database connection. No new frontend table or movie
model is needed. Credentials stay on the server; no ingestion/admin privileges
are required for rendering.

### Category and source selectors

Populate news categories from enabled sources, so a newly registered category
appears without a hardcoded category enum:

```sql
SELECT c.ID, c.Category_Name
FROM dbo.SourceCategories c
WHERE EXISTS (
 SELECT 1 FROM dbo.Sources s WHERE s.SourceCategoryId=c.ID AND s.Enabled=1
)
ORDER BY c.Category_Name, c.ID;
```

For an optional provider selector, retain enabled sources even when no article
has been collected yet:

```sql
SELECT ID, Label, Url, LastScrapedAt, LastError
FROM dbo.Sources
WHERE Enabled=1 AND SourceCategoryId=@CategoryId
ORDER BY Label, ID;
```

Reuse existing category/source query parameters. Validate a selected source belongs
to the selected category and is enabled. Resolve Box Office by its category name
for a dedicated navigation link if needed; use the resolved ID for parameterized
queries. Do not accept disabled source IDs to reveal their articles.

### Article query

Use the site's existing page size, with a bounded positive @PageSize and nonnegative
@Offset. Bind values as SQL parameters, including the nullable @SourceId:

```sql
SELECT p.ID AS ArticleId, p.Title, p.Description, p.Url AS ArticleUrl,
       p.ImageUrl, p.Published, p.ScrapedAt,
       s.ID AS SourceId, s.Label AS SourceLabel, s.Url AS FeedUrl,
       s.LastScrapedAt, s.LastError,
       c.ID AS CategoryId, c.Category_Name AS CategoryName
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID=p.SourceId
JOIN dbo.SourceCategories c ON c.ID=s.SourceCategoryId
WHERE s.Enabled=1
  AND c.ID=@CategoryId
  AND (@SourceId IS NULL OR s.ID=@SourceId)
ORDER BY COALESCE(p.Published,p.ScrapedAt) DESC, p.ID DESC
OFFSET @Offset ROWS FETCH NEXT @PageSize ROWS ONLY;
```

Use the same filters for a separate COUNT query if the existing paginator needs a
total. Do not join to Movies or derive money values from movie-release records.
If adding date filters, compute Central calendar boundaries, convert them to UTC
on the server, and use an inclusive lower/exclusive upper timestamp bound. No
Sunday-only, Monday-only, current-week or sports today-forward restriction is
needed for the default news category.

Map ArticleId/SourceId/CategoryId to .NET int; Published and LastScrapedAt to
DateTime?; Description/ImageUrl/SourceLabel/LastError are nullable strings.
ScrapedAt is nonnullable, but represents initial storage time and must not be
advertised as an article revision timestamp.

### Article card and timestamps

Reuse the existing news card. Show headline, short excerpt when available, image
when available, provider label, publication date and “Read at Deadline/Variety”.
Link to ArticleUrl, not FeedUrl. If SourceLabel is absent, show the validated
article/provider hostname rather than an empty attribution.

All stored DATETIME2 values represent UTC. Explicitly mark them UTC in .NET before
using the site's Central-time conversion; handle CST/CDT using the timezone helper.
Published is the publisher's earliest known article publication, not the scraper's
refresh time. If NULL, display “Publication date unavailable”; ScrapedAt may be
shown separately as “First collected”, not substituted as a claimed publication.

The collector already bounds excerpts. Render Description as encoded text unless
the existing component safely sanitizes it; do not use raw HTML rendering on feed
content. Only accept HTTP(S) article/image URLs. Missing/failed images should
leave a readable text card. Follow existing external-link conventions and use
noopener noreferrer for links opening a new tab.

### Loading, empty and refresh states

| Condition | Display behavior |
| --- | --- |
| Articles available | Render newest first with stable ArticleId keys. |
| Enabled sources, no articles | “No box-office stories available yet.” Keep selectors usable. |
| No enabled sources | “No box-office news sources are enabled.” |
| LastError set for a source | Keep previously stored articles; show a generic refresh message. |
| LastScrapedAt older than two hours | Indicate collection may be delayed; do not delete articles. |
| Database request fails | Show a retryable availability message; log details server-side. |
| Client request loading, if applicable | Use a loading state rather than briefly showing an empty list. |

LastScrapedAt records an attempt; it is not a separate last-success field. Combine
it with LastError when determining collection health. Public pages must not expose
raw LastError, connection details or SQL exceptions. Lack of a new Sunday or
Monday story alone is not a collection failure: publishers control their coverage.

Keep filters, page selection and links keyboard-accessible. Use semantic headlines
and source/date text, responsive cards, and consistent image dimensions. Any page
cache should include category/source/page in its key and be short relative to the
hourly collector (for example five minutes).

### Frontend acceptance checks

- Box Office is selectable through the existing news category filter.
- Both enabled publishers appear, with optional provider filtering and URL state.
- The deployed category initially has 20 article records; do not hardcode that count.
- Article links open publisher stories, not RSS endpoints.
- Cards retain attribution, correct Central publication dates and readable text
  when descriptions/images/publication dates are missing.
- Repeated ingestion updates existing ArticleId cards rather than duplicating them.
- Sunday estimates and Monday/Tuesday follow-ups remain ordinary attributed news;
  no weekday-derived status badges or headline-derived dollar totals appear.
- Disabled sources, empty/loading/error/stale states, mobile layout and keyboard
  interaction behave correctly.
- Other news categories, movie releases and sports pages retain their behavior.

Use existing frontend tests for filtering, pagination and card rendering; use
stored/fixture articles rather than live publisher requests. Deployment evidence:
[box-office deployment report](../reports/boxoffice-news-deployment-2026-10-12.md).
