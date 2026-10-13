# My-Newsfeed digital release correction

## Requested behavior

Keep the existing **current Central-time Monday–Sunday week**. Improve VOD/digital
coverage to resemble Movie Insider's 2026 digital listings. Do not expand the page
to the full year, and do not change theatrical, physical-disc or subscription views.

Reference: [Movie Insider's digital calendar](https://www.movieinsider.com/releases/vod-digital/2026).
The originally supplied browse URL and this public calendar use the same 2026
VOD/digital category. The public calendar works with the scraper's normal client.

## What changed in the scraper

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

## Deployment and enabling

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

## Frontend implementation request

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

## Acceptance checks

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
[the existing movie data contract](movie-releases.md).
