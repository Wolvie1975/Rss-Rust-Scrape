# MLB schedule verification — 2026-10-09

- Added MLB league support to the shared ESPN schedule pipeline, reusing existing
  event models and locked stable-ID upserts. No results-sync changes or new schema.
- `--configure-mlb` and `sql/mlb_seed.sql` register a disabled lookup row:
  ProviderKey espn-mlb, ProviderSchoolId 0. Existing hourly --events-from-db handles
  the enabled provider after deployment; MLB namespaced IDs isolate other leagues.
- Verified ESPN MLB monthly scoreboard's teams/home-away, venues, UTC dates,
  timeValid, statuses, broadcasts/logos and game links, plus unresolved playoff
  participants with negative ESPN placeholder IDs.
- Live default dry-run against isolated staging returned 2,916 unique fixtures:
  22 remaining 2026 games and 2,894 announced 2027 games, dates Oct 10, 2026 through
  Sept 26, 2027. Asserted every preview date is >= Oct 9, 2026 Central. No past or
  completed games imported; explicit future cancellation observations remain for
  reconciliation. No production rows/feed settings were written.
- All-target tests passed: 72 main-binary plus 19 example tests; eight SQL tests
  remain opt-in. MLB SQL integration test separately passed with rollback-only
  fixtures in WebScraper_Mlb_Validation_20261009. Covers configuration preservation,
  separate doubleheader rows, repeat/reschedule/postponement identity, first-seen
  preservation, retained missing games and independent equal IDs in other leagues.
- All-target cargo check and diff whitespace checks passed.
- Configuration and My-Newsfeed query/selector handoff: docs/mlb-schedule.md.
  MLB is not deployed/enabled; prior WNBA/Current production deployment unchanged.
