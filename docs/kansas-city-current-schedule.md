# Kansas City Current schedule import / My-Newsfeed handoff

`src/nwsl.rs` now wraps shared ESPN parsing in `src/espn_schedule.rs`, also used
by WNBA. Kansas City filtering and event identity remain unchanged.

This is schedule ingestion into the existing `dbo.SportsEvents` model. It never
reads or writes `ResultGames`, `ResultFollows`, scores, winners, or result-sync
configuration. My-Newsfeed's ESPN NWSL results sync continues independently.

## Source verification (2026-10-09; initial full-season audit)

ESPN's first-party NWSL scoreboard:
<https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?dates=202610&limit=1000>
returns October 3, 17 and 25 Kansas City fixtures. The full 2026 monthly collection
was verified with 30 unique Kansas City games, including November 1.
Team identity is ESPN ID **20907**; select by competitor ID, not text matching.
The team `/teams/20907/schedule?season=2026` endpoint returned only 27 completed
games in this check, so it is unsuitable for the upcoming schedule. Long date
ranges returned HTTP 400; `dates=YYYYMM` worked. The endpoint is public and needs
no API key, but is undocumented and offers no stability guarantee.

Verified JSON fields: `events[].id`, competition `date`, `timeValid`, `status.type.name`,
`neutralSite`, `venue.fullName`, competitor `homeAway`, `team.id/displayName/logo`,
broadcast names, and HTTPS summary/watch links. Optional venue, broadcast, logo,
and watch links may be absent. `LiveStatsUrl` contains the provider summary/game
link; `StreamUrl` is populated only when an actual watch link exists. `Url` is a
stable ID-based ESPN match URL, never a date-based identifier. No end time is guessed.

## Configure and run

```sh
# Initialize additive schema and register the feed disabled.
cargo run --locked -- --configure-nwsl
```

Enable the registered row through the existing My-Newsfeed feed administration,
or in the shared SQL database:

```sql
UPDATE dbo.SportsEventsType SET Enabled=1
WHERE ProviderKey='espn-nwsl' AND ProviderSchoolId=20907;
```

Then the existing hourly `--events-from-db` command includes it automatically:

```sh
cargo run --locked -- --events-from-db --dry-run -o /tmp/schedule-preview.xml
cargo run --locked -- --events-from-db -o /tmp/schedule-preview.xml
```

`SportsEventsType.RssUrl` stores the provider URL despite its historical column
name. Default:
`https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907`.
Append `&season=2026` to restrict collection to one calendar year for validation.
Without `season`, collection requests the current month through the end of next
Central calendar year (15 requests on 2026-10-09). Only **today or later** Central
dates and unfinished games are returned; completed games earlier today are also
excluded. A fixed `season` still respects this cutoff, so a past year imports nothing.
The importer sends only `dates` and `limit` upstream; `team` is a local identity
filter. This provider supports Kansas City Current only. Do not add duplicate
configuration rows for different seasons; edit the registered row's URL instead.
Configuration reruns preserve enabled settings and edited URLs.
Explicit `--events-feed 'URL'` is also supported, using existing feed registration
behavior; a matching disabled row remains disabled on save.

The ESPN requests identify as `My-Newsfeed/1.0`; the existing generic scraper user
agent returned HTTP 403 during verification. This override is limited to NWSL.
Feed attempt/success/error/count tracking uses the existing columns. Any fetch or
validation error prevents writes for that feed and leaves its last good schedule.
An empty or missing game observation never deletes an existing game or infers cancellation.

## Identity, time and My-Newsfeed database contract

- `GameId` is the ESPN event ID; `ProviderKey='espn-nwsl'` and
  `CanonicalKey='espn:usa.nwsl:<event ID>'` namespace it independently of Big 12.
  The canonical key's existing unique index plus a locked upsert prevents duplicate
  rows. Repeat imports and changed dates/statuses preserve `ID` and `FirstSeenAt`.
- `EventDate` is the date in `America/Chicago`, including away fixtures, consistent
  with the existing Kansas schedule. Confirmed `StartsAtUtc` is UTC. SQL returns
  an unspecified-kind datetime: mark it UTC before converting in .NET. Use Central
  dates for weekly/day filters. This correctly handles DST and games after UTC midnight.
- ESPN timestamps may omit seconds. Both minute-precision `Z` and full RFC3339 are
  accepted. `timeValid=false` or absent means `TimeTbd=1`, `StartsAtUtc=NULL`.
  Explicit postponed/canceled/delayed/suspended statuses also clear confirmed starts.
  Restored scheduled games receive their new time on the same row.
- `IsAway` is from Kansas City's competitor perspective. Neutral games use
  `IsNeutral=1`, `IsAway=NULL`; home and away venues come from the provider.
- Additive migration **`sql/nwsl_events.sql`** adds nullable
  **`SportsEvents.ScheduleStatus NVARCHAR(100)`**; `Db::open` runs it automatically
  after existing sports migrations. `sql/nwsl_seed.sql` registers the lookup row.
  No existing columns, tables, or results schemas are removed or renamed.

My-Newsfeed should read the Kansas City lookup ID directly from `SportsEvents`
(join `SportsEventsType` and filter provider/team), alongside its existing school
schedule query. The Big 12 `SportsEventSchoolCalendar` view includes only school
source observations; this importer does not insert into that view's source table.
Map the new nullable `ScheduleStatus` property in the schedule DTO/EF model if the
UI needs cancellation/postponement labels. Preserve canceled rows with their label,
or exclude `STATUS_CANCELED`/`STATUS_CANCELLED` from the upcoming list. Show postponed,
delayed, suspended and unconfirmed starts as TBD. Use `LiveStatsUrl ?? Url` for the
game link. Existing school events have NULL schedule status and remain valid.

The My-Newsfeed checkout is not available in this workspace; its query/model/UI
changes must be applied there using this contract. Production feed enabling,
host deployment and scheduler restart were not performed.

## Validation

```sh
cargo test --locked --all-targets
# SQL test initializes only a separate staging database; fixture writes roll back.
NWSL_TEST_DATABASE=WebScraper_Nwsl_Validation_20261009 \
  cargo test --locked --bin web_scraper db::nwsl_tests -- --ignored --nocapture
```

Tests cover live home/away fixtures, missing/malformed payloads, Central date/DST
conversion, TBD times, neutral sites, stable identity through rescheduling and
cancellation, monthly requests/deduplication, disabled registration, repeated SQL
imports, preserved first-seen/IDs, and retained games absent from later snapshots.

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
