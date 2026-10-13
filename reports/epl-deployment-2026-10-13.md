# Premier League deployment — October 13, 2026

Completed approximately 4:10 p.m. Central.

Image web-scraper:epl-20261013 (also latest).
Image ID sha256:00cc4a3a8d111f98762b23ceac88814ffd15f6fefb9027f24fd1870ccb8d8c67.

Premier League schedule source ID 5357 and standings source ID 12 are enabled.
ProviderKey=espn-epl, competition eng.1 / ESPN league 700, current season 2026–27.
Both sources are discovered by the existing hourly events/standings collectors.

Production targeted import exited 0. Source LastError fields are NULL with fresh
success timestamps. Audit: 320 fixtures, 20 standings teams, zero past EPL
fixtures, zero historical EPL results and zero superseded EPL standings rows.

The collector begins with the current month and stops at current-season June.
Today's final observations can reconcile existing scheduled rows but are never
inserted as historical results. Existing retention removes past/canceled matches.
London TBD placeholders preserve the source calendar date; known UTC instants
use Central display dates. ESPN requests use the existing My-Newsfeed/1.0 header.

EPL standings delete only this provider's non-current rows within the successful
snapshot transaction. Other leagues' history policies are unchanged. DELETE on
SportsStandings was added to the runtime role for this operation; configuration
editing and DDL remain unavailable. No schema changes or SQL Server restart.

Validation: 100 binary tests pass; 15 integration tests skipped in the default
suite. The isolated standings SQL test passed for failure rollback, stable
repeat imports, EPL rollover leaving only 20 current rows, preservation of the
other league's historical rows, and runtime permissions. The temporary test
database was removed. Clippy with warnings denied and formatting checks pass.

Frontend: existing /standings?sourceId=12 discovers the table automatically.
Add Premier League to any hardcoded schedule selector and reuse soccer table
columns. Queries and display contract: [my-newsfeed-epl.md](../docs/my-newsfeed-epl.md).
No separate web-app rebuild performed.

Verified COPY_ONLY/CHECKSUM/compressed backup:
/var/opt/mssql/backup/WebScraper_before_epl_20261013.bak.
Rollback image: web-scraper:before-epl-20261013.
Previous container: web-scraper-rollback-epl-20261013 (stopped).
Host source backup: .deployment-backups/20261013-epl/source.tar.
Disable both EPL sources before reverting to a binary without EPL support.
No commits or pushes performed.
