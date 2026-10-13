# My-Newsfeed MLS standings

Add **MLS** to the existing Standings source selector. Show the full national
league as separate Eastern and Western Conference tables, preserving provider
order/rank within each group. Do not filter to Sporting Kansas City or to teams
with upcoming games. This change adds standings only, not MLS schedules/results.

## Data contract

Use the existing SportsStandingsSources/SportsStandings models and queries from
[the standings frontend handoff](my-newsfeed-standings-frontend.md).

- ProviderKey: `espn-mls`; Sport: `Soccer`; DisplayName: `MLS`.
- API: `https://site.api.espn.com/apis/v2/sports/soccer/usa.1/standings`.
- Verified 2026-10-11: 2026 MLS, 30 teams, 15 Eastern / 15 Western.
- Use stored TeamName, LogoUrl, TeamUrl, GroupName, Season and Title.
- ESPN team IDs remain scoped by provider; do not resolve them via Big 12 metadata.
- Columns use the soccer mappings already used for NWSL: GP=`gamesPlayed`,
  W=`wins`, D=`ties`, L=`losses`, GF=`pointsFor`, GA=`pointsAgainst`,
  GD=`pointDifferential`, Pts=`points`. Render each statistic's displayValue;
  missing values remain unavailable rather than being calculated.
- Preserve conference groups. Position is display order; display Rank only when
  supplied. Do not invent a league-wide rank or Supporters' Shield ordering.
- Retain source season labels and UTC refresh timestamps, with Central display
  conversion and the same last-good/stale/empty-state behavior as other standings.
- Queries must include both source Enabled=1 and row IsCurrent=1. No schedule
  today-forward date restriction belongs on standings.

## Configuration and deployment

No new schema or privileges are required beyond the existing standings tables.
The source is added to `sql/standings_seed.sql`; re-running --configure-standings
with migration credentials registers MLS disabled and preserves other settings.

```sql
UPDATE dbo.SportsStandingsSources SET Enabled=1 WHERE ProviderKey='espn-mls';
```

Deploy the updated scraper before importing the new provider. The existing
--standings-from-db hourly flag collects enabled MLS alongside the other tables.
Optional `?season=2026` pins a season; the default follows ESPN's current season.
Existing atomic snapshots, stable IDs and incomplete-response guards are reused.

Production deployment/enabling completed 2026-10-11: 30 MLS teams are stored and
the existing /standings?sourceId=9 page displays both conferences.
The separate My-Newsfeed frontend must recognize espn-mls as soccer for its
column mappings if it branches on provider keys. If its selector is database-driven,
the MLS option appears when enabled, including when rows are not available yet.
