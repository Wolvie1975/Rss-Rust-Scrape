# MLB schedule import / My-Newsfeed handoff

MLB uses the existing ESPN schedule pipeline (`src/mlb.rs`,
`src/espn_schedule.rs`) and scraper-owned `SportsEvents`/`SportsEventsType` tables.
Collection is **today forward in America/Chicago**, including today's unfinished
fixtures. Past dates and completed games, including games finished earlier today,
are excluded. Results, scores, pitchers, standings and existing result-sync tables
are not imported or modified.

## Configure and collect

Deploy the updated Rust image/binary, then run:

```sh
cargo run --locked -- --configure-mlb
```

This registers an MLB league feed **disabled**, preserving existing Enabled and
edited URL settings on rerun. Enable through the existing sports feed admin or SQL:

```sql
UPDATE dbo.SportsEventsType
SET Enabled=1, RssUrl='https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard?season=2026'
WHERE ProviderKey='espn-mlb' AND ProviderSchoolId=0;
```

The existing hourly `--events-from-db` command then includes MLB automatically:

```sh
cargo run --locked -- --events-from-db --dry-run -o /tmp/schedule-preview.xml
cargo run --locked -- --events-from-db -o /tmp/schedule-preview.xml
```

Registered URL:
`https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard`.
For this deployment, keep it league-wide with **season=2026** and no team parameter.
This restricts collection to the requested remainder of 2026. Do not remove that
restriction unless the user requests a subsequent season.
Like WNBA, optional local `team=<ESPN ID>,<ESPN ID>` filtering is supported, and
`season=2026` can restrict validation to a year without bypassing the today cutoff.
Past-year configuration performs no requests. Edit the registered row rather than
adding a feed per team or season. Filters do not delete already stored games.

Requests cover the current month through December of the next calendar year;
announced next-season fixtures are included, not just this year's playoffs.
Monthly requests use `dates=YYYYMM&limit=1000`; potentially truncated responses fail
validation instead of importing a silently incomplete schedule. All requests must
succeed and validate before that feed's collected snapshot is saved. Failure is
recorded in the existing feed status/error columns and preserves stored games.

## Database and My-Newsfeed instructions

No additional schema migration beyond the existing shared `ScheduleStatus` column
is required. `sql/mlb_seed.sql` configures the lookup row; `Db::open` initializes
existing sports schema and the nullable `NVARCHAR(100)` status field if missing.
`ProviderSchoolId=0` identifies a league feed, not a baseball team.

In My-Newsfeed, add MLB as a supported professional schedule provider and enabled
source selector option. Use the existing direct SportsEvents professional query,
adding `ProviderKey='espn-mlb'` to its supported provider set. Keep the separate
Big 12 school-source query working. Do not implement another ESPN fetcher in the
site. Apply its selected date bounds and **EventDate >= Central today** to MLB,
just as for Current and WNBA. Hide completed/canceled statuses; postponed, delayed
and suspended games show status plus Time TBD. NULL legacy statuses stay valid.

The data contract matches WNBA:

- `Sport='Baseball'`. `SchoolName` is the home team; `Opponent` is the away team.
  `Title` is home vs away. `TeamLogoUrl`/`OpponentLogoUrl` follow the same perspective.
  `IsAway=0` represents the stored home perspective; neutral games use NULL.
- `SchoolId`/`OpponentSchoolId` store ESPN participant IDs namespaced by
  `ProviderKey='espn-mlb'`. Never resolve them using Big 12 school metadata. Either
  ID can match a team filter. Negative ESPN playoff placeholder IDs identify
  unresolved sides such as TBD or CLE/CHW; do not treat them as resolved teams.
- `Location` is the venue. `Tv` includes supplied broadcasts. `StreamUrl` is an
  actual watch link when available. Use `LiveStatsUrl ?? Url` for the MLB gamecast.
- Confirmed `StartsAtUtc` is UTC; mark SQL datetime values UTC before Central
  display conversion. `EventDate` is the Central date, including starts after UTC
  midnight. Unknown starts use `TimeTbd=1`, `StartsAtUtc=NULL`, and the stored date.
  ESPN midnight-Eastern date placeholders retain their advertised date rather
  than shifting to the previous Central day. No end time is invented.
- `GameId` is the ESPN event ID, `CanonicalKey='espn:mlb:<ID>'`, and `Url` is the
  stable ID-based MLB game URL. Doubleheaders stay separate even when participants
  and dates match. Changed time, date, venue or playoff opponents update the same
  database ID and preserve FirstSeenAt on repeat import.
- Explicit future cancellations/postponements are collected for reconciliation,
  preventing previously scheduled rows from remaining falsely active. They must
  not be shown as ordinary scheduled games. A missing observation never implies
  cancellation or deletes a game. Existing historical rows, if any, remain stored
  but the page's Central-today filter excludes them.

MLB is now deployed and enabled in production as feed **4193**. The hourly
collector imports the league-wide source. Initial and repeat imports saved 2,916
upcoming fixtures with stable IDs; existing WNBA/Current events are preserved.
See [deployment verification](../reports/mlb-deployment-2026-10-09.md).
The live /events page renders MLB games but lacks a dedicated MLB selector option;
no My-Newsfeed code was edited or web deployment performed here.

## Source verification and tests

Verified ESPN source on 2026-10-09:
<https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard?dates=202610&limit=1000>.
It supplies event IDs, home/away participants, venues, UTC dates, timeValid, status,
logos, broadcasts and MLB gamecast links, including unresolved playoff participants.
ESPN's public endpoint is undocumented and can change.

The default live CLI preview collected 2,916 unique today-forward MLB fixtures,
including announced next-season schedules, with no past dates. This was a dry-run
against isolated staging; no production events/configuration were written.

```sh
cargo test --locked --all-targets
MLB_TEST_DATABASE=WebScraper_Mlb_Validation_20261009 \
  cargo test --locked --bin web_scraper db::mlb_tests -- --ignored --nocapture
```

Coverage includes MLB home/away/venue/link mapping, playoff placeholder resolution,
rescheduling, independent doubleheader IDs, team filtering and league identity
isolation. Shared tests cover inclusive today, completed-game exclusion, Central
DST/date boundaries, TBD dates and empty past seasons. The rollback-only SQL test
covers disabled registration, preserved edited configuration, repeated imports,
rescheduling/postponement, stable IDs/FirstSeenAt, doubleheader rows and retained
missing games.

## Scope correction: remainder of 2026 only

The user clarified the desired MLB season after deployment. Production feed 4193
now uses `?season=2026`, retaining the rolling Central-today cutoff. The 2,894
previously imported 2027 MLB rows were backed up and removed; **22 remaining 2026
MLB games** remain. A repeat import confirmed no 2027 games returned. All other
959 sports events were retained. No image rebuild was needed; the deployed
provider already supports the season setting.
