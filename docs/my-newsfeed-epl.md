# Premier League — frontend handoff

Scope: the current Premier League table and today's/future fixtures, across the
whole league. Historical EPL fixtures and match results are not imported.
ProviderKey is `espn-epl`, sport Soccer, competition `eng.1`, ESPN league ID 700.
The initial season is 2026–27 (stored standings Season `2026`). Use provider labels.

The scraper's existing hourly events and standings jobs refresh enabled sources.
Sources are registered with `--configure-epl`, disabled by default; repeated setup
preserves configured URLs and enable settings. Production schedule source ID is **5357** and standings source ID is **12**.
The existing table route is `/standings?sourceId=12`.

## Fixtures

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

## Current table

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
