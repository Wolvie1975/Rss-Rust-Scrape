# Operations, audits and maintenance history

Historical evidence and the former long handoff are preserved here. Earlier table counts, runtime images, open items, permissions and retention instructions may be superseded. The operations guide and short root handoff describe the current documented state.

## Contents

- [Movie retailer links deployment — October 16, 2026](#movie-commerce-deployment-2026-10-16)

- [Article URL identity deployment — October 16, 2026](#article-identity-deployment-2026-10-16)
- [NHL production deployment and validation — October 14, 2026](#nhl-operations-deployment-2026-10-14)
- [SQL log maintenance — October 13, 2026](#sql-log-maintenance-2026-10-13)
- [Retention deployment — October 13, 2026](#retention-deployment-2026-10-13)
- [Initial optimization release — October 13, 2026](#optimizations-deployment-2026-10-13)
- [Codebase optimization review — October 13, 2026](#codebase-optimization-review-2026-10-13)
- [Movie and Big 12 parser fixes — production deployment](#parser-fixes-deployment-2026-10-12)
- [Audit remediation — October 10, 2026 (Central)](#audit-remediation-2026-10-10)
- [Rust scraper application audit — October 10, 2026 (America/Chicago)](#application-audit-2026-10-10)
- [web_scraper — handoff notes](#previous-handoff)

<a id="movie-commerce-deployment-2026-10-16"></a>

## Movie retailer links deployment — October 16, 2026

Deployed `web-scraper:movie-commerce-20261016`, image
`sha256:931488d3f6ea962215f0d773f6bac12081036bdf1a10ab6a424522054dc05e5f`.
`:latest` points to the same image. User authorized deployment and requested a
standalone frontend integration Markdown file. Theatrical provider is Fandango;
digital provider is Fandango at Home. AMC was replaced before this deployment.

Applied `sql/movie_commerce.sql`, `sql/movie_commerce_view.sql` and
`sql/movie_commerce_permissions.sql` as separate migration-credential batches.
Runtime access to the cache/view was checked with the restricted ingestion account.
Frontend read access is already covered by its existing permissions. No runtime
DDL or new credentials were introduced.

Read-only new-image preview found 34 releases, zero source issues (36 seconds).
Targeted real production run **160** completed **9:35:27–9:36:03 a.m. Central**:
34 releases saved, one source-stage success, zero failures; 22 posters preserved,
one unmatched. Twelve movie/provider jobs were checked. The cache/view contain
13 actions: nine theatrical and four digital, all search fallbacks. No direct
movie match was verified; one digital job awaits the next bounded pass. Buy labels
are not inferred from generic digital release dates.

Before/after comparisons preserved all 23 movie IDs, 23 source links and 34
release identities/dates/source URLs. Enabled source settings and the existing
Pure Xbox retired-ID mapping remain unchanged. The replacement container matched
production Env, Cmd, entrypoint, user, working directory, healthcheck, mounts,
network, restart policy, limits, capabilities, security options and logging.
Hourly scheduling remains enabled; next slot at verification was **10:00 a.m.
Central**. Scraper health check passed; unchanged frontend container was healthy.
No UI image was deployed.

158 offline tests passed, strict Clippy and formatting passed. The opt-in cache
integration test ran separately against an isolated database under the restricted
runtime user and passed. Independent SQL replay/view/retention tests passed; test
databases were removed. Current production behavior is verified search fallbacks;
direct/expired/purchase states are validated by fixtures.

A fresh verified copy-only/checksum backup is retained at
`/var/opt/mssql/backup/webscraper-managed/manual/WebScraper_before_movie_commerce_20261016.bak`.
Source/config backup and validation evidence are protected under host
`.deployment-backups/20261016-movie-commerce/`. Rollback image
`web-scraper:before-movie-commerce-20261016` and stopped container
`web-scraper-before-movie-commerce-20261016` retain the prior article-identity build.
The additive cache/view may remain on rollback; original movie data was not
rewritten by retailer enrichment. Source and documentation updates are synced to
the host deployment checkout; no Git commit/push was performed.

Give the frontend project [the standalone integration request](../docs/movie-commerce-frontend-integration.md).
Cache semantics, budgets, explicit setup and fallback behavior:
[movie purchase links](../docs/movie-purchase-links.md).

<a id="article-identity-deployment-2026-10-16"></a>

## Article URL identity deployment — October 16, 2026

Deployed `web-scraper:article-identity-20261016`, image
`sha256:a58252c3c576c4a66e840fc8cf8d5b75189d48d95ae16e90c72b2b26563e4f66`.
The user authorized implementation/deployment and explicitly approved keeping the
older Pure Xbox article ID and recording a mapping for the retired newer ID.

### Implementation and actual migration

Added conservative source-scoped redirect/canonical evidence, retained URL aliases,
a bounded pass over the latest ten stored articles, rotating work order and an
atomic SQL upsert. The global URL-hash uniqueness boundary remains unchanged.
Titles/images/timestamps never authorize identity. Private/unrelated/generic
redirects, bad canonicals and failed requests do not authorize destructive merges.
Missing metadata remains intact. Unknown foreign keys block retirement rather than
having their references guessed; cross-source ownership collisions are skipped.

Applied `sql/article_identity.sql`, `sql/upsert_article_identity.sql`, and
`sql/article_identity_permissions.sql` as separate migration batches. Actual schema
changes: two new tables (`ArticleUrlAliases`, `ArticleIdRedirects`) and one owner-
executed procedure (`UpsertArticleIdentity`), with runtime permission grants.
No existing `Pages` columns or unique indexes changed. The live URL hash is persisted
`CONVERT(binary(32),HASHBYTES('SHA2_256',Url))` over NVARCHAR, and `UQ_Pages_UrlHash`
remains unique. Normal ingestion does not perform DDL.

### Existing duplicate dry run and approved cleanup

Confirmed target: **sql2025 / WebScraper**, source **1764**, configured URL
`https://www.purexbox.com/`. The old `nine-xbox-games-are-out-today-...` URL still
redirected to the successful `10-xbox-games-are-out-today-...` article. The Rust
identity client independently verified both URLs converged on the same target.

The read-only procedure preview listed exactly IDs **25604** and **25647** and
proposed **25604** as survivor. Before cleanup, a fresh COPY_ONLY / CHECKSUM /
compressed full database backup was taken and passed RESTORE VERIFYONLY:
`/var/opt/mssql/backup/webscraper-managed/manual/WebScraper_before_article_identity_20261016_1340.bak`.
Managed log backups were also current. No external foreign keys referenced Pages
before this additive migration; write-time checks revalidated references/collisions.

Applied only that explicit pair as the initial cleanup. Article **25604** now has
the current URL/headline. Article **25647** was archived and maps to **25604**;
both URLs map to the survivor. Publication time, useful metadata and the survivor's
original ID/scrape timestamp were retained. A second cleanup pass performed no
additional retirement. Comparing every pre/post Pages row confirmed that only
25604 changed and 25647 left the public table; all other articles and all source
settings were identical at that cleanup checkpoint.

Read-only verification of `/news?category=11` showed exactly one card for the
confirmed article, with ID 25604. The frontend was not deployed. Browser saves
using 25647 need the frontend to follow the persisted `ArticleIdRedirects` mapping;
that browser-storage recovery is explicitly outside this scraper implementation.
No title-based purge or broad existing-article cleanup was performed.

### Validation and deployment settings

- Full offline suite: **152 passed** (125 main-binary tests plus 27 example tests).
  Nineteen opt-in tests were skipped in that offline run.
- The article SQL integration test passed separately in
  `WebScraper_ArticleIdentity_Validation_20261016`. It covered repeated collections,
  dry-run behavior, identical-content distinct URLs, old/new convergence with
  deterministic HTTP, stable IDs, metadata preservation, rollback after an injected
  failure, unknown references, cross-source ownership and concurrent sessions.
- Formatting, strict Clippy and Cargo audit passed.
- The release image's read-only Pure Xbox preview passed. Real targeted import 157
  passed with one source success and zero failures. Runtime logs confirmed the
  expected two retained-URL checks and persisted their check timestamps.
- Runtime environment, scheduler arguments, mounts, TLS settings, volume, network,
  restart policy, logging, security options and resource limits matched the
  pre-deployment container. The scheduler logged its next aligned slot for
  09:00 Central on October 16. Collection frequency was not changed.

The regular **09:00 Central scheduled run 158** completed at 09:03:06 with
**101 source successes, zero failures**, and healthy scraper/frontend containers.
It saved 320 news articles, applied ordinary ten-per-source retention, and logged
the next aligned slot for 10:00 Central. Source URLs/enabled flags were unchanged;
the frontend still showed one Pure Xbox card and the retired-ID mapping remained.

Diagnostic review then identified overly conservative rejection of legitimate
single-segment article slugs. The final image accepts those only with verified
article metadata, while still rejecting generic root/news/home/login/challenge/
error destinations. New regression tests cover single slugs and error pages with
misleading article metadata. The full manual collection using the original runtime arguments (without the
hourly loop) completed as **run 159**, with **101 source successes, zero failures**,
in 147 seconds. The final scraper and frontend were healthy; the hourly collector
still awaited its normal 10:00 Central slot. The final frontend check again showed
one Pure Xbox card, original publication time and the single retired-ID mapping.
All configured source URLs/enabled flags remained unchanged.

Seven identity checks were safely deferred/rejected in that final validation:
five pages lacked verifiable article metadata or looked like challenge/error
pages, and two redirect destinations failed publisher/target validation. No new
existing-row retirements, cross-source ownership skips or unknown-reference skips
occurred. These unverified identities remain unchanged; there was no content-
similarity fallback. The isolated validation database and temporary deployment
containers/tools were removed after verification.

### Rollback and follow-up

Previous image: `web-scraper:before-article-identity-20261016` (NHL release).
The original container is retained stopped as
`web-scraper-before-article-identity-20261016`. Protected host source/configuration
and evidence backups are under `.deployment-backups/20261016-article-identity/`.
They include private connection profiles and must keep restricted permissions.

For a code rollback, stop/rename the new collector, restore the retained old
container as `web-scraper`, retag the rollback image as latest, and start it with
its saved configuration. Restore the old host source before rebuilding. The new
tables/procedure are additive and can stay in place; do not drop the retired-ID
mapping or restore duplicate rows as an automatic part of image rollback. Any
scoped data reversal should use the protected pre-cleanup row snapshots after
reviewing subsequent imports. A whole-database restore needs separate coordination
because other applications share the database.

The frontend can implement saved-ID recovery from the documented mapping. Broader
publisher host aliases require explicit validation/tests. Future unresolved or
cross-source collisions remain diagnostic candidates, never fuzzy merges.
No Git commits or pushes were performed.

[Identity and SQL contracts](../docs/article-url-identity.md) ·
[Consolidated operations guide](../docs/operations.md#article-url-identity).

<a id="nhl-operations-deployment-2026-10-14"></a>

## NHL production deployment and validation — October 14, 2026

Deployed image `web-scraper:nhl-20261014`; the production scraper and front-end were
healthy at verification. NHL sources 5637 (schedule/results) and 13 (standings)
were enabled. Imported 1,239 fixtures, 170 results and 32 current standings teams.
Targeted runs 104, 105 and release-image run 106 succeeded; repeat imports kept
stable identities and first-seen timestamps. Existing sports source settings and
hourly container configuration matched the pre-deployment snapshot.

Full offline tests, formatting, strict Clippy and Cargo audit passed. The existing
poster SQL method received a scoped argument-count lint allowance, and Rustls was
patched from 0.23.45 to 0.23.46 for newly published advisories. Read-only release
verification and the SQL test in an isolated validation database passed. The
validation database and temporary operator tools were removed afterward.

Rollback image `web-scraper:before-nhl-20261014` and the stopped previous container
were retained. Source and private configuration backups are in
`.deployment-backups/20261014-nhl/` on the Docker host. Disable NHL sources before
restoring the older collector. No SQL Server restart, frontend deployment, Git
commit or push was part of this release.

Complete data counts, validation and rollback evidence:
[sports deployment history](sports-history.md#nhl-deployment-2026-10-14).
Current operation: [NHL setup guide](../docs/operations.md#nhl-operations).

<a id="sql-log-maintenance-2026-10-13"></a>

## SQL log maintenance — October 13, 2026

Completed approximately 2:54 p.m. Central.

Before: WebScraper_log 2,248 MB, approximately 2,201 MB used, FULL recovery,
log_reuse_wait_desc=LOG_BACKUP. SQL Agent was stopped; host crond was running.

Two CHECKSUM/compressed transaction-log backups were taken and verified. The first
released reusable log space, but active tail segments prevented file shrink.
After the second backup, one-time DBCC SHRINKFILE reclaimed the inactive tail.

After: log file 264 MB, approximately 1.2 MB used, log_reuse_wait_desc=NOTHING.
Data file remains 72 MB. File growth is fixed at 64 MB. FULL recovery is retained.
The file reduction is 1,984 MB; approximately 620 MB of new verified backups were
preserved, giving roughly 1,364 MB (1.33 GiB) net initial disk improvement.

Installed host cron: verified log backups every 15 minutes, COPY_ONLY full backups
at 2:35 a.m. Central daily. Managed full/log files rotate after eight/ten days.
Both scheduled runner commands were executed successfully and verified against
backup history. Cron is installed; no SQL Server restart or SQL Agent enablement
was required. Initial/manual backup files are excluded from managed rotation.

Policy and operations: [operations.md#sql-backups](../docs/operations.md#sql-backups).
No scraper/web-app rebuild, recovery-model change, commits or pushes performed.

<a id="retention-deployment-2026-10-13"></a>

## Retention deployment — October 13, 2026

Deployed approximately 2:46 p.m. Central as web-scraper:retention-20261013
(also latest), image sha256:8f581316006d45354ea03c354f8781c2d26b2b0262b5d07fad7d23623d615884.

Policies are documented in [operations.md#data-retention](../docs/operations.md#data-retention).
Runtime cleanup now removes past sports and canceled games, past movie releases
and episodes, orphan movie metadata, old ranking polls and finished run logs
older than seven days. Today is retained using America/Chicago; undated/TBD
records remain. TV tracking settings survive. News remains 10/source. YouTube
CLI default and hourly command now keep 10/channel.

Rankings remove prior polls within the replacement transaction, after a successful
save. Fetch/validation/save failures retain the last good current poll. Scoped
DELETE permissions were granted on the affected existing tables; no schema or
server-role changes were made. Cleanup is transactional and skips dry runs.

Verification:

- 97 binary tests passed; 14 integration tests skipped in the default suite.
- Isolated ranking SQL test passed, including failed replacement rollback,
  successful rollover deleting old polls, and rejection of stale responses.
- Retention SQL ran inside an outer rollback transaction under both migration
  and restricted runtime credentials. Current/future records, undated records
  and tracking configuration survived; old records were removed in the transaction.
- Clippy with warnings denied and formatting checks passed.
- Production targeted ranking/YouTube collection and cleanup succeeded (run 73).
- Counts: sports 981 → 937; movie releases 140 → 50; episodes 90 → 31;
  ranking polls 8 → 5. Every one of 15 YouTube channels has 10 videos.
- Audit reports zero expired sports, releases, episodes, old polls or old finished
  run logs. Scraper is healthy; restart count is zero. No SQL Server restart.

Verified COPY_ONLY backup with CHECKSUM:
/var/opt/mssql/backup/WebScraper_before_retention_20261013.bak.
Rollback image: web-scraper:before-retention-20261013.
Previous container: web-scraper-rollback-retention-20261013 (stopped).
Host source backup: .deployment-backups/20261013-retention/source.tar.
Binary rollback stops the new retention behavior; deleted data requires restoring
from the database backup if recovery is needed. The temporary isolated ranking
validation database was removed after testing. No commits or pushes performed.

<a id="optimizations-deployment-2026-10-13"></a>

## Initial optimization release — October 13, 2026

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

Metrics and scope: [operations.md#scraper-performance](../docs/operations.md#scraper-performance).
No full-hourly-run speedup is claimed until equivalent full runs are measured.
Far-month caching, Big 12 batching and bounded concurrency remain follow-up items.

Verified COPY_ONLY/CHECKSUM/compressed backup:
/var/opt/mssql/backup/WebScraper_before_optimizations_20261013.bak.
Rollback image: web-scraper:before-optimizations-20261013.
Previous container: web-scraper-rollback-optimizations-20261013 (stopped).
Host source backup: .deployment-backups/20261013-optimizations/source.tar.
SQL Server was not restarted. No commits or Git pushes performed; production was deployed.

<a id="codebase-optimization-review-2026-10-13"></a>

## Codebase optimization review — October 13, 2026

Reviewed collection orchestration, ESPN/Big 12 schedules, movie discovery, SQL
persistence and retention. Production inspection was read-only. No application
code, database settings or production deployment changed during this review.

<a id="codebase-optimization-review-2026-10-13--baseline-and-limits"></a>

### Baseline and limits

Recent completed full hourly runs took 390–449 seconds (6.5–7.5 minutes).
Recent targeted league imports took 5–6 seconds. These are different workloads;
the targeted times do not establish a speedup achievable for the full collector.
The latest full run in that sample predates the final EPL deployment.
There are no persisted per-stage duration/request/write counters, so the dominant
stage cannot be conclusively identified from these run records alone.
Cached SQL execution statistics did not return a usable breakdown in this check.
Query Store currently uses 29 MB with a 1,000 MB cap and 30-day stale-query retention.

<a id="codebase-optimization-review-2026-10-13--prioritized-findings"></a>

### Prioritized findings

<a id="codebase-optimization-review-2026-10-13--1-batch-sports-database-writes--high-expected-benefit-mediumhigh-effort"></a>

#### 1. Batch sports database writes — high expected benefit, medium/high effort

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

<a id="codebase-optimization-review-2026-10-13--2-reduce-repeated-far-futurehistory-fetches--high-expected-benefit-medium-effort"></a>

#### 2. Reduce repeated far-future/history fetches — high expected benefit, medium effort

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

<a id="codebase-optimization-review-2026-10-13--3-avoid-unchanged-row-and-expired-movie-write-churn--mediumhigh-benefit-medium-effort"></a>

#### 3. Avoid unchanged-row and expired-movie write churn — medium/high benefit, medium effort

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

<a id="codebase-optimization-review-2026-10-13--4-add-bounded-http-concurrency--medium-benefit-medium-effort"></a>

#### 4. Add bounded HTTP concurrency — medium benefit, medium effort

Evidence: src/main.rs:617 fetches news sources sequentially; Big 12 school RSS and
monthly metadata fetches are also sequential. Movie details are processed in a
single loop (src/movies.rs:615), with intentional 300 ms pacing.

Suggestion: use a small worker pool for independent requests across hosts, with
per-host limits, existing timeouts/DNS/redirect checks, rate-limit handling and
deterministic result merging. Keep intentional pacing where appropriate; avoid
unbounded requests or removing TVMaze retry/backoff. Persist SQL writes separately.
Reuse clients for each request policy to preserve pooling and consistent headers.

<a id="codebase-optimization-review-2026-10-13--5-make-retention-predicates-cheaper-as-data-grows--medium-future-benefit-medium-effort"></a>

#### 5. Make retention predicates cheaper as data grows — medium future benefit, medium effort

Evidence: sql/retention.sql:7 extracts and converts event dates from every source
JSON payload. Its episode predicate wraps StartsAtUtc in timezone conversions,
which may prevent useful range seeks despite existing schedule indexes.
Orphan removal uses several whole-table anti-joins within one transaction.

Suggestion: store/index a normalized source-event date instead of extracting it
from JSON during every cleanup. For timestamped episodes, compute today's Central
midnight as a UTC cutoff once and compare StartsAtUtc directly; handle date-only
rows separately. Verify plans and row counts before adding indexes. At current
small movie/episode counts, this ranks below network and write batching.

<a id="codebase-optimization-review-2026-10-13--6-add-stage-metrics-before-broader-tuning--low-effort-necessary-validation"></a>

#### 6. Add stage metrics before broader tuning — low effort, necessary validation

Persist or emit duration, HTTP requests/bytes, cache hits, rows examined/changed,
SQL calls and retention deletion counts for each collector. Track full hourly runs
separately from targeted imports and compare median/tail durations over several
cycles. Existing ScraperRuns stores whole-run status/heartbeat but cannot explain
where a seven-minute run spends its time.

Suggested implementation order: metrics → ESPN write batching and expired-movie
filter → monthly caching policy → measured Big 12 batching/concurrency.

<a id="codebase-optimization-review-2026-10-13--lower-priority-opportunities"></a>

### Lower-priority opportunities

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

<a id="parser-fixes-deployment-2026-10-12"></a>

## Movie and Big 12 parser fixes — production deployment

Completed October 12, 2026, approximately 10:52 p.m. Central.

Image: `web-scraper:parser-fixes-20261012` (also `latest`).
Image ID: `sha256:2ea1258c52f6371966190fdd7e597c2849bcae5ec44f6b77a02e42c4673366d0`.

Movie Insider now reads explicitly dated VOD / Digital release-intro paragraphs
as well as the dedicated digital section. A theatrical-only detail intro remains
an unmatched candidate. Estimated and ambiguous digital dates remain rejected.

Big 12 JSON date_utc values at midnight Central are recognized as date-only
placeholders only when JSON's local kickoff exactly matches the RSS instant in
America/Chicago on the same date. Other timing disagreements still warn; RSS
remains authoritative. UTC values are parsed rather than compared as text.

Verification:

- 96 binary tests passed; 14 database-dependent tests skipped. Clippy with warnings
  denied and formatting checks passed.
- Runtime health check exited successfully. Hourly scheduler retained, next run
  at 11:00 p.m. Central. My-Newsfeed remained healthy.
- Targeted production import of the Arizona State feed and enabled movie sources
  exited successfully. ScraperRuns ID 53 succeeded with no error summary.
- Arizona State feed ID 3184 has LastIssueCount=0 and LastError=NULL.
- Game 179594 has no warning and retains 2026-10-17T19:30:00Z (2:30 p.m. Central).
- Movie Insider source ID 2011 refreshed successfully and LastError is NULL.

The host checkout received src/big12.rs, src/movies.rs and the Big 12 guide.
No schema, credential, permission or SQL Server changes were needed.

Rollback image: `web-scraper:before-parser-fixes-20261012`.
Previous container retained stopped as `web-scraper-rollback-20261012`.
Host source backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261012-parser-fixes/source.tar`.
No commits or pushes performed.

<a id="audit-remediation-2026-10-10"></a>

## Audit remediation — October 10, 2026 (Central)

All thirteen audit findings are now addressed and deployed. After the user approved
the shared interruption, sql2025 restarted with the validated server certificate
and the scraper switched to CA verification. Website credentials were not changed.

| Finding | Remediation |
|---|---|
| F01 | Seven-day existing-ID reconciliation; final/past observations never insert historical games |
| F02 | Dedicated webscraper_ingest account, table-scoped grants; migration credentials outside scheduler |
| F03 | Public URL/IP/DNS/redirect boundary and article/canonical/image checks |
| F04 | Hostname-validated server certificate active; CA verification required; wrong CA rejected |
| F05 | Native SQL TLS backend and dependency refresh; audit clean |
| F06 | Checked retention bounds and UTF-16-aware SQL title limits |
| F07 | Bounded readers and container resource limits |
| F08 | Connection/operation deadlines, poisoned-session guard, heartbeat and read-only healthcheck |
| F09 | Accurate persistence outcomes, nonzero failure/partial exit, per-source summaries |
| F10 | Atomic ESPN snapshots, prevalidation and save-error isolation |
| F11 | Monotonic publication timestamps under correction rules |
| F12 | Enabled movie-provider filtering for calendars and poster jobs |
| F13 | Both DST fall-back clock occurrences considered |

Validation: 99 regular tests passed; NWSL/WNBA/MLB SQL tests and a new hardening SQL
regression passed in isolated staging. The new regression verifies scheduled →
in-progress → final on the same ID/FirstSeenAt, excludes newly observed completed
rows, and forces a later MERGE failure to prove earlier snapshot writes roll back.
Formatting and Clippy with warnings denied pass. Dependency audit has zero advisories
and zero warnings. Schema addition is dbo.ScraperRuns; setup now requires an explicit
migration path rather than performing DDL during hourly collection.

The restricted account was tested live: sysadmin=0, db_owner=0, CREATE TABLE=0;
reads of ResultGames are denied. A full read-only production preview completed,
then a full live collection completed with **64 source successes, zero failures**.
Read-only health check passes. MLB game 401907994 now has STATUS_FINAL, with its
original database ID **17849**. Sports remain **981 rows**, with no 2027 MLB rows.

Final deployed image:
`sha256:15fb2ef4dc56ec94a522d0083a16e051b97558d8f744edb31b37e84b4ae46211`.
Service remains hourly with news, school/ESPN sports, YouTube, movies and TV.
The hardened container uses the restricted runtime profile only. Its first
replacement attempt hit a Compose tmpfs-list formatting error; the previous
collector remained running. Corrected the configuration and verified the intended
image and credentials before live collection.

Rollback image: `web-scraper:before-audit-fixes-20261011`.
Protected host backups: `.deployment-backups/20261011-audit-fixes/` (UTC-labelled
artifact folder), source.tar, previous private profile and pki material. Do not
restore old administrator runtime credentials except as a controlled emergency;
return to the restricted profile once the corrected image is restored.

Guidance: docs/operations.md#scraper-security-and-sql-credentials. The shared TLS cutover
completed after approval; correct-CA authentication and wrong-CA rejection are
verified. Post-TLS professional schedule refresh completed with three source
successes and zero failures. No Git commit/push; existing work retained, plus
remediation code/docs/tests.

<a id="audit-remediation-2026-10-10--tls-cutover-completed-after-user-approval"></a>

### TLS cutover completed after user approval

- SQL certificate is active; shared sql2025 restarted once. Scraper uses the trusted
  CA with certificate bypass removed, retains webscraper_ingest and encrypted SQL.
- Authenticated health check passes. A deliberate unrelated CA fails with
  certificate verify failed (unable to get local issuer certificate), exit code 1.
- My-Newsfeed remains healthy; /events returns HTTP 200. Read-only sports audit
  over verified TLS returns 981 events / 19 feeds / 1,444 school observations and
  zero canonical duplicates.
- The initial helper execution resolved relative mount paths against its virtual
  checkout location, creating an empty CA directory. SQL remained running; the
  old collector could still connect. Removed the empty directory, supplied the
  actual host project path and recreated the scraper successfully. Scripts now
  accept SCRAPER_HOST_PROJECT_DIR for helper use.
- CA is read-only mounted at /data/certs/sql-ca.crt. Local runtime/migration profiles
  use the same CA-relative path from the repository root. No plaintext secrets are
  printed. Protected pre-TLS profiles/configuration remain available for rollback.
- Rollback uses the cached Rust helper image with --volumes-from so it can restore
  server configuration even if SQL's container cannot start.

<a id="application-audit-2026-10-10"></a>

## Rust scraper application audit — October 10, 2026 (America/Chicago)

Remediation status: all thirteen findings have since been addressed and deployed.
See [remediation and TLS verification](operations-history.md#audit-remediation-2026-10-10). Findings
below preserve the original pre-remediation audit evidence.


The collector is operating and its stored identities are sound, but two issues
need attention first: completed professional games do not receive their final
schedule status, and the runtime SQL account has server-wide administrator rights.

This audit reviewed the Rust scraper workspace, SQL migrations/upserts, all
collector modules, dependencies, deployment configuration, and the running Docker
collector. Production SQL checks were SELECT-only, using the same credentials as
the deployed container. My-Newsfeed received a read-only /events spot check; its
application source and authorization controls were not audited. No production
configuration, schema, event data, image or service was changed. Existing uncommitted
work was preserved. Temporary audit examples were removed.

<a id="application-audit-2026-10-10--findings-in-priority-order"></a>

### Findings, in priority order

P1 means address promptly; P2 means address in the next hardening/correctness work;
P3 means lower-priority reliability work. Evidence distinguishes live confirmation,
deterministic reproduction, and source review. None of these claims an observed
server compromise.

| ID | Priority | Finding | Evidence |
|---|---|---|---|
| F01 | P1 | Finished games stop receiving schedule-status updates | Live DB/API mismatch and visible finished MLB game |
| F02 | P1 | Hourly collector uses SQL Server sysadmin credentials | Live role query, deployed credential match verified privately |
| F03 | P2 | Untrusted feed URLs can contain script schemes or internal destinations | Real parser accepted both adversarial fixtures |
| F04 | P2 | SQL certificate authentication is disabled | Deployed TrustServerCertificate=true |
| F05 | P2 | Old SQL TLS dependency has three RustSec advisory alerts | cargo audit and dependency tree; reachability caveats below |
| F06 | P2 | Oversized retention values wrap negative and can delete every row | CLI acceptance reproduced; destructive SQL not executed |
| F07 | P2 | Most downloaded bodies have no size limit | Source review; runtime has no memory cap |
| F08 | P2 | SQL operations have no application deadlines or progress watchdog | Source review and live missing healthcheck |
| F09 | P2 | Failed collections can report success | All-news-failed CLI run returned exit code zero |
| F10 | P2 | ESPN source writes are partial on failure and can abort unrelated collectors | Per-row autocommit and propagated error paths |
| F11 | P2 | Future-dated news can continually move its stored publication time | Parser clamp plus unconditional non-null publication update |
| F12 | P2 | Disabling a movie source does not stop its fetching/poster processing | Calendar collector and poster job query ignore Enabled |
| F13 | P3 | Hourly schedule skips the repeated fall-back 01:00 occurrence | Deterministic execution of the actual next_slot implementation |

<a id="application-audit-2026-10-10--f01--completed-game-status-reconciliation-is-missing"></a>

#### F01 — Completed-game status reconciliation is missing

[espn_schedule.rs](../src/espn_schedule.rs), lines 133–141, returns only
`event_date >= today && !completed`. Once a stored game finishes, its final
observation is discarded before `save_espn_schedule` can update ScheduleStatus.
Earlier in-progress or scheduled values remain indefinitely. The Central-date
page cutoff eventually hides older games, but cannot hide a finished game today.
This flaw is in the today-forward filtering added during the schedule work.

Live example: MLB ESPN game **401907994**, Chicago White Sox at Cleveland Guardians,
EventDate **2026-10-10**. ESPN reports **STATUS_FINAL**; the DB retains
**STATUS_IN_PROGRESS**. The live /events HTML still contains its game link and
7:00 P.M. CT scheduled card. Two October 9 WNBA games, 401918299 and 401918300,
also retain IN_PROGRESS although ESPN reports FINAL; those are now hidden by date.

**Fix:** distinguish new upcoming imports from reconciliation of already-stored
IDs. Allow final observations to update existing rows' schedule status without
inserting completed historical games or importing scores/results. Consider
reconciliation of recently stored IDs that moved to an earlier date/month as well.
Add the missing end-to-end scheduled → in-progress → final regression test and
verify same-day finished cards disappear without losing IDs/FirstSeenAt.

<a id="application-audit-2026-10-10--f02--runtime-database-privileges-are-excessive"></a>

#### F02 — Runtime database privileges are excessive

[db.rs](../src/db.rs), lines 46–59 and subsequent DDL, opens master and initializes
schema during normal collection. The live audit returned **IsSysadmin=1** and
**IsDbOwner=1**, using credentials privately confirmed identical to the running
collector. Compromise or misuse of those credentials therefore affects the SQL
server beyond the scraper's owned tables.

**Fix:** separate schema/deployment credentials from hourly ingestion credentials.
Use a dedicated least-privilege account for ordinary reads/writes to the required
scraper tables. Run migrations/configuration with a separately controlled account;
remove routine startup's need for master/CREATE DATABASE/ALTER VIEW. Do not simply
revoke rights before separating initialization, because current startup needs DDL.

<a id="application-audit-2026-10-10--f03--upstream-url-validation-does-not-cover-feed-item-links"></a>

#### F03 — Upstream URL validation does not cover feed item links

[feed.rs](../src/feed.rs), lines 79–87, checks only link length. Its actual parser
accepted `javascript:alert(1)` and `http://127.0.0.1:8080/admin` in safe local RSS
fixtures. [main.rs](../src/main.rs), lines 104–111, subsequently fetches item URLs
when looking for missing images. Page canonical URLs are also joined/stored without
a final http(s) scheme check. Image validation elsewhere does not protect these
article links. A malicious or compromised upstream feed can therefore introduce
an internal-network request target or an unsafe link for a downstream renderer.

**Fix:** validate article/canonical/feed URLs before storage or fetch; allow only
approved schemes, block credentials and unintended internal/private destinations,
and apply the same checks after redirects and DNS resolution. Explicitly distinguish
trusted administrator source configuration from untrusted upstream-discovered links.
Validate link schemes in My-Newsfeed as defense in depth. No currently stored
non-http(s) news URLs were found. Browser script execution and internal endpoint
exploitation were not attempted or proven by this audit.

<a id="application-audit-2026-10-10--f04--sql-tls-does-not-authenticate-the-server"></a>

#### F04 — SQL TLS does not authenticate the server

The deployed connection has **TrustServerCertificate=true**. Tiberius's enabled
TLS feature defaults to required encryption, so this is not a finding of cleartext
transport; it is a finding that certificate authentication is bypassed.

**Fix:** configure a trusted server certificate/CA and hostname validation, then
remove the bypass. Coordinate SQL Server certificate configuration and container
trust settings so the existing collector remains available.

<a id="application-audit-2026-10-10--f05--dependency-advisory-alerts-require-tls-path-remediation"></a>

#### F05 — Dependency advisory alerts require TLS-path remediation

[Cargo.lock](../Cargo.lock), line 1697, retains rustls-webpki **0.101.7** through:
`tiberius 0.12.3 → tokio-rustls 0.24.1 → rustls 0.21.12 → rustls-webpki 0.101.7`.
The HTTP client's separate rustls-webpki **0.103.15** is newer; updating only that
copy will not address the SQL TLS path. cargo audit v0.22.2 flagged:

- [RUSTSEC-2026-0098](https://rustsec.org/advisories/RUSTSEC-2026-0098.html): URI name constraints.
- [RUSTSEC-2026-0099](https://rustsec.org/advisories/RUSTSEC-2026-0099.html): wildcard name constraints.
- [RUSTSEC-2026-0104](https://rustsec.org/advisories/RUSTSEC-2026-0104.html): CRL parsing panic.

The two name-constraint issues require certificate misissuance. The CRL advisory
explicitly excludes applications that do not use CRLs; this application does not
configure CRL use, so that alert is not evidence of a reachable current DoS.
Current SQL certificate bypass also limits the applicability of validation-path
findings, while independently causing F04. These are dependency-version alerts,
not three demonstrated application exploits.

Additional warnings: unmaintained rustls-pemfile **1.0.4** and yanked yoke-derive
**0.8.3**. Review an upgraded/replaced Tiberius TLS path and lockfile refresh with
SQL compatibility tests. Do not force an incompatible transitive semver update.
Add a repeatable dependency audit check to the project's validation workflow.

<a id="application-audit-2026-10-10--f06--retention-counts-can-become-destructive-negative-values"></a>

#### F06 — Retention counts can become destructive negative values

CLI limits accept positive u32 values up to 4,294,967,295. [db.rs](../src/db.rs),
lines 398, 422 and 818, casts keep to i32 before `DELETE ... WHERE rn > @P1`.
**2,147,483,648 becomes -2,147,483,648**, making every positive row number qualify
for deletion. Affects global news, per-source news, and per-channel video retention.

CLI acceptance was reproduced with oversized --keep/--per-source values and an
intentionally nonexistent URLs file; it reached the file-read error, not argument
validation. No destructive SQL was executed. Current production defaults of 10
news stories and 5 videos are safe.

**Fix:** enforce i32-safe bounds at the CLI and DB API boundaries, using checked
conversion, or bind counts as SQL BIGINT. Test boundary values without deletion.

<a id="application-audit-2026-10-10--f07--http-timeout-is-not-a-response-size-limit"></a>

#### F07 — HTTP timeout is not a response-size limit

Full-body `.text()` reads in [feed.rs](../src/feed.rs):16,
[main.rs](../src/main.rs):61/111, [espn_schedule.rs](../src/espn_schedule.rs):127,
[movies.rs](../src/movies.rs):323/366, [tvmaze.rs](../src/tvmaze.rs):203, and other
collectors have no byte cap. A fast large response can consume substantial memory
before the 15/20-second timeout. Parsing/checking event count happens afterward.
The running container also has **Memory=0 / NanoCpus=0** (no configured limits).
Poster image reads already have a useful 5 MB cap.

**Fix:** introduce shared bounded HTTP readers with suitable per-resource limits,
reject over-limit responses before full allocation/parsing, and set reviewed
container memory/CPU limits. Do not rely on Content-Length alone. No OOM test was
performed against production.

<a id="application-audit-2026-10-10--f08--a-stalled-sql-operation-can-freeze-an-apparently-running-collector"></a>

#### F08 — A stalled SQL operation can freeze an apparently running collector

[db.rs](../src/db.rs):20 calls TcpStream::connect without an application timeout;
connect/query futures throughout are awaited without deadlines. HTTP timeouts do
not cover those operations. One stalled DB connection/lock/read can block every
later collector and the hourly loop. The running container has no healthcheck;
restart policy cannot detect a process that remains alive but makes no progress.

**Fix:** add bounded connection/query deadlines with safe cancellation/reconnection,
and a persisted run heartbeat/last-success monitor. Keep useful per-source progress
when individual providers fail. The observed runtime is currently making progress:
its latest captured cycle started at 23:00 and finished at 23:06 Central.

<a id="application-audit-2026-10-10--f09--collection-success-is-reported-inconsistently"></a>

#### F09 — Collection success is reported inconsistently

A safe dry-run against localhost port 1 failed all news fetch attempts, logged the
failure, produced zero pages, and still returned **exit code 0**. The fetch-error
paths in [main.rs](../src/main.rs):371/614/742/748 log and continue, while run_once
ultimately returns Ok. Partial source failures can be acceptable, but there is no
aggregate successful/partial/failed outcome for external monitoring. News source
success is recorded at lines 348–351 before persistence, so subsequent save failures
can leave fresh success-looking source metadata.

**Fix:** collect structured per-source outcomes, mark success after persistence,
return failure when all required work fails, and expose partial failures without
preventing independent providers from completing. Preserve the existing failure
isolation intent rather than terminating on every harmless source warning.

<a id="application-audit-2026-10-10--f10--espn-persistence-has-no-snapshot-transaction-or-save-error-isolation"></a>

#### F10 — ESPN persistence has no snapshot transaction or save-error isolation

[db.rs](../src/db.rs):516–560 saves each fixture through a separate MERGE. HOLDLOCK
protects that event's identity, not the entire source snapshot. A later SQL error
can leave earlier rows committed. [main.rs](../src/main.rs):610 propagates a save
error before event_result records it; later independent feeds/movies/TV are skipped.
Unbounded venue/name fields and UTF-16-vs-character limits can trigger SQL rejection.
This is source-confirmed behavior, not an observed failed production import.

**Fix:** choose explicit source-level transactional persistence or validated
per-row quarantine with an accurate partial outcome, record save errors, and
continue unrelated collectors. Reuse the Big 12 per-observation error isolation
and TV's atomic snapshot pattern where appropriate. Add a forced middle-row failure
regression test against staging and verify LastError and unaffected providers.

<a id="application-audit-2026-10-10--f11--news-publication-dates-can-drift-forward-during-repeated-polling"></a>

#### F11 — News publication dates can drift forward during repeated polling

[feed.rs](../src/feed.rs):123 clamps future published dates to the current poll
instant. [db.rs](../src/db.rs):382 overwrites an existing Published whenever the
new value is non-null. The same URL with a misdated future pubDate consequently
moves forward on each poll and can displace genuinely newer stories during pruning.
This is also documented as an existing open issue in HANDOFF.md.

**Fix:** retain raw/provider date provenance, preserve the first valid stored
publication time under the intended correction rules, and test repeated imports
of a future-misdated story. No publication timestamps were modified by this audit.

<a id="application-audit-2026-10-10--f12--disabled-movie-providers-still-get-fetched-and-used-for-posters"></a>

#### F12 — Disabled movie providers still get fetched and used for posters

[movies.rs](../src/movies.rs):287–339 builds and requests both providers' calendars
without reading Enabled. [db.rs](../src/db.rs):210 selects poster jobs across all
MovieReleaseSources, also without Enabled. Release upserts correctly check Enabled,
but disablement does not stop calendar traffic and does not stop poster cache/movie
metadata updates based on that source. Both current movie providers are enabled,
so this is not an observed current disabled-provider incident.

**Fix:** pass enabled-provider configuration into collection and filter poster jobs;
make disablement semantics consistent for fetching, releases, posters and statuses.
Test a disabled provider with a request-count mock and staging assertions.

<a id="application-audit-2026-10-10--f13--hourly-fall-back-scheduling-skips-the-second-0100"></a>

#### F13 — Hourly fall-back scheduling skips the second 01:00

[main.rs](../src/main.rs):792 uses LocalResult.earliest(), discarding the later
valid occurrence of an ambiguous local time. The actual next_slot function,
executed with now=**2026-11-01T06:30:00Z** (01:30 CDT) and hours=1, returns
**08:00Z** (02:00 CST); the next eligible hourly clock slot is **07:00Z** (01:00 CST).
The deployed one-hour collector therefore has a two-hour gap across fall-back.
Existing tests cover three-hour schedules, which do not expose this case.

**Fix:** consider both occurrences of ambiguous local slots before filtering
strictly-future instants. Add one-hour fall-back regression coverage. No production
clock or scheduler was changed to reproduce this.

<a id="application-audit-2026-10-10--validation-and-healthy-properties"></a>

### Validation and healthy properties

- `cargo test --locked --all-targets`: **91 passed**, eight DB tests opt-in.
- NWSL, WNBA and MLB SQL integration tests: **three additional passes**, with
  rollback-only fixtures in their isolated validation databases. Five remaining
  opt-in DB tests were not executed because this audit avoided production writes
  and broader staging changes.
- `cargo clippy --locked --all-targets -- -D warnings`: **failed**. Bin: 20 lints;
  bin test: 21; movie_preview example: seven. Primarily collapsible branches,
  map-entry/sort idioms and an argument-count lint; do not confuse these with
  confirmed security defects. `cargo fmt --all -- --check` also fails baseline
  formatting. No automated CI workflow was found in this workspace.
- `cargo audit --json`: three version advisories and the two warnings noted above;
  RustSec database revision 7eebec69c352c7191b1f13eb95dd510eeca5d1de.
- Production: **981 sports events**, **19 feeds**, **1,444 school observations**;
  zero duplicate canonical keys; all foreign/check constraints enabled and trusted.
- MLB feed 4193 retains **season=2026**; **22 stored MLB games**, zero 2027 rows.
  Default seeding still has no season parameter: preserve/verify this deployment's
  explicit URL during recreation or restore to respect the user's scope.
- News: 28 sources, 280 pages, 10 per source; no stored non-http(s) page links and
  no current source errors. YouTube: 11 feeds and 55 videos.
- Movies: 33 movies, 33 links, 76 releases; 28 matched posters and five unmatched.
  Both movie source statuses healthy. Existing unmatched posters are not new defects.
- TV: five tracked shows, 90 episodes, no current errors; zero timed episodes
  missing UTC timestamps in the production snapshot. ID validation, validated
  snapshot persistence, cache/retry state and leases are sound foundations.
- Three Big 12 provider records remain intentionally quarantined (two self-opponent,
  one missing alias). Timing disagreements are retained as warnings; public game
  identities remain unique. They are upstream quality issues, not 16 broken imports.
- Runtime: expected image 257a666459d34838a8431d61f9db2efbf5c5857854afabe58f7b67ca1111bcbd,
  running with zero restarts, existing hourly flags and non-root scraper user.
  Not privileged. Actual Docker logging is bounded to three 10 MB files.
- SQL values are parameterized; database-name interpolation is restricted to
  ASCII letters/digits/underscore. No .env file was found in tracked Git history;
  workspace .env mode is 0600. No credential values were printed or stored in this report.

<a id="application-audit-2026-10-10--recommended-work-order-and-limits"></a>

### Recommended work order and limits

Fix F01 first, then separate runtime SQL privileges (F02) while planning certificate
and dependency remediation together (F04/F05). Next address URL/body boundaries,
retention bounds, truthful run outcomes and save-error isolation. Follow with news
publication rules, disabled-provider semantics, DST handling, and formatting/CI.
Keep results import behavior separate from schedule-status reconciliation.

This was a source/runtime/data audit, not a penetration test, formal correctness
proof, load test, or container OS-package vulnerability scan. My-Newsfeed source,
its CSP/authentication/link handling, and backup restore procedures were not fully
tested. No exploit of the dependency alerts was attempted. The JSON companion
contains sanitized evidence and counts; raw production snapshots remain in /tmp.

<a id="previous-handoff"></a>

## web_scraper — handoff notes

Archived handoff captured before consolidation. Working conventions remain in the current root handoff; runtime/schema snapshots here are historical.

<a id="previous-handoff--posters-across-movie-sections-2026-10-13"></a>

### Posters across movie sections (2026-10-13)

- All 10 box-office chart films and 12 future forecast films have valid posters.
- PosterUrl / PosterLookupStatus added to chart and forecast tables.
- Shared CinemaPosterCache: title/original year; 30-day success, 7-day miss,
  6-hour error retries; unused cache entries removed after 90 days.
- Hourly box-office collectors reattach cached images; --movie-posters backfills
  releases and cinema sections. Release coverage 24/34; ten unmatched placeholders.
- Existing release lookup now also tries validated The Numbers posters by year.
- Image web-scraper:cinema-posters-20261013; front-end doc:
  docs/movies-and-box-office.md#movie-section-posters-frontend-handoff.


<a id="previous-handoff--mls-scheduleresults-2026-10-13"></a>

### MLS schedule/results (2026-10-13)

- Schedule/results source 5525, espn-mls enabled; existing standings source 9.
- 90 upcoming fixtures, 421 current-season final results; no expired schedules.
- Existing --events-from-db discovers MLS hourly. Image web-scraper:mls-20261013.
- Shared soccer results writer preserves shootout tallies separately from goals.
- New nullable columns SportsMatchResults.HomeShootoutScore/AwayShootoutScore.
- Current-season results retained until successful new-season results import.
- Front-end work: docs/sports-schedules.md#mls-schedule-results-frontend-handoff.


<a id="previous-handoff--boxoffice-pro-predictions-2026-10-13"></a>

### Boxoffice Pro predictions (2026-10-13)

- --boxoffice-forecasts adds latest domestic preview and future-film forecasts.
- dbo.BoxOfficeForecasts stores numeric USD ranges, target dates and attribution;
  dbo.BoxOfficeForecastSources tracks publication/import/error status.
- Separate from actuals; current/future only, atomic replacement without history.
- Verified 12 future films, no current preview (latest covers Oct 9–11), zero past rows.
- 107 tests passed plus isolated SQL repeat/rollback/expiration checks.
- Image web-scraper:forecasts-20261013; --boxoffice-forecasts added hourly.
- Front-end work pending: docs/movies-and-box-office.md#boxoffice-forecasts-frontend-handoff.


<a id="previous-handoff--weekend-domestic-box-office-2026-10-13"></a>

### Weekend domestic box office (2026-10-13)

- The Numbers latest weekend top 10 in dbo.BoxOfficeWeekend; --boxoffice hourly.
- Atomic replacement, no archived weekends, last good chart preserved on failure.
- Dedicated Deadline/Variety box-office news disabled and saved stories removed.
- Frontend chart still needs implementation; docs/movies-and-box-office.md#boxoffice-top-10-frontend-handoff.
- Image web-scraper:boxoffice-20261013. Verified 10 rows for Oct 9–11.


<a id="previous-handoff--initial-optimizations-2026-10-13-deployed"></a>

### Initial optimizations (2026-10-13, deployed)

- ESPN schedules now use 100-row JSON batches with whole-feed rollback.
- Stage durations, HTTP-text counts/bytes and SQL batch counts logged.
- Past movie releases filtered before save/poster jobs; freshness/pacing unchanged.
- Production verification: six source successes, zero failures; 915 observations
  in 13 SQL upsert calls; 18 past releases skipped. 785 event identities preserved.
- Image web-scraper:optimizations-20261013 (also latest).
- docs/operations.md#scraper-performance and reports/operations-history.md#optimizations-deployment-2026-10-13.
- Caching, Big 12 batching and concurrency remain candidates, not deployed changes.

<a id="previous-handoff--premier-league-2026-10-13-deployed"></a>

### Premier League (2026-10-13, deployed)

- EPL source ID 5357 and standings source ID 12 enabled; current 2026–27 data.
- 320 current/future fixtures, 20 table teams; no past fixtures or historical results.
- EPL table rollover deletes superseded rows only for espn-epl; other standings
  retain their existing history. Runtime DELETE grant added for table cleanup.
- Image web-scraper:epl-20261013 (also latest), targeted import succeeded.
- Handoff: docs/sports-schedules.md#my-newsfeed-epl. Report: reports/sports-history.md#epl-deployment-2026-10-13.

<a id="previous-handoff--efl-championship--wrexham-2026-10-13-scraper-deployed"></a>

### EFL Championship / Wrexham (2026-10-13, scraper deployed)

- Full Championship schedule/table and current-season final results. Wrexham ID 352.
- Sports feed ID 5326 and standings source ID 11 enabled. Production: 437 upcoming
  fixtures, 24 standings teams, 115 results; Wrexham 36 upcoming and 10 results.
- New SportsMatchResults is independent of SportsEvents retention and ResultGames.
  Only explicit finals with valid goals imported; repeats/corrections preserve IDs.
- Current-season results retained until successful new-season result import.
- ESPN scoreboard requests use the existing My-Newsfeed/1.0 collector header.
- Image web-scraper:championship-20261013 (also latest, r2); source errors cleared.
- Frontend needs Wrexham filter/results view. Handoff:
  docs/sports-schedules.md#my-newsfeed-championship-wrexham. Existing table route sourceId=11 works.
- Report: reports/sports-history.md#championship-deployment-2026-10-13.

<a id="previous-handoff--sql-log-maintenance-2026-10-13-active"></a>

### SQL log maintenance (2026-10-13, active)

- WebScraper log reduced from 2248 MB to 264 MB after verified log backups.
- FULL recovery retained; no SQL restart. Log growth 64 MB.
- Host cron runs log backups every 15 minutes and verified COPY_ONLY full backups
  daily at 2:35 a.m. Central. Managed full/log retention eight/ten days.
- docs/operations.md#sql-backups and reports/operations-history.md#sql-log-maintenance-2026-10-13.

<a id="previous-handoff--retention-2026-10-13-deployed"></a>

### Retention (2026-10-13, deployed)

- Current/future dated sports, movie releases and TV episodes only; today is
  America/Chicago. Canceled sports removed. Undated/TBD records and TV tracking
  settings remain. Child/source/cache rows are deleted before parent rows.
- Ranking imports retain only the latest poll per source after successful replacement.
- Finished scraper runs: 7 days. YouTube: 10/channel. News: unchanged 10/source.
- Image web-scraper:retention-20261013 (also latest); production cleanup succeeded.
- 937 sports events, 50 movie releases, 31 episodes, 5 current polls remain;
  all 15 YouTube channels have 10 videos. No expired rows remain under the rules.
- Policy: docs/operations.md#data-retention. Report: reports/operations-history.md#retention-deployment-2026-10-13.

<a id="previous-handoff--wnba-stale-fixtures-2026-10-13-production-corrected"></a>

### WNBA stale fixtures (2026-10-13, production corrected)

- Confirmed unnecessary semifinal games 401918303/401918304 marked STATUS_CANCELED
  locally; history and timestamps preserved. They no longer appear as upcoming.
- Next active game is Oct 17 at 2:30 p.m. Central.
- Quick live review across 19 schedule feeds through Nov 30 found no other missing
  upcoming records. No binary change; omitted-fixture reconciliation remains manual.
- Report: reports/sports-history.md#wnba-stale-correction-2026-10-13.

<a id="previous-handoff--nfl-divisions-2026-10-13-configured-in-production"></a>

### NFL divisions (2026-10-13, configured in production)

- NFL source 10 now uses ?level=3: 32 unique teams in eight four-team divisions.
- Deployed parser already supports nested groups; refresh succeeded without rebuild.
- Existing page renders all division tables. AFC/NFC parent-heading implementation
  is documented in docs/sports-tables.md#my-newsfeed-nfl-standings for the frontend builder.
- Report: reports/sports-history.md#nfl-divisions-2026-10-13.

<a id="previous-handoff--nfl-standings-2026-10-13-deployed"></a>

### NFL standings (2026-10-13, deployed)

- NFL source ID 10 enabled; 2026 standings contain all 32 teams (16 AFC, 16 NFC).
- Image web-scraper:nfl-standings-20261013 (also latest); hourly collector includes
  NFL automatically. Runtime health check and production import pass.
- Existing /standings?sourceId=10 renders NFL and Kansas City; no frontend rebuild.
- Report: reports/sports-history.md#nfl-standings-deployment-2026-10-13.
- Display contract: docs/sports-tables.md#my-newsfeed-nfl-standings.

<a id="previous-handoff--parser-fixes-2026-10-12-deployed"></a>

### Parser fixes (2026-10-12, deployed)

- Movie Insider release-intro digital dates supported; theatrical-only candidates
  skipped. LastError cleared after production refresh.
- Big 12 midnight-Central JSON placeholders do not warn when local kickoff agrees
  exactly with RSS; real conflicts still warn. Arizona State has zero issues,
  game 179594 keeps 2:30 p.m. Central.
- Image web-scraper:parser-fixes-20261012 (also latest), runtime health check passes;
  targeted production ScraperRuns ID 53 succeeded. 96 tests pass, Clippy clean.
- Report: reports/operations-history.md#parser-fixes-deployment-2026-10-12.

Written 2026-09-21 to continue this project in a new chat. Read this first. It records where things
stand, what was decided, and what is still open. It contains no passwords.

<a id="previous-handoff--box-office-news-2026-10-12-deployed"></a>

### Box-office news (2026-10-12, deployed)

- User chose free news feeds: Deadline /v/box-office/feed/ and Variety
  /v/film/box-office/feed/, including Sunday estimates and Mon/Tue follow-ups.
- Box Office category ID 5615; enabled source IDs 7094 and 7095. Existing hourly
  --from-db covers seven days; no binary/schema/restart needed.
- Imported 20 articles, then repeated import with stable IDs/publication dates;
  zero errors, duplicates or pruning. Health checks pass. No structured totals
  or estimate/actual flags fabricated from news headlines or weekdays.
- SQL config seed: sql/boxoffice_news_seed.sql; frontend guide:
  reports/entertainment-history.md#my-newsfeed-boxoffice-news (expanded with frontend queries/cards/states); report: reports/entertainment-history.md#boxoffice-news-deployment-2026-10-12.

<a id="previous-handoff--rankingsmls-production-deployment-2026-10-11-0936-central"></a>

### Rankings/MLS production deployment (2026-10-11, 09:36 Central)

- Deployed web-scraper:rankings-mls-20261011; five poll feeds and MLS enabled.
  Production import: 125 ranking entries (23 Big 12), 30 MLS teams; 14 successes,
  no failures. Nine-source standings total 170 current rows.
- Scraper and website healthy; MLS visible in existing /standings?sourceId=9.
  /rankings still needs the separate frontend implementation. No web-app rebuild
  or SQL restart performed by this deployment. Backup and rollback image retained.
- Report: reports/sports-history.md#rankings-mls-deployment-2026-10-11.

<a id="previous-handoff--mls-standings-2026-10-11-deployed"></a>

### MLS standings (2026-10-11, deployed)

- Added espn-mls to existing standings importer/configuration; ESPN soccer/usa.1
  provides 30 teams in Eastern/Western conferences. No MLS schedule/result changes.
- Re-run configure-standings to register disabled, then enable and deploy updated
  importer. No additional standings schema/permissions. Both are now deployed.
- Frontend/configuration handoff: docs/sports-tables.md#my-newsfeed-mls-standings.

<a id="previous-handoff--national-top-25-rankings-2026-10-11-scraper-deployed"></a>

### National Top 25 rankings (2026-10-11, scraper deployed)

- User chose full national polls with current Big 12 members highlighted.
- ESPN AP football/men's/women's basketball, USC/IWSOC women's soccer and AVCA
  women's volleyball endpoints verified. Published dates and labels preserved;
  basketball currently returns April 2026 polls with 2025–26 season labels.
- New rankings module and additive SportsRankingSources/Polls/Teams schema;
  configure-rankings registers disabled, rankings-from-db imports hourly.
- Atomic corrections preserve IDs, publication snapshots retain history, stale
  provider responses cannot replace newer polls. Unknown fields stay nullable.
- Current official Big 12 names map highlights to separate Big12SchoolId;
  ESPN IDs are never used as conference IDs. Texas/Oklahoma excluded.
- Verified 119 offline tests plus isolated SQL/live tests; staging contains 125
  ranking entries, 23 Big 12 spots. CLI dry run: five successes, no failures.
- Frontend/deployment handoff: docs/sports-tables.md#my-newsfeed-top25-rankings.
  Report: reports/sports-history.md#national-top25-2026-10-11.

<a id="previous-handoff--production-deployment-2026-10-11-0852-central"></a>

### Production deployment (2026-10-11, 08:52 Central)

- Today's standings/digital updates deployed as web-scraper:standings-digital-20261011.
- All eight standings feeds and Movie Insider enabled; hourly scraper healthy.
  First production import succeeded: 140 current standings rows, 94 movie releases,
  zero failures/issues. Prior movie IDs/FirstSeenAt and schedule configuration preserved.
- Existing /events and /movies healthy. My-Newsfeed frontend changes still required;
  /standings is not implemented in the deployed web app. Data/handoff files ready.
- Verified SQL backup and rollback image retained; no SQL restart or Git push.
- Report: reports/entertainment-history.md#standings-digital-deployment-2026-10-11.

<a id="previous-handoff--digital-release-coverage-correction-2026-10-11-deployed"></a>

### Digital release coverage correction (2026-10-11, deployed)

- User wants current week, with digital coverage resembling Movie Insider.
- New Movie Insider provider confirms detail-page digital dates, scans all calendar
  pages, uses numeric IDs/digital:first through date or slug changes, and supports
  owned posters. --configure-movieinsider registers disabled, preserving settings.
- When To Stream now imports separate VOD dates as digital, plus existing SVOD.
- Live Oct 5–11 preview: Movie Insider 10 digital releases; When To Stream 5 digital
  and 9 subscription observations, zero issues. Existing observations retained.
- No new movie schema required. Movie Insider enabled and scraper deployed; My-Newsfeed
  should offer provider selection with Movie Insider preferred for Digital/VOD.
- Frontend/configuration handoff: docs/movies-and-box-office.md#my-newsfeed-digital-releases.

<a id="previous-handoff--sports-standings-2026-10-11-scraper-deployed"></a>

### Sports standings (2026-10-11, scraper deployed)

- Eight independent configurable standings feeds: MLB (2026), WNBA, full NWSL,
  Big 12 football, men's/women's basketball, soccer and volleyball.
- `--configure-standings` registers disabled feeds; `--standings-from-db` refreshes
  enabled sources, integrated into hourly compose and run outcome tracking.
- Additive `sql/standings.sql`: SportsStandingsSources/SportsStandings, complete
  atomic snapshots, stable per-season team/group IDs, last-good retention on errors.
  Restricted runtime grants updated; apply migration/grants before new compose.
- Official ESPN API and Big 12 tables verified. Soccer discovers the working
  current-season link; college basketball retains published 2025–26 labels.
- Validated: 105 offline tests plus isolated SQL and restricted-account live tests;
  all eight feeds imported 140 current rows and retained IDs on re-import.
- Separate My-Newsfeed application must add the UI/read models. Scraper deployment
  and enabling are complete. Full contract: docs/sports-tables.md#sports-standings.
  Frontend implementation handoff: docs/sports-tables.md#my-newsfeed-standings-frontend.

<a id="previous-handoff--audit-remediation-2026-10-10-central"></a>

### Audit remediation (2026-10-10 Central)

- Hardened scraper deployed; 12 findings addressed. Dedicated webscraper_ingest
  login is non-admin and cannot read results tables; .env is runtime-only,
  .env.migrations preserves setup credentials outside the hourly container.
- Ordinary DB connections do no DDL; --initialize-db/configuration uses migration
  credentials. Additive ScraperRuns health/progress table; deadlines and isolation.
- Recent existing-ID reconciliation updates finished schedules without importing
  historical results. MLB remains season=2026. Validated game 401907994 is FINAL.
- HTTP public/DNS/redirect and size guards, checked retention, publication-date
  preservation, provider disable semantics, atomic ESPN snapshots and RSS output,
  fall-back scheduler fix, native SQL TLS backend and clean dependency scan.
- 99 tests plus four isolated SQL tests pass; full restricted-account live run:
  64 successes, zero failures. Runtime read-only health check passes.
- SQL certificate activation completed after user approval: sql2025 restarted,
  CA-verified scraper connection works and unrelated CA is rejected. My-Newsfeed
  remains healthy; all thirteen findings now addressed. CA is read-only mounted;
  profiles use TrustServerCertificateCA, with no runtime certificate bypass.
- Guide: docs/operations.md#scraper-security-and-sql-credentials;
  report: reports/operations-history.md#audit-remediation-2026-10-10.

<a id="previous-handoff--mlb-schedule-2026-10-09"></a>

### MLB schedule (2026-10-09)

- `src/mlb.rs` uses the shared ESPN schedule parser/upsert; league-wide, today-forward
  Central dates only, excluding completed games. Announced next-season games included.
- `--configure-mlb` registers a disabled feed: `espn-mlb`, ProviderSchoolId 0;
  enable for existing hourly `--events-from-db`. Additive configuration `sql/mlb_seed.sql`.
- Baseball sport, home/away names/logos/IDs, venues, broadcast/game links, UTC starts,
  TBD dates and stable IDs through reschedules/cancellations. Doubleheaders stay separate;
  unresolved playoff participants update in place. No results-sync changes.
- No new schema beyond shared ScheduleStatus. Guide/My-Newsfeed integration contract:
  `docs/sports-schedules.md#mlb-schedule`. Deployed/enabled as feed 4193; 2,916 upcoming MLB rows
  imported twice with stable identities. User then restricted MLB to remainder
  of 2026: feed 4193 now uses season=2026; 2,894 backed-up 2027 rows removed,
  leaving 22 upcoming MLB games. Existing 959 sports events preserved.
  Report: `reports/sports-history.md#mlb-deployment-2026-10-09`. Live site renders games but lacks
  the dedicated MLB selector; web-app changes/deployment remain separate.

<a id="previous-handoff--wnba-schedule-2026-10-09"></a>

### WNBA schedule (2026-10-09)

- Both ESPN schedule imports now request current-month through next-year fixtures
  and return only today-or-later Central dates with unfinished status. Future
  cancellations remain available for reconciliation; page queries exclude them.
  Historical rows retained; My-Newsfeed must also apply its Central-today cutoff.
- WNBA league schedules use `src/wnba.rs` and shared `src/espn_schedule.rs` parsing;
  Kansas City continues through its provider wrapper. Scores/results untouched.
- `--configure-wnba` registers a disabled SportsEventsType row; existing hourly
  `--events-from-db` imports enabled feeds. Optional `season` and ESPN team-ID filters.
- One home-perspective row per game, WNBA-namespaced stable IDs, Central dates,
  UTC confirmed times, venues, links, logos, schedule status and cancellation handling.
- No extra schema beyond shared ScheduleStatus; seed `sql/wnba_seed.sql`.
  My-Newsfeed query/model/UI handoff: `docs/sports-schedules.md#wnba-schedule`; app checkout absent.
  Scraper deployed, both ESPN feeds enabled, hourly container recreated; 3 Current
  and 13 WNBA events imported twice with stable identities. See deployment report
  `reports/sports-history.md#scraper-professional-schedules-deployment-2026-10-09`.
  Live web app still lacks WNBA selector; web-app deployment is separate.

<a id="previous-handoff--kansas-city-current-schedule-2026-10-09"></a>

### Kansas City Current schedule (2026-10-09)

- `src/nwsl.rs` imports ESPN NWSL monthly scoreboards, filtering competitor 20907,
  into existing SportsEvents; separate My-Newsfeed results sync is untouched.
- `--configure-nwsl` registers a disabled feed; enable via SportsEventsType and
  existing `--events-from-db` handles collection. RssUrl supports `&season=YYYY`.
- Additive `sql/nwsl_events.sql`: nullable ScheduleStatus; canonical ESPN IDs,
  locked upserts, Central dates/UTC starts, explicit cancellations retained.
- Guide/My-Newsfeed query/model contract: `docs/sports-schedules.md#kansas-city-current-schedule`.
  Production feed 4191 enabled and imported; scraper deployed. My-Newsfeed
  UI deployment remains separate; source is accessible only through host mounts.

<a id="previous-handoff--tvmaze-episode-tracking-2026-10-09"></a>

### TVmaze episode tracking (2026-10-09)

- Added TvSeries, TvEpisodes and TvTrackedSeries using `sql/tvmaze.sql`; seed script
  tracks explicit IDs 83073,64950,45039,33352,90632 idempotently, preserving disabled settings.
- Final schema/column contract delivered before production migration to chat and
  MyNewsFeed checkout (`db/tvmaze.sql`, `db/tvmaze_seed.sql`,
  `deployment/tvmaze-developer-handoff.md`). No TV frontend UI implemented here.
- `src/tvmaze.rs` fetches full episode lists with specials. Production has 5 series,
  90 episodes (83 date-only, 7 timed, 2 specials); no collection errors.
  Blank airtime/noon airstamp -> NULL confirmed UTC time, with raw fields retained.
- Original scheduling only: NBC Line of Fire is ID90632 (2026), not the 2003 show.
  Never derive Peacock or regional availability from these tables.
- Configure `--configure-tv`, collect `--tv-from-db`, manual cache bypass
  `--tv-force-refresh`. Hourly Docker command includes --tv-from-db. Tracking
  has enabled, last-attempt/success/error/count, one-hour validated JSON cache,
  retry-after and per-series lease controls. No episode deletion on missing lists.
- Staging/SQL/mock tests passed for IDs, batch/special/date-only semantics,
  reschedules, disabled seeds, incomplete snapshots, cache and failure isolation.
  API data: CC BY-SA 4.0; app attribution/license links required.
- Docs: tv.md#tvmaze and tv.md#tvmaze-developer-handoff; initial-import report in reports.
  Runtime rollback image before-tvmaze-20261009; staging database
  WebScraper_Tvmaze_Validation_20261009 retained for inspection.

<a id="previous-handoff--movie-posters-2026-10-09"></a>

### Movie posters (2026-10-09)

- Source-first poster enrichment implemented in `src/movie_metadata.rs` and `src/posters.rs`.
  The existing Movies.PosterUrl field now has 28 HTTPS posters across 33 movie rows.
  All movie/source-link/release IDs and counts (33/33/76) preserved; OriginalYear populated
  from movie-specific source metadata. No title-only or release-week-year matching.
- Prefer linked DVD movie portrait posters; WTS landscape stills rejected. Cross-source
  title/original-year matching uses verified source pages and excludes ambiguities.
  IMDb/TMDB matches take precedence; existing valid posters remain intact.
- Additive `sql/movie_posters.sql` creates cache and status/provenance/retry fields.
  Positive lookups/validity cached 30 days, misses 7 days, source errors 6 hours.
  Normal `--movies` reuses metadata from already fetched details. Failure cannot undo
  release ingestion. Warm deployed-container run: 28 preserved, 33 hits, 0 source requests.
- Poster-only backfill: `--movie-posters`; force cached misses: `--retry-missing-posters`
  alongside it. Audit: `cargo run --example audit_movies -- WebScraper /tmp/movie-audit.json`.
- User selected linked source posters plus a generic local UI placeholder, not an
  API fallback (2026-10-09). MyNewsFeed's Movies/home poster areas use shared
  `img/movie-placeholder.svg`; missing or failed image links reveal it. NULL remains
  in the DB for unmatched movies so enrichment can retry. Host app source updated
  at `/home/jcarpio/Projects/My-Newsfeed`; rollback image tag:
  `mynewsfeed-web:before-movie-placeholder-20261009`.
- Five unmatched: Olmo (2025), The Birthday Party (2025), Animals (2026), Oasis: Don’t
  Look Back in Anger (2026), V/H/S Mixtape (2026). Leave NULL; no metadata API keys
  configured in scraper or MyNewsFeed. TMDB would require credentials and attribution
  plus an API client; current implementation uses existing free sources only.
- Six saved posters visually/HTTP verified. MyNewsFeed Movies/home HTML renders them,
  including five Friday theater posters. Standalone service updated, hourly flags unchanged.
- Guide/report: `docs/movies-and-box-office.md#movie-posters`, `reports/entertainment-history.md#movie-posters-2026-10-09`.
  Rollback image: before-posters-20261009; host backup: `.deployment-backups/20261009-posters/`.

<a id="previous-handoff--big-12-expansion-2026-10-09"></a>

### Big 12 expansion (2026-10-09)

- All 16 current provider full-member school feeds are enabled. Scope is Football,
  Men's Basketball, Women's Basketball, Soccer and Volleyball; Oklahoma State has
  no volleyball feed coverage. Kansas retains configuration ID 1 and its historical IDs.
- `src/big12.rs` caches school metadata, collects school RSS, and loads structured
  identity metadata in bounded monthly requests. Linked game IDs produce one public
  SportsEvents row with per-feed records in SportsEventSources. Invalid identity data
  is quarantined; known timing conflicts retain primary RSS times and record warnings.
- Additive schema: `sql/big12_events.sql`, source backfill/upsert scripts, and
  `dbo.SportsEventSchoolCalendar` view. School-filtered .NET queries should use this
  view, not only SportsEvents.SportsEventsTypeId. The .NET repo was not changed.
- Production: 943 public events, 1,444 source observations, 3 quarantined provider
  records. All 101 original Kansas IDs, FirstSeenAt, primary source, opponents and
  logos preserved. All 16 logo URLs load. Per-feed attempts/success/error/counts saved.
- Setup: `--configure-big12` registers new feeds disabled; `--enable-big12` explicitly
  enables them. Existing settings are preserved by configuration alone. Neither flag
  can run with `--dry-run` or `--every-hours`. Existing hourly `--events-from-db` works.
- Full staging runs repeated concurrently without count/ID changes. Staging database:
  `WebScraper_Big12_Validation_20261009`; fixture/import SQL tests are opt-in and rollback.
- Guide: `docs/sports-schedules.md#big12-sports`. Read-only audit: `cargo run --example audit_sports --
  WebScraper /tmp/sports-audit.json`. Production rollback image: before-big12-20261009;
  host source/sports snapshot: `.deployment-backups/20261009-big12/`.

<a id="previous-handoff--school-logos-2026-10-09"></a>

### School logos (2026-10-09)

- Conference RSS `<s:teamlogo>` was the Big 12 site logo. `fetch_events` now
  resolves the feed's nonzero `school_id` through `/calendar.aspx` members
  metadata (`id` -> `image.url`), keeping opponent logo extraction unchanged.
  Missing metadata/image produces NULL, never a conference/channel fallback.
- Existing RSS events update via the URL-keyed upsert; retained older events
  receive a TeamLogoUrl-only update scoped to the single-school feed type.
- Representative live Kansas RSS and calendar metadata fixtures are in
  `tests/fixtures`; tests cover Kansas, Kansas State, missing images, all-schools
  feeds, standalone school logos, and conference/channel rejection.
- Kansas member ID 3 supplies `/images/logos/jhwk4C_RF_OL (3).png`.
  HTTP 200 and visual Jayhawk verification completed.
- Deployed and refreshed all 101 existing Kansas rows: 91 live RSS upserts plus
  10 retained-event logo updates. Row count stayed 101; IDs, URLs, FirstSeenAt and
  opponent logos were unchanged. Report: `reports/sports-history.md#kansas-logo-fix-2026-10-09`.
  43 all-target tests and the rollback-only school-logo SQL test passed.

<a id="previous-handoff--movie-release-extension-2026-10-08"></a>

### Movie release extension (2026-10-08)

- Scope: **U.S. only**, theatrical, digital purchase/rental, subscription streaming,
  and DVD/Blu-ray/4K releases; user chose **free public sources**.
- Source recommendation: DVDs Release Dates for theatrical/digital/disc, When To
  Stream for subscription dates. Calendar and sample streaming detail HTTP access
  verified. Movie Insider detail access returned 403; Watchmode paid API not selected.
- Four new tables created in live `WebScraper`: `MovieReleaseSources`, `Movies`,
  `MovieSourceLinks`, `MovieReleases`. Movie ingestion is populated and active.
- Migration: `sql/movie_releases.sql`, included in `Db::open`. Standalone setup:
  `cargo run --example setup_movie_schema`; append `-- --verify` for rollback-only
  SQL fixtures. Migration rerun and SQL integration checks passed, as did 19 unit tests.
- Read `docs/movies-and-box-office.md#movie-releases` for source evidence, fields, identity/rescheduling
  rules, weekly query, and limitations. Dates are DATE; audit timestamps are UTC.
- **Read-only movie preview implemented** in `src/movies.rs`, exposed through
  `cargo run --example movie_preview -- --week-of YYYY-MM-DD --output report.json`.
  It opens no DB connection. Calendar discovery and detail-date extraction cover
  theatrical, digital, disc, and subscription releases. Eight parser tests pass.
  Main CLI ingestion/database upserts are now implemented via `--movies`; the standalone Docker service includes movies in its hourly runs.
- Live preview for 2026-10-05 through 2026-10-11 completed: 5 theatrical, 16 digital,
  9 subscription records (8 titles), 9 disc editions (4 titles). Checked 56 detail
  pages with zero HTTP/parser issues after fixes; excluded 4 TV series. No DB writes.
  Reports: `reports/entertainment-history.md#movie-preview-2026-10-05` and `.json`. All 27 Rust tests pass.
- Live schema inspection also found `ResultFollows` and `ResultGames`, beyond the
  seven original tables documented below. They were not changed.

<a id="previous-handoff--what-this-project-is"></a>

### What this project is

A Rust CLI (`/workspaces/Rust_Projects/web_scraper`, branch `feature/next-stage-build`, remote
`Wolvie1975/Rss-Rust-Scrape`) that scrapes feeds and pages and stores them in a **SQL Server**
database, `WebScraper`, on the host `sql2025` (Docker network name, port 1433). A separate .NET
front end (not in this repo) will read the database and also act as the admin UI.

The database is the only link between the two programs. The admin edits lookup rows; the scraper
reads them on its next run.

<a id="previous-handoff--source-layout"></a>

### Source layout

| File | Role |
|---|---|
| `src/main.rs` | CLI flags, `run_once()` (one full pass), hourly loop, Central-time slot math |
| `src/db.rs` | All SQL: schema creation (`Db::open`), upserts, pruning, lookup reads |
| `src/feed.rs` | News scrape: finds a site's RSS/Atom feed (own URL, advertised, common paths) |
| `src/events.rs` | Sidearm calendar RSS (custom `ev:`/`s:` fields) -> `SportsEvents` |
| `src/youtube.rs` | YouTube channel RSS, falling back to the channel Videos page -> `YouTubeVideos` |

Connection string: `MSSQL_CONNECTION_STRING` in a git-ignored `.env` in `web_scraper/`, ADO format,
`Database=master` (the scraper creates/uses `WebScraper` itself). Value must stay in single quotes.
Never print or commit it. Tests: `cargo test` (19 unit tests, all passing).

<a id="previous-handoff--database-7-tables-dbo"></a>

### Database (7 tables, `dbo`)

Scraper-owned columns are overwritten on each run; lookup tables are managed by the user/admin.

- **Sources** (lookup, drives the news scrape): `Url`, `Label`, `Enabled`, `SourceCategoryId`
  -> **SourceCategories** (`Category_Name`). Scraper writes `LastScrapedAt`, `LastError`.
- **Pages**: one row per story. `SourceId` (NOT NULL FK -> Sources), `Url` = canonical URL (do not
  join to Sources on it), `Published`, `ImageUrl`, `Description`, `ScrapedAt`.
- **SportsEventsType** (lookup): `RssUrl` (max 450, unique), `EventsTypeName`, `SchoolName`
  ("Kansas": used to split sport from school in titles). -> **SportsEvents** via `SportsEventsTypeId`.
- **SportsEvents**: `EventDate` (Kansas-local date), `StartsAtUtc`/`EndsAtUtc`, `TimeTbd` (football),
  `IsAway`, `Sport`, `Opponent`, `Tv`, links/logos. Past games are kept; nothing removes cancelled ones.
- **YoutubeVideoFeed** (lookup): `ChannelId` (`UC...`, unique), `ChannelName`, `Url`. -> **YouTubeVideos**
  via `YoutubeVideoFeedId`. Currently 3 channels: CultOfMush (id 1), So Much AI News (43), Destin (44).
- **YouTubeVideos**: `VideoId` unique, `PublishedAt`, `ViewCount`, `Description`, ...
- `UrlHash` on Pages/Sources/SportsEvents is a computed, persisted SHA-256 column used for uniqueness.
  Never write to it.
- All FKs are `NO ACTION`: a source/feed/type row with children cannot be deleted. Sources have `Enabled`;
  YouTube feeds and event types have **no** enable flag (every row is scraped).
- No SQL client is installed in the container. To inspect the DB, add a throwaway `examples/chk_tmp.rs`
  (tiberius + `cargo add --dev tokio --features macros,rt,net`), run it, then delete it and
  `cargo remove --dev tokio`. This was done many times; leave the project clean afterwards.
- `Db::open` creates every table/column/index if missing and adds newer columns to older databases.
  It changed nothing on the live DB. The user adds columns by hand sometimes; **re-read the live
  schema when asked to "review the tables"** and compare with `db.rs`.

<a id="previous-handoff--cli-targetreleaseweb_scraper-see---help"></a>

### CLI (`./target/release/web_scraper`, see `--help`)

- News: `--from-db` (scrape enabled Sources), `--per-source N` (cap + prune per source), `--keep N`
  (global cap), `--last-days N`, `--require-date`, `--save-sources -f urls.txt`, `-o feed.xml`.
- Events: `--events-from-db`, `--events-feed URL`. YouTube: `--youtube-from-db`, `--youtube-feed URL`,
  `--youtube-latest N` (default 5).
- `--dry-run`: no row writes (but it still opens the DB and creates missing tables). `--every-hours N`:
  loop, aligned to midnight **America/Chicago** (DST-aware); errors in a run are logged, loop continues.
- Any DB flag (`--from-db`, `--events-*`, `--youtube-*`, `--save-sources`) implies saving. Quote URLs
  containing `&` in the shell.

<a id="previous-handoff--runtime-state-2026-10-08"></a>

### Runtime state (2026-10-08)

- The standalone `web-scraper` Docker service runs hourly independently of VS Code,
  with news, sports, YouTube and `--movies`. Restart policy: `unless-stopped`.
- Host Compose checkout: `/home/jcarpio/Projects/Rss-Rust-Scrape`.
  Workspace checkout: `/workspaces/Rust_Projects/Rss-Rust-Scrape`.
  Both were updated for movie ingestion. Future changes must reach the host checkout
  before `docker compose up -d --build` is run there.
- Logs: `docker logs --tail 50 -f web-scraper`. Stop: `docker stop web-scraper`.
  Persistent RSS output: `/data/feed.xml` in volume `webscraper_scraper-data`.
- First live import saved 39 current-week releases. A refresh inside the rebuilt
  container saved 73 releases including other announced dates on tracked movies,
  with zero source issues. SQL repeat-import, reschedule and disabled-source tests
  passed with rollback-only fixtures; 29 main-binary tests and 10 preview tests pass.
- Rollback image: `web-scraper:before-movies-20261008`; host source backup:
  `.deployment-backups/20261008-movies/source.tar`. No credentials are in this file.
- Git: working tree was clean at last check; the user commits and **pushes themselves**
  (the container has no GitHub credentials). Do not commit unless asked. `feed.xml` is tracked and is
  rewritten by every run, so it shows as modified.
- Latest DB contents: 19 enabled sources x 10 stories, 38 Kansas events, 15 videos (5 per channel).

<a id="previous-handoff--decisions-made"></a>

### Decisions made

- Stored times are **UTC**. Central conversion is done in the .NET front end (not in the DB). A view
  with `AT TIME ZONE` was offered as an alternative and not built. `EventDate` needs no conversion.
- News: per-source cap **10** (older rows deleted, undated rows first). No global cap in use.
- Each source is scraped feed-first; falls back to the page's own metadata (one undated row).
  Article-page `og:image` fallback fills missing images; an image shared by several articles is treated
  as a site logo and dropped (why Linux Today has none).
- Upserts never blank stored image/description/date with an empty value and do not refresh `ScrapedAt`.
- YouTube: latest **5** per channel. RSS first; if it fails, the channel Videos page
  (`/channel/<id>/videos`). Page dates are **estimates** from "N days ago" (short form "9h ago" is also
  handled); an estimate is used only for a video not yet stored and never overwrites a stored date.
  Views/ratings/description are not taken from the page (user said not to worry about view count).
- Calendar `Sport` comes from `SportsEventsType.SchoolName`, falling back to a shared-words guess.

<a id="previous-handoff--open-items--known-issues"></a>

### Open items / known issues

1. **ESPN dates drift.** ESPN's feed labels times "EST" all year, so dates are ~1 hour ahead. `Published`
   is capped at scrape time, and because that cap is re-applied on each run, ESPN stories float to the
   top with the latest scrape time. Proposed fix (not built): on update never move an existing
   `Published` later. The user had not yet said yes.
2. **YouTube RSS returns 404** for all channels (also via plain curl) since 2026-09-20; page fallback covers it.
   Check whether RSS has recovered.
3. The channel-page fallback depends on YouTube's undocumented page data and has already changed
   layout once. The official alternative is the YouTube Data API (needs a Google Cloud API key in `.env`).
4. Feed streams: the Videos tab excludes live streams; a stream stored earlier via RSS can outrank a
   newer video until it ages out.
5. Scheduler durability: not started on container restart. Offered: devcontainer `postStartCommand`,
   systemd unit, or Dockerfile/compose.
6. Cancelled/removed future games stay in `SportsEvents`; a cleanup was offered, not built.
7. Big 12 sports (`big12sports.com`) news has no RSS; the calendar feed is separate and works.

<a id="previous-handoff--front-end-net-notes-given-to-the-user"></a>

### Front-end (.NET) notes given to the user

- Convert UTC to Central at display time: `TimeZoneInfo.FindSystemTimeZoneById("America/Chicago")`,
  and call `DateTime.SpecifyKind(x, DateTimeKind.Utc)` first (SQL returns Kind=Unspecified).
- Compare "today/yesterday" using Central dates. Filter upcoming games with `EventDate >= today (Central)`.
- Handle nulls: `ImageUrl`, `Published`, `Description`, event `Tv`/`StreamUrl`/logos, `IsAway`.
  `TimeTbd = 1` means show "time TBD". Show `YoutubeVideoFeed.ChannelName`, not the video row's author name.
- Admin: edit only lookup tables (`Sources`, `SourceCategories`, `YoutubeVideoFeed`, `SportsEventsType`);
  disable a source with `Enabled = 0` instead of deleting; never write scraper-owned columns or `UrlHash`.
- EF Core: map `UrlHash` read-only/ignored; use `AsNoTracking()`; don't rely on ID order (gaps from pruning).

<a id="previous-handoff--moving-to-another-computer-summary-of-advice-given"></a>

### Moving to another computer (summary of advice given)

Clone the repo; install Rust >= 1.85 (project uses edition 2024; container has 1.97); recreate `.env`;
point `Server=` at a host reachable from the new machine (`sql2025` only resolves inside the Docker
network); move the DB by BACKUP/RESTORE (keeps categories and lookup rows) or let the scraper recreate
an empty schema and re-seed with `--save-sources -f urls.txt`; run the loop under systemd/Docker/Task Scheduler.

<a id="previous-handoff--working-conventions-with-this-user"></a>

### Working conventions with this user

- Confirm before anything outward-facing; the user pushes to GitHub, and restarts/pauses the scheduler
  on their own call.
- Prefer verifying against the live DB and live feeds (dry-run first, then a real run) and report
  exactly what was and wasn't checked.

