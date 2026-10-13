# NFL standings — frontend handoff

## Deployed data

NFL standings were deployed October 13, 2026. Production source ID is **10**.
The current configuration stores **32 teams for 2026**, with eight division groups
of four teams each, nested under AFC and NFC in the frontend.
The existing hourly scraper refreshes this source automatically.

| Setting | Value |
| --- | --- |
| ProviderKey | `espn-nfl` |
| Sport | `Football` |
| DisplayName | `NFL` |
| MinimumTeams | `32` |
| Enabled | `1` |
| Provider URL | `https://site.api.espn.com/apis/v2/sports/football/nfl/standings?level=3` |
| Existing frontend route | `/standings?sourceId=10` |

Resolve the source by ProviderKey when configuring another environment; its
numeric ID may differ. The source follows ESPN's published current season.
Display stored Season/Title instead of substituting the calendar year.

Use the existing contract in [the standings frontend guide](my-newsfeed-standings-frontend.md).
No database migration or new read permissions are needed for an application that
already reads SportsStandingsSources and SportsStandings.

## Frontend changes

1. Include NFL in the Standings source selector. Read enabled sources dynamically
   rather than keeping a fixed list of supported leagues.
2. Render AFC and NFC conference headings, each with East, North, South and West
   division tables. Each division contains four teams.
3. Use the compact NFL columns below instead of the generic list of every statistic.
4. Preserve source selection in the URL, season labels, refresh status and the
   existing last-good, empty and error states.

The deployed My-Newsfeed page already discovers NFL and renders its generic
statistic columns. The recommended adjustment is a compact NFL column mapping;
no web-app rebuild was required for initial availability. NFL standings are
independent of schedule coverage and must include teams without upcoming games.

## Queries and model

Source selector:

```sql
SELECT ID, ProviderKey, Sport, DisplayName, LatestSeason,
       LastSuccessAt, LastError, LastTeamCount
FROM dbo.SportsStandingsSources
WHERE Enabled = 1
ORDER BY DisplayName, ID;
```

Current selected-source standings (parameterize @SourceId):

```sql
SELECT r.*
FROM dbo.SportsStandings r
JOIN dbo.SportsStandingsSources s ON s.ID = r.SourceId
WHERE s.Enabled = 1 AND r.IsCurrent = 1 AND r.SourceId = @SourceId
ORDER BY r.GroupName, r.Position, r.TeamName;
```

Group rows by GroupId and display GroupName. Use TeamName, LogoUrl and TeamUrl
from the row; ESPN team IDs are scoped to the provider. Keep Position as provider
order within a conference. Do not infer a league-wide rank or playoff qualification.
With `level=3`, GroupId identifies the division and GroupName is `AFC East`,
`AFC North`, `AFC South`, `AFC West`, `NFC East`, `NFC North`, `NFC South`, or
`NFC West`. Conference membership follows the AFC/NFC prefix. No new columns
are needed; conference membership is derived from the published division name.

Render conferences in AFC, NFC order, and divisions in East, North, South, West
order rather than sorting numeric GroupId. Within each division, preserve Position.
The existing flat group renderer already displays eight division tables; add a
conference heading and wrap its four tables for the requested hierarchy.

```csharp
string[] conferences = ["AFC", "NFC"];
string[] divisions = ["East", "North", "South", "West"];
foreach (var conference in conferences)
{
    // Render the conference heading.
    foreach (var division in divisions)
    {
        var group = groups.FirstOrDefault(g => g.Name == $"{conference} {division}");
        if (group is null) continue;
        // Render the existing StandingsTable for group.
    }
}
```

If a future response contains an unrecognized group name, retain it in a fallback
section rather than silently dropping its teams. Historical conference-level rows
remain stored with IsCurrent=0; the current-row query excludes them.

## NFL columns

StatsJson is an array of statistics. Match the case-sensitive `name` and render
`displayValue`. Ignore additional properties and show an em dash for unavailable
values. Do not calculate missing records, percentages or point differentials.

| Column | Statistic name |
| --- | --- |
| W | `wins` |
| L | `losses` |
| T | `ties` |
| Pct | `winPercent` |
| PF | `pointsFor` |
| PA | `pointsAgainst` |
| Diff | `pointDifferential` |
| Streak | `streak` |

Optional columns: Home=`Home`, Away=`Road`, Division=`divisionRecord`.
Start with the Team column and its optional logo. Preserve supplied formatting
for percentages, streaks, signs and record strings.

For the existing My-Newsfeed code, add this branch before the generic fallback in
`src/MyNewsFeed.Web/Data/StandingsQuery.cs`, inside `Columns`:

```csharp
if (provider == "espn-nfl")
    return [new("wins", "W"), new("losses", "L"), new("ties", "T"),
        new("winPercent", "Pct"), new("pointsFor", "PF"),
        new("pointsAgainst", "PA"), new("pointDifferential", "Diff"),
        new("streak", "Streak")];
```

The existing `StandingColumn` and `StatisticValue` helpers support this mapping.
Keep the table usable on mobile with horizontal scrolling and accessible headers.

## Refresh states

Use LastSuccessAt for the last successful refresh and convert UTC timestamps to
America/Chicago for display. If LastError is populated, retain the last-good
standings and show the existing stale/error indication. Do not replace records
with zeros. If the enabled source has no current rows, show an empty state.
Do not offer disabled sources or display historical rows with IsCurrent=0.

## Verification checklist

- NFL appears in the selector and direct source URLs preserve selection.
- The 2026 snapshot shows 32 unique teams across eight four-team divisions.
- AFC/NFC headings each contain East, North, South, West; Kansas City is in AFC West.
- W/L/T, percentages and points match the stored displayValue fields.
- Team order within each division follows Position; no invented overall ranking is shown.
- Missing statistics, logos and links are handled gracefully.
- Disabled, empty, stale and failed-refresh states preserve existing behavior.
- Existing MLB, WNBA, soccer and Big 12 column mappings still work.

Deployment details: [NFL deployment report](../reports/nfl-standings-deployment-2026-10-13.md).
