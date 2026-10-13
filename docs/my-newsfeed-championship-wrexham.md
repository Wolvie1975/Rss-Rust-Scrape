# EFL Championship schedule, table and results — frontend handoff

## Scope and identity

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

## Schedule queries

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

## League table

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

## Results model and queries

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

## Suggested frontend flow

Add EFL Championship to sports and standings selection. Offer Schedule, Table
and Results tabs. Place a Wrexham AFC toggle on Schedule/Results and keep the
whole league table visible with Wrexham highlighted. Preserve filters in URLs,
for example `league=championship&team=352` or the site's existing query convention.
On small screens, allow the standings table to scroll horizontally.

The existing standings selector discovers new enabled sources dynamically.
The dedicated Wrexham filter and results view require frontend implementation;
this scraper task does not rebuild the separate My-Newsfeed application.

## Checks

- Table has 24 unique teams and includes/highlights Wrexham ID 352.
- Schedule excludes past/canceled/final games; Wrexham filter works home and away.
- Results contain final scores only and use current-season selection.
- Re-imports and score corrections preserve row IDs and FirstSeenAt.
- Disabled feeds and failed refreshes preserve existing status behavior.
- Past schedule cleanup does not remove SportsMatchResults.
- Existing NWSL results synchronization and other leagues remain independent.
