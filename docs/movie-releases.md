# U.S. movie releases

Scope agreed 2026-10-08: U.S. theatrical releases, digital purchase/rental,
subscription streaming, and physical discs. The database schema, read-only preview, and main-CLI database ingestion are implemented.
`--movies` imports the current Central-time week and refreshes tracked movie detail pages on each run.

## Sources investigated

**DVDs Release Dates — recommended first parser for theatrical/digital/disc.**

- Theatrical: https://www.dvdsreleasedates.com/new-movies-2026/ (select year dynamically).
- Digital: https://www.dvdsreleasedates.com/digital-releases/
- Disc: https://www.dvdsreleasedates.com/
- Detail example: https://www.dvdsreleasedates.com/movies/11703/moana

Ordinary HTTP requests returned 200 for the home and theatrical calendars.
The detail page inspected through web research separates theatrical, Digital HD,
and individual disc products, with dates, formats, UPCs, poster, and IMDb link.
Calendars include television titles: the movie scraper must exclude those.
Disc bundles are physical products; a bundled digital copy does not establish a
new standalone digital premiere. Preserve estimates as estimates. Treat digital
as unspecified purchase/rental unless the source explicitly separates them.
U.S. market/date semantics and representative detail-page parsing still need
fixture validation before enabling ingestion. Robots allows public paths; this
is technical accessibility, not a license to republish artwork or editorial text.

**When To Stream — recommended free subscription-streaming source.**

The calendar cards link to movie detail pages. A direct HTTP request to
https://whentostream.com/grizzly-night-2026/ returned 200 and supplied the movie
title/year, distinct VOD and SVOD dates, and the subscription platform in plain
HTML. Use calendar links for discovery, then parse detail fields; do not derive
titles from image metadata. This provides a workable free source alongside DVDs
Release Dates. Representative parser fixtures and coverage checks remain for the
scraper implementation stage. Free public sources are the user's chosen approach.

**Alternatives and limitations investigated.**

- Movie Insider: https://www.movieinsider.com/new-movies-on-streaming and
  https://www.movieinsider.com/releases/netflix. Calendar HTTP requests returned
  200; a tested movie detail page returned 403. Lists are paginated and a card can
  show several distribution channels beside one date, so do not assign that date
  to every channel. Detail-date extraction remains unverified. Terms:
  https://www.movieinsider.com/contact/ (commercial reuse requires permission).
- When To Stream: https://whentostream.com/streaming-october-2026/ explicitly
  covers the U.S. and separates VOD/Digital from SVOD. Ordinary HTTP returned 200,
  but movie cards rely heavily on images. Image titles include suffixes and even
  generic names such as "Screenshot"; those are not trustworthy movie identities.
  Follow the linked movie detail pages, whose explicit release fields were verified above.
- Watchmode: https://api.watchmode.com/docs documents `/title-release-dates`,
  U.S. region filters, date ranges, provider IDs, and streaming verification state.
  That endpoint requires a paid plan and API key. Documentation was inspected;
  authenticated requests were not tested and no account was created. Not selected:
  the user chose free public sources.

Other options: The Numbers' theatrical schedule combines U.S./Canada
(https://www.the-numbers.com/movies/release-schedule). TMDB has country-specific
release types but its generic digital category alone does not reliably separate
rental/purchase from subscription premieres
(https://developer.themoviedb.org/docs/region-support).

## Database layout

Migration: `sql/movie_releases.sql`, also executed by `Db::open`.
It is additive and repeatable, creates no source or movie data, and uses NO ACTION
foreign keys like the existing tables. Existing non-movie tables are untouched.

| Table | Purpose |
| --- | --- |
| MovieReleaseSources | Source website/API, stable source key, enable flag, last scrape/error |
| Movies | Shared movie identity, title/year, optional IMDb/TMDB IDs, synopsis, poster, certification, runtime |
| MovieSourceLinks | Maps a provider's movie ID and detail URL to Movies; allows multiple providers for one film |
| MovieReleases | Individual source-reported release events, linked through MovieSourceLinks |

A movie can have multiple theatrical openings (limited/wide), multiple platforms,
and multiple disc editions. `MovieReleases` includes:

- `ReleaseType`: theatrical, digital, digital_purchase, digital_rental,
  subscription, disc. `digital` means purchase/rental was not distinguished.
- `ReleaseDate` as SQL DATE, with `DateStatus` announced/estimated/tbd. TBD is NULL.
  Never substitute the scrape date or an invented first-of-month date.
- `CountryCode`: US only, enforced by a constraint.
- `Platform` (required for subscription), `Format` (DVD/Blu-ray/4K or bundle),
  `ReleaseScope` (wide/limited/etc.), `Distributor`, `Edition`, `IsReRelease`.
- `ReleaseStatus`: scheduled/cancelled/withdrawn. Scheduled dates in the past are
  not proof of present playback availability; this is a release calendar.
- `SourceUrl`, notes, UTC first/last-seen timestamps.

Uniqueness is `(MovieSourceLinkId, ReleaseKey)`. A release key identifies a stable
source event, such as `theatrical:wide:first`, `subscription:provider-id:first`, or
`disc:upc`. **Do not include the release date or mutable title in the key**: a
reschedule must update the same row. Distinct reissues need distinct keys. If a
source does not provide enough identity, flag ambiguity instead of guessing.

IMDb/TMDB IDs are unique when present; titles are deliberately not unique. Match
across sources by verified external IDs, not title alone. Source-reported release
rows can disagree; the future importer/UI must choose a preferred source per
category or display provenance, rather than silently treating both as new events.
Missing calendar entries alone do not prove cancellation. No deletion or retention
policy is implemented. The schema keeps current dates, not a date-change history.

## Weekly display

Use Monday through Sunday by default. The UI supplies the desired U.S. calendar
week; release dates themselves have no UTC conversion. Only audit timestamps
are UTC. The following excludes estimated dates and explicit cancellations:

```sql
DECLARE @WeekStart DATE = '20261005'; -- bind from the UI
SELECT m.ID AS MovieId, m.Title, m.OriginalYear, m.PosterUrl,
       r.ReleaseDate, r.ReleaseType, r.Platform, r.Format,
       r.ReleaseScope, r.IsReRelease, r.SourceUrl, s.Name AS SourceName
FROM dbo.MovieReleases r
JOIN dbo.MovieSourceLinks l ON l.ID = r.MovieSourceLinkId
JOIN dbo.Movies m ON m.ID = l.MovieId
JOIN dbo.MovieReleaseSources s ON s.ID = l.MovieReleaseSourceId
WHERE r.CountryCode = 'US'
  AND r.ReleaseDate >= @WeekStart
  AND r.ReleaseDate < DATEADD(day, 7, @WeekStart)
  AND r.DateStatus = 'announced'
  AND r.ReleaseStatus = 'scheduled'
ORDER BY r.ReleaseDate, m.Title, r.ReleaseType;
```

Fetch adjacent calendar months when a week crosses a month/year boundary, follow
pagination, and refresh future records to catch changes. Weekly display is
independent of polling cadence; the existing hourly scheduler was not modified.

## Setup and validation

From `web_scraper/`, using the existing git-ignored `.env`:

```sh
cargo run --offline --locked --example setup_movie_schema
cargo run --offline --locked --example setup_movie_schema -- --verify
cargo test --offline --locked
```

The setup command connects to existing WebScraper and applies only this migration.
The optional SQL integration test rolls back all fixtures (IDENTITY gaps are normal).
It checks multiple release types, rescheduling, weekly date boundaries, duplicate
keys, date/status consistency, U.S. restriction, required subscription platform,
and foreign-key protection. Source rows default disabled until parsers are ready.

## Live preview

Run a Monday-Sunday U.S. calendar preview without loading `.env` or opening SQL Server:

```sh
mkdir -p reports
cargo run --offline --locked --example movie_preview -- --week-of 2026-10-05 --output reports/movie-preview-2026-10-05.json
cargo test --offline --locked --example movie_preview
```

`--week-of` accepts any day in the week; omitted, it uses the current Central-time
week. Monthly URLs are generated for all months touched by that week. Theatrical
week headings are discovery hints only; exact dates come from movie detail pages.
Disc bundles stay physical releases. Digital purchase/rental remains `digital`
when the source does not distinguish them. Streaming dates require an explicit
SVOD date and service on the detail page. Apparent TV seasons are excluded by
movie title or disc-edition text; this is a preview heuristic, not a complete content-type classifier.

The JSON contains release rows, per-category counts, calendar candidate counts,
request/parser issues, excluded TV titles, and candidates with no matching detail
date. HTTP/parser errors produce a nonzero exit after saving the partial report.
Zero calendar candidates require review rather than silently proving no releases.
The report preserves source URLs and disc UPCs when supplied. Multiple disc editions
of one film count as separate release rows. Source claims are not independently
verified against studios or platform catalogs. No movie data is saved to SQL Server.


## Scheduled ingestion

`cargo run --locked -- --movies` imports movie releases into SQL Server. Combine
`--movies` with the existing news/events/YouTube flags and `--every-hours 1` for
hourly collection. `--dry-run` parses movies without writing movie/source rows.
The standalone Docker Compose service includes `--movies` and keeps its existing
persistent feed volume and hourly Central-time schedule.

The two supported movie sources are registered enabled on their first import.
Existing disabled source rows remain disabled, and their release rows are skipped.
Tracked detail URLs are revisited so date changes outside the current week can
update the same release. Absent entries do not delete or cancel releases.
IMDb IDs link verified DVD-source identities; titles alone never merge movies.
Streaming identities remain separate when no verified shared ID is available.
Disc keys use UPC when available, otherwise a unique format within the report.
Ambiguous duplicate keys abort movie ingestion and are logged for review.
The source exposes one theatrical/digital event per movie; separate reissues
without explicit source identity cannot be distinguished automatically.

Verification: `cargo test --locked --all-targets`; live rollback-only upsert test:
`cargo test --locked --bin web_scraper movie_upserts_are_repeatable -- --ignored`.


## Posters

Movie ingestion now performs optional source-first poster enrichment after release
upserts. See [the poster guide](movie-posters.md) for exact-ID/title-year matching,
portrait validation, cache policy and backfill commands. Poster failures are logged
separately; movie/release ingestion continues. MyNewsFeed uses the existing PosterUrl.
