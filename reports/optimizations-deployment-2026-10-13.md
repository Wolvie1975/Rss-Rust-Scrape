# Initial optimization release — October 13, 2026

Image web-scraper:optimizations-20261013 (also latest).
Image ID sha256:1e7345e7f85bb22ddaa22d9b7089c3c7233ea92bc3e036859be61d787c84c3c6.

Implemented stage/HTTP-text metrics, ESPN schedule upserts in 100-row JSON batches
within a single feed transaction, and past-release filtering before movie save
and poster work. No refresh-frequency, provider-pacing, schema, permission or
separate web-app changes. Hourly command and retention policies preserved.

Validation: 101 default binary tests passed, 16 integration tests skipped. Four
isolated SQL tests passed: NWSL/WNBA/MLB identity, reschedule/cancellation behavior,
and a forced second-chunk failure proving the first chunk also rolls back. The
101-row successful/repeat import preserved IDs/FirstSeenAt. Test databases removed.
Clippy with warnings denied and formatting checks passed.

Production all-five-ESPN-schedule plus movie verification exited 0: six source
successes, zero failures, 34.3 seconds. 915 observations used 13 upsert calls.
EPL: 320 rows/four batches; Championship: 552 observations/six batches. SQL stage
timings were 538 ms and 849 ms respectively in that run. Movies skipped 18 past
releases and saved 50 current/future releases with zero issues. All 785 retained
ESPN event IDs, game IDs, provider keys and FirstSeenAt timestamps matched the
pre-deployment snapshot exactly. Source/network data can change between runs.

Metrics and scope: [scraper-performance.md](../docs/scraper-performance.md).
No full-hourly-run speedup is claimed until equivalent full runs are measured.
Far-month caching, Big 12 batching and bounded concurrency remain follow-up items.

Verified COPY_ONLY/CHECKSUM/compressed backup:
/var/opt/mssql/backup/WebScraper_before_optimizations_20261013.bak.
Rollback image: web-scraper:before-optimizations-20261013.
Previous container: web-scraper-rollback-optimizations-20261013 (stopped).
Host source backup: .deployment-backups/20261013-optimizations/source.tar.
SQL Server was not restarted. No commits or Git pushes performed; production was deployed.
