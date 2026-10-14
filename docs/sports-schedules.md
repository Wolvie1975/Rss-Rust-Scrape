# Sports schedules and match results

Current fixtures use the existing SportsEvents model. NHL, MLS and Championship final results use SportsMatchResults, separately from schedule retention; shootout tallies are separate from match goals. MLS source 5525, Championship 5326 and EPL 5357 and NHL 5637 are enabled. Past/canceled schedules are removed. The provider design proposal at the end is historical; deployed implementation and current retention take precedence.

## Contents

- [NHL schedules and final results](#nhl-schedule-results)
- [Big 12 calendar ingestion](#big12-sports)
- [MLS schedule, standings and results](#mls-schedule-results-frontend-handoff)
- [EFL Championship schedule, table and results — frontend handoff](#my-newsfeed-championship-wrexham)
- [Premier League — frontend handoff](#my-newsfeed-epl)
- [MLB schedule import / My-Newsfeed handoff](#mlb-schedule)
- [WNBA schedule import / My-Newsfeed handoff](#wnba-schedule)
- [Kansas City Current schedule import / My-Newsfeed handoff](#kansas-city-current-schedule)
- [Big 12 sports calendar expansion proposal](#big12-sports-expansion-plan)

<a id="nhl-schedule-results"></a>

## NHL schedules and final results

NHL schedule/results source **5637** is enabled, with provider key `espn-nhl`,
sport `Hockey`, ESPN league ID `90`, and feed:
`https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/scoreboard`.
The deployed hourly collector reads it through `--events-from-db`.

Verification on October 14, 2026 recorded **1,239 current/future fixtures** and
**170 final results** for ESPN season **2027 (2026–27)**. These are deployment
snapshots, not guaranteed current counts. See the [deployment history](../reports/sports-history.md#nhl-deployment-2026-10-14).

### Schedule read contract

Use `dbo.SportsEvents` joined to enabled `dbo.SportsEventsType` rows. A league game
has one row from the home team's perspective; `SchoolId`/`SchoolName` identify the
home NHL team and opponent fields identify the away team. These ESPN team IDs
are scoped to the NHL provider and do not map to Big 12 school metadata.
Canonical identity is `espn:nhl:<gameId>`; links use the ESPN NHL game URL.
UTC start time, Central calendar date, venue, broadcasts and both team logos use
the shared event columns. Preserve TBD, postponement and cancellation handling.
Completed observations reconcile existing rows without inserting past results
as fixtures. Existing schedule retention removes past/canceled entries.

```sql
SELECT e.* FROM dbo.SportsEvents e
JOIN dbo.SportsEventsType s ON s.ID=e.SportsEventsTypeId
WHERE s.Enabled=1 AND e.ProviderKey='espn-nhl'
ORDER BY e.EventDate,e.StartsAtUtc;
```

### Results and season handling

Final scores live in `dbo.SportsMatchResults` independently of fixture retention.
Identity is `(ProviderKey,GameId)`; corrections update scores and preserve `ID`
and `FirstSeenAt`. Include home/away names, logos and scores in the results view.
Render `STATUS_FINAL` as Final, `STATUS_FINAL_OT` as Final/OT, and
`STATUS_FINAL_SO` as Final/SO. Score columns hold ESPN's final scoreboard totals,
including the deciding shootout goal; optional shootout fields are not inferred.
Live games and tied NHL scores never become final results.

```sql
DECLARE @Season INT=2027; -- 2026–27, using ESPN's ending year
SELECT r.* FROM dbo.SportsMatchResults r
JOIN dbo.SportsEventsType s ON s.ID=r.SourceId
WHERE s.Enabled=1 AND r.ProviderKey='espn-nhl' AND r.Season=@Season
ORDER BY r.StartsAtUtc DESC;
```

Collection covers July through June and rolls to the next season in July.
Preseason, regular season and playoffs are included when published by ESPN.
Each event must match the requested season; preseason league headers can lag,
so NHL event season fields are authoritative. Empty responses do not delete
previous results. Prior seasons are removed only after a successful nonempty
new-season results import.

Standings source **13** and its display contract are covered in the
[NHL standings section](sports-tables.md#nhl-standings). The separate My-Newsfeed
application can use these shared read models; scraper deployment does not
establish whether its NHL selectors or pages have been implemented.

Setup and activation: [operations guide](operations.md#nhl-operations).
Detailed collector/reference commands: [NHL reference](nhl.md).

<a id="big12-sports"></a>

## Big 12 calendar ingestion

The scraper collects football, men's basketball, women's basketball, soccer, and
volleyball from the provider's current full-member schools. Oklahoma State has no
volleyball coverage. Membership and school logos come from the provider calendar
members metadata; sport filtering is explicit in `src/big12.rs`.

<a id="big12-sports--setup-and-operation"></a>

### Setup and operation

```sh
# Apply additive schema, register verified member feeds disabled, preserve Kansas.
cargo run --locked -- --configure-big12
# Deliberately enable all registered Big 12 school feeds.
cargo run --locked -- --enable-big12
# Inspect without event/source/status writes (schema initialization still runs).
cargo run --locked -- --events-from-db --dry-run -o /tmp/calendar-preview.xml
# Import now; the existing hourly service uses this same flag.
cargo run --locked -- --events-from-db -o /tmp/calendar-preview.xml
# Read-only database audit.
cargo run --locked --example audit_sports -- WebScraper /tmp/sports-audit.json
```

`--database NAME` selects an isolated staging database. The default is WebScraper.
The setup command preserves existing feed IDs, URLs, enable settings and event IDs.
Only `--enable-big12` explicitly enables disabled member feeds. Repeated setup
recognizes school IDs despite RSS query-parameter order; no duplicate configuration
is created. Affiliates and retired members are not imported as current full members.
Existing retired configurations are reported as failures rather than silently reassigned.

<a id="big12-sports--one-event-multiple-source-perspectives"></a>

### One event, multiple source perspectives

`SportsEvents` contains one public row per provider game group. A mirrored game can
have different provider IDs and URLs in each school's RSS. Structured calendar
`id`/`opp_game_id` links establish a sorted stable pair key; unpaired games use their
provider game ID. Dates, titles and team names are never deduplication keys.

`SportsEventSources` preserves each feed's provider game ID, source URL, school and
opponent IDs, JSON snapshot, warnings and timestamps. Its unique key is feed ID plus
provider game ID. Both aliases resolve to the same public row, including when an
unpaired game later gains a linked ID. Conflicting existing public identities are
quarantined for review; records are never deleted automatically.

Historical Kansas rows are backfilled as primary observations, keeping their IDs,
FirstSeenAt, school perspective, URLs and logos. For new events a home observation
is processed first; neutral games use deterministic feed ordering. The primary
perspective remains stable on subsequent imports. Only that source replaces its
public schedule; a known secondary start can fill a same-date TBD time. Secondary
stale/TBD information cannot overwrite a known primary time. Nonempty broadcast
information can fill missing public fields. Each full source snapshot remains available.

School-specific queries **must use the source links**, not just the public row's
primary SportsEventsTypeId. The SQL view provides that join:

```sql
SELECT * FROM dbo.SportsEventSchoolCalendar
WHERE SchoolId = 3 AND EventDate >= @WeekStart AND EventDate < DATEADD(day, 7, @WeekStart);
```

The normal public list can continue reading SportsEvents, which is deduplicated.
The view's `SportsEventId` is the same public event ID in both schools' calendars;
logos, opponent and home/away information reflect the selected school perspective.
Radio and AudioUrl supplement Tv, StreamUrl and LiveStatsUrl. The separate .NET
application was not changed; its school-filtered queries should adopt this view.

<a id="big12-sports--unknown-data-provider-errors-and-status"></a>

### Unknown data, provider errors and status

RSS is authoritative for source dates and UTC instants. Metadata adds explicit
neutral-site status, participant/sport IDs and media. Neutral events use IsAway=NULL
and IsNeutral=1. Unknown venues, logos, times and broadcast links stay NULL.
Conference/channel logos are never used as school logos. Source timezone labels are
stored as supplied; RSS UTC times are not recalculated from ambiguous abbreviations.

Conference-wide RSS and oversized JSON ranges omit some future games. The scraper
fetches school RSS and requests JSON identity metadata one observed month at a time.
Missing or inconsistent identity metadata keeps a new observation out of the public
calendar, while retaining it in SportsEventSources with SportsEventId=NULL and Issue.
Known self-opponent records and member games without explicit partner links are
quarantined. Missing records never remove historical games or imply cancellation.

JSON `date_utc` sometimes contains midnight Central rather than the announced kickoff.
When JSON’s local `date` exactly matches the RSS instant in America/Chicago and
`date_utc` is midnight on that same Central date, this date-only placeholder does
not raise a warning. RSS timing remains authoritative. Other RSS/JSON UTC
disagreements and paired-RSS timing disagreements are recorded as warnings;
source values are retained for review. A failed refresh preserves a source's last good
payload and event link. One failed school does not stop other school feeds.
SportsEventsType records Enabled, LastAttemptAt, LastSuccessAt, LastError,
LastEventCount and LastIssueCount. LastSuccessAt means RSS observations were processed
and persisted; a nonzero LastIssueCount identifies warnings or quarantined records.
If collection or SQL persistence fails, LastSuccessAt is preserved. Disabling a feed
stops polling; disabling a feed during a run also prevents subsequent source writes.

<a id="big12-sports--validation"></a>

### Validation

```sh
cargo test --locked --all-targets
# Named staging database populated from Kansas data and configured first:
cargo test --locked --bin web_scraper mirrored_imports -- --ignored
```

Fixtures cover membership, linked Kansas/Utah IDs, primary/secondary perspective,
neutral/TBD events, inconsistent UTC metadata, doubleheaders, missing identity and
self-opponents. Staging validates migration reruns, Kansas historical identities,
repeated/concurrent imports, source attribution and persisted feed status.

<a id="big12-sports--deployment-verification-and-rollback"></a>

### Deployment verification and rollback

See [the October 9 verification report](../reports/sports-history.md#big12-expansion-2026-10-09).
The staging database remains available for inspection. The host checkout has a
pre-expansion source/sports snapshot under `.deployment-backups/20261009-big12/`.
The rollback image is `web-scraper:before-big12-20261009`. A legacy image must run
with the explicit Kansas `--events-feed` URL replacing `--events-from-db`; it cannot
safely poll all newly registered feeds because it lacks canonical deduplication.

<a id="mls-schedule-results-frontend-handoff"></a>

## MLS schedule, standings and results

MLS now has league-wide fixtures and current-season final results from ESPN
competition `usa.1` (league ID `770`). Existing MLS standings remain unchanged.
Production schedule/results source ID: **5525**, ProviderKey `espn-mls`.
Standings source ID: **9**. Resolve IDs by ProviderKey in other environments.

<a id="mls-schedule-results-frontend-handoff--upcoming-schedule"></a>

### Upcoming schedule

```sql
SELECT e.* FROM dbo.SportsEvents e
JOIN dbo.SportsEventsType f ON f.ID=e.SportsEventsTypeId
WHERE f.Enabled=1 AND e.ProviderKey='espn-mls'
 AND e.EventDate>=@TodayCentral
 AND (e.ScheduleStatus IS NULL OR
      (e.ScheduleStatus NOT IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY','STATUS_FULL_TIME')
       AND e.ScheduleStatus NOT LIKE 'STATUS_FINAL%'))
ORDER BY e.EventDate,e.StartsAtUtc,e.ID;
```

Use existing schedule UI: home team is SchoolId, away team is OpponentSchoolId.
Show kickoff, venue, team logos and provider-supplied watch links. Convert known
StartsAtUtc values to America/Chicago; preserve TBD when a kickoff is not known.
Canonical game key is `espn:usa.1:<gameId>`. Optional team filters should match
both SchoolId and OpponentSchoolId; do not add separate team feeds.

<a id="mls-schedule-results-frontend-handoff--results"></a>

### Results

```sql
SELECT r.* FROM dbo.SportsMatchResults r
JOIN dbo.SportsEventsType f ON f.ID=r.SourceId
WHERE f.Enabled=1 AND r.ProviderKey='espn-mls' AND r.Season=@Season
ORDER BY r.MatchDate DESC,r.StartsAtUtc DESC,r.ID DESC;
```

Use the current provider season (initially 2026). Results include regular-season
and playoff games when ESPN publishes them. They are independent of the pruned
SportsEvents table and do not populate existing ResultGames/ResultFollows.

Map these fields:

| Field | Meaning |
| --- | --- |
| ID | bigint / .NET long; stable local result ID |
| SourceId / GameId | int; source and stable ESPN game ID |
| Season | Provider season year; initial season 2026 |
| MatchDate / StartsAtUtc | Central date / UTC kickoff |
| HomeTeamId / AwayTeamId | ESPN team IDs |
| HomeTeamName / AwayTeamName | Provider names |
| HomeScore / AwayScore | Final match goals, excluding shootout penalties |
| HomeShootoutScore / AwayShootoutScore | Nullable int; separate penalty shootout tallies |
| HomeLogoUrl / AwayLogoUrl | Nullable provider logos |
| ScheduleStatus | Provider final status |
| SourceUrl | Original ESPN match page |
| FirstSeenAt / LastSeenAt | UTC first import / most recent observation |

For STATUS_FINAL_PEN, display match goals plus the penalty tally, e.g.
**1–1 (2–4 on penalties)**. Use penalties to identify the winner; do not show a
shootout match as a draw or add penalty tallies to match goals. For other final
statuses, show the match score and supplied status (including extra time).
Do not infer playoff-series aggregate winners from a single match.

<a id="mls-schedule-results-frontend-handoff--standings-and-collection"></a>

### Standings and collection

Keep the existing MLS conference standings and route `/standings?sourceId=9`.
The source's Eastern/Western conference groups provide membership; fixture rows
are league-wide and do not carry a conference label.

`--configure-mls` registers the schedule disabled and adds nullable shootout
columns to the shared results table using migration credentials. Enable its
SportsEventsType row to collect. Existing `--events-from-db` hourly collection
then discovers MLS automatically; no extra scheduler argument is required.

Current calendar-season months are fetched to reconcile score corrections and
reschedules. Results require an explicit final status, valid nonnegative goals,
and distinct teams. Final penalty games require valid unequal shootout scores.
Repeated imports/corrections preserve IDs and FirstSeenAt. Disabled sources are
protected by the writer; provider/game identity keeps MLS separate from other
leagues, including Championship results using the same table.

<a id="mls-schedule-results-frontend-handoff--retention-and-empty-states"></a>

### Retention and empty states

Upcoming schedules follow existing rules: past dates and canceled/unnecessary
fixtures are removed. Current-season final results remain available for the
Results view. Prior-season results are removed after a successful nonempty
new-season results import; this avoids deleting the last good season during the
off-season or after a failed request. The front-end filters Season explicitly.

Show an empty schedule when no future MLS fixtures have been published. Do not
invent playoff matchups or next-season fixtures. Show source import errors via
the existing sports source status UI; retain the last good data on fetch/parse
failure. Front-end views still need implementation.

Initial verification: 90 upcoming fixtures and 421 completed 2026 results;
no past or canceled fixtures remained after retention.

<a id="my-newsfeed-championship-wrexham"></a>

## EFL Championship schedule, table and results — frontend handoff

<a id="my-newsfeed-championship-wrexham--scope-and-identity"></a>

### Scope and identity

Show the full **EFL Championship** (England's second division), with a **Wrexham AFC**
focus toggle. This is not the Premier League. ESPN competition key is `eng.2`,
league ID `3914`, and Wrexham's team ID is **352**. Display the provider season
label; the initial import is **2026–27** (season start year `2026`).

Three views:

- Schedule: all upcoming Championship fixtures, with a Wrexham-only filter.
- Table: all 24 teams, highlighting Wrexham. Do not reduce the league table to one row.
- Results: completed current-season Championship matches, with a Wrexham-only filter.

Schedule and results use the same enabled sports feed with ProviderKey
`espn-championship`. Standings use a separate enabled SportsStandingsSources row
with the same ProviderKey. Resolve IDs by ProviderKey rather than assuming numeric
IDs match another environment. Production fixture/results source ID is **5326**; standings source ID is **11**.
The existing standings route is `/standings?sourceId=11`.

<a id="my-newsfeed-championship-wrexham--schedule-queries"></a>

### Schedule queries

Use the existing SportsEvents model and upcoming-game behavior. Wrexham may be
home or away; ESPN schedule rows use the home-team perspective.

```sql
SELECT e.*
FROM dbo.SportsEvents e
JOIN dbo.SportsEventsType f ON f.ID=e.SportsEventsTypeId
WHERE f.Enabled=1 AND e.ProviderKey='espn-championship'
  AND e.EventDate>=@TodayCentral
  AND (e.ScheduleStatus IS NULL OR
       (e.ScheduleStatus NOT IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY','STATUS_FULL_TIME')
        AND e.ScheduleStatus NOT LIKE 'STATUS_FINAL%'))
  AND (@WrexhamOnly=0 OR e.SchoolId=352 OR e.OpponentSchoolId=352)
ORDER BY e.EventDate,e.StartsAtUtc,e.ID;
```

Render home and away teams, kickoff, venue and supplied broadcast/watch links.
Known instants use StartsAtUtc converted to America/Chicago. TBD dates remain
TBD; do not invent a kickoff. For Championship date placeholders at midnight
London, preserve the source calendar date. Identity is `espn:eng.2:<gameId>`.
Schedules are pruned when past/canceled under the existing retention policy.
Do not register a second Wrexham schedule feed: filter these shared league rows.

<a id="my-newsfeed-championship-wrexham--league-table"></a>

### League table

Use the existing standings models, Enabled/IsCurrent filters and source selector.
ProviderKey=`espn-championship`, Sport=`Soccer`, DisplayName=`EFL Championship`,
MinimumTeams=`24`. Data comes from:
https://site.api.espn.com/apis/v2/sports/soccer/eng.2/standings

Render all 24 teams in provider Position order and highlight TeamId=`352`.
Use each statistic's `displayValue`, preserving provider formatting:

| Column | StatsJson name |
| --- | --- |
| P | `gamesPlayed` |
| W | `wins` |
| D | `ties` |
| L | `losses` |
| GF | `pointsFor` |
| GA | `pointsAgainst` |
| GD | `pointDifferential` |
| Pts | `points` |

Show supplied Rank when available. Do not calculate missing values or invent
promotion/relegation qualifications. Source URL, season/title, logos, links,
last-good and empty/error states follow the existing standings contract.

<a id="my-newsfeed-championship-wrexham--results-model-and-queries"></a>

### Results model and queries

Completed results live in **dbo.SportsMatchResults**, independently of SportsEvents.
The scraper does not write the site's existing ResultGames or ResultFollows tables.
Read access requires SELECT on this new table.

| Column | .NET type / meaning |
| --- | --- |
| ID | `long`, database identity |
| SourceId | `int`, SportsEventsType foreign key |
| ProviderKey | `string`, `espn-championship` |
| GameId | `int`, stable ESPN game identity |
| Season | `int`, starting year, e.g. 2026 |
| MatchDate | date, Central calendar date |
| StartsAtUtc | UTC datetime |
| HomeTeamId / AwayTeamId | `int`, ESPN team IDs |
| HomeTeamName / AwayTeamName | `string`, provider labels |
| HomeScore / AwayScore | `int`, validated final goals |
| HomeLogoUrl / AwayLogoUrl | nullable URL |
| ScheduleStatus | final provider status |
| SourceUrl | ESPN match link |
| FirstSeenAt / LastSeenAt | UTC timestamps |

```sql
SELECT r.*
FROM dbo.SportsMatchResults r
JOIN dbo.SportsEventsType f ON f.ID=r.SourceId
WHERE f.Enabled=1 AND r.ProviderKey='espn-championship'
  AND r.Season=@SeasonStartYear
  AND (@WrexhamOnly=0 OR r.HomeTeamId=352 OR r.AwayTeamId=352)
ORDER BY r.StartsAtUtc DESC,r.GameId DESC;
```

Render `HomeTeamName HomeScore – AwayScore AwayTeamName`. For Wrexham rows,
derive W/D/L by comparing goals from Wrexham's home or away perspective for
regular full-time matches. If a final status indicates penalties, display that
status and do not infer shootout winners from tied goal totals; this initial model
does not store separate shootout scores. Never
assume the home team won. A draw is valid. Do not display scheduled, canceled,
postponed or live matches as final results, and never treat missing goals as zero.
The importer accepts validated completed matches only; corrections update the
same row instead of duplicating it.

Keep current-season results despite the schedule's past-game deletion policy.
After a validated new-season result import, older-season result rows are removed.
If a fetch or save fails, retain the last good results and show the feed's refresh
error/status. An empty new-season response does not purge the previous results.

<a id="my-newsfeed-championship-wrexham--suggested-frontend-flow"></a>

### Suggested frontend flow

Add EFL Championship to sports and standings selection. Offer Schedule, Table
and Results tabs. Place a Wrexham AFC toggle on Schedule/Results and keep the
whole league table visible with Wrexham highlighted. Preserve filters in URLs,
for example `league=championship&team=352` or the site's existing query convention.
On small screens, allow the standings table to scroll horizontally.

The existing standings selector discovers new enabled sources dynamically.
The dedicated Wrexham filter and results view require frontend implementation;
this scraper task does not rebuild the separate My-Newsfeed application.

<a id="my-newsfeed-championship-wrexham--checks"></a>

### Checks

- Table has 24 unique teams and includes/highlights Wrexham ID 352.
- Schedule excludes past/canceled/final games; Wrexham filter works home and away.
- Results contain final scores only and use current-season selection.
- Re-imports and score corrections preserve row IDs and FirstSeenAt.
- Disabled feeds and failed refreshes preserve existing status behavior.
- Past schedule cleanup does not remove SportsMatchResults.
- Existing NWSL results synchronization and other leagues remain independent.

<a id="my-newsfeed-epl"></a>

## Premier League — frontend handoff

Scope: the current Premier League table and today's/future fixtures, across the
whole league. Historical EPL fixtures and match results are not imported.
ProviderKey is `espn-epl`, sport Soccer, competition `eng.1`, ESPN league ID 700.
The initial season is 2026–27 (stored standings Season `2026`). Use provider labels.

The scraper's existing hourly events and standings jobs refresh enabled sources.
Sources are registered with `--configure-epl`, disabled by default; repeated setup
preserves configured URLs and enable settings. Production schedule source ID is **5357** and standings source ID is **12**.
The existing table route is `/standings?sourceId=12`.

<a id="my-newsfeed-epl--fixtures"></a>

### Fixtures

Use SportsEvents and SportsEventsType with the existing upcoming-events display:

```sql
SELECT e.*
FROM dbo.SportsEvents e
JOIN dbo.SportsEventsType f ON f.ID=e.SportsEventsTypeId
WHERE f.Enabled=1 AND e.ProviderKey='espn-epl'
  AND e.EventDate>=@TodayCentral
  AND (e.ScheduleStatus IS NULL OR
       (e.ScheduleStatus NOT IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY','STATUS_FULL_TIME')
        AND e.ScheduleStatus NOT LIKE 'STATUS_FINAL%'))
ORDER BY e.EventDate,e.StartsAtUtc,e.ID;
```

Rows use home-team perspective: SchoolId/SchoolName identify home; OpponentSchoolId
and Opponent identify away. Scope ESPN team IDs to provider. Canonical identity is
`espn:eng.1:<gameId>`; it is separate from Championship and other soccer fixtures.
Known kickoffs are stored UTC and displayed in America/Chicago. For unknown kickoff
times, display TBD and the retained source date; midnight-London placeholders use
the London calendar date. Use supplied venue, logos and broadcast/watch links.

Past dates and canceled games are deleted by end-of-run retention. Today's
completed observations can update previously scheduled games to final status,
but are never inserted as results or shown as upcoming. The collector starts at
the current month and stops at the end of the current season, without past backfill.

<a id="my-newsfeed-epl--current-table"></a>

### Current table

Use the existing standings source selector with ProviderKey=`espn-epl`,
DisplayName=`Premier League`, MinimumTeams=20. The current page discovers enabled
sources dynamically. Query only Enabled sources and IsCurrent rows. Sort by
provider Position and render all 20 teams.

Recommended columns from StatsJson `displayValue`:

| Column | Statistic name |
| --- | --- |
| P | `gamesPlayed` |
| W | `wins` |
| D | `ties` |
| L | `losses` |
| GF | `pointsFor` |
| GA | `pointsAgainst` |
| GD | `pointDifferential` |
| Pts | `points` |

Use stored TeamName, LogoUrl, TeamUrl, Season, Title and optional Rank. Missing
statistics render as an em dash. Preserve published values and notes; do not
invent qualifications. The current table includes cumulative season statistics,
not individual historical result rows. Superseded EPL table rows are deleted
within the successful snapshot transaction; other leagues retain existing policies.

Failed fetches or saves preserve the last good table. Show LastSuccessAt and
LastError with the same status/empty/stale behavior as other leagues. No new schema
or frontend model is required for schedules or standings. Add Premier League to
any hardcoded sports selectors and reuse the soccer standings column mapping.
This task updates the scraper/data; it does not rebuild the separate web app.

Endpoints:

- https://site.api.espn.com/apis/site/v2/sports/soccer/eng.1/scoreboard
- https://site.api.espn.com/apis/v2/sports/soccer/eng.1/standings

Validate 20 table teams, future dates, correct home/away identities and preservation
of other leagues. Keep horizontal scrolling available for small screens.

<a id="mlb-schedule"></a>

## MLB schedule import / My-Newsfeed handoff

MLB uses the existing ESPN schedule pipeline (`src/mlb.rs`,
`src/espn_schedule.rs`) and scraper-owned `SportsEvents`/`SportsEventsType` tables.
Collection is **today forward in America/Chicago**, including today's unfinished
fixtures. Past dates and completed games, including games finished earlier today,
are excluded. Results, scores, pitchers, standings and existing result-sync tables
are not imported or modified.

<a id="mlb-schedule--configure-and-collect"></a>

### Configure and collect

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

<a id="mlb-schedule--database-and-my-newsfeed-instructions"></a>

### Database and My-Newsfeed instructions

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
See [deployment verification](../reports/sports-history.md#mlb-deployment-2026-10-09).
The live /events page renders MLB games but lacks a dedicated MLB selector option;
no My-Newsfeed code was edited or web deployment performed here.

<a id="mlb-schedule--source-verification-and-tests"></a>

### Source verification and tests

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

<a id="mlb-schedule--scope-correction-remainder-of-2026-only"></a>

### Scope correction: remainder of 2026 only

The user clarified the desired MLB season after deployment. Production feed 4193
now uses `?season=2026`, retaining the rolling Central-today cutoff. The 2,894
previously imported 2027 MLB rows were backed up and removed; **22 remaining 2026
MLB games** remain. A repeat import confirmed no 2027 games returned. All other
959 sports events were retained. No image rebuild was needed; the deployed
provider already supports the season setting.

<a id="wnba-schedule"></a>

## WNBA schedule import / My-Newsfeed handoff

The scraper supports all WNBA fixtures using ESPN's first-party basketball WNBA
scoreboard. It shares the Kansas City schedule parser and locked upsert through
`src/espn_schedule.rs`, with provider configuration in `src/wnba.rs`. Scores,
winners, box scores and existing results-sync tables are never imported or changed.

<a id="wnba-schedule--setup"></a>

### Setup

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

<a id="wnba-schedule--data-and-my-newsfeed-integration"></a>

### Data and My-Newsfeed integration

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

<a id="wnba-schedule--source-checks-and-tests-initial-full-season-audit"></a>

### Source checks and tests (initial full-season audit)

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

<a id="wnba-schedule--upcoming-only-refinement-2026-10-09"></a>

### Upcoming-only refinement (2026-10-09)

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

<a id="wnba-schedule--scraper-deployment-completed-2026-10-09"></a>

### Scraper deployment completed (2026-10-09)

The production scraper is deployed and both professional feeds are enabled.
Initial and repeat imports saved 3 Kansas City and 13 WNBA games with stable IDs;
the existing hourly service includes both. See
[deployment verification](../reports/sports-history.md#scraper-professional-schedules-deployment-2026-10-09).
Earlier statements about unperformed deployment describe the implementation phase.
The live web app still needs its separate Schedule-selector deployment.

<a id="kansas-city-current-schedule"></a>

## Kansas City Current schedule import / My-Newsfeed handoff

`src/nwsl.rs` now wraps shared ESPN parsing in `src/espn_schedule.rs`, also used
by WNBA. Kansas City filtering and event identity remain unchanged.

This is schedule ingestion into the existing `dbo.SportsEvents` model. It never
reads or writes `ResultGames`, `ResultFollows`, scores, winners, or result-sync
configuration. My-Newsfeed's ESPN NWSL results sync continues independently.

<a id="kansas-city-current-schedule--source-verification-2026-10-09-initial-full-season-audit"></a>

### Source verification (2026-10-09; initial full-season audit)

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

<a id="kansas-city-current-schedule--configure-and-run"></a>

### Configure and run

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

<a id="kansas-city-current-schedule--identity-time-and-my-newsfeed-database-contract"></a>

### Identity, time and My-Newsfeed database contract

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

<a id="kansas-city-current-schedule--validation"></a>

### Validation

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

<a id="kansas-city-current-schedule--upcoming-only-refinement-2026-10-09"></a>

### Upcoming-only refinement (2026-10-09)

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

<a id="kansas-city-current-schedule--scraper-deployment-completed-2026-10-09"></a>

### Scraper deployment completed (2026-10-09)

The production scraper is deployed and both professional feeds are enabled.
Initial and repeat imports saved 3 Kansas City and 13 WNBA games with stable IDs;
the existing hourly service includes both. See
[deployment verification](../reports/sports-history.md#scraper-professional-schedules-deployment-2026-10-09).
Earlier statements about unperformed deployment describe the implementation phase.
The live web app still needs its separate Schedule-selector deployment.

<a id="big12-sports-expansion-plan"></a>

## Big 12 sports calendar expansion proposal

Historical design proposal. Big 12 implementation is documented above; proposed changes are not all outstanding work.

Inspection date: October 9, 2026. **Implemented and deployed October 9, 2026.**
The proposal below records the pre-implementation inspection. Current operation:
[Big 12 ingestion guide](sports-schedules.md#big12-sports). Production now has 16 enabled school
feeds, 943 public events and 1,444 source observations, with 3 quarantined records.

<a id="big12-sports-expansion-plan--existing-configuration-and-live-schema"></a>

### Existing configuration and live schema

The only configured SportsEventsType row is ID 1, “Kansas – all sports”, SchoolName=Kansas,
with sport_id=0, school_id=3, schedule_id=0. Its 101 stored SportsEvents rows contain
Football (9), Men's Basketball (32), Women's Basketball (31), Soccer (10), and Volleyball (19).
The current live Kansas RSS contains 91 upcoming rows. Retained historical rows remain in SQL.

SportsEventsType has ID, RssUrl, EventsTypeName, DateAdded and SchoolName. RssUrl is unique.
It has no enable flag, provider school ID, polling timestamps or LastError.
SportsEvents has ID, Url, computed UrlHash, GameId, Title, Sport, Opponent, IsAway,
Location, EventDate, StartsAtUtc, EndsAtUtc, TimeTbd, Tv, StreamUrl, LiveStatsUrl,
TeamLogoUrl, OpponentLogoUrl, FirstSeenAt, LastSeenAt and SportsEventsTypeId.
UrlHash is unique; GameId is nullable and is not unique. Its only inspected foreign key
is SportsEventsTypeId -> SportsEventsType.ID. ResultGames/ResultFollows are separate
live tables; their inspected foreign keys do not reference SportsEvents.

The current MERGE matches the URL hash, then overwrites the school perspective and
SportsEventsTypeId. This cannot provide multi-feed attribution or deduplicate mirrored IDs.
The scraper logs feed errors but does not persist a per-calendar attempt/success/error status.
The existing school-logo fix resolves school_id through the calendar members component;
opponent logos still come from RSS. Preserve that behavior.

<a id="big12-sports-expansion-plan--provider-verification"></a>

### Provider verification

- Calendar and current full-member metadata: https://big12sports.com/calendar.aspx
- RSS pattern: https://big12sports.com/services/responsive-calendar-subscription.ashx/calendar.rss?sport_id=0&school_id={ID}&schedule_id=0
- Conference-wide RSS: the same URL with school_id=0; HTTP 200, 749 items.
- School/sport availability: https://big12sports.com/services/responsive-calendar.ashx/sport_schools?sport_id={SPORT_ID}
- Structured calendar: https://big12sports.com/services/responsive-calendar.ashx?start=2026-10-01&end=2026-10-31%2023:59:59&sport_id=0&school_id=0

Current members were selected from the provider members component using school_active=true
and member_type=F, then cross-checked against sport_schools. All 16 school-specific RSS
requests returned HTTP 200. Sport IDs: Football 4, Men's Basketball 7, Women's Basketball 15,
Soccer 31 (provider gender metadata identifies women's soccer), Volleyball 37 (women's).
All 16 are listed for the first four sports; volleyball lists 15, excluding Oklahoma State.
Affiliate schools in other provider sport lists are not automatically included.

Counts below are upcoming source observations, not deduplicated events or full-season guarantees.
All feeds use the RSS pattern above with their school ID.

| School | ID | Football | Men's basketball | Women's basketball | Soccer | Volleyball |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Arizona | 34 | 7 | 32 | 30 | 6 | 15 |
| Arizona State | 35 | 8 | 32 | 32 | 6 | 15 |
| Baylor | 1 | 7 | 33 | 31 | 6 | 13 |
| BYU | 32 | 8 | 32 | 32 | 6 | 14 |
| UCF | 31 | 7 | 32 | 32 | 6 | 14 |
| Cincinnati | 30 | 7 | 32 | 31 | 6 | 15 |
| Colorado | 11 | 7 | 32 | 30 | 6 | 14 |
| Houston | 33 | 7 | 32 | 31 | 6 | 13 |
| Iowa State | 2 | 7 | 33 | 31 | 6 | 14 |
| Kansas | 3 | 8 | 32 | 31 | 6 | 14 |
| Kansas State | 4 | 8 | 32 | 32 | 6 | 14 |
| Oklahoma State | 6 | 8 | 32 | 32 | 6 | 0 |
| TCU | 7 | 7 | 33 | 32 | 6 | 14 |
| Texas Tech | 9 | 7 | 32 | 30 | 6 | 13 |
| Utah | 36 | 8 | 32 | 30 | 6 | 13 |
| West Virginia | 10 | 7 | 32 | 30 | 6 | 13 |

<a id="big12-sports-expansion-plan--coverage-and-field-limitations"></a>

### Coverage and field limitations

The five-sport snapshot contains 1,434 school-feed observations. Exclude unrelated sports
(e.g. wrestling present in some all-sport feeds). Start with an explicit shared allowlist
of the five currently collected sports; review future scope changes rather than treating
an empty seasonal Kansas feed as proof that a sport should be removed.

RSS supplies opponent name/logo, vs/at perspective, local date, UTC start/end, venue,
TV text, video URL and live-stats links. Members metadata supplies official school name,
ID and logo. The JSON calendar supplies school/opponent IDs, sport ID/gender,
location_indicator (H/A/N), neutral_hometeam, explicit paired game IDs and media details.
Do not interpret every RSS “vs” as a home game: JSON identified 126 neutral-site records
in the sampled monthly five-sport metadata. For neutral events, use IsNeutral=true and
IsAway=NULL unless an explicit designated perspective is retained separately.

Missing in the five-sport RSS snapshot: 9 venues, 391 opponent logos, 1,147 TV labels,
1,223 video links and 1,252 live-stats links. 868 starts are date-only/TBD. All current
full members have school image URLs in members metadata. No member-opponent logo is
missing in this snapshot; absent opponent logos are outside that exact member-name set.
Store unknown values as NULL; do not invent times, logos, venues or broadcast availability.
Retain supplied nonempty broadcast information from either source perspective, with provenance;
50 paired games had differing TV values across the two RSS feeds. JSON also supplied
radio text on 96 and audio links on 2 sampled target-sport records; preserve these in
source metadata, with optional Radio/AudioUrl columns for public display.

Both conference-wide RSS and broad JSON queries omitted later games present in school RSS.
Conference RSS ended January 5, 2027, while the school RSS snapshot extends to March 6.
One September-April JSON query stopped January 16. Six bounded monthly JSON queries
(October-March) covered all 1,434 current target-sport source IDs via id or opp_game_id.
Request metadata in bounded months covering observed RSS dates and retained dates needed
for backfill; validate coverage and report unresolved IDs instead of assuming completeness.

JSON date_utc disagreed with timed RSS starts for 19 observed football records. For Kansas
at Utah, RSS reports October 11 02:15 UTC, while JSON reports October 10 05:00 UTC even
though its local date is October 10 21:15. Use RSS UTC/date fields as the existing timing
source; use JSON for identity/neutral/media enrichment, and log time disagreements.
Preserve supplied local calendar dates and source timezone labels; do not assume every
school is Central or substitute JSON UTC blindly. Source labels include CT, ET, MT, MST,
and missing values. NULL/TBD times must remain distinct from midnight placeholders.

<a id="big12-sports-expansion-plan--recommended-collection-and-identity-design"></a>

### Recommended collection and identity design

1. Keep the Kansas configuration row, URL, event IDs and historical rows. Add 15 school
   configurations using verified IDs, initially disabled for validation. Use one all-sport
   RSS feed per school, with the shared five-sport allowlist. Do not replace Kansas with
   a conference-wide RSS feed or enable unrelated provider sports.
2. Fetch/cache the members metadata once per collection pass. Use each feed school_id
   for names and logos. Use structured school/opponent/sport IDs for identity; keep RSS
   opponent extraction and use verified provider identity for enrichment when needed.
3. Use JSON id/opp_game_id mappings from bounded monthly requests to link mirrors.
   Example: Kansas at Utah has Kansas ID 179871 and Utah ID 179872; the two school JSON
   responses explicitly link them reciprocally. Their RSS URLs are different, so the
   existing UrlHash unique constraint would currently allow two public events.
4. For an explicitly linked pair, use a provider-namespaced stable group identity such
   as big12:pair:179871:179872 (sorted IDs). Unpaired games use big12:game:{id}. Dates,
   title text, time and logo must not be part of identity. Validate sport and participant
   IDs before linking; do not merge by teams/date alone, which would collapse doubleheaders.
5. Preserve the existing Kansas SportsEvents.ID as the canonical row when either alias
   already matches it. Add another source attribution rather than inserting another event.
   Resolve aliases before inserting; if a formerly unpaired game later receives a pair link,
   attach it to the existing event and update its canonical key in a transaction. If two
   existing canonical rows are found, flag a reconciliation conflict rather than deleting
   records or silently rewriting references. Historical backfill must resolve existing
   GameId/event-URL IDs before any insertion; leave unresolved old rows intact.
6. Preserve each school/feed perspective in a source junction table. Do not allow the
   latest feed to flip canonical TeamLogoUrl/OpponentLogoUrl, IsAway or SportsEventsTypeId.
   Existing rows keep their current primary perspective. New events can choose the home
   source initially, with a deterministic choice for neutral events, and keep that choice.
   Public listing reads one canonical SportsEvents row. A school's calendar uses the
   source/participant links, not only the single SportsEventsTypeId foreign key.

The snapshot contains 498 explicit mirrored pairs; all 498 matched reciprocal school
names and sports. Grouping them gives 936 candidate public events from 1,434 observations.
This is a validation count, not permission to publish every record: two observations list
Colorado/Colorado and Iowa State/Iowa State, and another Utah/Iowa State observation lacks
an explicit partner ID. Retain such source records for review; do not guess a match or
publish an unresolved duplicate.

<a id="big12-sports-expansion-plan--schema-changes-needed"></a>

### Schema changes needed

Keep the two existing tables and public event IDs. Use an additive migration:

- SportsEventsType: ProviderKey, ProviderSchoolId, Enabled, LastAttemptAt, LastSuccessAt,
  LastError and LastEventCount. Keep SchoolName and the existing unique RssUrl. Add a
  configurable sport scope (e.g. validated SportIds JSON) if it should be admin-managed.
  Preserve Kansas's existing row and enable setting during seeding; never re-enable
  disabled feeds or create a second row due only to query-parameter ordering.
- SportsEvents: a nullable, uniquely indexed CanonicalKey for staged historical backfill;
  explicit ProviderKey, SchoolId, SchoolName, OpponentSchoolId, ProviderSportId and IsNeutral.
  Retain GameId/Url/UrlHash as the primary source identity and public compatibility fields.
  CalendarTimeZone, Radio and AudioUrl are useful additions for fields absent today.
- New SportsEventSources junction: SportsEventId FK, SportsEventsTypeId FK, ProviderGameId,
  SourceUrl, provider school/opponent IDs and FirstSeenAt/LastSeenAt. Unique constraint on
  (SportsEventsTypeId, ProviderGameId). Preserve source perspective/schedule/media fields
  in validated JSON or dedicated columns, including school/opponent names/logos, IsAway,
  neutral status and raw source times. This is required for many feeds to attribute one
  public event without abusing the current single-feed foreign key.

Use unique canonical keys plus transaction/locking protection for concurrent polling.
Feed success/failure updates are independent; a failed feed must not erase the last successful
observations. Missing RSS entries do not delete history or prove cancellation. A partial
RSS/JSON metadata failure should record an enrichment issue and keep unresolved observations
out of duplicate public publication, while other feeds continue.

Without the junction table, the alternatives are losing secondary attribution or keeping
mirrored SportsEvents rows and deduplicating only in a view. Neither is the recommended
one-event/many-sources model. The .NET front end will need participant/source-link queries
for school filtering, plus IsNeutral support; no front-end source was changed here.

<a id="big12-sports-expansion-plan--small-validation-plan-before-rollout"></a>

### Small validation plan before rollout

1. Save representative Kansas/Utah mirrored RSS+JSON fixtures, Kansas State multi-word
   school names, Oklahoma State's absent volleyball, a neutral event, TBD/time disagreement,
   doubleheader and missing-logo cases. Validate each field and explicit pair identity.
2. On a staging copy, run the migration twice and backfill aliases for all 101 Kansas rows.
   Confirm unchanged event IDs, FirstSeenAt, logos and primary perspective; check historical
   GameId/URL matches before enabling other school rows.
3. Dry-run Kansas and Utah first. Expect one public event and two source attributions for
   179871/179872. Check merged nonempty broadcast fields, source-specific status, failed-feed
   isolation, neutral handling and atomic reschedules. Import twice and concurrently;
   event/source counts must stay stable, including retained Kansas history.
4. Dry-run all 16, compare per-sport observations against the table, validate monthly metadata
   coverage, review self-opponent/unpaired anomalies, then enable the 15 new school feeds.
   Verify public uniqueness and each school calendar before changing the production service.

The expansion is implemented and deployed. Staging repeated/concurrent imports
preserved event/source counts and all 101 original Kansas identities and logos.
The public primary source controls timing; conflicting secondary source values are
retained and reported. Setup flags cannot be combined with dry-run or scheduled mode.

