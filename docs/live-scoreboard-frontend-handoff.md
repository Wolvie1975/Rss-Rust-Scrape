# Live scoreboard: frontend return handoff

Implemented and deployed October 23, 2026 (America/Chicago). Verified at approximately
10:18 p.m. CDT (October 24, 03:18 UTC). This replaces proposed backend details in
`New Development/live-scoreboard-scraper-handoff.md` with the deployed contract.

Current retention update: scoreboard history is limited to the previous Central
calendar day. Today and future games remain available; see navigation below.

## Ready for frontend implementation

The new independent **web-scoreboard** service, image **web-scoreboard:previous-day-20261023**,
collects persisted scoreboard samples. Query **dbo.SportsScoreboard** for games and
**dbo.SportsScoreboardCoverage** for enabled coverage. No browser request should
trigger upstream collection. Frontend implementation and rollout are managed separately from this scraper task.

Initial population plus the corrected football import produced **257 distinct
games**, **20 enabled coverage sources**, and **13 sport/league combinations**.
Two college basketball leagues currently have no games in the request window.
The first deployed-container run refreshed four due league/date requests with
zero failures or partial responses, including actual live MLB, NHL and NWSL samples.
These counts and statuses are dated observations, not permanent coverage promises.

## Coverage and ownership

All providers below are ESPN. Scope is resolved dynamically from enabled
SportsEventsType and ResultFollows rows; numeric IDs are not deployment constants.

| Sport / League keys | Enabled collection scope | Live details when supplied |
| --- | --- | --- |
| football / nfl | League follow | Score, period, sampled clock, status label |
| football / college-football | Big 12 conference plus Kansas follow | Same; regular/postseason only |
| baseball / mlb | League follow plus existing schedule feed | Inning label, outs/count/bases, inning scores |
| basketball / wnba | League follow plus existing schedule feed | Period, clock, period scores |
| basketball / mens-college-basketball | Big 12 conference plus Kansas follow | Same; no games in current window |
| basketball / womens-college-basketball | Big 12 conference follow | Same; no games in current window |
| hockey / nhl | Existing league schedule feed | Period, clock, OT/SO status, shootout values |
| soccer / usa.1 | League follow plus existing schedule feed | Clock, period, status, shootout values |
| soccer / usa.nwsl | League follow plus existing schedule feed | Same |
| soccer / eng.1 | Existing league schedule feed | Same |
| soccer / eng.2 | Existing league schedule feed | Same |
| soccer / usa.ncaa.w.1 | Big 12 conference follow | Same; sport-specific team IDs from validated membership |
| volleyball / womens-college-volleyball | Big 12 conference plus Kansas follow | Sets won, set number, per-set scores, status label |

Conference membership comes from the web worker's existing ResultFollowTeams
mapping. It is read afresh each collector run; this collector does not write it.
College team/conference follows preserve the existing regular/postseason policy;
women's college soccer follows the existing default-season policy. The MLB
schedule feed's pinned season is honored for that association; the independent
league follow retains its own broader eligibility.

No disabled feed is enabled, and team/conference coverage does not become a
national feed. Football uses ESPN's FBS group 80; college basketball uses group 50.
Other sports omit that filter, notably volleyball, where group 50 omitted games.
Provider responses are deduplicated before retaining only eligible games.

Known gaps: other Big 12 calendar sports have no verified live adapter here; NBA
has no enabled follow/feed and is not collected. F1 remains outside this matchup
model. Aggregate totals and watch URLs are not collected in this release. Broadcast
names are available when supplied. Optional provider fields may be absent, and
no upstream real-time availability or latency guarantee is made.

Storage is scraper-owned and additive. ResultGames/ResultFollows and their web
synchronization worker remain intact; SportsEvents, SportsMatchResults and existing
results readers are unchanged. HomeEventResults is an existing frontend helper,
not a SQL view this release replaces. Continue using it for existing flows until
the frontend intentionally adopts the scoreboard.

## Read model and type mapping

SportsScoreboard returns one row per **(Provider, Sport, League, EventId)**.
Provider is currently `espn`. Event IDs and team IDs are strings. Use the namespace
Sport + League + TeamId for browser team preferences; unknown team IDs cannot be
used as stable preferences. Do not join participants by their display names.

| Fields | SQL types / semantics |
| --- | --- |
| ID | BIGINT → .NET long |
| Provider, Sport, League, EventId | Required strings; EventId NVARCHAR(80) |
| ScoreboardDate | DATE → DateOnly; current Central calendar day assignment |
| OriginalScoreboardDate | DATE; first observed day assignment |
| ProviderDate | DATE; Eastern-calendar provider request day, operational metadata |
| StartsAtUtc, OriginalStartsAtUtc | Nullable DATETIME2 → DateTime? marked UTC |
| TimeTbd, WasRescheduled | Required BIT → bool |
| HomeTeamId, AwayTeamId | Nullable NVARCHAR(80) |
| HomeName, AwayName | Required NVARCHAR(200), fallback TBD |
| HomeLogoUrl, AwayLogoUrl | Nullable safe HTTP(S) URL strings |
| HomeScore, AwayScore | Nullable DECIMAL(12,3) → decimal?; do not zero-fill |
| Status | Required normalized string; vocabulary below |
| Completed | BIT; true only for supported provider-completed finals |
| ProviderStatus, StatusText | Raw code (required) and optional descriptive label |
| Period, DisplayClock | Nullable INT and NVARCHAR(80); sampled values only |
| NeutralSite | Nullable BIT; home designation does not imply venue advantage |
| HomeShootout, AwayShootout | Nullable INT, separate from the principal score |
| WinnerSide | Nullable `home` / `away`, only when provider explicitly identifies a final winner |
| Venue, Broadcasts, SourceUrl | Nullable text / comma-separated broadcast names / safe provider event URL |
| LiveDetailsJson | Nullable JSON text, schema below |
| ProviderUpdatedAtUtc | Nullable provider revision timestamp |
| LastObservedAtUtc | Required successful sample request-start UTC timestamp |
| FirstSeenAtUtc, FinalFirstObservedAtUtc | Required first-seen and nullable first-final observation timestamps |
| IsStale | Computed BIT: active/paused sample older than 25 minutes |
| HasBoxScore | Computed BIT; optional cache presence only |
| BoxScoreSummaryUrl, BoxScoreUrl | Nullable URLs from existing box-score cache |
| RequestLastAttemptAt, RequestLastSuccessAt, RequestNextAttemptAt | Nullable UTC timestamps for this game’s latest provider-day request |
| RequestAvailability | Nullable `pending`, `available`, `empty`, `partial`, `error` |

Current supported scores are integers, but decimal storage avoids lossy integer
casts. Display integral values without `.000`. Negative values, unsupported
precision beyond three decimals and invalid scores are rejected, retaining the
last-good game. Upcoming placeholder scores are stored as NULL. Actual live 0–0
scores remain zero. A final lacking valid scores is not published as a new sample.

LiveDetailsJson is JSON text, not an already-deserialized object:

```json
{
  "statusLabel": "Top 9th",
  "shortLabel": "Top 9th",
  "homePeriods": [{"period": 1, "score": 2, "displayValue": "2"}],
  "awayPeriods": [{"period": 1, "score": 0, "displayValue": "0"}],
  "city": "Milwaukee",
  "outs": 1, "balls": 1, "strikes": 0,
  "onFirst": true, "onSecond": true, "onThird": false
}
```

Baseball count/base fields appear only for baseball and can be NULL. Inning half
uses the supplied statusLabel; do not infer it from the home/away score. The period
arrays represent innings/quarters/halves/sets as appropriate, including extras.
Period numbers can use provider order when no explicit number is supplied. Empty
arrays mean unavailable period detail. Baseball and volleyball DisplayClock are
NULL; use their status/inning/set labels. All other clocks stay fixed at the sample.
Never run a local ticking game clock or infer a winner from a live score.

## Status display and freshness

| Status | Suggested frontend behavior |
| --- | --- |
| upcoming | Start time in Central, or TBD; broadcasts when present |
| live | Scores, explicit status label and sampled clock/period |
| final | Final scores, supplied OT/SO label, optional box-score link |
| postponed | Postponed; show a revised start only if supplied |
| cancelled | Cancelled; never count as a completed final |
| suspended | Suspended; keep last-good score with explicit label |
| delayed | Delayed; keep last-good score with explicit label |
| unknown | Status unavailable; never imply active play or completion |

Explicit postponed/cancelled/suspended/delayed codes override a provider's generic
post/completed flags. Halftime, intermission, overtime and shootout descriptions
remain in StatusText/ProviderStatus. Do not replace those with a fabricated clock
or change regulation scores to include separate shootout values.

Show **Scores update approximately every 10 minutes** and **Updated [time]** from
LastObservedAtUtc. Poll persisted reads every **five minutes while visible**, with
an immediate read on return to the page. Backend cadence:

- Live, delayed, suspended, or starting within one hour: approximately ten minutes.
- Newly observed finals: 15 minutes for the first hour, then hourly while on the
  current provider day and daily for past days within reconciliation range.
- Other upcoming/inactive days: hourly. Past final-only/empty days: daily.

Shared requests use the most frequent applicable interval. A minute-level due-work
loop and provider latency can add delay. Live interval is configurable from ten
to fifteen minutes. Failures back off from 15 minutes to four hours; Retry-After
can extend that. No public ESPN SLA or supported quota was verified.

If IsStale is true, show **Scores may be outdated — last updated [time]**, retaining
last-good values. If RequestAvailability is error/partial, show a short availability
message even before the stale threshold. An empty successful request does not
refresh a missing event; use LastObservedAtUtc rather than request success as its
Updated timestamp. Never claim a score changed just because its sample refreshed.

Coverage LastAttemptAt / LastSuccessAt describe any request for that source, not
all its dates. Raw upstream errors are never part of the public read contract.
Missing games do not mean cancellation. Database read errors should show a retryable
availability message without clearing an already-displayed last-good card.

## Parameterized frontend queries

Bind sport/league filters from enabled coverage; use separate fixed query shapes
when a filter is absent. Do not interpolate provider strings into SQL. Coverage
is automatically filtered for current enablement, and SportsScoreboard uses EXISTS
so overlapping enabled sources do not multiply rows.

```sql
SELECT ID, SourceKind, OriginId, Sport, League, Label,
       LastAttemptAt, LastSuccessAt, Availability
FROM dbo.SportsScoreboardCoverage
ORDER BY Sport, League, Label;
```

Deduplicate sport/league selector options in the application. SourceKind + OriginId
identifies the configured origin; ID is the scoreboard source-association identity.
Disabling an origin immediately hides its rows unless another enabled association
covers the game. Retained scoreboard history is not deleted by disabling a source.

For a selected Central calendar day, calculate **both midnight boundaries** using
America/Chicago in the application; convert each independently to UTC. Never add
24 hours to the first UTC boundary across daylight saving changes. SQL Server's
equivalent zone name is `Central Standard Time`.

```sql
-- Bound @SelectedDay DATE, @DayStartUtc DATETIME2, @DayEndUtc DATETIME2,
-- @Sport NVARCHAR(40), @League NVARCHAR(60).
SELECT ID, Provider, Sport, League, EventId, ScoreboardDate,
       StartsAtUtc, TimeTbd, WasRescheduled,
       HomeTeamId, HomeName, HomeLogoUrl, HomeScore,
       AwayTeamId, AwayName, AwayLogoUrl, AwayScore,
       Status, Completed, ProviderStatus, StatusText, Period, DisplayClock,
       NeutralSite, HomeShootout, AwayShootout, WinnerSide,
       Venue, Broadcasts, SourceUrl, LiveDetailsJson,
       LastObservedAtUtc, IsStale, RequestAvailability,
       HasBoxScore, BoxScoreSummaryUrl, BoxScoreUrl
FROM dbo.SportsScoreboard
WHERE ScoreboardDate=@SelectedDay AND Sport=@Sport AND League=@League
  AND (TimeTbd=1 OR (StartsAtUtc>=@DayStartUtc AND StartsAtUtc<@DayEndUtc))
ORDER BY CASE WHEN Status='live' THEN 0 ELSE 1 END, StartsAtUtc, ID;
```

For example, Central November 1, 2026 spans **05:00 UTC November 1 to 06:00 UTC
November 2**, a 25-hour day. UTC timestamps from SQL DATETIME2 must explicitly be
marked UTC in .NET before display conversion.

The homepage should combine today's games with carried-over active games, then
deduplicate by canonical identity. For the second read:

```sql
-- Bound @TodayCentral DATE. Add fixed sport/league predicates if selected.
SELECT ID, Provider, Sport, League, EventId, ScoreboardDate,
       HomeName, AwayName, HomeScore, AwayScore,
       Status, StatusText, Period, DisplayClock,
       LastObservedAtUtc, IsStale, RequestAvailability, HasBoxScore
FROM dbo.SportsScoreboard
WHERE Status IN('live','delayed','suspended')
  AND ScoreboardDate<@TodayCentral
ORDER BY ScoreboardDate, StartsAtUtc, ID;
```

Keep an overnight game on its assigned day and include it in the current active
section. Rescheduling updates ScoreboardDate/StartsAtUtc while retaining originals
and WasRescheduled. TBD/date-only events use their explicit date or provider day
and a NULL start; never present a guessed kickoff. Indexed date/status paths and
indexed association checks support these reads without browser-triggered imports.

Current navigation is **yesterday through seven days ahead**, using America/Chicago
calendar days. Retention keeps today, the previous day and future games; older
scoreboard rows are removed. This is a calendar-day policy, not a rolling 24-hour
window. Even older active/paused games expire at that boundary. The collector
requests only one previous provider day, and filters older Central dates before
persisting them. Existing schedule/results/box-score history is independent. A day with
no games is different from unavailable data; consult coverage freshness and show
that distinction. Request-level empty/partial outcomes are available on existing
game rows; an empty date has no fabricated game row.

## Existing schedule, results and box-score identity

Reuse the current box-score link builder with **(Sport, League, EventId)** when
HasBoxScore=true. Existing storage is dbo.SportsBoxScoreCache; dbo.SportsBoxScores
is its public read view. A missing/failed box score must not hide a scoreboard game.
Current enrichment primarily targets completed games, so live games often have no
cached box score. Browser reads must not fetch upstream statistics.

ESPN schedule canonical keys are `espn:{League}:{EventId}`. ResultGames uses
Sport/League/EventId; SportsMatchResults provider-specific IDs map only through
known ESPN league mappings. Never compare a bare numeric GameId across providers.

There is **no validated bridge from Big 12 calendar game IDs to ESPN events** in
this release. For covered competitions, render scoreboard games directly rather
than joining by team names or concatenating IDs from the calendar. Existing other
calendar sports can remain in their existing schedule section. Avoid appending
both covered calendar fixtures and scoreboard games into one list without a
validated deduplication strategy. This is an explicit limitation, not a silent
change to the existing homepage filtering helper.

## Examples and validation

The companion examples below are selected public fields from actual production
rows at verification. They are samples, not a current-score claim. The stale
example is illustrative and was not inserted into production.

Live sample:

```json
{
  "EventId": "401908042",
  "Provider": "espn",
  "Sport": "baseball",
  "League": "mlb",
  "HomeName": "Milwaukee Brewers",
  "AwayName": "Tampa Bay Rays",
  "HomeScore": 3.0,
  "AwayScore": 1.0,
  "Status": "live",
  "Completed": false,
  "StatusText": "Top 9th",
  "ScoreboardDate": "2026-10-23",
  "StartsAtUtc": "2026-10-24T00:00:00",
  "LastObservedAtUtc": "2026-10-24T03:18:21.9887404",
  "IsStale": false,
  "HasBoxScore": false
}
```

Final sample:

```json
{
  "EventId": "401918307",
  "Provider": "espn",
  "Sport": "basketball",
  "League": "wnba",
  "HomeName": "Atlanta Dream",
  "AwayName": "Golden State Valkyries",
  "HomeScore": 97.0,
  "AwayScore": 71.0,
  "Status": "final",
  "Completed": true,
  "StatusText": "Final",
  "ScoreboardDate": "2026-10-21",
  "StartsAtUtc": "2026-10-22T00:00:00",
  "LastObservedAtUtc": "2026-10-24T03:18:22.2659977",
  "IsStale": false,
  "HasBoxScore": true
}
```

Upcoming sample:

```json
{
  "EventId": "401908043",
  "Provider": "espn",
  "Sport": "baseball",
  "League": "mlb",
  "HomeName": "Milwaukee Brewers",
  "AwayName": "Tampa Bay Rays",
  "HomeScore": null,
  "AwayScore": null,
  "Status": "upcoming",
  "Completed": false,
  "StatusText": "Scheduled",
  "ScoreboardDate": "2026-10-24",
  "StartsAtUtc": "2026-10-25T00:00:00",
  "LastObservedAtUtc": "2026-10-24T03:03:07.1001589",
  "IsStale": false,
  "HasBoxScore": false
}
```

Illustrative stale rendering (no fabricated clock advancement):

```json
{"Status":"live","HomeScore":3,"AwayScore":1,"StatusText":"Top 9th","LastObservedAtUtc":"2026-10-24T02:40:00Z","IsStale":true,"RequestAvailability":"error"}
```

Backend validation passed: 206 offline tests, one explicit rollback-only SQL
integration test, Clippy with warnings denied, format check, repeated additive
setup, zero retained fixture rows and zero duplicate scoreboard identities. The
deployed container completed its first due run successfully; its health check and
the existing scraper/frontend health checks passed. The actual frontend database
principal successfully read both views. The web application's UI was not changed.

Frontend acceptance should cover all status labels, zero vs NULL scores, sampled
clocks, stale/partial/empty states, overnight carry-over, DST boundaries, reschedules,
disabled sources, duplicate follows, unknown participants, preferences by stable
team identity, optional box scores, mobile layout and visible-only five-minute
refresh. Run frontend tests/build and verify the homepage and Scoreboard page
before claiming that UI is deployed.

Setup and rollback: [operations guide](live-scoreboard-operations.md).
Deployment evidence: [release report](../reports/live-scoreboard-20261023.md).
Frontend runtime needs SELECT only on the two views; migrations provision the
WebScraperScoreboardReader role for an app-specific user. Existing frontend
credentials were not changed. No new frontend upstream polling, migrations or
scoreboard write permissions are needed.
