# Frontend integration: Fandango movie links

Backend deployed October 16, 2026; **frontend integration remains pending**.
Prepared for the separate My-Newsfeed application. This document
is the UI implementation request. Scraper deployment supplies the data; it does
not add buttons to the frontend image. Operational details and deployment status
are maintained in [movie purchase links](movie-purchase-links.md).

## Requested behavior

Add one secondary action to each eligible movie release card:

| ReleaseType | Retailer | Direct link label | Search fallback label |
| --- | --- | --- | --- |
| `theatrical` | Fandango (`www.fandango.com`) | Get tickets on Fandango | Search Fandango |
| `digital`, `digital_purchase`, `digital_rental` | Fandango at Home (`athome.fandango.com`) | Buy on Fandango at Home when purchase is verified; otherwise View on Fandango at Home | Search Fandango at Home |
| `subscription`, `disc` | None added | None | None |

Prefer verified, unexpired movie-specific links. Keep **search** in the fallback
label; it is not a verified movie match or proof of tickets/digital stock. Keep the
existing title/source-detail links and attribution. These are ordinary links to
the retailer's movie page, where readers choose showtimes or purchase options.
No prices, checkout, affiliate tracking, sign-in requirement or payment collection
are added to My-Newsfeed. Do not use AMC or a Google search fallback.

## Query and response fields

Keep the page's current date window, source selector, ordering and grouping.
Production currently uses its existing current-month presentation; this feature
does not change that policy. `@WindowStart` is inclusive and `@WindowEnd` exclusive,
computed with the application's existing Central-time date helper. Use SQL DATE
parameters. Preserve the selected source through `@SourceKey`.

```sql
SELECT r.ID AS ReleaseId, m.ID AS MovieId,
       m.Title, m.OriginalYear, m.PosterUrl,
       r.ReleaseDate, r.ReleaseType, r.Platform, r.Format,
       r.SourceUrl, r.LastSeenAt,
       s.SourceKey, s.Name AS SourceName,
       commerce.ActionUrl AS MovieActionUrl,
       commerce.ActionLabel AS MovieActionLabel,
       commerce.LinkKind AS MovieActionKind
FROM dbo.MovieReleases r
JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId
JOIN dbo.Movies m ON m.ID=l.MovieId
JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId
LEFT JOIN dbo.MovieCommerce commerce
  ON commerce.MovieId=m.ID
 AND commerce.Provider=CASE
       WHEN r.ReleaseType='theatrical' THEN 'fandango'
       WHEN r.ReleaseType IN ('digital','digital_purchase','digital_rental')
         THEN 'fandango-at-home'
       ELSE NULL END
WHERE s.Enabled=1 AND s.SourceKey=@SourceKey
  AND r.CountryCode='US'
  AND r.DateStatus='announced'
  AND r.ReleaseStatus='scheduled'
  AND r.ReleaseDate>=@WindowStart AND r.ReleaseDate<@WindowEnd
  -- Keep the existing tab's parameterized ReleaseType filter here.
ORDER BY r.ReleaseDate DESC, m.Title, r.ID;
```

Add nullable strings `MovieActionUrl`, `MovieActionLabel`, `MovieActionKind` to the
movie DTO/view model. A cache miss must not remove a movie: retain the LEFT JOIN.
The cache key is `(MovieId, Provider)` so this join does not multiply release rows.
Use the existing `MovieId`; never join retailers by title or merge provider movie
records by title alone.

`dbo.MovieCommerce` is a read-only view over the retailer link cache. Its
`LinkKind` is `direct` or `search`. It switches expired direct links to search and
hides cache entries if the film identity changed. The frontend must still handle
a missing row: a new film or new identity can appear before its next enrichment.
A database/query failure is an application error, not a missing-cache-row case;
retain the page's normal error handling.

The frontend account needs SELECT on `dbo.MovieCommerce`. Reuse its established
reader role. If it already has schema-wide SELECT or `db_datareader`, that covers
the view; otherwise the application's migration owner should grant SELECT on the
view to its existing role. No write access or new credentials are needed.

## Fallback generation

Use this behavior in the application's existing server-side link helper. This
JavaScript example specifies the URLs and encoding; translate it to the frontend's
language if needed (`Uri.EscapeDataString` or a query-string builder in C#).

```js
function movieActionFallback(title, releaseType) {
  if (releaseType === 'theatrical') {
    const url = new URL('https://www.fandango.com/search');
    url.searchParams.set('q', title);
    return { url: url.href, label: 'Search Fandango', kind: 'search' };
  }
  if (['digital', 'digital_purchase', 'digital_rental'].includes(releaseType)) {
    const url = new URL('https://athome.fandango.com/content/browse/search');
    url.searchParams.set('searchString', title);
    return { url: url.href, label: 'Search Fandango at Home', kind: 'search' };
  }
  return null;
}
```

For eligible types, use a valid returned cache action; otherwise use the generated
fallback. Do not infer a direct URL from the title or fabricate a retailer movie
ID. Preserve `&`, `+`, `?`, `#`, accented characters and apostrophes as one encoded
query value. Subscription and disc cards return no action even if unrelated cache
data exists for the same movie.

## Rendering and URL validation

Render the action as a secondary link/button below the existing movie details.
Use the exact action label, with the film title as additional accessible context
if needed. Use normal HTML/Razor encoding for labels and href attributes. Open in
the current tab, or use `target="_blank" rel="noopener noreferrer"` for a new tab.
No client-side fetch to retailer sites is required.

Accept HTTPS without credentials or a nonstandard port. Validate host equality
and the expected path, not substring matching:

- Theatrical direct: `www.fandango.com` or `fandango.com`,
  `/<movie-slug>-<numeric-id>/movie-overview`.
- Theatrical search: `www.fandango.com/search?q=<encoded-title>`.
- Digital direct: `athome.fandango.com`,
  `/content/browse/details/<movie-slug>/<numeric-id>`.
- Digital search: `athome.fandango.com/content/browse/search?searchString=<encoded-title>`.

Invalid cache destinations use the generated search fallback. Do not use a
Fandango theatrical link for a digital release, or an at-home link for tickets.
Missing posters/year and search-only retailer data must not hide release cards.
Do not add subscription services to the digital tab or change release dates.

## Acceptance checks

1. A theatrical card with `MovieActionKind='direct'` opens its verified Fandango
   movie page and says **Get tickets on Fandango**.
2. A theatrical card without a match says **Search Fandango** and searches that
   film title on Fandango, including titles containing punctuation.
3. A digital card uses Fandango at Home. Its label is **Buy** only when returned
   as such by the view; otherwise it says **View** or **Search** as supplied.
4. Expired direct data returns a search action. A missing cache row also generates
   the correct fallback without removing the release.
5. Subscription and disc tabs receive no new retailer action.
6. Existing current-month/week filters, source selection, movie counts, dates,
   posters, source links and mobile layout retain their behavior.
7. The production frontend reader can query the view. A database error is handled
   as an error; unsafe destinations are replaced with generated fallbacks.
8. Verify actual rendered cards and destination URLs after frontend deployment.
   Scraper health alone does not establish that this UI integration is complete.

## Production data available for integration

Verified after scraper deployment: `dbo.MovieCommerce` returns nine theatrical
and four digital actions. All 13 currently have `LinkKind='search'`; direct-link
rendering remains covered by isolated fixtures until a production movie match is
verified. Use these actual search actions for initial UI acceptance, and fixtures
for direct/expired/Buy/View variations. All original 23 movies, 23 source links and
34 release IDs/dates/source URLs were preserved. The frontend's existing database
permissions can read the view; no new runtime credentials are needed.
