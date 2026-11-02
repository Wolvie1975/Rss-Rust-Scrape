# Scraper handoff

## Tech front panels (Your Stack, Homelab Services, Ollama Library) — deployed November 1, 2026 CST

Deployed **web-scraper:tech-panels-20261101**, tagged as `web-scraper:latest` and healthy.
Added scraper support for the three new panels on the Tech front page:

1. **Your stack (`InYourStack = 1`):**
   - Added column `InYourStack BIT NOT NULL DEFAULT 0` to `dbo.TechReleaseProjects`.
   - Expanded project types (`sdk`, `toolchain`, `framework`, `runtime`, `compiler`) and source types (`dotnet_releases`).
   - Flagged existing projects with `InYourStack = 1`: `linux-mainline`, `fedora`, `mesa`, `proton`.
   - Added and seeded 5 new projects: `.NET` (official Microsoft `releases-index.json`), `Rust` (GitHub releases), `Qt` (Atom RSS), `Node.js` (GitHub releases), and `TypeScript` (GitHub releases).
   - 135 releases tracked across all enabled stack projects.
2. **Homelab service monitoring (`dbo.HomelabServices`, `dbo.HomelabServiceStatus`):**
   - Schema and status tables for homelab services with tracking for status, latency, detail, consecutive failures, and check timestamps.
   - Supports check types: `http`, `http_body`, `tautulli` (Plex stream parsing via API key from environment), and `self` (scraper heartbeat verifying last successful feed run < 120m).
   - Robust debouncing: requires 2 consecutive failures before reporting `down`; single timeouts report `slow`.
   - Seeded services: `plex`, `jellyfin`, `tautulli`, `ollama`, `open-webui`, `portainer`, `scraper`.
   - High-cadence monitoring: dedicated background daemon thread runs a 60-second checking loop alongside the hourly scraper, plus on-demand check via `--homelab`.
3. **Ollama library tracker (`dbo.OllamaLibraryModels`, `dbo.OllamaLibraryTags`):**
   - Scrapes newest Ollama models from `https://ollama.com/search?o=newest` and tag metadata/sizes from `/library/<name>/tags`.
   - Baseline seed of 20 models populated with `Change = 'new'` and `ScrapedAt = '2000-01-01'`.
   - Local Ollama verification against `OLLAMA_URL` (defaults to `http://ai-machine.local:11434`): marks installed tags; preserves flags if Ollama host is unreachable.
   - Hardware fit calculation: compares tag sizes and parameter counts against `OLLAMA_MEMORY_GB` (default 16GB) to set `Fits = 1`.
   - Supported via CLI flag `--ollama-library` and integrated into `--tech-from-db`.
4. **Permissions & Migration:**
   - Created and executed repeatable [`sql/tech_panels_migration.sql`](sql/tech_panels_migration.sql).
   - Updated canonical [`sql/tech_schema.sql`](sql/tech_schema.sql), [`sql/tech_seed.sql`](sql/tech_seed.sql), and [`sql/tech_permissions.sql`](sql/tech_permissions.sql).
   - Granted read permissions to `[WebScraperReader]`, `[WebScraperScoreboardReader]`, `[web_reader]`, and `[mynewspaper]`, and ingest to `[WebScraperIngest]`.
   - Added `extra_hosts: ["host.docker.internal:host-gateway"]` to `docker-compose.yml`.

## Top 25 scoreboard and upcoming ranked games — deployed November 1, 2026 CST

Deployed **web-scoreboard:top25-20261101**, tagged as `web-scoreboard:latest` and healthy.
Added Top 25 scoreboard support and weekly upcoming ranked games view:

1. **Rank Storage & Parsing:**
   - Added `HomeRank TINYINT NULL` and `AwayRank TINYINT NULL` to `dbo.SportsScoreboardGames` with `CHECK (... IS NULL OR ... BETWEEN 1 AND 25)`.
   - `src/scoreboard.rs` extracts `competitors[].curatedRank.current` (1–25, treating `99` as unranked `NULL`).
   - `sql/scoreboard_upsert.sql` updates ranks on live/upcoming games and freezes them when games are `final`.
2. **Top 25 Coverage (`SourceKind = 'rankings'`, `Scope = 'top25'`):**
   - Added union in `sql/scoreboard_config.sql` for enabled `dbo.SportsRankingSources` rows (`PollId = '1'`) for Football and Men's Basketball.
   - Updated `dbo.SportsScoreboardCoverage` to include rankings sources, and extended `SportsScoreboardSources.SourceKind` check to allow `'rankings'`.
   - `Coverage::accepts` in `src/scoreboard.rs` accepts regular/postseason games when either team is ranked.
3. **Views & Permissions:**
   - Updated `dbo.SportsScoreboard` to expose `HomeRank` / `AwayRank` and include games covered by rankings sources where at least one team is ranked.
   - Created `dbo.SportsScoreboardTop25Week` for upcoming games: filters to `college-football` and `mens-college-basketball`, current Monday–Sunday week (Central time), `Status <> 'cancelled'`, at least one ranked team, and excludes Big 12 matchups via `ResultFollowTeams`.
   - Granted `SELECT` on `dbo.SportsScoreboardTop25Week` to `WebScraperScoreboardReader` and `WebScraperIngest`.

Applied repeatable migration [`sql/scoreboard_top25_migration.sql`](sql/scoreboard_top25_migration.sql), view scripts, and verified clean health check.

## Tech section feeds (Release Tracker, Dev Reading, Service Status) — deployed November 1, 2026 CST

Deployed **web-scraper:tech-20261101**, tagged as `web-scraper:latest` and healthy.
Added three new background feeds for myNewspaper's upcoming Tech section:

1. **Release tracker (`dbo.TechReleaseProjects`, `dbo.TechReleases`):**
   - Tracks 9 enabled projects: Linux kernel mainline & LTS (`kernel_org`), Proton (`github_releases`), GNOME & Mesa (`gitlab_tags`), and Fedora, Ubuntu, KDE Plasma, and Linux Mint (`rss`).
   - Verified live collection imports 58 unique releases with zero errors. Upserts on `(ProjectId, Version)` preserve history; failures update `LastError` per project without blocking others.
2. **What developers are reading (`dbo.TechDevReading`):**
   - Collects top 10 stories from Hacker News (Firebase API) and Lobsters (`hottest.json`).
   - Atomic replacement per site in a single transaction; failed fetches preserve last good snapshot.
3. **Service status (`dbo.TechStatusServices`, `dbo.TechServiceStatus`):**
   - Tracks GitHub, Cloudflare, and Discord using Atlassian Statuspage `summary.json`.
   - Records indicator, description, active incidents, components, and links. Flipping to `unknown` requires 3 consecutive check failures.

Applied repeatable migrations [`sql/tech_schema.sql`](sql/tech_schema.sql), [`sql/tech_seed.sql`](sql/tech_seed.sql), and [`sql/tech_permissions.sql`](sql/tech_permissions.sql) granting `SELECT` to the web application and ingest permissions to `WebScraperIngest`.
Included in hourly collection via `--tech-from-db` in `docker-compose.yml`.

## KCTV5 and KMBC in Kansas City News — October 28, 2026 CDT

Added enabled sources **12137 (KCTV5)** and **12138 (KMBC 9)** to Kansas City News
using [the repeatable disabled-first seed](sql/kansas_city_news_seed.sql). The targeted
import had two successes and zero failures and saved 20 dated articles with images.
LastError is NULL for both. Hourly `--from-db` picks them up, so no rebuild or restart
is needed. KCTV5 uses its site-wide feed, which includes some national stories. See
[Kansas City news](docs/kansas-city-news.md#kctv5-and-kmbc--added-october-28-2026).

## Code structure cleanup — deployed October 25, 2026 CDT

Code-only refactor, merged to `main` and deployed as **web-scraper:code-cleanup-20261025**
and **web-scoreboard:code-cleanup-20261025**; no schema, SQL text, configuration or data
change. Modules now live in a library crate ([src/lib.rs](src/lib.rs)) shared by
`web_scraper`, `scoreboard` and the examples, with one `Error`/`Result` type. Page
scraping moved to [src/pages.rs](src/pages.rs); `run_once` in [src/main.rs](src/main.rs)
is split into per-stage `Run` methods; `Db` tests moved to [src/db/tests.rs](src/db/tests.rs).
Library tests now run with `cargo test --lib` (not `--bin web_scraper`), and `Db` test
paths are `db::tests::*`. Cargo.lock fixes RUSTSEC-2026-0382/0383/0386. First production
run matched the previous release (266 successes; the one failure is the pre-existing
TVmaze *High Potential* incomplete-episode guard). See [deployment and rollback](reports/code-cleanup-20261025.md).

## USA TODAY feeds — October 24, 2026 CDT

Added ten enabled Google News RSS sources **9412–9421** to existing categories:
Top Headlines, More News (Money), Sports, Technology, Entertainment, TV/Film,
Opinion, F1 News, Big 12 News and Men's College Basketball News. Production
preview/import each succeeded with ten sources and zero failures. Read-back:
92 dated articles, no thumbnails, all source errors NULL. Existing hourly
collection needs no rebuild/restart. Searches can overlap and Google News links
remain unless verified redirects resolve them. Personal, noncommercial reader
use only. See [source details and verification](docs/usatoday-news.md) and
[repeatable disabled-first seed](sql/usatoday_news_seed.sql). Frontend rendering
was not verified. No recent gaming results were found for the tested queries.

## Scoreboard history limited to yesterday — October 23, 2026 CDT

Scoreboard retention now keeps today, the previous America/Chicago calendar day,
and future fixtures. Default lookback/retention are one day. Older imports are
filtered and cleanup touches only scoreboard storage. Frontend date navigation
should start at yesterday. See [current operations](docs/live-scoreboard-operations.md)
and [verification](reports/scoreboard-previous-day-20261023.md).

## Persisted scoreboard — deployed October 23, 2026 CDT

New **web-scoreboard:20261023** service is healthy and separately scheduled from
hourly ingestion. `dbo.SportsScoreboard` / `dbo.SportsScoreboardCoverage` expose
257 initial deduplicated games across 13 configured sport/league combinations.
Live and near-start days use a ten-minute cadence; stale samples are flagged
at 25 minutes. Existing schedules, results worker and box scores remain intact.
No frontend UI was changed. [Frontend handoff](docs/live-scoreboard-frontend-handoff.md),
[operations](docs/live-scoreboard-operations.md), and
[deployment evidence/rollback](reports/live-scoreboard-20261023.md).

## Documentation

Start with [README.md](README.md). Five topic guides replace the overlapping individual handoffs; three history files preserve dated deployment evidence. SQL examples, validation evidence and the previous long handoff remain available there.

## Theatrical coverage without discs — deployed October 22, 2026

Image **web-scraper:theatrical-no-disc-20261022** is deployed and healthy.
DVDs Release Dates source **5** is enabled with new **CollectDisc=0**: theatrical
and digital collection restored, physical-disc discovery/writes blocked, 24 retained
disc rows withdrawn. All six October 23 theatrical dates were confirmed against
live detail pages; repeated production imports saved 17 non-disc releases with
zero issues and stable IDs. Movies shows six films and Home five on phone/desktop.
Other movie providers and the hourly command remain intact. See
[configuration and frontend contract](docs/theatrical-without-discs.md) and
[deployment, data checks and rollback](reports/theatrical-no-disc-20261022.md).
Disable source 5 before rolling back to an older image that ignores the disc policy.

## Box Office merged into Entertainment — October 22, 2026

Moved all four Box Office news sources and 40 existing articles into Entertainment
(ID 10), alongside Nerdist. Verified five enabled sources and 50 articles; empty
Box Office category 5615 removed. Repeatable migration applied twice successfully.
Registration scripts now target Entertainment; charts and forecasts are unchanged.
No rebuild/restart required. See [current news contract](docs/movies-and-box-office.md#boxoffice-news)
and [migration](sql/boxoffice_into_entertainment.sql).

## News categories and Guardian World — October 22, 2026

Renamed category **2627** from General News to **More News**: NPR 7093,
CBS News 8904 and St. Louis Post-Dispatch 9401. Moved Awful Announcing 7895
from TV/Film to Sports and Polygon 1 / IGN 1768 from Entertainment to Video Games.
Guardian 8906 stays in Top Headlines and now uses `https://www.theguardian.com/world/rss`.
Verified World-feed dry-run/import: 45 parsed, ten retained with images, zero
failures. Existing category/source IDs and enablement retained; standard retention
replaced the Guardian's prior ten articles. No restart required. See
[implementation and frontend contract](docs/news-category-cleanup.md) and
[repeatable migration](sql/news_category_cleanup_20261022.sql).

## STLtoday news — October 22, 2026

Added enabled source **9401**, **St. Louis Post-Dispatch — STLtoday**, under
**General**. The verified general RSS feed mixes local and national coverage.
Targeted Rust dry-run and first import succeeded; database read-back verified ten
articles with dates/images and no source error. Existing hourly collection needs
no rebuild or restart. See [source details](docs/stltoday-news.md) and
[disabled-first registration](sql/stltoday_news_seed.sql).

## Weekend result status — October 21, 2026 CDT

Deployed **web-scraper:boxoffice-status-20261022** (UTC tag). The additive status
columns and transactional refresh safeguards are live; the hourly command is preserved.
The provider explicitly marks some weekend grosses as estimates. Unmarked grosses
remain unknown because no live actuals signal was verified. Status applies to weekend
gross only. See [frontend contract and provider limits](docs/box-office-result-status.md)
and [deployment verification and rollback](reports/boxoffice-status-20261022.md).

## Box-score period labels — October 19, 2026

Release **web-scraper:period-labels-20261019** repairs version-1 line-score period
labels using explicit ESPN scoreboard/core identities. The historical repair
preserves scoring values, statistics and result identities; normal enrichment now
labels new snapshots too. See [deployment, preservation evidence and frontend
examples](reports/sports-period-labels-20261019.md). The existing frontend already
accepts string labels; no frontend deployment or schema migration is required.

## Men's college basketball news — October 19, 2026

Added four enabled sources to **Men's College Basketball News**: ESPN **9353**,
CBS Sports **9354**, ZAGSBLOG **9355**, and Heartland Big 12 Basketball **9356**.
Production run **1254** saved **37 articles** with zero failures; three existing
cross-source URLs were skipped without reassignment. Live category pagination
returned all 37 articles from the four publishers. No rebuild/restart was required.
See [feeds, verification and frontend contract](docs/mens-college-basketball-news.md)
and [disabled-first registration](sql/mens_college_basketball_news_seed.sql).

## Database review — October 19, 2026

Integrity check passed. Refreshed **38 stale optimizer statistics across 13 tables**;
no expired-policy records or genuine orphan data required deletion. Both services
remained healthy. [Review and follow-ups](reports/database-review-20261019.md)
records storage/backups, three deliberately unlinked future sports observations,
and the frontend database login's existing sysadmin privileges. No credentials,
retention settings, source data or application deployment were changed.

## Optional sports box scores — October 19, 2026

Deployed **web-scraper:boxscores-20261019**. The existing hourly
`--events-from-db` path now enriches stored completed games from both results
models with optional ESPN team/player statistics. `dbo.SportsBoxScores` exposes
availability, normalized JSON and validated provider links. Unavailable detail
preserves the existing result fallback; failed refreshes preserve last-good stats.
Initial backfill checked 1,355 games: 960 with statistics, 395 with no detail,
zero errors. The separate frontend still needs the UI described in [the frontend handoff](<New Development/sports-boxscores-frontend-handoff.md>).
See [deployment, backfill evidence and rollback](reports/sports-boxscores-20261019.md).

## Kansas City local news — October 19, 2026

Added five enabled sources to **Kansas City News**: KSHB **9172**, Flatland **9173**,
The Pitch Food & Drink **9174**, KCUR **9175** and The Beacon **9176**. Production
run **1244** collected **45 articles** with no failures. Live `/local` switched to
dedicated content; four cursor pages returned all 45 unique stories. No image,
scheduler or frontend deployment/restart was required. See [source scope and
verification](docs/kansas-city-news.md) and [disabled-first seed](sql/kansas_city_news_seed.sql).

## Indie video game news — October 19, 2026

Added Indie Games Plus **9037**, The Indie Informer **9038**, and Indie Hive **9039**
to **Video Games**. Production import **1242** succeeded with ten retained articles
per source, enabled sources and no source errors. Existing hourly collection
handles these feeds without a rebuild/restart. See [source operations](docs/operations.md#indie-video-game-news--october-19-2026)
and [repeatable registration](sql/indie_videogames_news_expand.sql).

## Box Office news expansion — October 19, 2026

Added Variety 9034, The Hollywood Reporter 9035 and IndieWire 9036 to the existing
Box Office category alongside Deadline 7094. Existing hourly news collection
handles the additional feeds without rebuilding or restarting the scraper.
Current sources and frontend contract: [Box Office news](docs/movies-and-box-office.md#boxoffice-news).

## F1 feeds — October 18, 2026

F1 source 1 in the new `F1Source` table is enabled in production. Initial runtime
import verified 23 races, 374 Grand Prix classifications, 132 sprint
classifications, 23 drivers and 11 constructors for season 2026, standings round
17. Four SQL views expose schedule, race/sprint results and both championships.
The updated collector includes F1 under `--events-from-db` and supports explicit
`--f1-from-db` refresh. **Deployed as `web-scraper:f1-20261018` on October 18.** The existing hourly
command and runtime settings are preserved. Deployed-container F1 run 1206
succeeded; the frontend database reader can query all four views. No frontend
application change was performed. See [operations and verification](docs/f1-operations.md)
and the [frontend integration request](docs/f1-frontend-integration.md).

## Big 12 news sources

Five enabled sources are assigned to **Big 12 News**: official all-sports 8997,
Heartland College Sports 8998, official football 8999, men's basketball 9000
and baseball 9001. The dedicated women's basketball feed was omitted by request;
the all-sports feed can still contain women's sports stories. Existing hourly
news collection handles these sources without a rebuild/restart. See the
[college hub frontend contract](docs/big12-news-frontend-integration.md).

## F1 news sources

Four enabled sources are assigned to **F1 News**: BBC Sport 8965, Autosport 8966,
Motorsport.com 8967 and The Guardian 8968. The existing production news collector
handles them without an image rebuild or scheduler restart. See the
[news frontend contract](docs/f1-news-frontend-integration.md) and
[registration/collection evidence](docs/f1-operations.md#f1-news-registration).

## Top Headlines sources

Six enabled production sources were added to the new **Top Headlines** category:
NBC News 8903, CBS News 8904, The New York Times 8905, The Guardian 8906,
Al Jazeera 8907 and Sky News 8908. BBC and NPR remain in General. Existing hourly
news collection handles these sources. [Source operations](docs/operations.md#top-headlines-sources)
and the [frontend handoff](docs/top-headlines-frontend-integration.md) describe
registration and integration; source registration alone does not verify rendering.

## Latest documented production state

Last verified deployment: `web-scraper:boxoffice-status-20261022`; scraper and front-end were healthy at verification. This document is not a live status check.

- Hourly collection includes news, rankings, standings, sports schedules (including F1), YouTube, movie releases, TV, actual box office and Boxoffice Pro forecasts.
- MLS schedule/results source 5525, standings 9; Championship schedule/results 5326, standings 11; EPL schedule 5357, standings 12; NFL standings 10.
- MLS and Championship current-season results are independent of schedule pruning; playoff shootouts are stored separately from goals.
- Latest actual box-office chart and domestic forecasts have poster URLs. Last checked: 10/10 chart films, 12/12 forecast films and 24/34 release films; unmatched films use placeholders.
- Box Office news has four enabled publishers: Deadline, Variety, The Hollywood Reporter and IndieWire; structured charts and forecasts remain separate.
- Current retention and backup rules: [operations guide](docs/operations.md#data-retention). SQL remains in FULL recovery with managed log/full backups.
- Initial performance changes deployed: ESPN batches of 100, stage/HTTP metrics, and filtering past movie releases before persistence. Further caching/concurrency work remains a candidate.
- Front-end work for the newer movie/sports sections is documented in the topic guides; scraper deployment alone does not implement those views.

## Article URL identity — October 16, 2026

Verified URL redirects/canonicals now reconcile same-source article URL changes
through `ArticleUrlAliases` and `UpsertArticleIdentity`. The additive migration
also creates `ArticleIdRedirects` and grants runtime permissions. No title/image/
timestamp similarity merges are allowed; runtime source settings and hourly
frequency are preserved.

The approved Pure Xbox cleanup keeps **25604**, archives/maps **25647 → 25604**,
and retains both URL aliases. The frontend showed one public card. Browser saves
using retired IDs need frontend recovery through that mapping; this release does
not change browser storage. A verified copy-only database backup and protected
host source/configuration backups are retained.

Read [article URL identity](docs/article-url-identity.md) for request budgets,
migration order, metadata/reference handling and saved-ID recovery. Actual schema,
tests, cleanup scope and rollback: [deployment evidence](reports/operations-history.md#article-identity-deployment-2026-10-16).

## Big 12 standings review — October 15, 2026

Read-only verification found soccer source **7** (2026, 16 teams) and men’s
basketball source **5** (2025-26, 16 teams) match the official tables in full.
The soccer app’s conference columns were correct; it omitted stored overall
and split statistics. The user plans frontend changes separately. No scraper or
production changes were needed. Full source-specific `StatsJson` mappings:
[standings guide](docs/sports-tables.md#big12-soccer-basketball-columns).
Recorded refresh times and comparison evidence:
[sports history](reports/sports-history.md#big12-standings-review-2026-10-15).

## NHL production deployment — October 14, 2026

Image `web-scraper:nhl-20261014` is deployed and healthy. NHL schedule/results
source **5637** and division standings source **13** are enabled. Verified 1,239
fixtures, 170 final results and 32 current standings teams for ESPN season 2027
(2026–27), with stable identities across repeated imports. The existing hourly
scheduler and runtime settings are preserved. See the consolidated [schedules/results](docs/sports-schedules.md#nhl-schedule-results),
[standings](docs/sports-tables.md#nhl-standings), [operations](docs/operations.md#nhl-operations)
and [deployment history](reports/sports-history.md#nhl-deployment-2026-10-14) for contracts,
validation and rollback.

## NWSL league-wide schedule — October 16, 2026

Source **4191** now collects the whole league, including Kansas City Current.
Verified **24 upcoming fixtures across all 16 teams**, stable on repeated import;
all three retained Current event IDs/first-seen/dates are preserved. The existing
frontend NWSL schedule selection returned eight current-window games, seven not
involving Current. Existing source ID, Enabled and legacy ProviderSchoolId=20907
compatibility tag are retained; collection scope comes from the unfiltered URL.
Hourly runtime settings and results synchronization are unchanged.

Read [the league schedule/frontend contract](docs/nwsl-league-schedule.md) and
[deployment evidence](reports/sports-history.md#nwsl-league-deployment-2026-10-16).
The older Kansas City-only handoff is historical. No frontend files/image were
changed; its separate in-progress work was preserved.

## Movie purchase links — deployed October 16, 2026

Current image `web-scraper:movie-direct-links-20261016` adds expiring reviewed
Fandango mappings and improved candidate selection. Production run 165 saved 34
releases with zero source issues. Eight of nine theatrical actions now use direct
Fandango movie pages; Sacrifice and four digital actions use search fallbacks.
The live `/Movies` page rendered eight direct buttons, including the exact Street
Fighter detail URL, without a frontend image replacement. Existing movie data,
hourly configuration and the article-ID recovery mapping are preserved.

Read [cache and operations](docs/movie-purchase-links.md), [the standalone frontend
integration request](docs/movie-commerce-frontend-integration.md) and [direct-link
deployment evidence](reports/operations-history.md#movie-direct-links-deployment-2026-10-16).
The existing frontend is now confirmed to consume the view and render direct links.

## Development and deployment

The validated [schema-only database snapshot](sql/schema/WebScraper.schema.sql) captures 31 tables and one view, without data or credentials. Article identity and F1 use separate additive migrations after this baseline. See the [operations guide](docs/operations.md#database-schema-snapshot) for scope and empty-database restore instructions.

Use runtime credentials for ingestion and migration credentials for explicit setup. Never print or commit credentials. Preserve existing environment, network, scheduler, retention and rollback settings during deployments. Use appropriate tests and verify the deployed image and targeted live imports. As of October 25, 2026, Claude may commit and push completed, verified work; recheck Git status and preserve concurrent changes first.

Historical images, table counts, earlier open items and source audits: [operations history](reports/operations-history.md#previous-handoff), [sports history](reports/sports-history.md), [entertainment history](reports/entertainment-history.md).

## Working conventions with this user

- Claude may commit and push to GitHub (authorized October 25, 2026). Other outward-facing
  actions still need confirmation; the user restarts/pauses the scheduler on their own call.
- Prefer verifying against the live DB and live feeds (dry-run first, then a real run) and report
  exactly what was and wasn't checked.
