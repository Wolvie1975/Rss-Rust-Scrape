# EFL Championship / Wrexham deployment — October 13, 2026

Completed approximately 4:01 p.m. Central.

Image web-scraper:championship-20261013 (also latest and championship-20261013-r2).
Image ID sha256:8d58371231a536982e9bea7cfc176cb15c12d0f841f12669f4ea5cbf279b29bb.

Sources enabled:

- Schedule/results SportsEventsType ID 5326, ProviderKey espn-championship.
- Standings SportsStandingsSources ID 11, same ProviderKey, minimum 24 teams.
- ESPN competition eng.2 / league 3914; Wrexham team ID 352.

Production verification: targeted import exited 0. Both source LastError fields
are NULL with fresh success timestamps. Current 2026–27 data contains 437 future
fixtures, 24 standings teams, and 115 unique completed match results. Wrexham has
36 future fixtures and 10 completed results. Counts can change with live matches.

The additive SportsMatchResults table stores final home/away goals separately
from SportsEvents, so past schedule cleanup does not delete current-season scores.
Prior-season results are removed only after a valid new-season result import.
No writes to the separate application's ResultGames or ResultFollows tables.
Current hourly events/standings commands discover enabled sources automatically.

Schedule identity uses espn:eng.2:<gameId>. Known kickoffs retain UTC and Central
display dates; TBD midnight-London placeholders preserve their source dates.
Final goal values are validated; non-final/abandoned statuses are not results.
An initial production scoreboard fetch was rejected with 403. The collector now
uses the same My-Newsfeed/1.0 request header as existing ESPN schedules; a matching
production-client preview and the subsequent production import succeeded.

Validation: 99 binary tests passed; 15 integration tests skipped in the default
suite. The isolated Championship SQL test passed for repeated imports, score
corrections, preserved IDs/FirstSeenAt and disabled-source rollback. The temporary
test database was removed. Clippy with warnings denied and formatting checks pass.

The existing standings selector supports sourceId=11. The Wrexham controls and
results view need separate frontend implementation. Contract and queries:
[my-newsfeed-championship-wrexham.md](../docs/my-newsfeed-championship-wrexham.md).
No web-app rebuild was performed.

Verified COPY_ONLY/CHECKSUM/compressed database backup:
/var/opt/mssql/backup/WebScraper_before_championship_20261013.bak.
Rollback image: web-scraper:before-championship-20261013.
Original rollback container: web-scraper-rollback-championship-20261013 (stopped).
Host source backup: .deployment-backups/20261013-championship/source.tar.
Disable both new sources before running a binary without Championship support.
SQL Server was not restarted. No commits or pushes performed.
