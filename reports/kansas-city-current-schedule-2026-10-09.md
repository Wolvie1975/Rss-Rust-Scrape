# Kansas City Current schedule verification — 2026-10-09

- Inspected repository handoff and existing Big 12/Sidearm pipeline; no AGENTS.md
  files were present in the workspace. My-Newsfeed checkout is absent here.
- ESPN team schedule returned 27 completed 2026 games, omitting upcoming fixtures.
  Monthly NWSL scoreboards returned 30 unique 2026 Kansas City fixtures, including
  Oct 17 at Utah, Oct 25 vs Washington, and Nov 1 at Racing Louisville.
- Live CLI dry-run with `season=2026` completed 12 monthly requests and reported 30
  fixtures. Default rolling-year dry-run completed 36 requests and reported 57.
- All-target tests passed (62 main-binary tests plus 19 example tests; six SQL
  integration tests opt-in). The new SQL integration test was separately run and
  passed on `WebScraper_Nwsl_Validation_20261009`, with fixture writes rolled back.
  It verifies disabled configuration preservation, repeat imports, timed
  rescheduling, cancellation, retention on absence, stable IDs and FirstSeenAt.
- Schema initialized only in the isolated staging database. No production feed
  configuration, schedule import, application deployment, or scheduler restart.
- Handoff/configuration/migration: `docs/kansas-city-current-schedule.md`.
