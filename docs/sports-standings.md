# Sports standings for My-Newsfeed

For the frontend build, use [the dedicated My-Newsfeed handoff](my-newsfeed-standings-frontend.md).

NFL is supported through `espn-nfl`, using ESPN football/nfl standings. The source
follows the published season and imports all 32 teams in eight AFC/NFC division groups using `?level=3`, including
ties, points and division records in StatsJson. MinimumTeams=32 rejects partial
snapshots. Existing hourly standings collection includes enabled NFL automatically.

MLS was added after the original eight-source deployment; see [MLS setup and frontend notes](my-newsfeed-mls-standings.md).

The scraper imports complete published standings independently of schedules and
My-Newsfeed's NWSL results sync. Eight configurable sources cover MLB, WNBA, the
full NWSL league, and Big 12 football, men's basketball, women's basketball,
soccer and volleyball. No SportsEvents, results, or existing schedule filters change.

## Sources and seasons

- ESPN: `https://site.api.espn.com/apis/v2/sports/{sport}/{league}/standings`.
  Paths are `baseball/mlb`, `basketball/wnba`, and `soccer/usa.nwsl`.
  This is **apis/v2**, unlike the scoreboard's **apis/site/v2** path.
- Big 12: official `https://big12sports.com/standings.aspx?path=...` tables:
  `football`, `mbball`, `wbball`, `soc`, `vb`.
  The bare soccer URL currently fails. For a soccer URL without a `standings`
  parameter the scraper resolves the working current soccer standings link from
  the conference navigation on its football standings page. An explicit URL
  containing `standings=...` overrides that discovery and pins that table.
- MLB defaults to `?season=2026`, matching the user's 2026 requirement. Change
  the configured URL when switching seasons. WNBA/NWSL follow ESPN's current
  season; add `?season=2026` to pin a year. Returned years must match explicit requests.
- Big 12 follows its published current table. Basketball currently publishes
  **2025–26**, not 2026–27. Display the stored season; do not substitute today's year.
- Verified 2026-10-11: MLB 30 teams (AL/NL), WNBA 15 (East/West), NWSL 16,
  Big 12 football/basketball/soccer 16 each and volleyball 15.
  Standings include all teams, even teams with no future scheduled games.
  These are season/conference tables, not a playoff bracket or playoff-only records.

## Configure and deploy

Use migration credentials for setup; keep them out of the hourly container.

1. Build the updated scraper (`cargo build --locked --release`, or its Dockerfile).
2. With the migration connection profile, run `web_scraper --configure-standings`.
   It applies additive `sql/standings.sql` and registers eight **disabled** sources
   using `sql/standings_seed.sql`. Repeating setup preserves configured URLs,
   enable settings and minimum team counts. Alternatively apply both SQL files
   directly, schema first, against WebScraper.

   For this repository's existing local migration profile:

   ```bash
   ( set -a; . ./.env.migrations; set +a; target/release/web_scraper --configure-standings )
   ```
3. Apply `sql/runtime_permissions.sql` with admin credentials after migration.
   The runtime role receives SELECT on configuration, UPDATE only on refresh
   metadata, and SELECT/INSERT/UPDATE on standings. No DDL, DELETE, result-table
   access or configuration editing is added to the runtime account.
4. Enable the requested sources with admin credentials:

   ```sql
   UPDATE dbo.SportsStandingsSources SET Enabled=1
   WHERE ProviderKey IN ('espn-mlb','espn-wnba','espn-nwsl','big12');
   ```

5. Preview using runtime credentials:
   `web_scraper --standings-from-db --dry-run -o /tmp/standings-preview.xml`.
   Then import: `web_scraper --standings-from-db -o /tmp/standings-import.xml`.
   Each reports source/team count/season. Dry runs do not change rows or refresh metadata.
6. Deploy/recreate the scraper. Updated `docker-compose.yml` adds
   `--standings-from-db` to the existing hourly command. **Migrate and grant before
   starting that command.** Disabled sources are skipped. Existing collectors keep
   running if an individual standings source fails; the run is marked partial/failed.
7. Implement/deploy the separate My-Newsfeed UI using the contract below.
   Its read account needs SELECT on both new tables; its admin account needs
   configuration UPDATE only if adding a source-management UI. Do not grant it
   the scraper ingestion role just to render tables.

`MinimumTeams` rejects empty/truncated tables. Update it deliberately when league
membership changes. Failed fetches/parses/writes retain the last complete snapshot
and record `LastError`; they never replace it with a partial response.

## My-Newsfeed read contract

`dbo.SportsStandingsSources`: ID, ProviderKey, Sport, DisplayName, Url, Enabled,
MinimumTeams, LastAttemptAt, LastSuccessAt, LastError, LatestSeason, LastTeamCount.

`dbo.SportsStandings`: ID (BIGINT), SourceId, Season, SeasonType, Title, SourceUrl,
GroupId, GroupName, TeamId, TeamName, LogoUrl, TeamUrl, Position, nullable Rank,
StatsJson, nullable Note, IsCurrent, FirstSeenAt, FetchedAt.

Use this query (with source/sport filters parameterized):

```sql
SELECT s.DisplayName, s.ProviderKey, s.Sport,
       s.LastSuccessAt, s.LastError,
       t.ID, t.Season, t.SeasonType, t.Title, t.SourceUrl,
       t.GroupId, t.GroupName, t.TeamId, t.TeamName, t.LogoUrl, t.TeamUrl,
       t.Position, t.Rank, t.StatsJson, t.Note, t.FetchedAt
FROM dbo.SportsStandingsSources s
JOIN dbo.SportsStandings t ON t.SourceId=s.ID AND t.IsCurrent=1
WHERE s.Enabled=1
ORDER BY s.DisplayName, t.GroupId, t.Position, t.TeamName;
```

- Add a **Standings** section with a league/sport selector. Show groups as separate
  tables, stored season, source link, and a “Last refreshed” timestamp. Do not apply
  the schedule page's today-forward cutoff to standings. NWSL is the full table,
  with Kansas City Current optionally highlighted; not filtered to team 20907.
- All SQL timestamps are UTC (`DATETIME2`). Explicitly mark them UTC in .NET before
  Central display conversion. `FetchedAt`/`LastSuccessAt` mean scraper refresh time,
  not a provider's own last update time. Show a stale/unavailable indicator after
  two missed hourly refreshes or when LastError is present. An enabled source
  without rows should show “Standings not available yet.”
- ESPN TeamId is scoped by ProviderKey (espn-mlb/wnba/nwsl); Big 12 TeamId uses
  official conference member IDs. Read TeamName/LogoUrl directly from standings.
  Never resolve ESPN IDs via Big 12 metadata. IDs are strings; row ID is 64-bit.
- `Position` preserves provider table order within GroupId. It is not a league-wide
  rank, seed, or computed tiebreaker. `Rank` is populated only when ESPN explicitly
  supplies a rank; Big 12 row positions must not imply official tie-breaking ranks.
  ESPN SeasonType is its numeric type serialized as text (MLB/WNBA 2, NWSL 1 in
  verified fixtures); Big 12 uses `Conference standings`. Do not assume type 2
  always means the same thing across sports.
- Parse `StatsJson` as an array. Each item includes `name`, `displayName`,
  `displayValue`, and optional numeric `value` plus provider metadata. Display
  provider `displayValue` (preserves dashes, percentages, W/L streaks, draws,
  clinch markers). HTML-encode all names, notes and values; only use HTTP(S) links.
- ESPN statistic names: `wins`, `losses`, `ties` (NWSL draws), `gamesPlayed`,
  `winPercent`, `gamesBehind`, `points`, `pointsFor`, `pointsAgainst`,
  `pointDifferential`, `streak`, `clincher`, `playoffSeed`, `overall`. Fields vary
  by sport. NWSL: GP/W/D/L/GF/GA/GD/Pts. MLB/WNBA: W/L/Pct/GB/Streak.
  **Do not label ESPN MLB/WNBA `points` as soccer standings points.** ESPN notes
  and clinch statistics can be shown as legends, not inferred playoff status.
- Big 12 statistics preserve visible official columns, with positional names
  `column1`, `column2`, ... and exact `displayName` headers: conference record,
  conference percentage, overall record, percentage, streak and home/away splits;
  soccer includes conference points. Repeated responsive/mobile columns are
  removed. Render a dynamic header/value table using published column order;
  repeated labels (Home/Away/Streak) may occur in conference and overall sections.
  Do not parse those records using ESPN's NWSL W-D-L convention: official Big 12
  soccer records are W-L-T. Missing numeric values are nullable, never zero-filled.

ESPN Season uses its numeric year as text; Title retains its published season
label (for example `2026 NWSL`). This keeps team identities stable when a display
label changes. Big 12 Season is its published year/span (for example `2025-26`);
Title retains its full table caption.

Each complete refresh commits rows and source success metadata together, serializing
writes per source. Identity is `(SourceId, Season, SeasonType, GroupId, TeamId)`;
repeat imports update rows and preserve ID/FirstSeenAt. Removed teams/previous
seasons remain stored with IsCurrent=0, so always include that filter. This stores
latest rows per season, not a history of every hourly refresh. Schedules/results
are never used to calculate standings.

## Verification

`cargo test --locked --all-targets`; `cargo clippy --locked --all-targets -- -D warnings`;
`cargo fmt --all -- --check`.

The six offline tests use trimmed live fixtures captured 2026-10-11 from all eight
sources, including structured official member metadata. The isolated SQL test
checks stable IDs, updated statistics, incomplete-response protection, transaction
rollback after a write error, season rollover and restricted permissions:

```text
STANDINGS_TEST_DATABASE=WebScraper_Standings_Validation_<date>
cargo test --locked standings_upserts_rollover -- --ignored --nocapture
```

Use the migration connection profile for that isolated test; it creates/migrates
only the named validation database. Do not use production for fixtures.

A second opt-in test uses the restricted runtime profile after the SQL setup test:
`cargo test --locked live_standings_all_sources -- --ignored --nocapture` with
`STANDINGS_TEST_DATABASE` set to the same isolated database. It fetches/imports
all eight live sources, repeats each snapshot, and checks IDs/FirstSeenAt.

## Request to give the My-Newsfeed application

“Add a Standings page/section for the enabled scraper-owned SportsStandingsSources.
Implement the read models and query in docs/sports-standings.md against the shared
WebScraper database, with a league/sport selector, grouped tables, provider names
and logos, published season labels, last-refreshed/stale state and source links.
Use StatsJson display values and provider ID namespaces exactly as documented.
The scraper owns migrations and refreshes; the web app reads the snapshots.
Keep schedule date filters and the separate ESPN NWSL results sync unchanged.”
