# Production deployment — standings and digital releases

Deployed 2026-10-11, approximately 08:52 Central.

## Deployed

- Scraper image `web-scraper:standings-digital-20261011`, also tagged latest.
- Image ID: `sha256:9bca0f33d513b62609fcc56f3f8c42abbdf3cd94cedbec4880ccbc643c472f97`.
- Additive SportsStandingsSources/SportsStandings schema and restricted runtime grants.
- Eight standings sources registered and enabled; Movie Insider registered/enabled.
- Updated hourly container includes --standings-from-db alongside all existing
  collectors. Movie Insider and separate When To Stream VOD parsing use --movies.
- Frontend/configuration handoff documents synchronized to the deployed scraper checkout.

## Production verification

First production collection succeeded (ScraperRuns ID 11), nine collector successes,
zero failures, zero movie parser issues. All refresh metadata shows no errors.

| Standings source | Current teams | Season |
| --- | ---: | --- |
| MLB | 30 | 2026 |
| WNBA | 15 | 2026 |
| NWSL | 16 | 2026 |
| Big 12 football | 16 | 2026 |
| Big 12 men's basketball | 16 | 2025–26 |
| Big 12 women's basketball | 16 | 2025–26 |
| Big 12 soccer | 16 | 2026 |
| Big 12 volleyball | 15 | 2026 |

140 current standings rows. Movie import saved 94 release observations; production
now holds 48 movies, 48 source links, 94 release rows and 30 verified posters.
18 movies still have no matched poster; the frontend must show placeholders.
All 76 pre-deployment movie release IDs and FirstSeenAt timestamps were preserved.

For the current October 5–11 week: Movie Insider has 10 digital observations,
When To Stream has 5, DVDs Release Dates has 16. Providers overlap; these counts
are not unique movie totals. The frontend handoff recommends explicit source
selection with Movie Insider preferred for the Digital/VOD view.

All 19 schedule feed configurations remain unchanged. SportsEvents remains 981
rows, zero duplicate canonical keys and zero 2027 MLB games. The 2026 MLB
restriction and separate My-Newsfeed NWSL results sync remain intact.

Runtime health check passes; scraper and My-Newsfeed containers are healthy.
/events and /movies return HTTP 200. Runtime account can insert standings, cannot
CREATE TABLE or DELETE standings, and retains verified SQL TLS.
SQL Server was not restarted for this deployment.

Pre-deploy verification: 113 offline tests, Clippy with warnings denied, formatting
checks and isolated digital SQL validation passed; standings SQL/live validation
had already passed before deployment.

## Frontend boundary

Only scraper implementation, database/configuration and handoff documents changed
in this repository. The My-Newsfeed web application was not rebuilt. Its new
/standings route still returns 404. The existing /movies page remains available;
preferred digital source selection is a separate frontend change. Data is ready;
the frontend builder still needs to apply:

- docs/my-newsfeed-standings-frontend.md
- docs/my-newsfeed-digital-releases.md

## Backup and rollback

Verified COPY_ONLY SQL backup with CHECKSUM:
`/var/opt/mssql/backup/WebScraper_before_standings_digital_20261011.bak` in sql2025's
persistent SQL storage (600 permissions).

Protected host source/configuration backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261011-standings-digital/`.
Includes source-before.tar and compose-before.yml. Runtime credentials and CA mount
were preserved during source synchronization.

Rollback image: `web-scraper:before-today-standings-digital-20261011`.
Restore the prior compose command together with that image: the previous binary
must not receive --standings-from-db. Additive tables can remain during a binary
rollback; restore the database only for a separately justified data recovery.

No commits or pushes performed.
