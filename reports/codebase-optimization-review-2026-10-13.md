# Codebase optimization review — October 13, 2026

Reviewed collection orchestration, ESPN/Big 12 schedules, movie discovery, SQL
persistence and retention. Production inspection was read-only. No application
code, database settings or production deployment changed during this review.

## Baseline and limits

Recent completed full hourly runs took 390–449 seconds (6.5–7.5 minutes).
Recent targeted league imports took 5–6 seconds. These are different workloads;
the targeted times do not establish a speedup achievable for the full collector.
The latest full run in that sample predates the final EPL deployment.
There are no persisted per-stage duration/request/write counters, so the dominant
stage cannot be conclusively identified from these run records alone.
Cached SQL execution statistics did not return a usable breakdown in this check.
Query Store currently uses 29 MB with a 1,000 MB cap and 30-day stale-query retention.

## Prioritized findings

### 1. Batch sports database writes — high expected benefit, medium/high effort

Evidence: src/db.rs:1034 loops through fixtures and sends one MERGE per game,
inside one transaction. src/db.rs:1231 sends the complete Big 12 SQL batch for each
observation; sql/upsert_big12_event.sql begins and commits a transaction per call.
The main loop also processes these observations individually.

Suggestion: send validated JSON arrays to SQL and upsert bounded batches, using
OPENJSON as the existing standings/rankings/results collectors already do. Start
with ESPN schedule writes; Big 12 batching requires more care because it includes
paired identity resolution, quarantine and primary/secondary perspective rules.

Measure requests, total elapsed time, SQL CPU/reads and transaction-log generation.
Preserve stable IDs/FirstSeenAt, source disabling, no insertion of completed/past
fixtures, reschedules, cancellation updates and failure rollback. Preserve Big 12
per-record quarantine and deterministic home/primary processing. Do not simply
parallelize writes through the single shared SQL connection.

### 2. Reduce repeated far-future/history fetches — high expected benefit, medium effort

Evidence: src/espn_schedule.rs:122 scans remaining months through the end of next
calendar year unless a season is pinned. On October 13 this means 15 monthly
requests each for the default WNBA and NWSL feeds, even when many months are empty.
MLB's pinned 2026 feed adds three months; Championship scans 12 season months and
EPL scans nine. Together these five schedule collectors make about 54 monthly
requests per full run, before news, movies, tables, rankings or Big 12 calendars.
Championship also reloads already completed current-season results every hour.

Suggestion: retain full schedule coverage but cache validated monthly snapshots.
Refresh near-term dates hourly; refresh distant dates and older completed results
less frequently, with periodic full scans and conditional HTTP requests when the
provider supports validators. Championship historical corrections must still be
rechecked. The acceptable delay for distant reschedules needs an explicit policy.
Do not reduce the requested range without retaining the previously validated
future schedule, or equate an empty/missing response with cancellation.

### 3. Avoid unchanged-row and expired-movie write churn — medium/high benefit, medium effort

Evidence: schedule and Big 12 upserts rewrite event data on every observation;
Big 12 also rewrites its JSON payload (sql/upsert_big12_event.sql:90). Standings
mark every current row inactive, then reactivate it even when nothing changed.
Movies collect a Monday–Sunday week, save rows and enrich posters in
src/main.rs:1218–1233; end-of-run retention then deletes dates before today.
Thus already-past dates earlier in the current week can be reinserted and deleted
on subsequent runs, potentially repeating metadata/poster work.

Suggestion: reject dated movie releases before today's Central date before
persistence/poster jobs. For retained events/results, compare normalized meaningful
fields or hashes before rewriting large payloads. Continue updating observation
freshness separately so unchanged content is not mistaken for a stale feed.
For unchanged standings, update refresh metadata without toggling every table row;
retain the existing transactional replacement path for actual changes.

### 4. Add bounded HTTP concurrency — medium benefit, medium effort

Evidence: src/main.rs:617 fetches news sources sequentially; Big 12 school RSS and
monthly metadata fetches are also sequential. Movie details are processed in a
single loop (src/movies.rs:615), with intentional 300 ms pacing.

Suggestion: use a small worker pool for independent requests across hosts, with
per-host limits, existing timeouts/DNS/redirect checks, rate-limit handling and
deterministic result merging. Keep intentional pacing where appropriate; avoid
unbounded requests or removing TVMaze retry/backoff. Persist SQL writes separately.
Reuse clients for each request policy to preserve pooling and consistent headers.

### 5. Make retention predicates cheaper as data grows — medium future benefit, medium effort

Evidence: sql/retention.sql:7 extracts and converts event dates from every source
JSON payload. Its episode predicate wraps StartsAtUtc in timezone conversions,
which may prevent useful range seeks despite existing schedule indexes.
Orphan removal uses several whole-table anti-joins within one transaction.

Suggestion: store/index a normalized source-event date instead of extracting it
from JSON during every cleanup. For timestamped episodes, compute today's Central
midnight as a UTC cutoff once and compare StartsAtUtc directly; handle date-only
rows separately. Verify plans and row counts before adding indexes. At current
small movie/episode counts, this ranks below network and write batching.

### 6. Add stage metrics before broader tuning — low effort, necessary validation

Persist or emit duration, HTTP requests/bytes, cache hits, rows examined/changed,
SQL calls and retention deletion counts for each collector. Track full hourly runs
separately from targeted imports and compare median/tail durations over several
cycles. Existing ScraperRuns stores whole-run status/heartbeat but cannot explain
where a seven-minute run spends its time.

Suggested implementation order: metrics → ESPN write batching and expired-movie
filter → monthly caching policy → measured Big 12 batching/concurrency.

## Lower-priority opportunities

- src/championship.rs:35–42 searches the raw events array again for each completed
  fixture. Index raw events by game ID once, or parse fixtures/results together.
  This removes quadratic lookup work, but monthly arrays are small, so it is
  unlikely to dominate the full run.
- src/movies.rs:7 reparses CSS selectors on demand. Cache frequently used selectors
  if profiling shows meaningful parser CPU cost.
- Centralize ESPN league metadata, request headers and date parsing shared by EPL,
  Championship and the older schedule collectors. This mainly reduces regressions
  and duplicate maintenance; the earlier 403 fix demonstrates the practical value.
- Consider a smaller Query Store cap/retention only if diagnostic storage is a
  priority. Its 29 MB is notable for this small database, but reducing it trades
  away useful performance history and is not an application runtime optimization.

Existing strengths worth retaining: HTTP connection pooling, payload size limits,
SQL transactions and indexes, TVMaze caching/leases/backoff, poster caching, atomic
RSS file replacement, disabled-source checks and last-good-data preservation.

This review recommends changes; it does not claim measured gains for unimplemented
optimizations or propose database shrinking as routine maintenance.
