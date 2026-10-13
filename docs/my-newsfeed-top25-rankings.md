# My-Newsfeed: national Top 25 with Big 12 highlighted

## Frontend implementation request

Add a **Top 25** page/section using the site's existing navigation, data access and
styling. Display the full national poll for each of the five tracked college sports;
highlight current Big 12 members without filtering other schools out. Rankings
are separate from conference standings, game schedules and results.

The scraper, production schema/configuration and five poll sources were deployed
on 2026-10-11. Production contains 125 ranking entries with 23 Big 12 markers.
The separate web-app rankings UI still needs implementation. Deployment order
below documents the setup for repeat deployments. This is the Markdown handoff for the separate
My-Newsfeed frontend builder.

## Selected polls and sources

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

## Big 12 identity and highlighting

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

## Database and queries

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

## Table layout and movement

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

## Dates, age and failure states

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

## Import behavior and deployment

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

## Frontend acceptance checks

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

## Backend validation commands

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
production. Validation evidence: reports/national-top25-2026-10-11.md.
