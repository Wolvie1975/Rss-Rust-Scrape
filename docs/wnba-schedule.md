# WNBA schedule import / My-Newsfeed handoff

The scraper supports all WNBA fixtures using ESPN's first-party basketball WNBA
scoreboard. It shares the Kansas City schedule parser and locked upsert through
`src/espn_schedule.rs`, with provider configuration in `src/wnba.rs`. Scores,
winners, box scores and existing results-sync tables are never imported or changed.

## Setup

```sh
# Initialize existing schema and register the WNBA league feed disabled.
cargo run --locked -- --configure-wnba
```

Enable it through feed administration or SQL in the shared My-Newsfeed database:

```sql
UPDATE dbo.SportsEventsType SET Enabled=1
WHERE ProviderKey='espn-wnba' AND ProviderSchoolId=0;
```

The existing hourly `--events-from-db` collection then includes WNBA:

```sh
cargo run --locked -- --events-from-db --dry-run -o /tmp/schedule-preview.xml
cargo run --locked -- --events-from-db -o /tmp/schedule-preview.xml
```

Default `SportsEventsType.RssUrl`:
`https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard`.
Coverage is **today forward in America/Chicago**. The importer requests the
current month through the end of next calendar year (15 requests on 2026-10-09),
then excludes games dated before today and completed results, including completed
games earlier today. Postseason and other ESPN-listed upcoming WNBA fixtures are
included. Append `?season=2026` to restrict to that year, still respecting today.
To restrict new observations to specific teams, append `?team=18,20`, or combine
parameters as `?season=2026&team=18,20`. Team numbers are ESPN IDs, not names; 18 is
Connecticut Sun and 20 Atlanta Dream in the verified fixture. Local filtering
matches either participant. Games between two selected teams still import once.
Team IDs may be looked up in ESPN's league teams API:
<https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/teams>.

Edit the registered feed row rather than creating one per team/year. Setup reruns
preserve the URL and Enabled setting. Filters do not delete previously imported
games; when narrowing a previously league-wide feed, My-Newsfeed should also apply
its selected-team filter at query time. `--events-feed 'URL'` provides manual preview
or import with the same registration behavior as other event feeds. Existing
disabled configuration is respected on writes.

## Data and My-Newsfeed integration

No additional schema changes beyond the preceding Kansas City importer are needed:
`sql/nwsl_events.sql` supplies shared nullable `SportsEvents.ScheduleStatus`.
`Db::open` initializes it plus the existing sports schema. `sql/wnba_seed.sql` is
lookup configuration, not a new results schema. ProviderSchoolId **0** identifies
this league feed and is not a team ID.

Read `SportsEvents` directly for WNBA; the Big 12 school-source view contains only
school observations. Suggested .NET query SQL:

```sql
SELECT e.*
FROM dbo.SportsEvents e
JOIN dbo.SportsEventsType f ON f.ID=e.SportsEventsTypeId
WHERE f.Enabled=1 AND e.ProviderKey='espn-wnba'
  AND e.EventDate>=@CentralToday
  AND e.EventDate>=@CentralStartDate AND e.EventDate<@CentralEndDate
  AND (e.ScheduleStatus IS NULL OR
       (e.ScheduleStatus NOT IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_FULL_TIME')
        AND e.ScheduleStatus NOT LIKE 'STATUS_FINAL%'))
ORDER BY e.EventDate,e.StartsAtUtc,e.ID;
```

Each league game uses a consistent **home-team perspective**, including when a
team filter selects only the away side:

- `SchoolName`: home team name; `Opponent`: away team name;
  `Title`: home team vs away team; `Sport`: Basketball.
- `SchoolId` and `OpponentSchoolId`: ESPN home/away team IDs, namespaced by
  `ProviderKey='espn-wnba'`; do not join these IDs to Big 12 school metadata.
  For a chosen team, filter either ID and derive its home/away designation from
  which column matched. `IsAway=0` describes the stored home-team perspective.
- `TeamLogoUrl` and `OpponentLogoUrl`: home/away logos. `IsNeutral=1` uses
  `IsAway=NULL`, while retaining ESPN's designated sides for naming.
- `Location`: venue; `Tv`: available broadcast names; `StreamUrl`: actual HTTPS
  watch link when supplied; `LiveStatsUrl`: ESPN gamecast link. Use
  `LiveStatsUrl ?? Url` for game navigation.
- `EventDate`: America/Chicago date, even when UTC falls on the next day.
  `StartsAtUtc`: confirmed UTC start, otherwise NULL with `TimeTbd=1`.
  Mark SQL datetimes UTC before display conversion in .NET. DST is handled by the
  same timezone rules as Kansas City and Big 12. End times are not invented.
- `GameId`: ESPN event ID; `CanonicalKey='espn:wnba:<ID>'` prevents collisions with
  NWSL. ID-based WNBA `Url` stays constant through title/link/date changes.
  Repeat imports preserve database IDs and FirstSeenAt.
- `ScheduleStatus`: raw ESPN schedule status. Explicit canceled/postponed/delayed/
  suspended games retain their row and clear confirmed start times. A later
  scheduled observation restores the start on the same row. Missing observations
  never delete a game or infer cancellation. Display status labels/TBD, or filter
  canceled statuses in the schedule query according to the existing UI convention.

Add WNBA to My-Newsfeed's schedule source selection and combine this direct query
with the existing school-calendar/Kansas City queries. Map nullable ScheduleStatus
in its schedule model if not already done. No My-Newsfeed checkout is present in
this workspace, so its UI/query edits and production enabling/deployment remain
for that repository. No scheduler restart was performed.

## Source checks and tests (initial full-season audit)

ESPN source:
<https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard?dates=202609&limit=1000>.
On 2026-10-09 the live CLI preview collected **378 unique 2026 fixtures**; the
default three-year refresh collected **705**. The September response alone
contained 38 games with event IDs, UTC timestamps,
timeValid, homeAway, venue, broadcast names, status and WNBA game links. Like the
NWSL endpoint, this is a public undocumented API and may change. The importer
validates responses, limits monthly payloads, reports failures per feed, and retains
last-good stored games if a request or parser check fails.

```sh
cargo test --locked --all-targets
# Separate database only; fixture writes roll back.
WNBA_TEST_DATABASE=WebScraper_Wnba_Validation_20261009 \
  cargo test --locked --bin web_scraper db::wnba_tests -- --ignored --nocapture
```

Coverage includes home/away participants and links, team filters, games shared by
selected teams, Central dates/DST, TBD and neutral sites, cancellation/reschedule
identity, disabled configuration and edited URLs, SQL repeats and first-seen
preservation, missing-game retention, and league identity isolation. Existing NWSL
parser, monthly fetch and SQL integration tests verify the shared implementation.

## Upcoming-only refinement (2026-10-09)

The import cutoff moves with the current Central date on each run. Today is
included, including games whose start time is TBD. Historical rows already stored
are retained, so My-Newsfeed must also use `EventDate >= @CentralToday` in its page
query. Exclude completed and canceled schedule statuses from the visible upcoming
list. Explicit future cancellations/postponements still reach the importer to
update existing rows; keeping these observations prevents a previously scheduled
game from remaining falsely active. They are maintenance observations, not games
to display as scheduled. No results sync or historical row deletion is performed.

Earlier full-season audit counts describe source completeness before this
upcoming-only refinement; they are not the current import counts.

ESPN uses midnight Eastern (04:00Z in summer / 05:00Z in winter) as a
date placeholder for some timeValid=false fixtures. Those retain the advertised
Eastern calendar date instead of shifting to the previous Central day. Confirmed
starts always use normal UTC-to-Central date conversion.
