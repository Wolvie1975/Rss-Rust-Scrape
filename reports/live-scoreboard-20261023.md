# Persisted scoreboard deployment — October 23, 2026 CDT

## Outcome

Deployed independent **web-scoreboard**, image **web-scoreboard:20261023**,
image ID `sha256:3bd302aeb066c6bcd1510f84eb18671cb602b9019b51b64169f5f23548fda110`.
Started **2026-10-24 03:18:21 UTC / October 23, 10:18:21 p.m. CDT**.
Docker health and explicit `scoreboard --health-check` passed. Zero restarts at
verification. The original hourly scraper retained its exact container ID,
configuration, environment, command and mounts; it and mynewsfeed-web stayed healthy.
No frontend UI or existing web results worker was changed.

The collector uses the existing restricted ingestion credentials, public CA mount,
non-root user, 256 MiB memory limit, one CPU, 64 PID limit, read-only root,
dropped capabilities, no-new-privileges, bounded /tmp and rotating logs. Its image
contains no environment profiles or certificate/private-key material. The build
uses one Cargo job and was capped at 1.5 GiB memory with 2 GiB memory-plus-swap.

## Audit and coverage

Production configuration contained seven enabled ESPN league schedule feeds,
13 enabled result follows and 16 validated members per Big 12 conference follow.
Those define 20 coverage sources across 13 sport/league combinations; neither
source IDs nor conference IDs were copied into collector logic.

Full league coverage: NFL, MLB, WNBA, NHL, MLS, NWSL, Premier League and EFL
Championship. Scoped Big 12/Kansas coverage: college football, men's basketball,
women's basketball, women's soccer and women's volleyball. NBA has no enabled
source. Other Big 12 calendar sports and F1 are outside this first scoreboard.
The current basketball window has no college games; empty success is preserved as
such, not reported as unavailable or filled with fabricated fixtures.

A live probe established that college football requires the FBS group 80 for
this scope; group 50 omitted the required games. Volleyball must omit that group
filter: the October 23 probe returned 117 games without it versus five with it.
Only followed teams' eligible games are retained from those broader responses.
A regression test now distinguishes football, basketball and volleyball filters.
The corrected football import included Baylor at Kansas and the other eligible
Big 12 fixtures, without enabling additional sources.

Source membership comes from the existing web worker's validated ESPN mappings,
including sport-specific soccer team IDs. There is no validated Big 12 calendar
GameId bridge, so no name-only join was introduced. The existing public
SportsBoxScores view is backed by SportsBoxScoreCache; both names remain valid.
HomeEventResults is frontend code, not a database object to replace.

## Database and runtime verification

The four additive setup scripts applied twice successfully. Runtime never performs
DDL. They add scoreboard storage, request tracking, enabled coverage and game read
views, plus a SELECT-only frontend role. The migration does not grant ingestion
writes on ResultGames. An impersonated ingestion check verified scoreboard write
permission, follow read permission and **no ResultGames UPDATE permission**.
Actual frontend credentials successfully read both views without being printed or
persisted by the verification helper. Existing broader frontend permissions were
not changed; WebScraperScoreboardReader is available for a dedicated app user.

Initial one-shot population: **156 requests, zero failed/partial**, followed by
**12 corrected football-date requests, zero failed/partial**. Read-back found
**257 unique games**, **zero duplicate canonical identities**, **zero retained
fixture games**, and successful/empty request outcomes without parse issues.
Live samples existed for MLB, NHL and NWSL; final and upcoming samples existed
across the other in-season competitions. Counts/statuses are dated observations.

First deployed-container run: **four due requests, zero failed/partial**. Live
sample observation timestamps advanced to approximately 03:18 UTC, and request
NextAttemptAt values were ten minutes later for live provider days. A subsequent
watch-loop run skipped not-yet-due work: **zero requests, zero failed/partial**.
This verifies startup collection and due-work skipping, not a full-day cadence soak.
Docker then reported all three application containers healthy.

A session application lock prevents concurrent scoreboard runs. The watch loop
reconnects after errors, checks due work after a 60-second pause, and shares
requests across overlapping follows. Live/paused and near-start games use ten
minutes; new finals use 15 minutes for an hour; inactive/future days use hourly
checks; older final-only days use daily reconciliation within the three-day
lookback. Defaults retain 30 days and fetch seven days ahead plus an Eastern/Central
boundary day. HTTP failure backoff and Retry-After are implemented, but no live
provider outage/throttling was deliberately induced during deployment.

## Validation

- `cargo test --locked --all-targets`: **206 passed**, 23 opt-in tests ignored.
- Explicit rollback-only scoreboard SQL integration test: **one passed**.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- Parser fixtures cover six sport types, status transitions, nullable/zero scores,
  partial responses, scope filters, source-specific season eligibility, safe links,
  TBD/date-only values, overnight/Central dates, period detail and shootout values.
- SQL fixture verifies idempotent identity, stale and empty preservation,
  out-of-order/provider revision rejection, downward corrections, rescheduling,
  optional box-score absence and effective coverage visibility. All fixture writes
  roll back. Existing source formatting was normalized to satisfy the format check.

## Files, deployment and rollback

[Operations/setup](../docs/live-scoreboard-operations.md) contains reproducible
commands, scheduling, permissions and rollback. The new service has its own
Dockerfile.scoreboard and Compose definition. Production source changes were
byte-compared after synchronization; previous files were saved in the protected
production `.deployment-backups/20261023-scoreboard` directory. Runtime environment
was copied directly from the existing restricted ingestion container, with an
explicit check excluding migration credentials.

Rollback stops/removes only web-scoreboard, restores the previous Compose file
or removes its service, and preserves additive data/views. Existing schedules,
results, box scores, hourly ingestion and web results synchronization continue.
If a frontend rollout has occurred, hide that feature or label retained samples
stale. Do not drop shared tables or roll back the original scraper image.

[Frontend contract](../docs/live-scoreboard-frontend-handoff.md) was written after
deployment and copied to `New Development/live-scoreboard-frontend-handoff.md`.
It records exact fields, status/freshness semantics, parameterized day/active reads,
public sample records and known coverage limits. Frontend rendering remains future
work. No new Git commit/push was requested for this implementation turn.
