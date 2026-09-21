# web_scraper — handoff notes

Written 2026-09-21 to continue this project in a new chat. Read this first. It records where things
stand, what was decided, and what is still open. It contains no passwords.

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

## Current state (end of 2026-09-20 session)

- **The hourly scheduler is STOPPED (user's request, 2026-09-21 ~08:24 Central; its last run finished
  at 08:00). Do not start it until the user says so.** It had run hourly overnight after being restarted
  that day. Check with `pgrep -af "^./target/release/web_scraper"`.
  It is a plain background process (no cron/systemd in this container), so it also stops if the
  container restarts. Log: `scraper.log` (git-ignored). Start command (release binary is current):
  ```
  cd /workspaces/Rust_Projects/web_scraper
  nohup ./target/release/web_scraper --from-db --per-source 10 --every-hours 1 -o feed.xml \
    --events-from-db --youtube-from-db >> scraper.log 2>&1 &
  ```
  Rebuild first (`cargo build --release`) after any code change. Stop with `pkill -f "^./target/release/web_scraper"`.
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
