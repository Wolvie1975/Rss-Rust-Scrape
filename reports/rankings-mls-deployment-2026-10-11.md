# Production deployment — national rankings and MLS

Completed 2026-10-11, approximately 09:36 Central.

- Image: web-scraper:rankings-mls-20261011 (also latest).
- Image ID: sha256:74ace0f94b42db58bf766bf095206c3e12d34525b56347ea9036ce2af0274fc2.
- Additive ranking schema and restricted grants installed; five poll sources enabled.
- MLS source registered/enabled through the existing standings configuration.
- Hourly command includes --rankings-from-db and --standings-from-db alongside
  the existing news, schedules, movies, TV and YouTube collectors.

First targeted production import: **14 source successes, zero failures**.
Five national polls each contain 25 teams (125 entries); 23 entries have current
Big 12 markers. Basketball remains correctly labeled 2025–26 with April 2026 poll
dates. Football AP is dated October 4, soccer October 6, volleyball October 5.

MLS: 30 teams, 15 Eastern / 15 Western; standings now total 170 current rows across
nine sources. Source error fields are NULL. The latest ScraperRuns record is
succeeded with no error summary. Schedules remain 981 rows and MLB has zero 2027
rows. Separate NWSL results and previously deployed movie imports were untouched.

Scraper and My-Newsfeed containers are healthy. Runtime health check passes;
/events, /movies, /standings and /standings?sourceId=9 return HTTP 200. The existing
standings frontend exposes MLS and displays both conferences, including Inter
Miami and Sporting Kansas City. The web app was not rebuilt by this deployment;
its /rankings route remains 404 and the separate frontend builder must implement
that page using docs/my-newsfeed-top25-rankings.md.

Runtime account still lacks CREATE TABLE and DELETE-ranking privileges. Verified
SQL TLS and runtime credential isolation are preserved; SQL Server was not restarted.

Validation before deployment: formatting, Clippy with warnings denied, 94 binary
offline tests pass; the full suite previously passed 119 offline tests. Rankings
and nine-source standings isolated SQL/live tests also passed.

Verified COPY_ONLY backup with CHECKSUM, permissions 600:
/var/opt/mssql/backup/WebScraper_before_rankings_mls_20261011.bak.

Protected source/compose backup on the Docker host:
/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261011-rankings-mls/.
Rollback image: web-scraper:before-rankings-mls-20261011. Restore the previous compose
command with that image (old binary must not receive --rankings-from-db); disable
espn-mls when reverting to a binary that lacks MLS support. Additive ranking tables
can remain during binary rollback. No commits or pushes performed.
