# Movies, box office and posters

Use The Numbers for actual weekend top-ten results and Boxoffice Pro for domestic previews and future-film predictions. These are separate structured tables, not news articles. Box Office news is available alongside the structured charts through Deadline, Variety, The Hollywood Reporter and IndieWire. The older news handoff is archived in the entertainment history; the current contract is below. Poster fields are available across the movie sections. Current/future release and forecast dates only; the actual chart retains the latest valid weekend.

## Contents

- [Box Office news sources](#boxoffice-news)

- [Fandango tickets and Fandango at Home links](movie-purchase-links.md)
- [Frontend integration request: movie purchase buttons](movie-commerce-frontend-integration.md)

- [Latest domestic weekend box-office top 10](#boxoffice-top-10-frontend-handoff)
- [Boxoffice Pro weekend previews and future movie forecasts](#boxoffice-forecasts-frontend-handoff)
- [Posters across movie sections](#movie-section-posters-frontend-handoff)
- [My-Newsfeed digital release correction](#my-newsfeed-digital-releases)
- [U.S. movie releases](#movie-releases)
- [Movie poster enrichment](#movie-posters)

<a id="boxoffice-news"></a>

## Box Office news sources

The production **Box Office** news category contains four enabled publishers:

| Source ID | Publisher | RSS URL |
| --- | --- | --- |
| 7094 | Deadline | https://deadline.com/v/box-office/feed/ |
| 9034 | Variety | https://variety.com/v/film/box-office/feed/ |
| 9035 | The Hollywood Reporter | https://www.hollywoodreporter.com/t/box-office/feed/ |
| 9036 | IndieWire | https://www.indiewire.com/c/box-office/feed/ |

Variety, The Hollywood Reporter and IndieWire were added October 19, 2026;
existing Deadline configuration was preserved. A production dry-run and actual
import succeeded with three source successes and zero failures; read-back verified
ten articles per new source, refreshed timestamps and no source errors. The deployed scraper's hourly
`--from-db` collector reads the feeds without an image rebuild or restart.
[`sql/boxoffice_news_expand.sql`](../sql/boxoffice_news_expand.sql) records this
registration, inserts missing URLs enabled and preserves existing source settings.
Apply only with migration credentials targeting WebScraper. The earlier
`boxoffice_news_seed.sql` is a historical disabled-source seed, not the current
production source inventory.

Frontend news queries should join `Pages.SourceId` to `Sources.ID`, then
`Sources.SourceCategoryId` to `SourceCategories.ID`, filtering `Enabled=1` and
`Category_Name=N'Box Office'`. Discover publishers dynamically rather than using
a fixed list of source IDs. Reuse existing article cards, source filtering,
pagination, attribution and saved-article behavior. Use Pages.Url for the article
link; Sources.Url is the RSS feed. Summaries, images and publication dates can
be missing; retain standard escaping, date handling and fallbacks. These feeds
provide article metadata, not publisher paywall access or numeric chart rows.

This expands news coverage; `BoxOfficeWeekend` and Boxoffice Pro forecast
collection remain independent. Source registration does not prove frontend
rendering, which was not changed as part of this update.

<a id="boxoffice-top-10-frontend-handoff"></a>

## Latest domestic weekend box-office top 10

Source: https://www.the-numbers.com/weekend-box-office-chart

The scraper stores the latest available three-day domestic weekend chart in
`dbo.BoxOfficeWeekend`. It is structured chart data, separate from the Box Office
news articles in `Pages` and the news RSS file.
The front-end needs a chart view; scraper deployment alone does not add that view.

<a id="boxoffice-top-10-frontend-handoff--query-and-display"></a>

### Query and display

```sql
SELECT Rank, WeekendStart, MovieTitle, WeekendGross, TotalDomesticGross,
       WeeklyChangePercent, Theaters, MovieUrl, SourceUrl, FetchedAt
FROM dbo.BoxOfficeWeekend
ORDER BY Rank;
```

Show a heading such as **Domestic box office — October 9–11, 2026**.
`WeekendStart` is Friday; the displayed end date is Sunday (+2 days).
Show rank, linked movie title, weekend gross, total domestic gross, and weekly
change. Amounts are USD integers; format as currency. Change is a percentage
number (`-63` means `-63%`); null means unavailable/new, not zero.
`FetchedAt` is UTC. Link to `SourceUrl` with attribution to The Numbers.
Do not label the data as final actuals: the source may revise its reported totals.

<a id="boxoffice-top-10-frontend-handoff--refresh-and-retention"></a>

### Refresh and retention

Run `--configure-boxoffice` once with migration credentials, then `--boxoffice`
with the normal ingestion account. Add `--boxoffice` to the hourly scraper command.
`--boxoffice --dry-run` fetches and validates without changing stored content.
The collector participates in scraper run summaries and logs failures.

A successful refresh atomically replaces all ten rows. Old weekends are removed
only after a valid replacement. Invalid, incomplete, out-of-order, future-dated,
or stale (>10 days from Friday) source charts leave the last good chart intact.
The SQL writer rejects dates older than the stored chart. The front-end should
show the weekend date and a stale indicator if the stored Friday is >10 days old.
The source sometimes omits rank numbers; rows are ranked by descending reported
weekend gross, with explicit ranks checked when supplied.

This does not change movie release, news, or other sports retention policies.

For upcoming previews and future movie predictions, see
[Boxoffice Pro forecast handoff](movies-and-box-office.md#boxoffice-forecasts-frontend-handoff).

PosterUrl and PosterLookupStatus are now available. See
[movie-section poster handoff](movies-and-box-office.md#movie-section-posters-frontend-handoff).

<a id="boxoffice-forecasts-frontend-handoff"></a>

## Boxoffice Pro weekend previews and future movie forecasts

The scraper collects domestic USD forecast ranges from
https://www.boxofficepro.com/category/forecasts-tracking/ . Forecasts are separate
from actual weekend results (`dbo.BoxOfficeWeekend`). No article bodies or
pros/cons text are copied; link readers to the original article for analysis.

<a id="boxoffice-forecasts-frontend-handoff--data-contract"></a>

### Data contract

```sql
DECLARE @Today DATE = CONVERT(date, SYSUTCDATETIME() AT TIME ZONE 'UTC'
                              AT TIME ZONE 'Central Standard Time');
SELECT Kind, TargetDate, PeriodEnd, MovieTitle, Distributor, PredictedRank,
       ForecastLow, ForecastHigh, ForecastAsOf, PublishedAtUtc, UpdatedAtUtc,
       SourceUrl, FetchedAtUtc
FROM dbo.BoxOfficeForecasts
WHERE PeriodEnd >= @Today
ORDER BY Kind, TargetDate, PredictedRank, MovieTitle;

SELECT Kind, LatestPublishedAt, LastAttemptAt, LastSuccessAt, LastError,
       LastMovieCount
FROM dbo.BoxOfficeForecastSources;
```

| Field | Meaning |
| --- | --- |
| Kind | `weekend-preview` or `long-range` |
| TargetDate | Weekend start for previews; domestic release date for long-range forecasts |
| PeriodEnd | Preview end date; equals TargetDate for long-range forecasts |
| MovieTitle / Distributor | Provider's movie title and distributor |
| PredictedRank | Preview rank, otherwise null |
| ForecastLow / ForecastHigh | Predicted domestic gross range, integer USD |
| ForecastAsOf | Date of this prediction; older tracking tables retain their explicit as-of date |
| PublishedAtUtc / UpdatedAtUtc | Article publication and modification timestamps |
| SourceUrl | Original article; attribution and analysis link |
| FetchedAtUtc | Latest successful local import |

The primary key is `(Kind, TargetDate, MovieTitle)`. Do not join movies by a
numeric ID or assume title matching is exact against the movie-release tables.
Forecasts describe opening-weekend gross for long-range films and the specified
weekend gross for previews. They are predictions, not actual results.

<a id="boxoffice-forecasts-frontend-handoff--front-end-layout"></a>

### Front-end layout

Keep **Weekend results — Top 10** from The Numbers. Add:

- **Upcoming weekend preview — Predictions**: group `weekend-preview` by
  TargetDate/PeriodEnd, display rank, title, distributor and predicted range.
- **Upcoming movie forecasts — Predictions**: group `long-range` by release date,
  display title, distributor, predicted opening range and ForecastAsOf.

Format ranges such as `$25M–$35M`. Show **Source: Boxoffice Pro** linked to each
row's SourceUrl. The preview is not necessarily a top ten; show only provider
ranks. Do not fill missing ranks or supply your own predictions.

When no current preview exists, display **The next weekend preview has not been
published yet** only when the source's last successful import is recent and
LastError is null. Otherwise display **Preview data is temporarily unavailable**.
No current future-film forecasts should use the equivalent empty state. Show a
refresh warning if an import failed; LastError is diagnostic data for admins.

<a id="boxoffice-forecasts-frontend-handoff--collection-and-retention"></a>

### Collection and retention

`--configure-boxoffice` creates both the actuals and forecast schemas with
migration credentials. Hourly production collection adds `--boxoffice-forecasts`.
`--boxoffice-forecasts --dry-run` fetches and validates without database changes.

The collector selects the newest domestic weekend-preview article and newest
long-range article from the category listing, using publication timestamps.
It excludes U.K./Ireland forecasts and general box-office stories. It reads the
long-range tracking table plus newly featured film headings; new headings take
precedence over the older tracking table in the same article. It does not fetch
older articles to reconstruct films omitted from the latest publication.

Both snapshots replace their prior data in one SQL transaction, keeping no poll
history. Invalid pages, malformed ranges, missing links, inconsistent preview
ranks and failed requests preserve the previous snapshot. Older publication
versions cannot replace newer stored ones. Article bodies older than 60 days
are rejected. An expired but valid preview is a successful empty snapshot.

Only current/future content is saved. Previews remain through their specified
end date; long-range forecasts remain through their release date. Expired rows
are removed by routine retention, even when a later collection fails. The SQL
query filters dates too, so the site does not wait for hourly cleanup.

October 13 verification: the latest preview covers October 9–11 and imports
zero current rows; the latest long-range article supplies 12 future-film rows.

PosterUrl and PosterLookupStatus are now available. See
[movie-section poster handoff](movies-and-box-office.md#movie-section-posters-frontend-handoff).

<a id="movie-section-posters-frontend-handoff"></a>

## Posters across movie sections

Movie releases continue using `dbo.Movies.PosterUrl`. Box-office results
(`dbo.BoxOfficeWeekend`) and upcoming previews/long-range forecasts
(`dbo.BoxOfficeForecasts`) now expose nullable `PosterUrl` and
`PosterLookupStatus` fields directly. Include these columns in the front-end
queries from the existing section handoffs.

Render a portrait image with MovieTitle as alt text. Use a neutral poster
placeholder when PosterUrl is null or the browser cannot load the image.
Do not substitute a news article hero image, logo or trailer thumbnail.
Portrait JPEG/PNG dimensions and image response type are checked before saving.
Movie title and original film year must match the source. Re-releases use the
original year from the linked The Numbers movie page URL, not the chart year.
For future forecasts, the target release year is matched against movie metadata;
ambiguous or conflicting matches leave the poster unavailable.

`PosterLookupStatus`: `matched` means a validated image, `missing` means no unique
match was found, and `error` means a lookup/validation request failed. An error
may retain the previous validated URL for the same identity. Status/error details
are for admin diagnostics; show the image or placeholder to readers.

The shared `dbo.CinemaPosterCache` uses normalized title plus original year.
It reuses validated release posters, otherwise tries the linked The Numbers
movie detail poster for actuals and the existing DVD Release Dates search.
Box-office and forecast replacement does not delete this cache. Normal hourly
imports reattach cached posters without downloading images again; successful
matches refresh after 30 days, missing matches retry after 7 days, and errors
retry after 6 hours. Entries unused for 90 days are removed by retention.

`--configure-boxoffice` applies the additive schema with migration credentials.
Hourly `--boxoffice` / `--boxoffice-forecasts` imports enrich both sections.
`--movie-posters` backfills releases and box-office/forecast posters together;
`--retry-missing-posters` forces existing release-section missing-image retries.
No new external API key is required. Images remain external HTTPS URLs.

Initial validated coverage: all 10 actual chart films and all 12 future forecasts.
The current preview dataset is empty because the last preview has expired;
newly published preview rows will use the same enrichment automatically.

Release-section backfill found one additional validated poster using The Numbers
as a fallback. Current releases: 24 of 34 movies have posters; the remaining ten
have no unambiguous title/year match and should display placeholders. Existing
release enrichment retries those missing images under its normal cache policy.

<a id="my-newsfeed-digital-releases"></a>

## My-Newsfeed digital release correction

<a id="my-newsfeed-digital-releases--requested-behavior"></a>

### Requested behavior

Keep the existing **current Central-time Monday–Sunday week**. Improve VOD/digital
coverage to resemble Movie Insider's 2026 digital listings. Do not expand the page
to the full year, and do not change theatrical, physical-disc or subscription views.

Reference: [Movie Insider's digital calendar](https://www.movieinsider.com/releases/vod-digital/2026).
The originally supplied browse URL and this public calendar use the same 2026
VOD/digital category. The public calendar works with the scraper's normal client.

<a id="my-newsfeed-digital-releases--what-changed-in-the-scraper"></a>

### What changed in the scraper

- Added provider `movieinsider` for confirmed VOD/digital dates. Calendars discover
  candidates; the movie's explicit `#digital-release-date` paragraph and its
  `<time datetime>` supply the date. Theatrical dates, estimates and disc dates
  do not become digital releases.
- All pagination is scanned, with a 20-page-per-year limit. The source has some
  out-of-order cards, so early stopping on dates could miss releases.
- Fixed `whentostream` to import its explicit VOD Release Date as `digital` as
  well as its separate SVOD date as `subscription`. These dates must not be merged.
- Digital remains generic rental/purchase availability (`ReleaseType='digital'`,
  `Format='VOD / Digital'`); platform and rental-versus-purchase are not invented.
- Movie Insider's numeric `m<ID>` identifies the film; `digital:first` identifies
  the release. Date changes and slug changes update existing records.
- Movie Insider's own labeled poster is supported by the existing validation
  pipeline. Unknown original years/IMDb IDs remain unknown; the scraper does not
  infer the original film year from its digital release date or search links.
- Existing sources and stored observations remain intact. Different providers
  can have different coverage and dates; no existing dates have been declared
  wrong or overwritten solely from a title match.

Live validation for October 5–11, 2026 found **10 Movie Insider digital releases**,
including His Monster, Karl, Line of Control, Carlton Taylor Has Major Problems
and The Florist. When To Stream provides **5 digital observations** and 9
subscription observations; some overlap with the other providers. These counts
are validation observations, not permanent expected page totals.

<a id="my-newsfeed-digital-releases--deployment-and-enabling"></a>

### Deployment and enabling

Existing movie tables and runtime permissions are reused. There is no new movie
column or table migration. Production deployment and enabling completed 2026-10-11; Movie Insider has 10
confirmed digital records for the October 5–11 week. Frontend changes below remain
to be implemented in the separate My-Newsfeed application.

1. Build/deploy the updated scraper binary/image using the existing workflow.
2. With migration credentials run `web_scraper --configure-movieinsider`, or
   apply `sql/movieinsider_seed.sql` against WebScraper. This registers a disabled
   source and preserves operator settings on repeat setup. Explicit CLI setup
   also applies this application's existing additive schema migrations.
3. Enable it with admin credentials:

   ```sql
   UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='movieinsider';
   ```

4. Preview the production collection with `web_scraper --movies --dry-run`.
   The existing hourly `--movies` command collects all enabled movie providers.
   Source-specific read-only previews are available locally:

   ```bash
   cargo run --locked --example movie_preview -- --week-of 2026-10-11 --source movieinsider --output /tmp/movieinsider-preview.json
   cargo run --locked --example movie_preview -- --week-of 2026-10-11 --source whentostream --output /tmp/whentostream-preview.json
   ```

5. Run the updated importer and apply the frontend changes below. Back up before
   any future cleanup; this change does not delete historical provider rows.

<a id="my-newsfeed-digital-releases--frontend-implementation-request"></a>

### Frontend implementation request

Make **Movie Insider the preferred source for the Digital/VOD view** when enabled,
with a source selector for When To Stream and DVDs Release Dates. Keep the selected
source visible and preserve the selection in the URL. This aligns the default
list with the user's reference and avoids silently mixing catalog availability,
new digital premieres and several provider observations of the same film.

If Movie Insider is enabled but has no current-week rows, show an honest empty
state and allow another source to be selected; do not silently blend legacy rows
into its list. If it has a recorded refresh error, show a public availability
message and retain previously collected rows. Keep raw errors in admin/server logs.

Reuse the existing Movies, MovieSourceLinks and MovieReleases models. Filter the
selected provider in the query, as well as the current week:

```sql
SELECT r.ID AS ReleaseId, m.ID AS MovieId,
       m.Title, m.OriginalYear, m.PosterUrl,
       r.ReleaseDate, r.ReleaseType, r.Platform, r.Format,
       r.SourceUrl, r.LastSeenAt,
       s.SourceKey, s.Name AS SourceName, s.LastScrapedAt, s.LastError
FROM dbo.MovieReleases r
JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId
JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId
JOIN dbo.Movies m ON m.ID=l.MovieId
WHERE s.Enabled=1 AND s.SourceKey=@SourceKey
  AND r.CountryCode='US'
  AND r.ReleaseType IN ('digital','digital_purchase','digital_rental')
  AND r.DateStatus='announced'
  AND r.ReleaseStatus NOT IN ('cancelled','withdrawn')
  AND r.ReleaseDate>=@WeekStart AND r.ReleaseDate<@NextWeekStart
ORDER BY r.ReleaseDate DESC, m.Title, r.ID;
```

Use parameterized SQL. Calculate Monday and the following Monday using the site's
Central-time date helper. ReleaseDate is a calendar DATE; render it directly,
without UTC conversion. LastSeenAt/LastScrapedAt are UTC DATETIME2 values and need
explicit UTC-to-Central conversion. LastScrapedAt records a collection attempt;
label it **Last checked**, not a guaranteed successful refresh time.

Group the list by day, newest first, and show poster, title, date, “VOD / Digital”,
source and a link to the movie detail page. Missing poster/year/platform must not
hide the release or produce fabricated values. Keep subscription platforms such
as Hulu/Paramount+ in the separate subscription view; “digital” does not prove a
subscription premiere. HTML-encode data and accept only HTTP(S) links/images.

Do not merge Movies by title alone. Existing IMDb/TMDB IDs can identify a shared
movie; absent IDs, keep source identities separate. An optional comparison view
may show provider observations with clear attribution rather than implying they
are several different release events. Never deduplicate using date in the key,
since dates can change.

<a id="my-newsfeed-digital-releases--acceptance-checks"></a>

### Acceptance checks

- The selected source shows only the current Monday–Sunday week, including earlier
  days in that week; the sports page's today-forward rule is not applied here.
- Movie Insider's confirmed digital dates are used, including films absent from
  the DVDs Release Dates digital list.
- The source selector does not quietly mix provider rows or include disabled feeds.
- VOD and SVOD records stay separate; disc bundles and estimated dates stay out.
- Re-importing a date/slug change preserves the release ID and does not duplicate it.
- Missing images, missing metadata, empty weeks and source failures render clearly.
- Existing theatrical, disc and subscription pages retain their behavior.

Give this document to the My-Newsfeed application along with
[the existing movie data contract](movies-and-box-office.md#movie-releases).

<a id="movie-releases"></a>

## U.S. movie releases

October 16 deployed addition: optional Fandango theatrical and Fandango at Home digital
links, with labeled search fallbacks. See [purchase-link implementation and frontend
handoff](movie-purchase-links.md). Backend and existing frontend direct buttons verified; eight theatrical films now link directly to Fandango.

2026-10-11 update: Movie Insider digital provider and separate When To Stream VOD
parsing are implemented. See [the digital release correction/frontend handoff](movies-and-box-office.md#my-newsfeed-digital-releases)
for deployment and the recommended current-week source selection. Older source
investigation notes below describe the original implementation.

Scope agreed 2026-10-08: U.S. theatrical releases, digital purchase/rental,
subscription streaming, and physical discs. The database schema, read-only preview, and main-CLI database ingestion are implemented.
`--movies` imports the current Central-time week and refreshes tracked movie detail pages on each run.

<a id="movie-releases--sources-investigated"></a>

### Sources investigated

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

<a id="movie-releases--database-layout"></a>

### Database layout

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

<a id="movie-releases--weekly-display"></a>

### Weekly display

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

<a id="movie-releases--setup-and-validation"></a>

### Setup and validation

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

<a id="movie-releases--live-preview"></a>

### Live preview

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


<a id="movie-releases--scheduled-ingestion"></a>

### Scheduled ingestion

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


<a id="movie-releases--posters"></a>

### Posters

Movie ingestion now performs optional source-first poster enrichment after release
upserts. See [the poster guide](movies-and-box-office.md#movie-posters) for exact-ID/title-year matching,
portrait validation, cache policy and backfill commands. Poster failures are logged
separately; movie/release ingestion continues. MyNewsFeed uses the existing PosterUrl.

<a id="movie-posters"></a>

## Movie poster enrichment

Movie posters are populated independently of release ingestion. The existing
`Movies.PosterUrl` column is consumed by MyNewsFeed; no frontend change is needed.
All assigned images use browser-loadable HTTPS URLs. Existing movie, source-link
and release identities are never merged or inserted by the poster backfill.

<a id="movie-posters--source-options-and-selected-approach"></a>

### Source options and selected approach

1. **DVDs Release Dates:** detail pages expose a movie-specific `img[itemprop=image]`
   and a dedicated larger “Movie Poster” link. Use the linked movie page first,
   retaining its IMDb ID, explicit original-film year, image and source provenance.
2. **When To Stream:** inspected movie pages supply landscape feature stills, not
   portrait posters. Those images are rejected. Its explicit title/year can match
   an existing verified source poster or a DVDs Release Dates search result.
3. **TMDB, investigated but not selected:** a key or read-access token is
   required. Prefer movie ID lookup, or IMDb external-ID lookup restricted to movie
   results. Without an ID, require an exact normalized title and original year and
   one distinct movie result. Its public HTTPS image CDN does not require exposing
   API credentials to the browser. Non-commercial API use requires attribution;
   commercial use requires a commercial agreement. The app's About/Credits section
   must carry TMDB's required logo and notice before enabling that provider.
   [Requirements](https://developer.themoviedb.org/docs/faq),
   [IMDb lookup](https://developer.themoviedb.org/reference/find-by-id),
   [image URLs](https://developer.themoviedb.org/docs/image-basics).
4. **OMDb, investigated but not selected:** requires an API key; the free tier currently permits 1,000
   requests/day. The dedicated high-resolution Poster API is patron-only. Use a
   normal metadata response's public Poster URL rather than storing an API-key URL
   in the database. [API](https://www.omdbapi.com/),
   [key and quota](https://www.omdbapi.com/apikey.aspx).

The user selected source poster links with a generic display placeholder instead of
an API fallback. MyNewsFeed hosts `img/movie-placeholder.svg` and uses it as the
background of both Movies-page and Friday-theater poster cards. A valid source
image overlays it; missing or failed images reveal the placeholder. Unmatched DB
PosterUrl values remain NULL so future source posters can be discovered normally.
No TMDB or OMDb credentials were configured in the workspace or running service.
The implemented backend therefore uses existing sources only; there is no active
external-provider API client. Adding credentials alone does not activate a provider.

<a id="movie-posters--matching-validation-and-preservation"></a>

### Matching, validation and preservation

- IMDb/TMDB ID matches take precedence; conflicting known IDs are rejected.
- Otherwise, require exact normalized title and original-film year. No fuzzy title
  matching, release-week year substitution or automatic remake/re-release merging.
- A linked source detail page can supply a missing original year for the same movie.
  OriginalYear is never overwritten once populated. Cross-source title/year ambiguity
  leaves PosterUrl NULL, even when only one candidate has an available image.
- Source search handles both result lists and single-result redirects. Recommended
  movies/sidebar links are excluded; every candidate detail page is checked.
- Only movie-specific portrait candidates are accepted. Generic logos/placeholders,
  landscape stills and unrelated image alt text are rejected. HTTPS image GETs must
  return JPEG/PNG with plausible portrait dimensions; no Referer or API key is needed.
- Valid existing posters are retained. A temporary image-network failure preserves
  an existing HTTPS poster for retry. An optimistic update prevents overwriting a
  poster changed concurrently by another process/admin.
- Source failures and image/matching failures are recorded without preventing movie
  or release ingestion. Poster metadata is optional in parser output.

<a id="movie-posters--cache-and-operation"></a>

### Cache and operation

The additive migration `sql/movie_posters.sql` adds poster status/provenance/retry
columns to Movies and a MoviePosterCache table keyed by the existing source-link ID.
Metadata cache identities include verified IDs or title/original year. Positive
source/image validation is cached for 30 days; missing metadata for 7 days; source
errors for 6 hours. Valid existing posters are rechecked after 30 days. Unmatched
searches are retried after 7 days, with an explicit operator override available.
Normal `--movies` runs reuse metadata extracted from already fetched detail pages.

```sh
# Poster-only backfill; no movie/release/source-link inserts.
cargo run --locked -- --movie-posters
# Retry cached missing posters now; existing valid posters stay intact.
cargo run --locked -- --movie-posters --retry-missing-posters
# Read-only database snapshot.
cargo run --locked --example audit_movies -- WebScraper /tmp/movie-audit.json
# Matching/parser tests, plus optional rollback-only SQL preservation test.
cargo test --locked --all-targets
cargo test --locked --bin web_scraper poster_backfill_preserves -- --ignored
```

Poster-only mode cannot be combined with dry-run or scheduled mode. Regular movie
scrapes perform enrichment automatically after their release upserts. A warm cache
backfill validated zero additional source requests and preserved all filled posters.

<a id="movie-posters--initial-verification--october-9-2026"></a>

### Initial verification — October 9, 2026

28 of 33 existing movie rows received posters; all 33 original years were populated
from movie-specific source metadata. Movie/source-link/release counts remained
33 / 33 / 76, and every original ID was preserved. Six saved images returned HTTP
200 and were visually checked: Moana (2026 live-action), Angel and the Badman (2026),
Disclosure Day (2026), Hokum (2026), A Merry Christmas Match (2019), and Matchbox (2026).

The remaining five have no safely matched portrait poster in the inspected sources:
Olmo (2025), The Birthday Party (2025), Animals (2026), Oasis: Don’t Look Back in Anger
(2026), and V/H/S Mixtape (2026). They remain NULL and are negatively cached for retry.

See [the verification report](../reports/entertainment-history.md#movie-posters-2026-10-09).

