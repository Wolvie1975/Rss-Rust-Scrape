# web_scraper — handoff notes

Written 2026-09-21 to continue this project in a new chat. Read this first. It records where things
stand, what was decided, and what is still open. It contains no passwords.

## Movie posters (2026-10-09)

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
- Guide/report: `docs/movie-posters.md`, `reports/movie-posters-2026-10-09.md`.
  Rollback image: before-posters-20261009; host backup: `.deployment-backups/20261009-posters/`.

## Big 12 expansion (2026-10-09)

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
- Guide: `docs/big12-sports.md`. Read-only audit: `cargo run --example audit_sports --
  WebScraper /tmp/sports-audit.json`. Production rollback image: before-big12-20261009;
  host source/sports snapshot: `.deployment-backups/20261009-big12/`.

## School logos (2026-10-09)

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
  opponent logos were unchanged. Report: `reports/kansas-logo-fix-2026-10-09.md`.
  43 all-target tests and the rollback-only school-logo SQL test passed.

## Movie release extension (2026-10-08)

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
- Read `docs/movie-releases.md` for source evidence, fields, identity/rescheduling
  rules, weekly query, and limitations. Dates are DATE; audit timestamps are UTC.
- **Read-only movie preview implemented** in `src/movies.rs`, exposed through
  `cargo run --example movie_preview -- --week-of YYYY-MM-DD --output report.json`.
  It opens no DB connection. Calendar discovery and detail-date extraction cover
  theatrical, digital, disc, and subscription releases. Eight parser tests pass.
  Main CLI ingestion/database upserts are now implemented via `--movies`; the standalone Docker service includes movies in its hourly runs.
- Live preview for 2026-10-05 through 2026-10-11 completed: 5 theatrical, 16 digital,
  9 subscription records (8 titles), 9 disc editions (4 titles). Checked 56 detail
  pages with zero HTTP/parser issues after fixes; excluded 4 TV series. No DB writes.
  Reports: `reports/movie-preview-2026-10-05.md` and `.json`. All 27 Rust tests pass.
- Live schema inspection also found `ResultFollows` and `ResultGames`, beyond the
  seven original tables documented below. They were not changed.

## What this project is

A Rust CLI (`/workspaces/Rust_Projects/web_scraper`, branch `feature/next-stage-build`, remote
`Wolvie1975/Rss-Rust-Scrape`) that scrapes feeds and pages and stores them in a **SQL Server**
database, `WebScraper`, on the host `sql2025` (Docker network name, port 1433). A separate .NET
front end (not in this repo) will read the database and also act as the admin UI.

The database is the only link between the two programs. The admin edits lookup rows; the scraper
reads them on its next run.

## Source layout

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

## Database (7 tables, `dbo`)

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

## CLI (`./target/release/web_scraper`, see `--help`)

- News: `--from-db` (scrape enabled Sources), `--per-source N` (cap + prune per source), `--keep N`
  (global cap), `--last-days N`, `--require-date`, `--save-sources -f urls.txt`, `-o feed.xml`.
- Events: `--events-from-db`, `--events-feed URL`. YouTube: `--youtube-from-db`, `--youtube-feed URL`,
  `--youtube-latest N` (default 5).
- `--dry-run`: no row writes (but it still opens the DB and creates missing tables). `--every-hours N`:
  loop, aligned to midnight **America/Chicago** (DST-aware); errors in a run are logged, loop continues.
- Any DB flag (`--from-db`, `--events-*`, `--youtube-*`, `--save-sources`) implies saving. Quote URLs
  containing `&` in the shell.

## Runtime state (2026-10-08)

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

## Decisions made

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

## Open items / known issues

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

## Front-end (.NET) notes given to the user

- Convert UTC to Central at display time: `TimeZoneInfo.FindSystemTimeZoneById("America/Chicago")`,
  and call `DateTime.SpecifyKind(x, DateTimeKind.Utc)` first (SQL returns Kind=Unspecified).
- Compare "today/yesterday" using Central dates. Filter upcoming games with `EventDate >= today (Central)`.
- Handle nulls: `ImageUrl`, `Published`, `Description`, event `Tv`/`StreamUrl`/logos, `IsAway`.
  `TimeTbd = 1` means show "time TBD". Show `YoutubeVideoFeed.ChannelName`, not the video row's author name.
- Admin: edit only lookup tables (`Sources`, `SourceCategories`, `YoutubeVideoFeed`, `SportsEventsType`);
  disable a source with `Enabled = 0` instead of deleting; never write scraper-owned columns or `UrlHash`.
- EF Core: map `UrlHash` read-only/ignored; use `AsNoTracking()`; don't rely on ID order (gaps from pruning).

## Moving to another computer (summary of advice given)

Clone the repo; install Rust >= 1.85 (project uses edition 2024; container has 1.97); recreate `.env`;
point `Server=` at a host reachable from the new machine (`sql2025` only resolves inside the Docker
network); move the DB by BACKUP/RESTORE (keeps categories and lookup rows) or let the scraper recreate
an empty schema and re-seed with `--save-sources -f urls.txt`; run the loop under systemd/Docker/Task Scheduler.

## Working conventions with this user

- Confirm before anything outward-facing; the user pushes to GitHub, and restarts/pauses the scheduler
  on their own call.
- Prefer verifying against the live DB and live feeds (dry-run first, then a real run) and report
  exactly what was and wasn't checked.
