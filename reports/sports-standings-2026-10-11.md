# Sports standings implementation and validation — 2026-10-11

Implemented eight configurable sources independent of schedule and results imports.

| Source | Teams | Published season |
| --- | ---: | --- |
| MLB | 30 | 2026 |
| WNBA | 15 | 2026 |
| NWSL, all teams | 16 | 2026 |
| Big 12 football | 16 | 2026 |
| Big 12 men's basketball | 16 | 2025–26 |
| Big 12 women's basketball | 16 | 2025–26 |
| Big 12 soccer | 16 | 2026 |
| Big 12 volleyball | 15 | 2026 |

ESPN's live standings API provides complete groups, provider team identities,
names/logos/links, season labels and sport-specific statistics. Official Big 12
HTML provides records, percentages, streaks, splits and soccer points; structured
member metadata supplies stable conference IDs. The soccer menu advertises a
working explicit standings ID, which the collector discovers automatically.

New SportsStandingsSources/SportsStandings tables hold last complete snapshots.
Refreshes are atomic, preserve row identities, retain older seasons as noncurrent,
and leave last-good data intact on fetch/parse/write failure. StatsJson retains
provider values and labels rather than calculating standings from partial schedules.
ESPN identity uses numeric season year, independent of display-label changes.

Added configuration/import CLI flags, hourly compose flag, restricted SQL grants,
live fixtures for all eight sources, tests and the My-Newsfeed UI/read contract.

Validation:

- Full offline suite: 105 passing tests, no failures; 11 opt-in SQL/live tests skipped.
- Six standings fixture tests pass across all eight feeds, including bad seasons,
  incomplete/duplicate tables, unknown members and unsafe links.
- Isolated SQL test passes: repeat imports and display-label changes keep IDs,
  updated statistics persist, incomplete imports are rejected, forced insert
  failure rolls back current flags, season rollover succeeds, and runtime account
  lacks DELETE/configuration-URL UPDATE/CREATE TABLE permissions.
- Live test with webscraper_ingest against
  WebScraper_Standings_Validation_20261011: all eight sources imported,
  **140 current rows**; repeated snapshots preserve IDs and FirstSeenAt.
- Live CLI dry run: eight successes, zero failures.
- Clippy with warnings denied and formatting checks pass.

Production database, running scraper and My-Newsfeed UI were not changed.
Before deployment apply the additive migration, seed/configuration and runtime
permission updates; enable sources, then deploy the new hourly command. The
separate My-Newsfeed checkout must implement the standings UI. See
[the configuration and integration guide](../docs/sports-standings.md).
