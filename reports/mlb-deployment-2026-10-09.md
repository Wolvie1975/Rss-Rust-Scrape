# MLB scraper deployment — 2026-10-09

MLB schedule importing is deployed and enabled in production.

- Built the workspace MLB changes on base commit `0230d69`; these changes are
  not committed by this deployment. Synced source to the host checkout at
  `/home/jcarpio/Projects/Rss-Rust-Scrape` without overwriting .env/Git metadata.
- Image: `web-scraper:latest`, also tagged `web-scraper:mlb-20261009`.
  Digest: `sha256:257a666459d34838a8431d61f9db2efbf5c5857854afabe58f7b67ca1111bcbd`.
- Recreated `web-scraper`. Verified it is running the new image with its unchanged
  hourly --events-from-db command, existing news/YouTube/movie/TV features,
  persistent RSS volume and unless-stopped restart policy.
- Ran --configure-mlb from the new image against WebScraper, then enabled feed
  **4193**: ProviderKey **espn-mlb**, ProviderSchoolId **0**. Its URL is league-wide
  with no team or fixed-season restriction. No additional schema was required.
- New-image preview, production import and repeat import returned **2,916 MLB
  fixtures**: **22 remaining 2026 games** and **2,894 announced 2027 fixtures**.
  All dates are today-forward in America/Chicago; all observations were scheduled.
- Database now contains **3,875 public sports events**, **19 feeds**, and **1,444
  school source observations**. All **959 preexisting event IDs, FirstSeenAt,
  URLs, GameIds and canonical keys** preserved, including WNBA/Current.
  Repeat import preserved all MLB identities/FirstSeenAt; zero canonical duplicates.
- 91 regular all-target tests and the separate rollback-only MLB staging SQL test
  passed before deployment. Live imports completed successfully.
- My-Newsfeed /events returned HTTP 200 and rendered MLB game links. It still has
  no dedicated MLB selector option; that UI/provider-selector change is separate.
  The web app was inspected read-only, not rebuilt or restarted.

Rollback image: **web-scraper:before-mlb-20261009**. Host backup folder:
`.deployment-backups/20261009-mlb/`, containing source.tar and before/first/after
sports snapshots. Disable only the MLB feed before running that older scraper,
which supports WNBA/Current but does not understand MLB. Retain imported events
and the shared nullable ScheduleStatus column; rollback needs no row deletion.

Temporary deployment helper and temporary SQL enable example removed after use.
Production credentials were neither printed nor committed. No Git commit/push.

## User-requested season correction

Following the user's clarification, feed 4193 was restricted to `season=2026`.
Backed up and deleted only the 2,894 MLB rows dated 2027 or later. Reimport from
the configured URL returned 22 upcoming 2026 games. Verified database totals:
981 sports events (959 preexisting plus 22 MLB), zero 2027 MLB games. All other
sports were retained. Backup: `mlb-2027-before-removal.json` in the deployment
backup folder. Hourly collection honors the saved URL; no rebuild/restart needed.
