# Frontend integration: Opinion

## Production status

Verified October 24, 2026 (America/Chicago) against production **WebScraper**.
The **Opinion** category is populated and enabled for the existing hourly
`--from-db` collector. This was a database configuration change using the existing
collector; no application rebuild or restart was required. Frontend navigation
and rendering have not been implemented or verified by this backend task.

Current read-back: **five enabled sources, 50 articles**, all with publication
dates and images, and NULL LastError on all five sources. Counts are a verification
snapshot, not a fixed UI requirement. Production retains up to ten articles per
source, subject to feed size and normal article ownership protection.

## Category and publishers

Add **Opinion** to the existing news category navigation. Reuse the site's news
repository/API, article cards, publisher filter, pagination and saved/read controls.
Use `SourceCategories.Category_Name = N'Opinion'` for discovery. Its current ID
is 6022; resolve membership dynamically instead of hardcoding category/source IDs.

| Coverage | Publisher | Current source ID |
| --- | --- | --- |
| U.S. sports | The Guardian — U.S. Sports Opinion | 9407 |
| Entertainment / film | IndieWire — Film Commentary | 9403 |
| U.S. domestic and world news | The Guardian — U.S. & World Opinion | 9404 |
| World news | Le Monde — World Opinion | 9405 |
| General video games, PC and consoles | Eurogamer — Opinions | 9411 |

This is one combined category. Topic labels in this table describe feed scope;
the database does not provide separate Opinion subcategories or a per-article
topic classification. Avoid deriving topic filters from title keywords.

## Article listing

Run queries server-side through the existing database access layer. Bind
parameters; validate `@Offset >= 0` and bound positive `@PageSize` using existing
application conventions. `@SourceId` is nullable and must remain scoped to this
category. Set `@CategoryName` to `Opinion`.

```sql
SELECT p.ID AS ArticleId,
       p.Title,
       p.Description,
       p.Url AS ArticleUrl,
       p.ImageUrl,
       p.Published,
       p.ScrapedAt,
       s.ID AS SourceId,
       s.Label AS SourceName,
       c.ID AS CategoryId,
       c.Category_Name AS CategoryName
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID = p.SourceId
JOIN dbo.SourceCategories c ON c.ID = s.SourceCategoryId
WHERE s.Enabled = 1
  AND c.Category_Name = @CategoryName
  AND (@SourceId IS NULL OR s.ID = @SourceId)
ORDER BY COALESCE(p.Published, p.ScrapedAt) DESC, p.ID DESC
OFFSET @Offset ROWS FETCH NEXT @PageSize ROWS ONLY;
```

Use the same filters for total counts:

```sql
SELECT COUNT_BIG(*) AS TotalArticles
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID = p.SourceId
JOIN dbo.SourceCategories c ON c.ID = s.SourceCategoryId
WHERE s.Enabled = 1
  AND c.Category_Name = @CategoryName
  AND (@SourceId IS NULL OR s.ID = @SourceId);
```

If the existing UI applies date windows, mute rules or other user filters, apply
them consistently to both listing and count queries. Existing cursor pagination
can use the same effective date and ArticleId ordering instead of OFFSET.

## Publisher selector

Populate options from enabled sources in the category, including publishers with
no currently retained articles. Reset pagination when the publisher changes.

```sql
SELECT s.ID AS SourceId, s.Label AS SourceName
FROM dbo.Sources s
JOIN dbo.SourceCategories c ON c.ID = s.SourceCategoryId
WHERE s.Enabled = 1
  AND c.Category_Name = @CategoryName
ORDER BY s.Label, s.ID;
```

**Always filter `Sources.Enabled = 1`** for category listings, counts and source
options. Retired sources still have stored articles in Opinion: Guardian's former
general sports blog (9402), PC Gamer (9406), Nintendo Life (9408), Push Square
(9409), and Pure Xbox (9410). They must not appear in current Opinion listings.
Preserve direct access to previously saved articles through existing saved-item
behavior; disabling a feed does not delete or reassign its articles.

## Rendering and article identity

- Display an **Opinion** label, publisher attribution, title, publication time,
  available summary/image, and a link to the original article.
- Use `Pages.ID` as the existing article identity for saves/read state. Preserve
  the application's `ArticleIdRedirects` recovery behavior.
- Link to `Pages.Url`, not the source RSS URL. Eurogamer's broad source 4088 is
  still in Video Games; Opinion uses its dedicated opinion feed, source 9411.
- Use the site's established missing-image, missing-summary and missing-date
  fallbacks. Format stored UTC dates using the viewer's configured timezone.
- Apply existing HTML encoding/sanitization and outbound URL checks. Feeds supply
  metadata, not guaranteed full article text or publisher subscription access.
- Show the normal category empty state for zero matching articles and the normal
  error state for failed requests. Keep publisher selection and pagination usable
  on mobile and desktop.

Use existing frontend read access to `Pages`, `Sources` and `SourceCategories`.
No additional credentials, browser-to-publisher fetches or new scraper endpoint
are needed. Collection settings remain backend-managed.

## Acceptance checks

1. Opinion appears in news navigation and lists only enabled Opinion publishers.
2. All five current publishers appear in the selector; selecting each scopes the
   articles correctly. Eurogamer is the only active gaming opinion source.
3. Counts and pagination use the same filters; equal timestamps have deterministic
   ArticleId ordering. Empty filters and publisher changes work correctly.
4. Retired sports/platform feeds are absent from current listings and selectors;
   existing saved articles retain their IDs and established access behavior.
5. Titles open article URLs, images/dates/summaries render with normal fallbacks,
   and existing save/read controls work on desktop and mobile.
6. Other news categories continue using their existing membership and behavior.

Backend registration and verification: [Opinion news](opinion-news.md).
Repeatable registration: [Opinion source seed](../sql/opinion_news_seed.sql).
