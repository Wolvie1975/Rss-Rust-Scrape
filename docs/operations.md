# Scraper operations

This is the canonical guide for retention, SQL backups, credentials/security and performance. Current policies supersede older deployment notes: run logs seven days, YouTube ten per channel, news ten per source, past/canceled schedules removed, and only current rankings. Keep credentials out of logs and Git. Performance measurements are workload-specific.

## Contents

- [Optional sports box scores](#optional-sports-box-scores)

- [Kansas City local news](#kansas-city-local-news)

- [Indie video game news](#indie-video-game-news--october-19-2026)

- [Big 12 news sources](#big12-news-sources)

- [F1 production operation](#f1-operations)
- [Top Headlines sources](#top-headlines-sources)

- [Movie retailer links](#movie-retailer-links)

- [News article URL identity](#article-url-identity)
- [NHL production setup and operation](#nhl-operations)
- [Database schema snapshot](#database-schema-snapshot)
- [Production data retention](#data-retention)
- [WebScraper SQL backup maintenance](#sql-backups)
- [Scraper hardening and SQL credential split](#scraper-security-and-sql-credentials)
- [Scraper performance instrumentation and optimizations](#scraper-performance)

## Box-score period labels

Release `web-scraper:period-labels-20261019` labels innings, quarters, halves,
overtime and explicitly identified shootouts in the existing version-1 cache.
`--repair-boxscore-periods` performs bounded historical label-only repair;
`--dry-run` previews it. Normal optional enrichment labels new snapshots through
the existing hourly schedule. No schema or frontend deployment is required.
[Mapping evidence, preservation checks and rollback](../reports/sports-period-labels-20261019.md).

## Men's college basketball news

Four enabled sources supply **Men's College Basketball News**: national ESPN,
CBS Sports and ZAGSBLOG, plus Heartland's Big 12 basketball feed. Initial production
import saved 37 articles with no failures and preserved existing URL ownership.
Live category rendering/pagination passed. No rebuild/restart was required.
[Source IDs, URLs and registration workflow](mens-college-basketball-news.md).

## Database review and statistics maintenance

[October 19 review](../reports/database-review-20261019.md): CHECKDB passed;
38 stale optimizer statistics were refreshed across 13 tables. Retention and
orphan checks found no application-data cleanup candidates. See the report for
storage, backup-history verification limits and the frontend login hardening
follow-up. [Targeted statistics maintenance](../sql/maintenance_refresh_stale_statistics.sql)
defaults to preview mode; no new maintenance schedule was installed.

## Optional sports box scores

Release `web-scraper:boxscores-20261019` adds bounded, cached ESPN game detail
through the existing hourly `--events-from-db` path. Explicit refresh:
`web_scraper --boxscores-from-db`; preview adds `--dry-run`. Migration setup is
`--configure-boxscores` with migration credentials and explicit WebScraper targeting;
runtime does no DDL. The collector writes only its new optional statistics cache,
never result scores. Missing detail leaves existing fallbacks intact and failed
refreshes retain last-good snapshots. See [limits, deployment evidence and
rollback](../reports/sports-boxscores-20261019.md) and [the frontend handoff](<../New Development/sports-boxscores-frontend-handoff.md>).

## YouTube channel categories

YouTube channels have their own category lookup, separate from news
`SourceCategories`, so YouTube-only categories never appear as empty news
categories. `dbo.YoutubeCategories` (`ID`, `Category_Name`) is referenced by the
nullable `dbo.YoutubeVideoFeed.YoutubeCategoryId`; videos inherit their channel's
category through `YouTubeVideos.YoutubeVideoFeedId`. Channels the collector adds
later stay uncategorized (NULL) until assigned.

| Category | Channels (October 26, 2026) |
| --- | --- |
| Sports | Field of 68, BYU Sports Nation, Locked On Big 12, Locked on BYU, Big 12 Sports, CBS - Eye on College Basketball, Open For Business, Sleepers Media, Awful Announcing, Robin Lundberg |
| Video Games | Destin, Indie Games Hub, Xbox Nation, ID@Xbox, Xbox, MrMatty Plays, Colton Eastwood, CultOfMush |
| Tech | WaveForm Podcast, The Verge, Android, Marques Brownlee, iJustine, MrMobile, ShortCircuit, C. Scott Brown, Shane Craig, Auto Focus, CodingBite, The Linux Experiment, Paul J Lipsky |
| News & Politics | Bulwark |

Schema: [`sql/youtube_categories.sql`](../sql/youtube_categories.sql) (additive, also
run by `--initialize-db`). Assignments: [`sql/youtube_categories_seed.sql`](../sql/youtube_categories_seed.sql),
keyed by `ChannelId`; rerunning it resets the listed channels to these categories.
Apply both with the migration profile against **WebScraper**. Collection needs no
rebuild or restart; the ingest role only gains `SELECT` on the lookup. Applied to
production twice on October 26, 2026; read-back: Sports 10 channels, Video Games 8,
Tech 13, News & Politics 1, none uncategorized, ingest grant present.
[Frontend handoff](<../New Development/youtube-categories-frontend-handoff.md>).

Frontend: join `YouTubeVideos` → `YoutubeVideoFeed` → `YoutubeCategories` by ID and
resolve categories by name, not hardcoded IDs. Decide how to show NULL-category channels.

## Kansas City local news

Five enabled publishers populate **Kansas City News**: KSHB 9172, Flatland 9173,
The Pitch Food & Drink 9174, KCUR's Kansas City topic 9175 and The Beacon's Kansas
City category 9176. Production run 1244 collected 45 articles; live `/local`
returned all 45 across four cursor pages. No restart or deployment was needed.
See [feed URLs, scope, safe first collection and verification](kansas-city-news.md).
The [repeatable seed](../sql/kansas_city_news_seed.sql) deliberately registers
missing sources disabled; populate and verify before enabling to avoid an empty
frontend category switch.

## Indie video game news — October 19, 2026

Three enabled indie-focused publishers were added to the existing **Video Games**
category (ID 11), alongside GamingOnLinux, Pure Xbox and Eurogamer:

| Publisher | Source ID | RSS feed |
| --- | --- | --- |
| Indie Games Plus | 9037 | https://indiegamesplus.com/feed/ |
| The Indie Informer | 9038 | https://the-indie-in-former.com/feed/ |
| Indie Hive | 9039 | https://indie-hive.com/feed/ |

Repeatable registration: [`sql/indie_videogames_news_expand.sql`](../sql/indie_videogames_news_expand.sql).
Apply with the migration profile, explicitly targeting **WebScraper**. Existing
source settings are preserved on repeated execution. No rebuild, restart or
scheduler changes were needed; normal hourly `--from-db` collection includes them.

All three live RSS feeds parsed successfully. The collector dry-run reported
three source successes and zero failures. Targeted production import run **1242**
succeeded; database read-back verified **ten articles per source**, enabled sources
and no `LastError`. The production exec exited zero and the scraper was healthy.
These are dated observations, not ongoing health guarantees.

The frontend uses the existing `Pages` / `Sources` / `SourceCategories` contract;
resolve category membership dynamically and filter enabled sources. Preserve the
ten-per-source retention, canonical verification and cross-source URL ownership
rules. No frontend rendering change was implemented or verified.

Indie Retro News also had an active feed but was not needed for this addition.
The Indie Game Website feed was stale (latest entry December 2021), Indie Game
Fans returned HTTP 500, and Indie Game Bundles mostly covered deals; none was added.

## Xbox Wire — October 27, 2026

**Xbox Wire** (source **11830**, https://news.xbox.com/en-us/feed/) was added to
**Video Games** (ID 11), now nine enabled sources. Repeatable registration:
[`sql/xbox_wire_news_seed.sql`](../sql/xbox_wire_news_seed.sql) registers it
**disabled**; apply with the migration profile (`examples/scoreboard_sql`
targets WebScraper). A repeat inserted zero rows before and after enablement.

The dry-run reported one success and zero failures. Targeted `--db --per-source 10`
collection stored **ten articles** (nine with images, from `content:encoded`) with
no `LastError`; ID 11830 was then enabled in a transaction guarded by category,
URL hash, clean error state and stored content. No rebuild, restart or scheduler
change; hourly `--from-db` collection includes it.

The feed is behind Cloudflare: a generic browser user agent receives HTTP 403,
while `My-Newsfeed/1.0` receives the RSS. A future block appears as `LastError`
on 11830; do not work around it. Xbox Wire occasionally posts off-site cross-links
(for example minecraft.net with a `#new_tab` fragment); article identity skips
the canonical check on failure and stores the feed link, which may lack an image.
To stop collection, disable 11830 only.

<a id="big12-news-sources"></a>

## Big 12 news sources

Five enabled sources in **Big 12 News** supply the college sports hub: official
all-sports 8997, Heartland 8998, football 8999, men's basketball 9000 and baseball
9001. The dedicated women's basketball source was omitted by request. Conference-wide
coverage can still include women's sports. Existing hourly `--from-db` collection
handles these sources without rebuilding or restarting the scraper.

The migration-profile script [`sql/big12_news_seed.sql`](../sql/big12_news_seed.sql)
was applied to WebScraper on October 18, 2026. Repeated execution preserves
existing source settings. A production dry-run and actual import passed: five
source successes, no failures, 46 retained articles. Four duplicate cross-source
URLs were skipped without changing ownership. Read-back verified enablement,
refresh timestamps and no source errors; both production containers were healthy.

See the [frontend integration contract](big12-news-frontend-integration.md) for
URLs, source IDs, category queries, overlap handling and UI acceptance. Disable
only these source IDs to stop their future collection; preserve other sources,
article ownership, shared categories and the hourly configuration.

<a id="f1-operations"></a>

## F1 production operation

F1 news is also enabled: four RSS sources in **F1 News** are collected through
normal hourly `--from-db` news ingestion. Initial production verification saved
ten articles per source, with four successes and zero failures. No further image
or scheduler change was required. See [news registration](f1-operations.md#f1-news-registration)
and the [frontend News tab contract](f1-news-frontend-integration.md).

Current verified scraper release: `web-scraper:f1-20261018`, deployed October 18,
2026. Enabled `F1Source` record 1 collects the calendar, Grand Prix/sprint
classifications and driver/constructor championships through the existing hourly
`--events-from-db` path. Deployment preserved the runtime environment, command,
TLS mount, data volume, network, resource/security limits and restart policy.
Deployed-container run 1206 succeeded; the frontend production reader could query
all four F1 views. The next hourly slot was logged, but not awaited during verification.

Follow [F1 operations](f1-operations.md) for additive migration order, source
configuration, read-only checks, manual refresh, failure preservation and disablement.
The baseline schema snapshot predates F1. Migration creates `F1Source`,
`F1Snapshot` and four read views; runtime collection performs no DDL.
[Deployment evidence and rollback](../reports/f1-production-20261018.md) identifies
the protected host backup and retained previous container/image.

<a id="top-headlines-sources"></a>

## Top Headlines sources

Six enabled production news sources are assigned to **Top Headlines**: NBC News
8903, CBS News 8904, The New York Times 8905, The Guardian 8906, Al Jazeera 8907,
and Sky News 8908. BBC News 4084 and NPR 7093 remain in General.
These use the existing `Sources` / `SourceCategories` / `Pages` contract and the
normal `--from-db` collector; no separate collector or scheduler arguments are
required. Resolve category IDs from `Category_Name`, rather than hardcoding them.

[`sql/top_headlines_seed.sql`](../sql/top_headlines_seed.sql) was applied to
production. It creates the category if missing, inserts missing feeds enabled,
and preserves existing source settings on repeated execution. It requires the
migration profile; the runtime account cannot create categories. Source
registration was verified, but first-collection success for each of the six feeds
was not separately verified during registration.

The [frontend handoff](top-headlines-frontend-integration.md) includes exact URLs,
source IDs, query fields, empty states and acceptance checks. No frontend
implementation status is inferred from source registration.

<a id="movie-retailer-links"></a>

## Movie retailer links

Hourly `--movies` collection optionally enriches theatrical Fandango and digital
Fandango at Home links. Apply `sql/movie_commerce.sql`, then
`sql/movie_commerce_view.sql`, then `sql/movie_commerce_permissions.sql` with
migration credentials; runtime ingestion performs no DDL. Existing source URLs,
release dates and the scheduler are preserved. Match failures use labeled search
fallbacks and do not fail the movie import. See [cache, verification and rollback
policy](movie-purchase-links.md) and [the standalone frontend integration request](movie-commerce-frontend-integration.md).

<a id="article-url-identity"></a>

## News article URL identity

News ingestion now uses verified, same-source redirect/canonical URL aliases to
retain a stable article ID after a publisher changes its URL. It never merges
from title, image or timestamp similarity. Unknown foreign references and global
URL ownership collisions are reported without deleting/reassigning articles.

Apply the separate additive migration `sql/article_identity.sql`, procedure batch
`sql/upsert_article_identity.sql`, and `sql/article_identity_permissions.sql`
before deploying the new collector. Runtime startup remains DDL-free. Two new
scraper-owned tables store URL aliases and retired-ID mappings; existing article
retention cascades to them. Retired metadata is archived during a merge.

The approved existing-duplicate policy keeps the oldest ID. Browser saved IDs
that refer to a retired row need the frontend to follow `ArticleIdRedirects`;
recording those mappings does not itself update browser storage.

Detailed matching rules, request budgets, SQL/permission contracts, saved-ID
recovery and tests: [article URL identity](article-url-identity.md).
Production evidence and rollback:
[October 16 deployment](../reports/operations-history.md#article-identity-deployment-2026-10-16).

<a id="nhl-operations"></a>

## NHL production setup and operation

NHL is deployed as `web-scraper:nhl-20261014`, verified October 14, 2026.
Schedule/results source **5637** and standings source **13** are enabled.
The existing hourly command already includes `--events-from-db` and
`--standings-from-db`; preserve its environment, TLS certificate mount, persistent
volume, network, limits, security settings, logging and restart policy.

For a new installation, initialize the shared schema and provision runtime
permissions first. Register NHL using the migration profile:

```sh
( set -a; . ./.env.migrations; set +a; cargo run --locked -- --configure-nhl )
```

Registration creates missing shared results/standings objects and seeds only NHL
sources disabled. Repeating it preserves URLs, enabled flags and limits. Runtime
imports use ingestion credentials; migration credentials stay out of the scheduler.
After deploying the updated image and validating its feeds, enable the sources:

```sql
BEGIN TRANSACTION;
UPDATE dbo.SportsEventsType SET Enabled=1
WHERE ProviderKey='espn-nhl' AND ProviderSchoolId=0;
UPDATE dbo.SportsStandingsSources SET Enabled=1
WHERE ProviderKey='espn-nhl' AND Sport='Hockey';
COMMIT;
```

Read-only feed verification:

```sh
cargo test --locked nhl::tests::live_nhl_feeds -- --ignored --nocapture
cargo run --locked -- --events-feed https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/scoreboard --dry-run
```

The SQL integration test requires a newly created isolated database named
`WebScraper_NHL_Validation_*` selected through `NHL_TEST_DATABASE` and migration
credentials. Never point it at production. October 14 verification passed this
test in `WebScraper_NHL_Validation_20261014`, which was removed afterward.

Production verification imported 1,239 fixtures, 170 results and 32 standings teams.
Repeated imports retained IDs and first-seen timestamps. Real release-image run
106 succeeded with zero source failures, and normal retention succeeded. The
container became healthy and logged its next hourly slot for 09:00 Central that
day; the scheduled slot was not awaited as part of deployment verification.

Rollback image: `web-scraper:before-nhl-20261014`. The prior container is retained
stopped as `web-scraper-before-nhl-20261014`. Protected host backups are under
`.deployment-backups/20261014-nhl/`. Disable both NHL sources before restoring the
older collector so it does not attempt unsupported feeds. Keep rollback profiles
private. Full verification and rollback details are in the
[deployment record](../reports/nhl-production-20261014.md) and
[operations history](../reports/operations-history.md#nhl-operations-deployment-2026-10-14).

<a id="data-retention"></a>

## Production data retention

Effective October 13, 2026. Cleanup runs after every non-dry-run collection,
including runs with individual provider failures. A SQL transaction removes
child records before parent records. Today is determined in America/Chicago.

| Data | Rule |
| --- | --- |
| News articles | Newest 10 per source; older rows deleted (unchanged) |
| YouTube videos | Newest 10 per channel; older rows deleted |
| Sports events | Delete dates before today and canceled/unnecessary games, with their source observations |
| Movie releases | Delete release dates before today; remove orphan poster caches, source links and movies |
| TV episodes | Delete dates before today; timestamped episodes use their Central date |
| Rankings | Keep only each source's current poll and current team rows; replace atomically after successful import |
| Scraper runs | Delete finished runs started more than 7 days ago |
| Standings | EPL retains only current rows; other leagues retain existing history |
| NHL, MLS and Championship results | Current season, separately from schedules; old seasons removed after a successful new-season result import |

Today's records remain even if their start time has passed. Unknown/TBD movie
release dates and TV episode dates remain because they cannot be classified as
past. TV tracking settings and their series metadata remain so future episodes
can continue to be discovered. Source/feed configuration is not removed.

Rankings retain the last good poll if fetching, validation or saving its replacement
fails. Corrections to the same poll update existing records. These rules delete
records rather than moving them into an archive. Database backups are separate.

Implementation: sql/retention.sql, Db::apply_retention, and end-of-run cleanup.
Ranking rollover also deletes obsolete polls in its existing save transaction.
The hourly command explicitly sets --youtube-latest 10; the CLI default is also 10.
Runtime DELETE grants cover only the affected tables. Retention skips dry runs.

Validation: tests/sql/retention.sql runs cleanup inside an outer transaction,
asserts current/future and tracking preservation, then rolls back. Run with the
appropriate connection profile; it is not an instruction to commit test deletions.

Box-office chart: keep only the latest successfully imported domestic weekend's
10 movies. Failed/partial refreshes preserve the last good chart; display its
weekend date so readers can recognize stale content.

Boxoffice Pro forecasts: latest domestic preview/long-range snapshots only.
Previews expire after their weekend ends; long-range film forecasts expire after
their release date. Failed refreshes preserve only still-current rows.

MLS: past/canceled schedule entries are removed. Completed MLS results are kept
for the current provider season in SportsMatchResults; prior-season results are
removed after a successful nonempty new-season import, as with Championship.

Shared cinema poster cache: remove entries unused for 90 days. Chart/forecast
replacement preserves cached images for remaining movies and future reimports.

<a id="sql-backups"></a>

## WebScraper SQL backup maintenance

Configured October 13, 2026. The database remains in FULL recovery mode.

Docker host cron: /etc/cron.d/webscraper-sql-backup.
Host runner: /usr/local/sbin/webscraper-sql-backup.

- Transaction-log backups every 15 minutes.
- COPY_ONLY full backups daily at 2:35 a.m. America/Chicago.
- CHECKSUM and compression enabled; each backup is checked with RESTORE VERIFYONLY.
- Backup commands run inside sql2025 using its existing local administrative
  credentials. No credentials are embedded in cron, repository files or command
  arguments, and the scraper's restricted database account is unchanged.
- Managed backups live on the SQL volume under
  /var/opt/mssql/backup/webscraper-managed/{full,log}.
- Successful full jobs rotate managed full files aged at least eight days and log
  files aged at least ten days. Extra log retention preserves the chain for retained
  full backups. Initial/manual backups outside these directories are untouched.
- A host flock prevents overlapping managed jobs. Failures reach
  /var/log/webscraper-sql-backup.log. These are local backups, not off-host copies.

Manual checks on the Docker host:

```bash
sudo /usr/local/sbin/webscraper-sql-backup log
sudo /usr/local/sbin/webscraper-sql-backup full
sudo tail -n 40 /var/log/webscraper-sql-backup.log
```

Deployment performed a one-time log shrink after verified transaction-log backups,
reducing WebScraper_log from 2,248 MB to 264 MB. Fixed growth remains 64 MB.
No automatic shrink is scheduled. Data files were not shrunk and SQL Server was
not restarted. SQL Agent was stopped, so the existing running host cron service
schedules maintenance without changing SQL Server service settings.

The initial log backups contain approximately 610 MB of compressed recovery data
and were preserved. The first managed full backup was approximately 9.9 MB.
The file shrink reclaimed 1,984 MB; subtracting newly created backups, the initial
net space improvement was approximately 1,364 MB (1.33 GiB).

Reference: [Microsoft guidance on transaction-log file size](https://learn.microsoft.com/en-us/sql/relational-databases/logs/manage-the-size-of-the-transaction-log-file).
RESTORE VERIFYONLY checks backup readability and checksums; it does not substitute
for a complete restore drill. The daily job's 2:35 a.m. time can be skipped on the
spring DST transition; log backups continue and full retention allows for that gap.

<a id="scraper-security-and-sql-credentials"></a>

## Scraper hardening and SQL credential split

The hourly collector now uses **webscraper_ingest**, a dedicated login with
**WebScraperIngest** table permissions. It has no sysadmin/db_owner membership,
no CREATE TABLE permission, and no access to result-sync tables. My-Newsfeed's
login and the shared administrator password were not changed.

<a id="scraper-security-and-sql-credentials--credential-files-and-setup"></a>

### Credential files and setup

- `.env`: restricted ingestion connection, generated strong service password.
- `.env.migrations`: existing deployment/admin connection, for explicit setup only.
- Both files are mode 0600, ignored by Git and excluded from Docker build context.
  The hourly Compose service loads only `.env`. Migration credentials are not
  present in its environment or filesystem.
- `sql/runtime_permissions.sql`: reviewable grants for the ingestion role. Login
  creation/password provisioning happens separately using bound SQL parameters;
  no password is stored in this script or committed.

Normal collection connects directly to the target database and does **no DDL**.
Use administrator/deployment credentials for explicit schema changes:

```sh
# Run from the repo root, in a temporary shell; do not use this profile for the scheduler.
( set -a; . ./.env.migrations; set +a; cargo run --locked -- --initialize-db )
```

The --configure-nhl/--configure-tv/--configure-big12/--configure-nwsl/--configure-wnba/--configure-mlb/--configure-standings/--configure-movieinsider/--configure-rankings
commands also belong to the deployment/admin path. `MSSQL_MIGRATION_CONNECTION_STRING`
can optionally override the setup connection; ordinary collection never uses it.
Do not inject that administrator variable into the hourly service.

For a fresh installation, initialize the schema, provision a dedicated login,
apply the reviewed permission script, then install its private runtime profile.
Keep migration credentials separately controlled. Rotate service passwords
explicitly with a coordinated profile/container update; automated expiry is off
so the collector does not silently stop while unattended. If the old administrator
secret was exposed, rotate it in a separate coordinated change covering every
consumer, including My-Newsfeed; this remediation did not rotate a shared secret.
My-Newsfeed should have its own app-specific SQL account with permissions reviewed
against its admin and results features, rather than reuse either scraper profile.

<a id="scraper-security-and-sql-credentials--scraper-fixes"></a>

### Scraper fixes

- Schedule collection includes a seven-day reconciliation window. Recently stored
  games receive final/postponed/canceled/moved-date observations. Completed or past
  observations can update existing IDs but cannot create new historical rows.
  Scores/results remain untouched; MLB's configured season=2026 restriction remains.
- ESPN fields are checked against SQL UTF-16 sizes before writes. A source batch
  commits atomically; any later-row failure rolls back earlier updates. Save errors
  are recorded and independent collectors continue.
- Retention limits are checked at both CLI and DB boundaries; integer wrapping
  cannot turn a large requested count into a delete-all predicate.
- Shared HTTP clients reject unsafe schemes, URL credentials, private/reserved
  addresses, unsafe redirects and DNS answers. DNS checks use the actual connection
  resolver. Ambient proxies are disabled so they cannot bypass those checks.
  News item/canonical/image links are validated before storage or fetch.
- Text responses are capped at 8 MiB, ESPN JSON at 32 MiB, poster images at 5 MB.
  Feed discovery and image enrichment have bounded work budgets.
- SQL connection/handshake/query deadlines and server lock timeouts bound stalls.
  A timed-out session is marked unusable and replaced on the next run. No additional
  statements are issued on a canceled/poisoned session.
- News source success is recorded after persistence. Partial/all-failed collections
  return nonzero and publish structured success/failure counts. Valid empty snapshots
  are handled as successful work. Run outcomes/heartbeats are stored in ScraperRuns.
- News publication times no longer move forward on repeated polls of misdated input.
  Earlier corrections and initially missing timestamps can still be populated.
- Disabled movie providers are skipped for calendar fetching and poster processing.
- The hourly scheduler considers both occurrences of ambiguous fall-back times.
- RSS publication uses same-directory atomic rename instead of overwriting in place.
- The old SQL Rustls stack was replaced with native TLS/OpenSSL. Cargo audit reports
  zero advisories and zero warnings. The runtime image includes libssl3.
- Formatting, Clippy and CI validation are configured; SQL integration tests remain
  isolated and opt-in. CI does not receive production credentials.

<a id="scraper-security-and-sql-credentials--runtime-monitoring-and-container-settings"></a>

### Runtime monitoring and container settings

`--health-check` performs read-only schema/progress checks, without running imports
or migrations. Compose now sets 512 MiB memory, two CPUs, 128 PIDs, dropped
capabilities, no-new-privileges, read-only root, and a bounded writable /tmp.
The existing /data volume remains writable. Logs rotate at 10 MB / three files.

Docker healthchecks have a 70-minute startup allowance because the hourly collector
waits for its next aligned slot. ScraperRuns retains 30 days of run outcomes. Check
latest run Status, HeartbeatAt and ErrorSummary. Docker restart policy alone does
not restart an unhealthy-but-running process; health needs to be monitored by the
operator/orchestrator. SQL deadline recovery closes a failed session and retries
with a new connection on the next collection cycle.

<a id="scraper-security-and-sql-credentials--sql-certificate-activation-completed"></a>

### SQL certificate activation completed

After user approval, the shared SQL server was restarted with the hostname-validated
certificate for **sql2025**. The ingestion profile now uses
`TrustServerCertificateCA=certs/sql-ca.crt` and no certificate-bypass option.
The CA is mounted read-only at /data/certs/sql-ca.crt. Correct-CA health checks pass;
a deliberate unrelated-CA test fails certificate verification. My-Newsfeed /events
returned HTTP 200 after reconnection. Local migration profiles also use the CA.

Prepared leaf files (mssql-owned, private key mode 0600) are under:
`/var/opt/mssql/certs/webscraper-audit-fixes/` inside sql2025. The CA public certificate
is `certs/sql-ca.crt` in the host checkout. Signing material stays outside the image
and SQL server, in the protected deployment backup's pki folder.

For a future coordinated reapplication, run on the Docker host:

```sh
bash scripts/activate_sql_tls.sh --activate
```

This saves the previous server configuration, selects the prepared certificate,
restarts sql2025, removes certificate bypass from the ingestion profile, mounts the
CA read-only, verifies the authenticated connection and recreates the scraper.
Its password/login remains the restricted account. It does not rotate My-Newsfeed's
credentials or change server-wide force-encryption/protocol policy.

Rollback, if the authenticated connection does not pass:

```sh
bash scripts/rollback_sql_tls.sh --rollback
```

Both scripts restart the shared SQL server and need an agreed interruption window.
Rollback restores the certificate settings/runtime profile while retaining the
restricted ingestion account. See Microsoft's
[SQL Server Linux TLS configuration](https://learn.microsoft.com/en-gb/sql/linux/sql-server-linux-encrypted-connections?view=sql-server-ver16)
for certificate ownership/trust and the SQL Server 2025-specific settings.

<a id="scraper-performance"></a>

## Scraper performance instrumentation and optimizations

<a id="scraper-performance--october-13-2026-release"></a>

### October 13, 2026 release

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

<a id="scraper-performance--metrics"></a>

### Metrics

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

<a id="scraper-performance--next-tuning-decisions"></a>

### Next tuning decisions

The current release implements metrics, ESPN batching and the expired-movie filter.
It keeps hourly freshness, request pacing and source coverage unchanged. Use the
new Big 12 HTTP/SQL timings to select the next bottleneck. Big 12 batching requires
preserving per-observation quarantine and deterministic primary perspectives.
Monthly caching and bounded HTTP concurrency remain candidates; changing distant
fixture refresh frequency needs a deliberate freshness policy. Neither change is
included in this release. Large-payload unchanged-row suppression also needs
comparison metrics before its benefit can be claimed.


<a id="database-schema-snapshot"></a>

## Database schema snapshot

The live structure is exported to [WebScraper.schema.sql](../sql/schema/WebScraper.schema.sql). Captured October 14, 2026 UTC (October 13 Central), using Microsoft SQL Server Management Objects with `ScriptSchema=true`, `ScriptData=false`, dependency ordering, and keys/constraints/indexes enabled.

This snapshot includes the whole shared `WebScraper` database: 31 user tables, 395 columns, 77 indexes including primary/unique keys, 48 default constraints, 34 check constraints, 18 foreign keys and one view. There were no user stored procedures, functions, or triggers at export time. Identity definitions and computed-column expressions are included; live identity counters and table rows are not.

Server logins, database users, grants, role memberships, secrets, database files, backup configuration and SQL Agent jobs are excluded. Environment-specific runtime permissions remain in the existing provisioning scripts, not this structural snapshot. No enabled source rows, source URLs stored as data, feeds, article contents or movie/sports records are exported.

Validation recreated the script in a temporary empty database on SQL Server 2025 and compared table, column, index, default, check, foreign-key and module metadata against production. Every compared definition matched, the temporary database contained zero rows, and it was removed afterward. Scans found no DML, security statements or known credential values in the saved SQL file. The running scraper was not stopped.

To recreate the structure, create an **empty** target database using collation `SQL_Latin1_General_CP1_CI_AS` and the source compatibility level `170` on a compatible SQL Server version, select that database, then execute the snapshot with a tool that recognizes `GO` batches (for example, SSMS or sqlcmd). The file deliberately contains no database creation or `USE` statement. It is a creation snapshot, not an idempotent migration: do not run it against the populated production database.

Regenerate from live metadata after schema changes rather than appending seed/import scripts. Source configuration and lookup data must be provisioned separately when starting a new installation.
