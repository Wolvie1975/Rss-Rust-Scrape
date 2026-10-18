# Sports standings and rankings

Use the shared standings read contract plus the league-specific sections below. NFL is grouped by conference and division; MLS uses conference groups; NHL uses divisions. National Top 25 polls retain only the current poll per source. Current production standings IDs: NFL 10, MLS 9, Championship 11, EPL 12, NHL 13. Dated import counts are snapshots, not guaranteed current values.

The college sports hub can also use the **Big 12 News** article category;
see its [news frontend contract](big12-news-frontend-integration.md). This is
independent of the structured sports data described below.

## Contents

- [F1 driver and constructor championships](#f1-standings)

- [Big 12 soccer and men’s basketball column mappings](#big12-soccer-basketball-columns)
- [NHL division standings — frontend handoff](#nhl-standings)
- [My-Newsfeed frontend: sports standings](#my-newsfeed-standings-frontend)
- [Sports standings for My-Newsfeed](#sports-standings)
- [NFL standings — frontend handoff](#my-newsfeed-nfl-standings)
- [My-Newsfeed MLS standings](#my-newsfeed-mls-standings)
- [My-Newsfeed: national Top 25 with Big 12 highlighted](#my-newsfeed-top25-rankings)

<a id="f1-standings"></a>

## F1 driver and constructor championships

F1 is deployed as `web-scraper:f1-20261018`. Its enabled `F1Source` record 1
is separate from `SportsStandingsSources`. Read `dbo.F1DriverStandings` and
`dbo.F1ConstructorStandings`, ordered by numeric `Position`. Both views include
`Season`, `StandingsRound`, decimal `Points`, `Wins` and UTC `FetchedAt`.
Driver rows preserve the provider's constructor list in `ConstructorsJson`.

October 18 deployment verification recorded 23 drivers and 11 constructors for
season 2026 through round 17. Counts and round are dated observations, not
constants for the UI. F1 refresh runs with `--events-from-db` together with the
calendar and classifications, rather than the shared `--standings-from-db` path.
Failed refreshes retain the previous entire F1 snapshot; disabling F1 hides its
views. Do not model championship points as match wins/losses or recalculate ranks.

Use the [frontend contract](f1-frontend-integration.md#standings) for queries and
field mappings, [operations](f1-operations.md) for setup/refresh, and
[deployment report](../reports/f1-production-20261018.md) for verification and rollback.

<a id="big12-soccer-basketball-columns"></a>

## Big 12 soccer and men’s basketball column mappings

Verified against the live production database and official conference pages on
**October 15, 2026**. These checks were read-only; no scraper code, source
configuration, production data or frontend changes were made during the review.
The user plans to adjust the frontend separately.

| Source | ID | Stored season | Verified teams | Last successful refresh observed (Central) |
| --- | ---: | --- | ---: | --- |
| Big 12 Soccer | 7 | 2026 | 16 | October 15, 1:00 p.m. |
| Big 12 Men’s Basketball | 5 | 2025-26 | 16 | October 15, 2:00 p.m. |

The soccer source resolves its current explicit standings URL from conference
navigation. The reviewed page was
[2026 soccer standings](https://big12sports.com/standings.aspx?standings=238&path=soc).
All 16 teams, their order and all 192 statistic values matched the database.
The [men’s basketball page](https://big12sports.com/standings.aspx?path=mbball)
matched all 16 teams, their order and all 160 statistic values, including every
column shown in the user’s 2025–26 screenshot. The basketball season label follows
the provider’s published table; it is not inferred from today’s calendar year.
Refresh times and counts above are recorded observations, not live status.

### Frontend field mapping

Team name comes from `TeamName`; table order comes from `Position`. The remaining
columns are retained in `StatsJson` as `name`, `displayName` and `displayValue`.
Use the positional `name` within the selected source’s mapping. Repeated labels
such as Pct., Home, Away and Streak represent different conference/overall fields;
do not index or deduplicate statistics by `displayName` alone. Preserve the
provider’s `displayValue`, including leading decimal points and record strings.

| Stat key | Soccer (source 7) | Men’s basketball (source 5) |
| --- | --- | --- |
| `column1` | Big 12 record | Big 12 record |
| `column2` | Conference percentage | Conference percentage |
| `column3` | Conference points | Conference home record |
| `column4` | Conference home record | Conference away record |
| `column5` | Conference away record | Overall record |
| `column6` | Conference streak | Overall percentage |
| `column7` | Overall record | Overall home record |
| `column8` | Overall percentage | Overall away record |
| `column9` | Overall home record | Overall neutral record |
| `column10` | Overall away record | Overall streak |
| `column11` | Overall neutral record | — |
| `column12` | Overall streak | — |

These mappings describe the verified table layouts. Positional keys are
source/layout-specific, not shared statistic identifiers across sports. If the
provider changes its table layout, recheck the published headers and mapping.

Soccer record strings use **W–L–T**, while the reviewed app displays separate
**W, D, L** columns. For example, Texas Tech’s conference `5-0-1` corresponds to
W 5, D 1, L 0. The reviewed soccer app showed conference GP/W/D/L/points correctly
for every team but omitted stored overall records, percentages, home/away splits,
neutral records and streaks. That difference was presentation, not missing data.
No basketball frontend rendering change was verified during this review.

Read only current rows from enabled sources using the existing shared contract:

```sql
DECLARE @SourceId INT=5; -- 5: men's basketball; 7: soccer
SELECT t.TeamName,t.Position,t.Season,t.StatsJson,t.FetchedAt
FROM dbo.SportsStandings t
JOIN dbo.SportsStandingsSources s ON s.ID=t.SourceId
WHERE s.ID=@SourceId AND s.Enabled=1 AND t.IsCurrent=1
ORDER BY t.Position;
```

Review evidence: [October 15 standings verification](../reports/sports-history.md#big12-standings-review-2026-10-15).

<a id="nhl-standings"></a>

## NHL division standings — frontend handoff

Production source **13** is enabled: ProviderKey `espn-nhl`, Sport `Hockey`,
DisplayName `NHL`, MinimumTeams `32`, URL
`https://site.api.espn.com/apis/v2/sports/hockey/nhl/standings?level=3`.
The existing hourly `--standings-from-db` collector refreshes this source.
October 14 deployment verification recorded **32 current teams** in four divisions,
with season `2027` representing **2026–27**.

Use the shared standings source selector and read contract below. Filter on
source enablement and `IsCurrent=1`; group by `GroupId`/`GroupName` and keep each
division's published `Position` order. Source IDs come from enabled configuration,
not a hardcoded provider count. Never infer NHL team identity from Big 12 metadata.

```sql
SELECT t.* FROM dbo.SportsStandings t
JOIN dbo.SportsStandingsSources s ON s.ID=t.SourceId
WHERE s.Enabled=1 AND s.ProviderKey='espn-nhl' AND t.IsCurrent=1
ORDER BY t.GroupName,t.Position;
```

`StatsJson` retains ESPN's statistics. Use provider `displayValue` for display,
show missing values as an em dash, and do not calculate missing statistics.

| Visible column | Statistic name |
| --- | --- |
| GP | gamesPlayed |
| W | wins |
| L | losses |
| OTL | otLosses |
| Pts | points |
| RW | regWins |
| ROW | rotWins |
| GF | pointsFor |
| GA | pointsAgainst |
| Diff | pointDifferential |
| Streak | streak |

Other retained fields include overtime/shootout win and loss counts. Keep provider
rank/seed metadata separate from published division position; do not infer
playoff qualification. Render season `2027` as `2026–27` for NHL, preserving the
stored ending-year value for queries. Convert refresh timestamps to Central time
using the site's existing conventions.

The collector requires all 32 distinct teams plus valid points, games played and
overtime-loss statistics before replacing a snapshot. A partial response leaves
previous standings available and records an error. Source `LastSuccessAt`,
`LastError`, `LatestSeason` and `LastTeamCount` support the existing stale/error UI.
Repeated imports preserve row IDs and first-seen timestamps.

Validate the selector, four division groups, NHL columns, season label, missing
stats/logos, disabled sources and stale/error states using
`tests/fixtures/standings/nhl.json`. Scraper deployment and enabling are complete;
My-Newsfeed UI changes belong to its separate checkout.

[Schedule/results contract](sports-schedules.md#nhl-schedule-results) ·
[Setup and activation](operations.md#nhl-operations) ·
[Production history](../reports/sports-history.md#nhl-deployment-2026-10-14).

<a id="my-newsfeed-standings-frontend"></a>

## My-Newsfeed frontend: sports standings

<a id="my-newsfeed-standings-frontend--implementation-request"></a>

### Implementation request

Add a `/standings` page and a Standings navigation link using My-Newsfeed's existing
page, data-access, styling and authorization patterns. Read the scraper-owned
standings from the shared WebScraper database. Support NHL, MLB, WNBA, MLS, full NWSL and
Big 12 football, men's basketball, women's basketball, soccer and volleyball.

This document is the frontend implementation handoff. Schema and scraper setup
are described in [sports-tables.md#sports-standings](sports-tables.md#sports-standings). The scraper code, production migration, enabling and deployment are complete
as of 2026-10-11; production contains 140 current standings rows.
The web application checkout is separate from this repository.

<a id="my-newsfeed-standings-frontend--data-ownership-and-prerequisites"></a>

### Data ownership and prerequisites

The scraper refreshes complete tables hourly. My-Newsfeed reads those snapshots;
it does not call ESPN or scrape Big 12 during page requests. Keep the existing
schedule page and separate ESPN NWSL results sync unchanged.

Before deploying the page, apply `sql/standings.sql`, register/enable sources, grant
the web application's database principal SELECT on `dbo.SportsStandingsSources`
and `dbo.SportsStandings`, then deploy/import with the updated scraper. Use the
scraper guide's deployment order. Keep SQL credentials on the server.

<a id="my-newsfeed-standings-frontend--page-layout"></a>

### Page layout

1. Page heading: **Standings**.
2. A selector populated from enabled sources, using their ID and DisplayName.
   Include all enabled sources, even when a source has no rows yet. Preserve
   the selected source in `?sourceId=<ID>` so links and reloads retain the selection.
   Default to the first enabled source in DisplayName order. Invalid or disabled
   IDs fall back to that default. If none are enabled, show the empty state.
3. Selected source heading and published season, followed by “Last refreshed” in
   Central time, a freshness message when necessary, and a link to the source table.
4. Separate standings tables for each GroupId, headed by GroupName. Preserve
   provider Position order within each group. Do not combine MLB AL/NL or WNBA
   East/West and invent a new league-wide rank.
5. Optional brief explanatory notes below each table, including provider notes or
   clinch markers when available.

Keep the full league/conference table visible, including eliminated teams and
teams without remaining games. Do not apply the schedule page's today-forward
date restriction. NWSL includes every team; optionally highlight Kansas City
Current (`ProviderKey='espn-nwsl'`, `TeamId='20907'`) while retaining the full table.

<a id="my-newsfeed-standings-frontend--sql-read-contract"></a>

### SQL read contract

Fetch source metadata separately so enabled sources with no rows remain selectable:

```sql
SELECT ID, ProviderKey, Sport, DisplayName, LastAttemptAt, LastSuccessAt,
       LastError, LatestSeason, LastTeamCount
FROM dbo.SportsStandingsSources
WHERE Enabled=1
ORDER BY DisplayName, ID;
```

Fetch only the selected enabled source's current rows, using a parameter:

```sql
SELECT t.ID, t.SourceId, s.ProviderKey, s.Sport, s.DisplayName,
       t.Season, t.SeasonType, t.Title, t.SourceUrl,
       t.GroupId, t.GroupName, t.TeamId, t.TeamName,
       t.LogoUrl, t.TeamUrl, t.Position, t.Rank,
       t.StatsJson, t.Note, t.FetchedAt
FROM dbo.SportsStandings t
JOIN dbo.SportsStandingsSources s ON s.ID=t.SourceId
WHERE s.Enabled=1 AND s.ID=@SourceId AND t.IsCurrent=1
ORDER BY t.GroupId, t.Position, t.TeamName, t.ID;
```

Never concatenate query-string values into SQL. Use an integer source parameter.
Do not join standings to SportsEvents or results to calculate records, ranks or
qualification. Historical rows remain stored with IsCurrent=0 and must be excluded.

Map SQL BIGINT row IDs to .NET `long`, TeamId/GroupId to `string`, nullable Rank to
`int?`, nullable timestamps to `DateTime?`, and StatsJson to a parsed statistic
collection. Names/logos/links come directly from each row. ESPN IDs are scoped
by ProviderKey and must never be resolved through Big 12 metadata.

<a id="my-newsfeed-standings-frontend--statistic-parsing-and-professional-table-columns"></a>

### Statistic parsing and professional table columns

StatsJson is an array with `name`, `displayName`, `displayValue` and optional
numeric `value`, plus provider metadata. A compatible .NET DTO is:

```csharp
public sealed class StandingStatistic
{
    [System.Text.Json.Serialization.JsonPropertyName("name")]
    public string Name { get; init; } = "";

    [System.Text.Json.Serialization.JsonPropertyName("displayName")]
    public string DisplayName { get; init; } = "";

    [System.Text.Json.Serialization.JsonPropertyName("displayValue")]
    public string DisplayValue { get; init; } = "";

    [System.Text.Json.Serialization.JsonPropertyName("value")]
    public double? Value { get; init; }
}
```

Ignore additional JSON properties. Handle invalid JSON gracefully and log the
row ID on the server; do not crash the entire page. Render missing values as an
em dash rather than zero. Use `displayValue` for rendering, preserving provider
percentages, plus signs, streaks and dashes. Do not calculate missing statistics.

| Source | Visible column | Statistic name |
| --- | --- | --- |
| MLB / WNBA | W | wins |
| MLB / WNBA | L | losses |
| MLB / WNBA | Pct | winPercent |
| MLB / WNBA | GB | gamesBehind |
| MLB / WNBA | Streak | streak |
| NWSL / MLS | GP | gamesPlayed |
| NWSL / MLS | W | wins |
| NWSL / MLS | D | ties |
| NWSL / MLS | L | losses |
| NWSL / MLS | GF | pointsFor |
| NWSL / MLS | GA | pointsAgainst |
| NWSL / MLS | GD | pointDifferential |
| NWSL / MLS | Pts | points |

Begin each table with Team. NWSL may also show Rank when explicitly supplied.
For MLB/WNBA, do not use Position or playoffSeed as an invented league-wide rank.
Only use the supplied Rank as an official rank; Position is display order.

Do not label MLB/WNBA `points` as soccer points. If showing `clincher`, retain its
provider display marker with accessible explanatory text from provider metadata.
Display Note when supplied, without inventing playoff qualifications.

<a id="my-newsfeed-standings-frontend--big-12-table-columns"></a>

### Big 12 table columns

Big 12 StatsJson preserves the official visible table columns in order. Its
statistic names are positional keys (`column1`, `column2`, ...), with published
headers in `displayName` and record strings in `displayValue`.

Build columns from that array in its stored order and match values by the statistic
name, not by displayName. Duplicate labels such as Home, Away and Streak can occur
in different sections. Do not deduplicate those columns by label. The scraper
already removes duplicated responsive/mobile versions of columns.

Show the conference and overall records and percentages prominently. Keep other
published columns available through horizontal scrolling on small screens.
Big 12 soccer can include conference points. Big 12 records use W-L-T when ties
are present; ESPN NWSL overall summaries use W-D-L. Prefer the original record
string to splitting/reordering it. Do not assign official ranks from row order.

<a id="my-newsfeed-standings-frontend--seasons-timestamps-and-freshness"></a>

### Seasons, timestamps and freshness

Use Season and Title from the rows. ESPN Season is a numeric year stored as text;
Title retains the published label, such as `2026 NWSL`. Big 12 Season can be a span,
such as `2025-26`; Title is its published caption. Do not replace either with the
current year. The current official basketball tables were 2025–26 when verified.
MLB is configured for 2026; the frontend must not request or fabricate 2027 data.
SeasonType is provider-specific text; do not reinterpret a numeric type as a
universal regular-season or playoff category.

SQL DATETIME2 values represent UTC. Mark them UTC explicitly in .NET before
converting with the site's existing Central-time helper. Display CST/CDT correctly.
FetchedAt and LastSuccessAt mean scraper refresh time, not the provider's own
last update. Refreshing a completed season's table does not make it a new season.

| Condition | Display behavior |
| --- | --- |
| Data present, last success within two hours, no error | Show table and last-refreshed timestamp. |
| Data present, last success older than two hours | Retain table; show “Standings may be out of date.” |
| Data present, source LastError set | Retain table; show “Latest refresh failed; showing last available standings.” |
| Enabled source has no current rows | Show “Standings not available yet.” Keep selector usable. |
| No enabled sources | Show “No standings sources are enabled.” |
| Database request fails | Show a retryable availability message; log details server-side. |
| Client-side request still loading, if applicable | Show a loading state without flashing the empty state. |

Keep raw LastError and exception details out of the public page. A source error
should not prevent viewing other sources. If using caching, keep it short relative
to the hourly refresh (for example five minutes) and include SourceId in the key.

<a id="my-newsfeed-standings-frontend--accessibility-and-rendering"></a>

### Accessibility and rendering

Use semantic tables with captions and column headers (`scope="col"`); use team
names as row headers where practical. Support keyboard operation of the selector.
On phones, scroll wide tables inside a labeled container without making the whole
page wider than the screen. Keep team names visible and logos at a consistent size.
A failed or missing logo should leave the team name readable. Decorative logos can
have empty alt text when adjacent team text supplies the accessible name.

Render names, statistics and notes through the framework's normal HTML encoding.
Accept only HTTP(S) URLs for images/team/source links. Use the site's existing
external-link behavior; links opening a new tab need `rel="noopener noreferrer"`.
Do not expose raw JSON, credentials, database configuration or scraper controls
in the public page.

<a id="my-newsfeed-standings-frontend--acceptance-checks-for-the-my-newsfeed-implementation"></a>

### Acceptance checks for the My-Newsfeed implementation

- All enabled sources are selectable, including sources with no rows.
- Professional tables use the correct sport-specific columns and provider groups.
- Full NWSL remains visible when highlighting Kansas City Current.
- Big 12 shows both conference and overall data, preserving duplicate column
  labels and the published basketball season span.
- Repeated refreshes do not add duplicate teams; noncurrent season rows are absent.
- Row order follows Position; no rank or playoff status is inferred.
- Missing statistics/logos, bad JSON, empty data and stale/error states render safely.
- UTC refresh timestamps convert to Central time using existing site conventions.
- Source selection survives reloads; disabled/invalid source IDs cannot reveal rows.
- Layout works on mobile and with keyboard/screen-reader navigation.
- The existing schedule and NWSL results pages retain their behavior.

Use the existing My-Newsfeed test patterns for query filtering, statistic mapping,
season labels and error/empty states. Do not make page tests depend on live ESPN
or Big 12 requests. Provider payload fixtures are available in this repository at
`tests/fixtures/standings/`; use them as reference for realistic stat shapes.

MLS configuration and conference-table details: [MLS handoff](sports-tables.md#my-newsfeed-mls-standings).

<a id="sports-standings"></a>

## Sports standings for My-Newsfeed

For the frontend build, use [the dedicated My-Newsfeed handoff](sports-tables.md#my-newsfeed-standings-frontend).

NFL is supported through `espn-nfl`, using ESPN football/nfl standings. The source
follows the published season and imports all 32 teams in eight AFC/NFC division groups using `?level=3`, including
ties, points and division records in StatsJson. MinimumTeams=32 rejects partial
snapshots. Existing hourly standings collection includes enabled NFL automatically.

MLS was added after the original eight-source deployment; see [MLS setup and frontend notes](sports-tables.md#my-newsfeed-mls-standings).

The scraper imports complete published standings independently of schedules and
My-Newsfeed's NWSL results sync. Eight configurable sources cover MLB, WNBA, the
full NWSL league, and Big 12 football, men's basketball, women's basketball,
soccer and volleyball. No SportsEvents, results, or existing schedule filters change.

<a id="sports-standings--sources-and-seasons"></a>

### Sources and seasons

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

<a id="sports-standings--configure-and-deploy"></a>

### Configure and deploy

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

<a id="sports-standings--my-newsfeed-read-contract"></a>

### My-Newsfeed read contract

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

<a id="sports-standings--verification"></a>

### Verification

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

<a id="sports-standings--request-to-give-the-my-newsfeed-application"></a>

### Request to give the My-Newsfeed application

“Add a Standings page/section for the enabled scraper-owned SportsStandingsSources.
Implement the read models and query in docs/sports-tables.md#sports-standings against the shared
WebScraper database, with a league/sport selector, grouped tables, provider names
and logos, published season labels, last-refreshed/stale state and source links.
Use StatsJson display values and provider ID namespaces exactly as documented.
The scraper owns migrations and refreshes; the web app reads the snapshots.
Keep schedule date filters and the separate ESPN NWSL results sync unchanged.”

<a id="my-newsfeed-nfl-standings"></a>

## NFL standings — frontend handoff

<a id="my-newsfeed-nfl-standings--deployed-data"></a>

### Deployed data

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

Use the existing contract in [the standings frontend guide](sports-tables.md#my-newsfeed-standings-frontend).
No database migration or new read permissions are needed for an application that
already reads SportsStandingsSources and SportsStandings.

<a id="my-newsfeed-nfl-standings--frontend-changes"></a>

### Frontend changes

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

<a id="my-newsfeed-nfl-standings--queries-and-model"></a>

### Queries and model

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

<a id="my-newsfeed-nfl-standings--nfl-columns"></a>

### NFL columns

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

<a id="my-newsfeed-nfl-standings--refresh-states"></a>

### Refresh states

Use LastSuccessAt for the last successful refresh and convert UTC timestamps to
America/Chicago for display. If LastError is populated, retain the last-good
standings and show the existing stale/error indication. Do not replace records
with zeros. If the enabled source has no current rows, show an empty state.
Do not offer disabled sources or display historical rows with IsCurrent=0.

<a id="my-newsfeed-nfl-standings--verification-checklist"></a>

### Verification checklist

- NFL appears in the selector and direct source URLs preserve selection.
- The 2026 snapshot shows 32 unique teams across eight four-team divisions.
- AFC/NFC headings each contain East, North, South, West; Kansas City is in AFC West.
- W/L/T, percentages and points match the stored displayValue fields.
- Team order within each division follows Position; no invented overall ranking is shown.
- Missing statistics, logos and links are handled gracefully.
- Disabled, empty, stale and failed-refresh states preserve existing behavior.
- Existing MLB, WNBA, soccer and Big 12 column mappings still work.

Deployment details: [NFL deployment report](../reports/sports-history.md#nfl-standings-deployment-2026-10-13).

<a id="my-newsfeed-mls-standings"></a>

## My-Newsfeed MLS standings

Add **MLS** to the existing Standings source selector. Show the full national
league as separate Eastern and Western Conference tables, preserving provider
order/rank within each group. Do not filter to Sporting Kansas City or to teams
with upcoming games. This change adds standings only, not MLS schedules/results.

<a id="my-newsfeed-mls-standings--data-contract"></a>

### Data contract

Use the existing SportsStandingsSources/SportsStandings models and queries from
[the standings frontend handoff](sports-tables.md#my-newsfeed-standings-frontend).

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

<a id="my-newsfeed-mls-standings--configuration-and-deployment"></a>

### Configuration and deployment

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

<a id="my-newsfeed-top25-rankings"></a>

## My-Newsfeed: national Top 25 with Big 12 highlighted

<a id="my-newsfeed-top25-rankings--frontend-implementation-request"></a>

### Frontend implementation request

Add a **Top 25** page/section using the site's existing navigation, data access and
styling. Display the full national poll for each of the five tracked college sports;
highlight current Big 12 members without filtering other schools out. Rankings
are separate from conference standings, game schedules and results.

The scraper, production schema/configuration and five poll sources were deployed
on 2026-10-11. Production contains 125 ranking entries with 23 Big 12 markers.
The separate web-app rankings UI still needs implementation. Deployment order
below documents the setup for repeat deployments. This is the Markdown handoff for the separate
My-Newsfeed frontend builder.

<a id="my-newsfeed-top25-rankings--selected-polls-and-sources"></a>

### Selected polls and sources

All five are available through ESPN's public `/apis/site/v2/sports/.../rankings`
API. The scraper selects the configured poll ID; it does not merge different polls.

| Sport | Selected poll | ESPN path | League ID | Poll ID |
| --- | --- | --- | --- | --- |
| Football | AP Top 25 (FBS) | football/college-football | 23 | 1 |
| Men's basketball | AP Top 25 | basketball/mens-college-basketball | 41 | 1 |
| Women's basketball | AP Top 25 | basketball/womens-college-basketball | 54 | 1 |
| Women's soccer | United Soccer Coaches/IWSOC Women's Top 25 | soccer/usa.ncaa.w.1 | 5499 | 86 |
| Women's volleyball | AVCA Division I Women's Top 25 | volleyball/womens-college-volleyball | 402 | 89 |

Football's response includes other polls/divisions. Only AP poll 1 is imported by
its seeded feed. A separate source can be configured for another poll, with the
correct LeagueId/PollId/URL and a distinct source identity.

Preserve PollName, SeasonLabel, Period and SeasonPhase exactly as published. Do
not label all polls AP or treat the soccer and volleyball rankings as AP polls.
Attribution can say “Poll: [PollName] · Data via ESPN”, linking SourceUrl.

Verified 2026-10-11: football's poll is dated October 4; soccer October 6;
volleyball October 5. Basketball endpoints still publish April 2026 polls for
**2025–26**. Do not label these October polls or invent a 2026–27 preseason poll.
The importer follows the latest available source response and retains its date.

<a id="my-newsfeed-top25-rankings--big-12-identity-and-highlighting"></a>

### Big 12 identity and highlighting

The scraper reads current active full-member metadata from Big 12's official
football standings page, using the existing conference member parser. It matches
the poll team's school/location to an exact normalized official school name.
No numeric ESPN ID is interpreted as a Big 12 ID. Missing membership data causes
an import failure that retains last-good rankings instead of clearing highlights.

Each ranking team has:

- TeamId: ESPN ID, scoped by ranking source/sport.
- Big12SchoolId: nullable official conference member ID.
- IsBig12: computed BIT, true when Big12SchoolId is populated.

Use IsBig12 to add a subtle row background, a visible **Big 12** badge and accessible
text. Do not rely on color alone. Former members such as Texas and Oklahoma are
not highlighted merely because they historically belonged to Big 12. Highlighting
reflects current membership, including when the latest available poll is older.
Do not replace the team's stored name/logo with a numeric Big 12 lookup.

Keep all national teams visible, including teams with no future scheduled games.
Do not apply the schedule page's today-forward date filter to ranking rows.

<a id="my-newsfeed-top25-rankings--database-and-queries"></a>

### Database and queries

`SportsRankingSources`: ID, ProviderKey, Sport, LeagueId, PollId, DisplayName, Url,
Enabled, LastAttemptAt, LastSuccessAt, LastError.

`SportsRankingPolls`: ID (BIGINT), SourceId, PollId, PollName, Season (INT),
SeasonLabel, PublishedAtUtc, ProviderUpdatedAtUtc, Period, SeasonPhase, SourceUrl,
IsCurrent, FirstSeenAt, FetchedAt.

`SportsRankingTeams`: ID (BIGINT), PollRowId, TeamId, TeamName, Rank, PreviousRank,
PreviouslyUnranked, Points, FirstPlaceVotes, Record, LogoUrl, TeamUrl, Big12SchoolId,
IsBig12, IsCurrent, FirstSeenAt.

Populate a selector from all enabled sources, including those with no data yet:

```sql
SELECT ID, DisplayName, Sport, LastSuccessAt, LastError
FROM dbo.SportsRankingSources
WHERE Enabled=1
ORDER BY DisplayName, ID;
```

For a selected, parameterized source:

```sql
SELECT p.ID AS PollRowId, p.PollName, p.Season, p.SeasonLabel,
       p.PublishedAtUtc, p.ProviderUpdatedAtUtc, p.Period, p.SeasonPhase,
       p.SourceUrl, p.FetchedAt,
       t.ID AS RankingRowId, t.TeamId, t.TeamName, t.Rank, t.PreviousRank,
       t.PreviouslyUnranked, t.Points, t.FirstPlaceVotes, t.Record,
       t.LogoUrl, t.TeamUrl, t.Big12SchoolId, t.IsBig12
FROM dbo.SportsRankingSources s
JOIN dbo.SportsRankingPolls p ON p.SourceId=s.ID AND p.IsCurrent=1
JOIN dbo.SportsRankingTeams t ON t.PollRowId=p.ID AND t.IsCurrent=1
WHERE s.Enabled=1 AND s.ID=@SourceId
ORDER BY t.Rank, t.TeamName, t.ID;
```

Both current filters are required: polls from previous weeks remain stored, and
teams removed during a correction remain noncurrent within that poll. Map BIGINT
IDs to .NET `long`, team IDs to strings, nullable points to `double?`, votes/ranks
and Big12SchoolId to `int?`, and nullable timestamps appropriately.

Use parameterized SQL and validate selected IDs against enabled sources. Preserve
selection in the URL, for example `/rankings?sourceId=...`. Choose the first enabled
source as a default; invalid/disabled selection falls back to a valid source.

<a id="my-newsfeed-top25-rankings--table-layout-and-movement"></a>

### Table layout and movement

Heading: selected sport, PollName, SeasonLabel, poll date, and supplied Period /
SeasonPhase if available. Below it show “Last checked/refreshed”, source link and
any availability message.

Columns: **Rank · Team · Record · Previous · Change · Points · First-place votes**.
Use team names/logos directly from the rows. Omit an all-missing optional column
or display an em dash; soccer's points are absent in the verified ESPN payload.
Missing fields must never be zero-filled or calculated from games/standings.

- Use the supplied Rank. Tied ranks are legitimate; do not renumber them by row
  position. Top 25 can include more than 25 teams when tied.
- PreviousRank present: show it; Change is PreviousRank minus Rank (positive = up,
  negative = down, zero = unchanged). Provide accessible text for movement icons.
- PreviousRank NULL and PreviouslyUnranked true: show **NR** / **New**.
- PreviousRank NULL and PreviouslyUnranked false: show an em dash; previous rank
  is unknown, so do not claim the team is newly ranked.
- Points and FirstPlaceVotes are poll statistics, not league standings points.
  Zero supplied by the provider is a real zero; NULL is unavailable.
- Record is provider text. Preserve its format; don't reorder soccer W/L/draw
  components or invent records from partial schedule data.

A “Big 12” legend explains highlighted rows. An optional highlight switch can
control emphasis, but the default view must retain the full national poll.

<a id="my-newsfeed-top25-rankings--dates-age-and-failure-states"></a>

### Dates, age and failure states

SQL DATETIME2 timestamps are UTC. Explicitly mark them UTC in .NET, then use the
site's Central-time conversion. Label PublishedAtUtc as **Poll dated [date]**;
its timestamp is the provider's poll date, not necessarily the time a publisher
announced it. ProviderUpdatedAtUtc is separate provider metadata; FetchedAt /
LastSuccessAt describe the scraper's refresh, not publication.

Separate poll age from collection freshness:

- Successful hourly fetch of an April poll does not make it a new October poll.
- If the poll date is over 14 days old, show **Latest available poll — dated [date]**.
  Keep SeasonLabel visible; do not claim the poll is this week's rankings.
- If LastSuccessAt is more than two hours old or LastError exists, show a refresh
  availability message while retaining last-good data.
- Enabled source with no current poll: “Rankings not available yet.”
- No enabled sources: “No ranking sources are enabled.”
- Database/network UI failure: show a retryable availability message and log
  technical details server-side. Don't expose raw LastError on the public page.
- Do not infer “Final” or “Preseason” from age or a season-ending week number.
  Render only the source's actual Period / SeasonPhase text when present.

Use semantic tables, keyboard-accessible source selection, readable team names
when logos fail, and a horizontally scrollable table on phones. HTML-encode all
provider text and accept only HTTP(S) links/images. Keep database credentials and
scraper controls off the public page.

<a id="my-newsfeed-top25-rankings--import-behavior-and-deployment"></a>

### Import behavior and deployment

A complete poll commits its rows and source success metadata atomically. Incomplete,
empty, malformed, wrong-league or wrong-poll responses preserve the previous poll.
Rows are identified by `(poll snapshot, ESPN TeamId)`; the snapshot identity is
`(SourceId, PollId, Season, PublishedAtUtc)`. Repeat fetches and corrections update
existing IDs. A new publication date creates a new snapshot and archives the old
poll via IsCurrent=0. Older publications or older provider revisions cannot roll
back the current table. Partial write failures roll back the entire snapshot.
No results, event, or standings tables are used to calculate these polls.

Deploy in this order:

1. Build the updated scraper. With migration credentials run
   `web_scraper --configure-rankings` (or apply `sql/rankings.sql` and then
   `sql/rankings_seed.sql`). It registers five disabled feeds, preserving settings.
2. Apply updated `sql/runtime_permissions.sql` with admin credentials after schema
   migration. Runtime receives SELECT on configuration, UPDATE on refresh metadata,
   and SELECT/INSERT/UPDATE on poll/team tables. No DELETE or DDL privilege is added.
3. Enable the requested feeds:

   ```sql
   UPDATE dbo.SportsRankingSources SET Enabled=1 WHERE ProviderKey='espn-rankings';
   ```

4. With runtime credentials preview `web_scraper --rankings-from-db --dry-run`,
   then import `web_scraper --rankings-from-db -o /tmp/rankings-import.xml`.
   Dry runs do not change rows or refresh metadata.
5. Deploy/recreate the scraper. Updated compose adds --rankings-from-db to the
   existing hourly command. Apply schema/grants before starting that command.
6. Grant the web application's read principal SELECT on all three new tables;
   implement/deploy the frontend. Do not give it the ingestion role solely to read.

The configured API Url can contain `?season=YYYY` to pin a season; mismatched
returned seasons are rejected. PollId and LeagueId identify the selected poll,
independent of API array order. Additional poll choices require separate source
rows and accurate provider IDs; don't relabel the existing source.

<a id="my-newsfeed-top25-rankings--frontend-acceptance-checks"></a>

### Frontend acceptance checks

- All five enabled sports have full national Top 25 tables with Big 12 badges.
- ESPN and Big 12 IDs remain distinct, including soccer's different team IDs.
- Texas/Oklahoma are not highlighted as current Big 12 members.
- Soccer/volleyball keep their actual poll names; football excludes FCS polls.
- Ties, missing statistics and previously unranked versus unknown ranks render correctly.
- April basketball poll dates and 2025–26 labels are visible rather than presented
  as a fresh October or 2026–27 poll.
- Switching sports/source preserves selection, and disabled sources stay inaccessible.
- Earlier snapshots and superseded correction rows do not appear in the latest view.
- Empty/loading/error/stale states, mobile layout and keyboard operation work.
- Schedule, standings, results and movie pages keep their existing behavior.

Offline poll fixtures and membership metadata are in `tests/fixtures/rankings/`.
Use them for page tests without requesting live providers. Backend validation also
includes isolated SQL correction/rollback tests and live imports under the
restricted account; neither writes fixtures to production.

<a id="my-newsfeed-top25-rankings--backend-validation-commands"></a>

### Backend validation commands

Regular checks: `cargo test --locked --all-targets`,
`cargo clippy --locked --all-targets -- -D warnings`,
`cargo fmt --all -- --check`.

For isolated SQL validation, set
`RANKINGS_TEST_DATABASE=WebScraper_Rankings_Validation_<date>` and load migration
credentials, then run:
`cargo test --locked national_rankings_repeat -- --ignored --nocapture`.
This creates/migrates the isolated database, tests corrections/rollback, enables
staging sources and clears artificial poll fixtures.

With the same database and restricted runtime credentials, run:
`cargo test --locked all_five_live_polls -- --ignored --nocapture`.
It leaves live poll snapshots in staging for review. Never point these tests at
production. Validation evidence: reports/sports-history.md#national-top25-2026-10-11.

