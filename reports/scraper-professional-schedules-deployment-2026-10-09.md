# Professional schedule scraper deployment — 2026-10-09

The updated scraper is deployed to production. Kansas City Current NWSL and
league-wide WNBA feeds are enabled and imported into the shared WebScraper DB.

## Runtime

- Source commit: `0230d69` (added two women's sports to schedule page).
- Host checkout: `/home/jcarpio/Projects/Rss-Rust-Scrape`; workspace source synced
  before the Compose image build, without overwriting `.env` or Git metadata.
- Image: `web-scraper:latest`, also tagged `web-scraper:pro-schedules-0230d69`.
- Image digest: `sha256:bd3b2344911aefbb9ff4eaa0793241c58af536276f9d4f246329b1e1379d0457`.
- `web-scraper` recreated and running with `unless-stopped`, zero restart failures.
  Existing hourly command preserved, including `--events-from-db`, news, YouTube,
  movies and TV. Hourly slots remain aligned to America/Chicago.
- Both configure commands ran from the new image against production. Verified
  `SportsEvents.ScheduleStatus` is nullable `NVARCHAR(100)`.
- Feed ID **4191**: `espn-nwsl`, ProviderSchoolId **20907**, enabled, Kansas City
  team parameter retained, no fixed season restriction.
- Feed ID **4192**: `espn-wnba`, ProviderSchoolId **0**, enabled, league-wide URL,
  no team or fixed season restriction.

## Validation

- 88 regular all-target tests passed; both NWSL/WNBA rollback-only staging SQL
  integration tests passed separately. Release binary and Docker image built.
- New-image production dry-run returned **3 Kansas City** and **13 WNBA** upcoming
  fixtures. Production import and repeat import ran inside the deployed container.
- Database now has **959 public events**, **18 feeds**, and **1,444 existing school
  source observations**. Both professional feeds have successful imports, expected
  event counts and NULL LastError.
- All **943 original event IDs, URLs, GameIds, canonical keys and FirstSeenAt values**
  preserved. All 16 new event identities/FirstSeenAt preserved on repeat import;
  zero duplicate canonical keys. No historical games deleted or results sync changed.
- Every imported professional game is scheduled and dated Oct 9 or later Central.
  WNBA TBD dates Oct 11/14 retain ESPN's advertised date. Conditional playoff
  games with TBD opponents remain provider-supplied future fixtures.
- Live My-Newsfeed `/events` returned HTTP 200 and rendered the new team names.
  Its current selector still says “All schools” and lacks a WNBA option; the web
  application's new Schedule selector needs its own deployment. My-Newsfeed was
  inspected read-only and not rebuilt/restarted by this scraper deployment.

## Rollback and evidence

- Previous image: `web-scraper:before-pro-schedules-20261009`.
- Host backup folder: `.deployment-backups/20261009-professional-schedules/`:
  `source.tar`, `sports-before.json`, `sports-first-import.json`, `sports-after.json`.
- Application rollback can use the previous image. Disable the two ESPN feeds
  before running that older scraper, which does not understand these JSON feeds.
  Leave the additive nullable column and event rows in place; no database deletion
  is required. Source backup excludes credentials and Git metadata.
- Production `.env`, persistent RSS volume and existing service command retained.
  Temporary deployment helper and temporary SQL example removed after verification.
