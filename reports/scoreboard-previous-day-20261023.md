# Scoreboard history: previous Central day — October 23, 2026

Deployed **web-scoreboard:previous-day-20261023** to web-scoreboard with
`--lookback-days 1 --retention-days 1`. Live cadence remains ten minutes and
lookahead remains seven days plus the provider/display boundary day.

History now includes today and yesterday in America/Chicago, plus future games.
It is not a rolling 24-hour window. Older live/paused games also expire. The
collector filters older Central dates before upsert, and atomic cleanup removes
only dedicated scoreboard associations, games and request records. Existing
schedule, web result, match result and box-score history have separate policies.

At cleanup, the Central cutoff was **2026-10-22**. Removed **16 games** dated
October 20–21; retained **241 games**, including 24 on October 22, 12 on October 23
and all future fixtures. Within the cleanup transaction, full-row EXCEPT comparisons
in both directions verified that every retained game was unchanged. Read-back
verified zero games older than the cutoff. This was user-authorized data cleanup.

The updated service passed its health check and Docker reported healthy. Its
first watch run correctly skipped all not-yet-due requests with zero failures.
The hourly scraper and frontend also reported healthy; this rollout changed only
the scoreboard container. The prior scoreboard container remains stopped as
web-scoreboard-before-retention-20261023. Starting that older release would restore
the former broader retention/import policy, so it is not a rollback that preserves
the newly requested one-day history policy.

Validation:

- Scoreboard tests: ten passed; one explicit database integration test left opt-in.
- New unit coverage verifies the previous Central day across UTC midnight and both
  occurrences of the fall DST hour.
- [Isolated SQL boundary test](../tests/sql/scoreboard_retention.sql) passed using
  temporary tables: removes older day, preserves yesterday/today/future, and removes
  matching associations/request metadata.
- Clippy across all targets with warnings denied, formatting and diff checks passed.
- New image built using the previous verified builder's dependency cache, one Cargo
  build job and the same bounded build memory settings as the initial deployment.

The current [frontend handoff](../docs/live-scoreboard-frontend-handoff.md), its
New Development copy, [operations guide](../docs/live-scoreboard-operations.md),
Compose arguments and CLI defaults now agree on one previous day. No schema
migration or frontend rebuild is required for the data cutoff to take effect.
Earlier deployment reports retain their historical counts and configuration.
