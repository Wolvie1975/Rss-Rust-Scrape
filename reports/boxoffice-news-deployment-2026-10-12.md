# Box-office news deployment — 2026-10-12

User selected free Deadline/Variety news coverage, including Sunday estimates and
Monday/Tuesday follow-ups. Deployed as configuration in the existing news pipeline;
no binary, schema, permission, container or web-app change was necessary.

- Category: Box Office (SourceCategories ID 5615).
- Deadline source ID 7094: https://deadline.com/v/box-office/feed/.
- Variety source ID 7095: https://variety.com/v/film/box-office/feed/.
- Both enabled, labeled and categorized; both LastError fields NULL.
- Initial import: 20 articles, ten per provider; two source successes, zero failures.
- Repeat import: 20 articles; IDs and original publication dates preserved,
  zero duplicate additions and zero old-page pruning.
- Latest ScraperRuns record succeeded with no error summary.
- Scraper and My-Newsfeed containers healthy; runtime health check passes;
  website root returns HTTP 200.
- Existing hourly --from-db collection discovers enabled sources dynamically and
  runs seven days a week. Sunday, Monday and Tuesday require no special scheduler.

These are news observations, not structured domestic/worldwide box-office totals.
No monetary fields or Estimate/Actual flags were inferred from headlines or weekdays.
Publisher RSS may expose revisions of existing stories; an update absent from the
RSS cannot be guaranteed to appear. Related box-office analysis is included.

Configuration: sql/boxoffice_news_seed.sql registers disabled feeds and preserves
operator settings on repeat setup. Explicit enabling was performed for this deploy.
Frontend/display guide: docs/my-newsfeed-boxoffice-news.md. A site that hardcodes
categories must add Box Office separately; no web-app rebuild was performed here.

Verified COPY_ONLY SQL backup with CHECKSUM, permissions 600:
/var/opt/mssql/backup/WebScraper_before_boxoffice_news_20261012.bak.
Rollback is configuration-only: disable source IDs 7094/7095. Leave imported rows
for history unless cleanup is separately requested. SQL Server was not restarted;
credentials, verified TLS and existing sports/movie sources remain unchanged.

No commits or pushes performed.
