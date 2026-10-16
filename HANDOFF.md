# Scraper handoff

## Documentation

Start with [README.md](README.md). Five topic guides replace the overlapping individual handoffs; three history files preserve dated deployment evidence. SQL examples, validation evidence and the previous long handoff remain available there.

## Latest documented production state

Last verified deployment: `web-scraper:movie-direct-links-20261016`; scraper and front-end were healthy at verification. This document is not a live status check.

- Hourly collection includes news, rankings, standings, sports schedules, YouTube, movie releases, TV, actual box office and Boxoffice Pro forecasts.
- MLS schedule/results source 5525, standings 9; Championship schedule/results 5326, standings 11; EPL schedule 5357, standings 12; NFL standings 10.
- MLS and Championship current-season results are independent of schedule pruning; playoff shootouts are stored separately from goals.
- Latest actual box-office chart and domestic forecasts have poster URLs. Last checked: 10/10 chart films, 12/12 forecast films and 24/34 release films; unmatched films use placeholders.
- Dedicated Deadline/Variety box-office story sources are disabled. Structured charts and forecasts replace them.
- Current retention and backup rules: [operations guide](docs/operations.md#data-retention). SQL remains in FULL recovery with managed log/full backups.
- Initial performance changes deployed: ESPN batches of 100, stage/HTTP metrics, and filtering past movie releases before persistence. Further caching/concurrency work remains a candidate.
- Front-end work for the newer movie/sports sections is documented in the topic guides; scraper deployment alone does not implement those views.

## Article URL identity — October 16, 2026

Verified URL redirects/canonicals now reconcile same-source article URL changes
through `ArticleUrlAliases` and `UpsertArticleIdentity`. The additive migration
also creates `ArticleIdRedirects` and grants runtime permissions. No title/image/
timestamp similarity merges are allowed; runtime source settings and hourly
frequency are preserved.

The approved Pure Xbox cleanup keeps **25604**, archives/maps **25647 → 25604**,
and retains both URL aliases. The frontend showed one public card. Browser saves
using retired IDs need frontend recovery through that mapping; this release does
not change browser storage. A verified copy-only database backup and protected
host source/configuration backups are retained.

Read [article URL identity](docs/article-url-identity.md) for request budgets,
migration order, metadata/reference handling and saved-ID recovery. Actual schema,
tests, cleanup scope and rollback: [deployment evidence](reports/operations-history.md#article-identity-deployment-2026-10-16).

## Big 12 standings review — October 15, 2026

Read-only verification found soccer source **7** (2026, 16 teams) and men’s
basketball source **5** (2025-26, 16 teams) match the official tables in full.
The soccer app’s conference columns were correct; it omitted stored overall
and split statistics. The user plans frontend changes separately. No scraper or
production changes were needed. Full source-specific `StatsJson` mappings:
[standings guide](docs/sports-tables.md#big12-soccer-basketball-columns).
Recorded refresh times and comparison evidence:
[sports history](reports/sports-history.md#big12-standings-review-2026-10-15).

## NHL production deployment — October 14, 2026

Image `web-scraper:nhl-20261014` is deployed and healthy. NHL schedule/results
source **5637** and division standings source **13** are enabled. Verified 1,239
fixtures, 170 final results and 32 current standings teams for ESPN season 2027
(2026–27), with stable identities across repeated imports. The existing hourly
scheduler and runtime settings are preserved. See the consolidated [schedules/results](docs/sports-schedules.md#nhl-schedule-results),
[standings](docs/sports-tables.md#nhl-standings), [operations](docs/operations.md#nhl-operations)
and [deployment history](reports/sports-history.md#nhl-deployment-2026-10-14) for contracts,
validation and rollback.

## Movie purchase links — deployed October 16, 2026

Current image `web-scraper:movie-direct-links-20261016` adds expiring reviewed
Fandango mappings and improved candidate selection. Production run 165 saved 34
releases with zero source issues. Eight of nine theatrical actions now use direct
Fandango movie pages; Sacrifice and four digital actions use search fallbacks.
The live `/Movies` page rendered eight direct buttons, including the exact Street
Fighter detail URL, without a frontend image replacement. Existing movie data,
hourly configuration and the article-ID recovery mapping are preserved.

Read [cache and operations](docs/movie-purchase-links.md), [the standalone frontend
integration request](docs/movie-commerce-frontend-integration.md) and [direct-link
deployment evidence](reports/operations-history.md#movie-direct-links-deployment-2026-10-16).
The existing frontend is now confirmed to consume the view and render direct links.

## Development and deployment

The validated [schema-only database snapshot](sql/schema/WebScraper.schema.sql) captures 31 tables and one view, without data or credentials. See the [operations guide](docs/operations.md#database-schema-snapshot) for scope and empty-database restore instructions.

Use runtime credentials for ingestion and migration credentials for explicit setup. Never print or commit credentials. Preserve existing environment, network, scheduler, retention and rollback settings during deployments. Use appropriate tests and verify the deployed image and targeted live imports. The user commits and pushes Git changes themselves unless they explicitly request otherwise.

Historical images, table counts, earlier open items and source audits: [operations history](reports/operations-history.md#previous-handoff), [sports history](reports/sports-history.md), [entertainment history](reports/entertainment-history.md).

## Working conventions with this user

- Confirm before anything outward-facing; the user pushes to GitHub, and restarts/pauses the scheduler
  on their own call.
- Prefer verifying against the live DB and live feeds (dry-run first, then a real run) and report
  exactly what was and wasn't checked.
