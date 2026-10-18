# Frontend integration: F1 News

Four F1-specific RSS news sources are enabled in the production `WebScraper`
database under **F1 News**. Add a **News** tab to the F1 sports section alongside
Schedule, Results and Standings. This tab uses the existing news article system;
it does not read Jolpica race/standings views.

## Production sources

| Source ID | Publisher | RSS URL |
| --- | --- | --- |
| 8965 | BBC Sport — F1 | https://feeds.bbci.co.uk/sport/formula1/rss.xml |
| 8966 | Autosport — F1 | https://www.autosport.com/rss/f1/news/ |
| 8967 | Motorsport.com — F1 | https://www.motorsport.com/rss/f1/news/ |
| 8968 | The Guardian — F1 | https://www.theguardian.com/sport/formulaone/rss |

Resolve category membership through `SourceCategories.Category_Name = N'F1 News'`,
not a hardcoded source-ID list. The existing general BBC/Guardian sources and
Top Headlines assignments are preserved. The news feeds are independent from
singleton ID 1 in `F1Source`, which configures Jolpica sports data.

## Query and rendering

Use the existing server-side news repository/API, article DTOs, cards, pagination,
source selector and saved-article behavior. Bind category, source and pagination
parameters using existing application conventions.

```sql
SELECT p.ID AS ArticleId,p.Title,p.Description,p.Url AS ArticleUrl,p.ImageUrl,
       p.Published,p.ScrapedAt,s.ID AS SourceId,s.Label AS SourceName,
       c.ID AS CategoryId,c.Category_Name AS CategoryName
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID=p.SourceId
JOIN dbo.SourceCategories c ON c.ID=s.SourceCategoryId
WHERE s.Enabled=1 AND c.Category_Name=@CategoryName -- N'F1 News'
  AND (@SourceId IS NULL OR s.ID=@SourceId)
ORDER BY COALESCE(p.Published,p.ScrapedAt) DESC,p.ID DESC
OFFSET @Offset ROWS FETCH NEXT @PageSize ROWS ONLY;
```

Validate pagination bounds and retain the site's established date-window filters
consistently for listing and counts. Populate source options from enabled sources
in the category, including sources with no retained articles. Use `Pages.ID` for
saved article identity and `Pages.Url` as the destination, not the RSS URL.
Retain the existing `ArticleIdRedirects` recovery behavior for saved items.

Display publisher attribution, title, available summary/image, publication time
and a link to the original article. Handle missing summaries/images/dates with
the site's established fallbacks. Use existing timezone formatting, HTML
encoding/sanitization and outbound URL checks. Full article text, live timing,
and publisher paywall access are not supplied by these RSS feeds.

No browser-to-publisher requests, new credentials, frontend database writes, or
new scraper HTTP endpoints are required. Use the frontend's existing read access
to `Pages`, `Sources` and `SourceCategories`. Show a normal empty state for no
articles and the standard error state for query failures.

## Collection and acceptance

The deployed `web-scraper:f1-20261018` image already supports these feeds through
its hourly `--from-db` collection. No rebuild or restart is required for source
registration. Production keeps up to ten articles per source under its existing
retention policy; identical article URLs already owned by another source can be
skipped by existing cross-source identity protection.

Verify that the F1 News tab lists collected articles, publisher filters work,
article destinations and save behavior remain correct, disabled sources are
excluded, and empty/image-missing states work on desktop and mobile. Keep the
existing schedule/results/standings tabs and other news categories intact.
Frontend rendering is a separate implementation step.

Registration: [`sql/f1_news_seed.sql`](../sql/f1_news_seed.sql).
Deployment verification: [F1 operations](f1-operations.md#f1-news-registration).
Structured sports data: [F1 frontend integration](f1-frontend-integration.md).
