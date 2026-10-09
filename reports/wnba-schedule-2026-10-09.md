# WNBA schedule verification — 2026-10-09

- Added league-wide WNBA schedule support, with optional local ESPN team-ID filters.
  Uses existing SportsEvents/Types and shares ESPN parsing/upserts with Kansas City.
- Live ESPN September scoreboard returned 38 games with both teams, home/away,
  venues, confirmed UTC starts, statuses, broadcasts and WNBA gamecast links.
- Live 2026 dry-run completed 12 monthly requests, returning 378 unique fixtures.
  Default rolling-year dry-run completed 36 requests, returning 705 unique fixtures.
- All-target tests passed: 65 main-binary tests and 19 example tests; seven DB tests
  remain opt-in. Separately ran and passed both WNBA and NWSL SQL integration tests
  with rollback-only fixture writes in isolated validation databases.
- WNBA SQL test covers disabled seed, Enabled/edited URL preservation on rerun,
  stable IDs and FirstSeenAt across repeats and postponement/rescheduling, retained
  missing games, and independent public rows for equal numeric IDs in other leagues.
- Default WNBA rows consistently represent the home team and away opponent, even
  when filtered by the away team. Provider-namespaced team IDs are stored in existing
  SchoolId/OpponentSchoolId columns for My-Newsfeed query filtering.
- Added --configure-wnba and sql/wnba_seed.sql; no new schema beyond the shared
  ScheduleStatus migration from the Kansas City importer. Results sync untouched.
- My-Newsfeed checkout absent. UI/query contract and enabling steps are documented
  in docs/wnba-schedule.md. No production schema/enable/import/deployment or restart.
