# Scraper performance instrumentation and optimizations

## October 13, 2026 release

ESPN schedule persistence sends JSON batches of at most 100 unique games through
OPENJSON. One transaction covers every batch for a feed, preserving rollback if a
later batch fails. Stable canonical/URL matching, FirstSeenAt, disabled-source
checks, field-length validation, reschedule/status updates and the rule against
inserting completed/past fixtures remain. Duplicate canonical input IDs keep the
last observation. SQL batch counts are logged.

Production movies discard releases dated before today's America/Chicago date
before saving movie/source/release rows or building poster jobs. Undated/TBD rows
and source issues are retained. Report counts are recomputed after filtering.
Calendar/detail discovery still follows the same weekly provider coverage.

## Metrics

Rotating container logs contain lines such as:

```text
metrics: espn_schedule rows=320 sql_batches=4
metrics: stage=epl_schedule elapsed_ms=2024 http_text_responses=9 http_text_bytes=2546247
metrics: movies past_releases_skipped=18
```

Stage timings cover rankings, standings, news, Big 12 collection and persistence,
individual ESPN schedules, YouTube, movies, TV, retention and the overall run.
Schedule SQL timings include validation/JSON preparation and the database work.
HTTP counters cover responses consumed by the shared text/JSON reader and their
body bytes, not failed connections, rejected statuses, HEAD requests or all image
traffic. They are not full wire-byte/request totals. Parent stages include nested
stages; do not add all stage timings or response counts together.

Dry runs emit timings but skip persistence and retention, so they are unsuitable
for SQL throughput comparisons. Compare full hourly runs with full hourly runs;
targeted verification runs have different workloads.

Production verification across all five ESPN schedules plus movies completed in
34.3 seconds, with six source successes and zero failures. It processed 915 schedule
observations in 13 upsert calls, compared with 915 calls in the former per-game loop.
EPL handled 320 rows in four batches and Championship 552 observations in six.
Movie filtering skipped 18 past releases. All 785 retained ESPN event identities
and FirstSeenAt values matched before/after, with no additions or removals in the
verification snapshot. These measurements do not establish a full-run speedup.

## Next tuning decisions

The current release implements metrics, ESPN batching and the expired-movie filter.
It keeps hourly freshness, request pacing and source coverage unchanged. Use the
new Big 12 HTTP/SQL timings to select the next bottleneck. Big 12 batching requires
preserving per-observation quarantine and deterministic primary perspectives.
Monthly caching and bounded HTTP concurrency remain candidates; changing distant
fixture refresh frequency needs a deliberate freshness policy. Neither change is
included in this release. Large-payload unchanged-row suppression also needs
comparison metrics before its benefit can be claimed.
