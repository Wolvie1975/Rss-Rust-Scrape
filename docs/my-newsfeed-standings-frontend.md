# My-Newsfeed frontend: sports standings

## Implementation request

Add a `/standings` page and a Standings navigation link using My-Newsfeed's existing
page, data-access, styling and authorization patterns. Read the scraper-owned
standings from the shared WebScraper database. Support MLB, WNBA, MLS, full NWSL and
Big 12 football, men's basketball, women's basketball, soccer and volleyball.

This document is the frontend implementation handoff. Schema and scraper setup
are described in [sports-standings.md](sports-standings.md). The scraper code, production migration, enabling and deployment are complete
as of 2026-10-11; production contains 140 current standings rows.
The web application checkout is separate from this repository.

## Data ownership and prerequisites

The scraper refreshes complete tables hourly. My-Newsfeed reads those snapshots;
it does not call ESPN or scrape Big 12 during page requests. Keep the existing
schedule page and separate ESPN NWSL results sync unchanged.

Before deploying the page, apply `sql/standings.sql`, register/enable sources, grant
the web application's database principal SELECT on `dbo.SportsStandingsSources`
and `dbo.SportsStandings`, then deploy/import with the updated scraper. Use the
scraper guide's deployment order. Keep SQL credentials on the server.

## Page layout

1. Page heading: **Standings**.
2. A selector populated from enabled sources, using their ID and DisplayName.
   Include all nine enabled sources, even when a source has no rows yet. Preserve
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

## SQL read contract

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

## Statistic parsing and professional table columns

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

## Big 12 table columns

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

## Seasons, timestamps and freshness

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

## Accessibility and rendering

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

## Acceptance checks for the My-Newsfeed implementation

- All nine enabled sources are selectable, including sources with no rows.
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

MLS configuration and conference-table details: [MLS handoff](my-newsfeed-mls-standings.md).
