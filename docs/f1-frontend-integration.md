# F1 section: schedule, results and standings

Prepared October 18, 2026 for My-Newsfeed. Add **F1** in the sports area, with
**Schedule**, **Results**, and **Standings** tabs. Standings has **Drivers** and
**Constructors** views. Reuse the site's sports navigation, season label, loading,
error and mobile table patterns.

The scraper collects structured JSON from [Jolpica F1](https://github.com/jolpica/jolpica-f1),
the Ergast-compatible F1 data API. These are structured sports feeds exposed by
SQL views, rather than RSS article feeds. Browser clients should read through the
frontend's existing server-side database/API layer, not call Jolpica directly.
No HTTP endpoints for My-Newsfeed were created in the scraper repository.

## Read contract

Four production views expose one complete, most recently fetched season snapshot.
They automatically hide data when `dbo.F1Source.Enabled=0`. Source ID **1** is
specific to the new `F1Source` table; it is not a shared SportsEventsType or
SportsStandingsSources ID. Resolve the current season from the returned data.

| View | Row identity | Contents |
| --- | --- | --- |
| `dbo.F1Schedule` | `(Season, Round)` | Entire season calendar and available weekend sessions |
| `dbo.F1Results` | `(Season, Round, SessionType, DriverId)` | Classified drivers in each Grand Prix or sprint |
| `dbo.F1DriverStandings` | `(Season, DriverId)` | Drivers' championship points, wins, rank, constructor list |
| `dbo.F1ConstructorStandings` | `(Season, ConstructorId)` | Constructors' championship points, wins, rank |

All views include `FetchedAt` (UTC). Standings also includes `StandingsRound`,
which is the provider's as-of round; show it independently of the next scheduled
race. Source health comes from `dbo.F1Source.LastAttemptAt`, `LastSuccessAt`, and
`LastError`. Do not expose raw operational errors to public users. Failed
refreshes preserve the previous complete snapshot and its original timestamp.

These views preserve a full season's races/results, including past races. They
are independent of match-schedule pruning and do not model a race as home/away
teams or a football-style score. Only the latest collected season is retained;
a historical season selector requires additional backend work.

## Schedule

```sql
SELECT * FROM dbo.F1Schedule
WHERE Season=@Season
ORDER BY Round;
```

Display Round, Grand Prix (`RaceName`), circuit (`CircuitName`), location
(`Locality`, `Country`), and race date/time. `RaceUrl`, when available, is the
provider's race reference link, often Wikipedia; it is not a broadcast stream.

`RaceDate` is a SQL date. `RaceTimeUtc` is a nullable UTC string such as
`04:00:00Z`. Combine them only when the time exists and convert with the site's
existing timezone formatter. With an absent time, display the calendar date and
“TBD” for time; do not invent a midnight start or convert a date-only value as a
UTC instant.

The following columns are nullable JSON objects containing `date` and optional
`time`: `FirstPractice`, `SecondPractice`, `ThirdPractice`, `Qualifying`,
`Sprint`, `SprintQualifying`, and `SprintShootout`. Parse only available sessions
and sort chronologically for an expandable weekend timetable. Include the race
itself using `RaceDate`/`RaceTimeUtc`. Sprint weekends do not necessarily have
three practice sessions. `SprintShootout` accommodates older provider naming;
avoid duplicate qualifying entries if equivalent fields are supplied.

Use a future/current-weekend filter for the upcoming display and an optional
full-calendar display. Do not derive “completed” solely from the scheduled start
passing. A race with Race result rows has published classifications; a past race
without them should say “Results pending/unavailable,” not “Canceled” or “Final.”
The provider is not a live timing or authoritative cancellation-status service.

## Results

Populate the race selector from `F1Schedule`, with Grand Prix and sprint options
when corresponding classifications exist. Results uses the session values
**`Race`** and **`Sprint`** exactly.

```sql
SELECT r.*, c.RaceName, c.CircuitName, c.RaceDate
FROM dbo.F1Results r
JOIN dbo.F1Schedule c ON c.Season=r.Season AND c.Round=r.Round
WHERE r.Season=@Season AND r.Round=@Round AND r.SessionType=@SessionType
ORDER BY r.Position;
```

Columns for the table:

| UI field | SQL field |
| --- | --- |
| Position | `PositionText` (order with numeric `Position`) |
| Driver | `GivenName` + `FamilyName`; optional `DriverCode` |
| Team | `ConstructorName` |
| Starting grid | `Grid`; zero may mean pit-lane/unavailable, not “P0” |
| Laps | `Laps` |
| Time / gap | `TimeDisplay` exactly as supplied |
| Status | `Status`, including retirements, disqualification, laps behind |
| Points | `Points` as a decimal, preserving fractional points |

`TimeMillis`, `FastestLapTime`, and `FastestLapRank` are nullable optional detail
fields. A finisher's time can be a gap such as `+5.123`; it is not necessarily an
absolute duration. Do not convert missing times into zero or award extra points
from fastest lap: championship rules vary and provider points already reflect
the published classification. Keep Race and Sprint results separate.

## Standings

```sql
SELECT * FROM dbo.F1DriverStandings
WHERE Season=@Season ORDER BY Position;

SELECT * FROM dbo.F1ConstructorStandings
WHERE Season=@Season ORDER BY Position;
```

Drivers: Position, driver name/code, team(s), points, wins.
`ConstructorsJson` is an array of provider constructor objects with
`constructorId`, `name`, `nationality`, and optional `url`. A driver may have
multiple constructors in one season; display the supplied list rather than
assuming its first entry is the current team. Nationality is also available on
the driver row.

Constructors: Position, `ConstructorName`, Points, Wins; Nationality is optional.
Order both tables by numeric `Position`, retain `PositionText` for display, and
never recompute ranks or championship points in the UI. Points are decimals.
No driver portraits, team logos, or live timing feed are supplied by this change.

## Empty data, freshness and permissions

Before the season's first race, schedule data can exist while results and
standings are empty. Show an appropriate empty state, not a failed-page message.
If a refresh fails, render the last stored snapshot with its update time and the
site's normal stale-data treatment. A database failure is an error, not an empty
season. This is periodic collection, not live lap-by-lap timing; corrections
appear on a successful later refresh.

Reuse the existing frontend database reader identity. The schema migration
provisions the scraper's ingestion permissions, not a new frontend login. If the
frontend already has schema-wide SELECT or db_datareader, no extra grant is
needed. Otherwise its migration owner should grant SELECT on these four views
(and `F1Source` only if operational freshness is read) to its existing reader
role. Do not grant browser access, runtime DDL, or write access.

Keep standard HTML encoding, JSON parsing and validated outbound URLs. Add a
visible data-source attribution link: **Data: Jolpica F1** linking to
https://github.com/jolpica/jolpica-f1. Do not describe it as an official F1 feed.

## Acceptance checks

1. Sports navigation exposes F1 with Schedule, Results and both championship tables.
2. The calendar shows available sessions with correct timezone conversion and
   date-only/TBD behavior. It supports both ordinary and sprint weekends.
3. Race and sprint selectors show separate classifications, ordered positions,
   team names, laps, retirement/status information and decimal points.
4. Championship tables use provider rankings, all supplied drivers/constructors,
   and the published as-of round. No hardcoded grid or team count.
5. Missing times, gaps, optional fastest-lap details, empty preseason data, and
   multiple teams for a driver render correctly on desktop and mobile.
6. Disabled configuration hides the feed; a failed update preserves prior data
   and freshness. Other sports remain unchanged.
7. Verify the actual deployed frontend reader can SELECT the views and verify
   rendered production content. A successful import alone does not prove the
   frontend implementation is complete.

Backend setup, refresh commands and deployment status: [F1 operations](f1-operations.md).
