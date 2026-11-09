# Frontend integration: Big 12 News

Add a News section to the Big 12-focused college sports hub using the production
**Big 12 News** category. This is the existing news article system, separate
from structured Big 12 schedules, results, rankings and standings.

## Sources and scope

Five sources are selected: the official Big 12 all-sports feed, Heartland College
Sports, and official football, men's basketball and baseball feeds. The dedicated
women's basketball feed is omitted at the user's request. The all-sports feed
can still include women's sports; omitting a dedicated source does not filter
its stories from conference-wide coverage.

The official all-sports and sport-specific feeds overlap. Existing global URL
ownership prevents saving an identical article URL under multiple sources;
category-wide news remains available from the owning source. A publisher's feed
may contain ten entries while that configured source owns fewer than ten saved
articles. Do not move article ownership or weaken existing identity rules to
inflate source counts. Discover source membership dynamically by category.

## Production source reference

| ID | Source | RSS URL |
| --- | --- | --- |
| 8997 | Big 12 Conference — All Sports | https://big12sports.com/rss.aspx |
| 8998 | Heartland College Sports | https://www.heartlandcollegesports.com/feed/ |
| 8999 | Big 12 Conference — Football | https://big12sports.com/rss.aspx?path=football |
| 9000 | Big 12 Conference — Men's Basketball | https://big12sports.com/rss.aspx?path=mbball |
| 9001 | Big 12 Conference — Baseball | https://big12sports.com/rss.aspx?path=baseball |

Resolve the category ID from its name rather than hardcoding it. These are news
Sources IDs, not SportsEventsType, rankings or standings IDs.

## Frontend query

Use the site's existing news repository/API, article cards, publisher attribution,
source filter, pagination and saved-article behavior.

```sql
SELECT p.ID AS ArticleId,p.Title,p.Description,p.Url AS ArticleUrl,p.ImageUrl,
       p.Published,p.ScrapedAt,s.ID AS SourceId,s.Label AS SourceName,
       c.ID AS CategoryId,c.Category_Name AS CategoryName
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID=p.SourceId
JOIN dbo.SourceCategories c ON c.ID=s.SourceCategoryId
WHERE s.Enabled=1 AND c.Category_Name=@CategoryName -- N'Big 12 News'
  AND (@SourceId IS NULL OR s.ID=@SourceId)
ORDER BY COALESCE(p.Published,p.ScrapedAt) DESC,p.ID DESC
OFFSET @Offset ROWS FETCH NEXT @PageSize ROWS ONLY;
```

Bind parameters, validate pagination bounds, and apply existing date-window
filters consistently to listings and counts. Populate source options from enabled
Sources in this category even when a source owns no retained article rows.

Link articles with `Pages.Url`, not the RSS URL; preserve `Pages.ID` and existing
retired-ID recovery for saved articles. Render available summaries and images
with existing escaping/sanitization and missing-data fallbacks. Format UTC dates
using the site's existing display-time helper. Use the existing frontend database
reader; no browser database credentials, direct publisher fetches, or database
writes are required. A query error must not appear as an empty news feed.

## Operation and acceptance

The production collector already discovers enabled Sources through hourly
`--from-db` collection. Registration requires no image rebuild or restart. The
existing retention limit is ten articles per source. Official announcements and
independent reporting are complementary; sport feeds are not guaranteed to
publish during the offseason.

Verify category-wide and source-filtered results, chronological ordering, valid
article destinations, saved articles, pagination, mobile cards, and empty states.
Keep structured sports data and other news categories unchanged. This document
does not assert that the frontend News section has been implemented.

Repeatable registration: [`sql/big12_news_seed.sql`](../sql/big12_news_seed.sql).
Structured sports: [schedules/results](sports-schedules.md) and
[standings/rankings](sports-tables.md).

## Production verification — October 18, 2026

The running production image passed a five-source dry-run, then imported
**46 articles** with five source successes and zero failures. Four overlapping
article URLs were skipped by existing cross-source identity protection. Read-back
confirmed all five sources enabled, updated LastScrapedAt and no LastError:
all-sports 10, Heartland 10, football 9, men's basketball 7, baseball 10.
Both production containers were healthy. These counts are dated observations.
No rebuild or scheduler restart was needed. The dedicated women's basketball
feed was not registered.
