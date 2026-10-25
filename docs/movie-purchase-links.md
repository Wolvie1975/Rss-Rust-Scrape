# Movie tickets and digital purchase links

Backend deployed October 16, 2026; current image is
`web-scraper:movie-direct-links-20261016`. The existing My-Newsfeed frontend is
confirmed to render these actions. User preference:
direct movie pages when verified, with explicitly labeled search fallbacks.

## Reader behavior

| Release | Verified movie page | Fallback |
| --- | --- | --- |
| Theatrical | Get tickets on Fandango | Search Fandango |
| Digital / digital purchase / digital rental | Buy on Fandango at Home when a current purchase offer is explicit; otherwise View on Fandango at Home | Search Fandango at Home |
| Subscription or physical disc | No new commerce button | No new commerce button |

Digital purchases use **Fandango at Home**, `athome.fandango.com`, rather than
Fandango's theatrical ticket service. A digital release date does not establish
retailer availability, purchase versus rental, or pricing. The destination handles
location, showtimes, account, price and checkout. These are ordinary outbound
links, without affiliate identifiers or local payment handling.

Theatrical tickets use `www.fandango.com`, with movie pages such as
`https://www.fandango.com/moana-2026-243429/movie-overview` and the public search
route `https://www.fandango.com/search?q=...`. Digital purchases retain
`https://athome.fandango.com/content/browse/search?searchString=...`.
The original AMC proposal was replaced before deployment after AMC blocked
requests. Both fallback searches now go directly to the selected Fandango service.
Search terms are URL-encoded; movie slugs and retailer IDs are never invented.

## Data and verification

`dbo.MovieCommerceLinks` caches one row per `(MovieId, Provider)`; providers are
`fandango` and `fandango-at-home`. `dbo.MovieCommerce` exposes:

| Field | Meaning |
| --- | --- |
| MovieId / Provider | Existing movie ID and retailer |
| ActionUrl / ActionLabel | Destination and exact display text |
| LinkKind | `direct` or `search` |
| CheckedAt / VerifiedUntil | UTC check and direct-link expiry timestamps |

The view automatically falls back after expiry and hides cache rows when the
stored title/original year/IMDb identity changes. It preserves source links and
existing release/movie IDs. Retention deletes retailer cache entries with their
parent movie; no retention or scheduler rules change.

Normal non-dry-run `--movies` collection seeds search fallbacks for enabled U.S.
announced, scheduled theatrical/digital releases from the current Central-time
Monday onward. This includes earlier days of the current week for frontend views.
The frontend retains its existing month/week filters. No subscription/disc links
are generated. Cache enrichment is optional: failure does not fail release saves;
a missing migration logs a message and skips enrichment without runtime DDL.

Retailer candidates come from source-page links already fetched for that movie,
then public search links if needed. A title search never directly selects a
purchase destination. At most two missing source pages, one search and three
retailer candidate pages are fetched per job. Up to 12 due movie/provider jobs run
per import, ordered by oldest check. No new HTTP request starts after a 30-second
commerce budget; one in-flight request may finish afterward. Requests have a
five-second timeout, shared public-DNS/redirect checks and response-size limits.
Already collected source pages are reused. JavaScript-only results, challenges,
HTTP errors and missing/ambiguous identities produce search fallbacks.

Direct links require HTTPS, an exact approved retailer host, a retailer movie-page
path and a successful final response on that retailer's movie-page path. The page
must supply exactly one matching Movie structured-data identity: normalized exact
title plus original year, or matching IMDb ID without conflicting year/IMDb.
When Movie structured data is absent, an exact matching title **and original year
in both the page title and sole H1** may establish identity (as seen in public
Fandango HTML). Missing original year is never filled from a release date or URL.
Conflicting structured identities and multiple matching direct destinations are
not selected. An ordinary title heading alone does not suffice.

A digital **Buy** label additionally requires a structured offer explicitly marked
Sell, InStock and carrying a valid price. Rental-only and unspecified offers use
**View on Fandango at Home**. No price is shown by this feature. Direct matches
expire/recheck after seven days; missing/error results retry after six hours.
Transient failures preserve a previous verified link only until its original
expiry. A successful unmatched check clears that link.

## Reviewed movie mappings

`src/movie_commerce_mappings.json` holds explicitly reviewed retailer detail URLs
for pages that can be read through the web reader but return HTTP 403 to the
collector. This is a separate evidence path: HTTP 403 itself never proves a match.
Each entry records provider, exact movie title/original year, full validated URL,
review timestamp, expiry timestamp and the evidence checked. Optional IMDb IDs
must match when present. The registry is embedded in the scraper image.

Reviewed mappings are evaluated before network discovery, even when the existing
search-fallback cache is not due. A matching reviewed entry may replace that
fallback without another blocked request. Wrong/unknown original years, wrong
providers, conflicting IMDb IDs, unsafe URLs, duplicate matching entries,
future reviews, expired reviews and review periods longer than 30 days are
rejected. Reviewed mappings never assert digital purchase stock or prices.

A cache write expires at the earlier of its usual seven-day lifetime and the
review's absolute expiry; RetryAfter is also capped. Hourly imports do not extend
the review timestamp. After expiry, ordinary independent network verification or
a new documented review is required; otherwise the view uses search fallbacks.
The existing DTO, view and frontend query remain unchanged.

To add/renew a mapping, read the actual retailer detail page, verify film title
and original year (check cast/director for ambiguous remakes), and record the full
observed URL and UTC review/expiry in the registry. Do not copy a search snippet
as purchase evidence or invent a retailer ID. Validate with the commerce tests,
build/deploy the image, and run a targeted movie import. The current registry
contains eight reviewed theatrical movies, including
[Street Fighter (2026)](https://www.fandango.com/street-fighter-2026-246472/movie-overview).

Automatic search discovery now selects exact-title/year anchors before applying
the candidate request limit, so unrelated sidebar films cannot consume the first
three requests. Search results remain candidates: the final detail page must
verify identity. Movie JSON-LD names containing a parenthesized original year are
supported, with conflicting years rejected.

## Frontend integration

Give the frontend project [this standalone integration request](movie-commerce-frontend-integration.md).

Keep current movie/source/date filters, title attribution and source-detail links.
Add this left join to the existing movie-release query:

```sql
LEFT JOIN dbo.MovieCommerce commerce
  ON commerce.MovieId=m.ID
 AND commerce.Provider=CASE
       WHEN r.ReleaseType='theatrical' THEN 'fandango'
       WHEN r.ReleaseType IN ('digital','digital_purchase','digital_rental')
         THEN 'fandango-at-home'
       ELSE NULL END
```

Select `commerce.ActionUrl`, `commerce.ActionLabel`, `commerce.LinkKind` and render
one additional button. For a missing cache row, generate the same search fallback
in the application, so a newly discovered film does not wait for enrichment:

```js
function movieCommerceFallback(title, releaseType) {
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

HTML-encode labels/attributes and allow only HTTPS destinations. Expected direct
hosts are `fandango.com`, `www.fandango.com`, `athome.fandango.com`; search
fallbacks use `/search?q=...` or `/content/browse/search?searchString=...` respectively.
Use `rel="noopener noreferrer"` if opening a new tab. Keep search labels even when
the card is styled as a button. Do not describe a fallback as confirmed stock,
a confirmed local engagement, or a purchase offer.

## Setup and checks

Using migration credentials against the intended database, apply each file as a
separate batch in this order:

1. `sql/movie_commerce.sql` (cache table).
2. `sql/movie_commerce_view.sql` (view).
3. `sql/movie_commerce_permissions.sql` (existing ingestion role grants).

The base movie schema must already exist. `--initialize-db` includes the table and
view; `examples/setup_movie_schema.rs` also applies both. Existing runtime movie
permissions are unchanged; `sql/runtime_permissions.sql` includes the new grants.
Grant SELECT on `dbo.MovieCommerce` to the frontend's existing reader role during
its deployment; that principal is managed by the separate application.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
# Migration credentials; ONLY an isolated database with this prefix is accepted.
MOVIE_COMMERCE_TEST_DATABASE=WebScraper_MovieCommerce_Validation_example \
MOVIE_COMMERCE_TEST_PROFILE=.env.migrations \
cargo test --locked --lib movie_commerce_cache_preserves -- --ignored
```

The opt-in test creates its isolated schema and leaves its database for inspection;
remove that test database afterward. It checks the real parameterized cache writes
under the restricted ingestion user, idempotent fallback seeding, expiry, temporary
errors and identity changes. `tests/sql/movie_commerce.sql` checks the view/labels
and retention cascade inside a rolled-back transaction. Schema repeatability is
also checked separately. Production migration, targeted import and container replacement are complete;
see the deployment evidence below.

Rollback: stop using the frontend join/buttons and deploy the prior scraper image.
The additive table/view can remain. Existing movie records, dates and source URLs
are not rewritten by this feature.

## October 16 validation evidence

- 158 offline tests passed (131 main binary, 27 preview example); 20 opt-in tests
  were skipped by the normal suite. Strict Clippy and formatting checks passed.
- The commerce opt-in test ran separately and passed under restricted ingestion
  permissions in `WebScraper_MovieCommerce_Validation_20261016_Rust`.
- Independent SQL schema replay and view/retention fixtures passed in
  `WebScraper_MovieCommerce_Validation_20261016`; both validation databases were
  removed after testing.
- Production deployment applied the additive cache/view/grants and replaced the
  scraper container. Existing movie data, source settings and hourly configuration
  were preserved; the new cache was populated by a targeted import.
- Live public Fandango detail HTML exposed the matching title/H1 identity for
  Moana (2026); search content was JavaScript-driven. AMC returned HTTP 403
  and was replaced by Fandango for theatrical links before deployment.
  Automated direct-link coverage is not guaranteed; these cases retain fallbacks.

Provider references: [Fandango digital purchase guidance](https://streamsupport.fandango.com/s/article/So-how-do-I-rent-or-buy-movies-and-TV-shows)
and [Fandango theatrical movie pages](https://www.fandango.com/moana-2026-243429/movie-overview).

## Initial production deployment — October 16, 2026

Image `web-scraper:movie-commerce-20261016` (`:latest` points to the same image):
`sha256:931488d3f6ea962215f0d773f6bac12081036bdf1a10ab6a424522054dc05e5f`.
Targeted production run **160**, 9:35:27–9:36:03 a.m. Central, saved **34 releases**
with **zero source issues**. The cache/view expose **13 actions**: nine theatrical
and four digital, all currently **search fallbacks**. Twelve due jobs were checked;
one digital job remains for the next bounded hourly pass. No direct movie-page
match was verified, and no unverified purchase offer is labeled Buy.

The 23 movie IDs, 23 source links and 34 release identities/dates/source URLs
matched the pre-deployment snapshot exactly. The existing enabled source settings,
article ID recovery mapping, hourly command and runtime/security settings remain
intact. Frontend view access is covered by its existing database permissions.
The scraper and unchanged frontend container were healthy at verification.

Backup: verified copy-only/checksum full database backup
`/var/opt/mssql/backup/webscraper-managed/manual/WebScraper_before_movie_commerce_20261016.bak`.
Rollback container: `web-scraper-before-movie-commerce-20261016` (stopped);
rollback image: `web-scraper:before-movie-commerce-20261016`.
Protected host source/configuration backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261016-movie-commerce/`.
The additive cache can remain when rolling back; existing movie data needs no
restore merely to remove the new feature. Frontend integration was pending at this initial deployment; see the direct-link verification below.

## Direct-link deployment — October 16, 2026, 1:30 p.m. Central

Current image `web-scraper:movie-direct-links-20261016` (`:latest`):
`sha256:3343b5f8854e8ae7fafa3e9227f3c479c8c2f6aec3963c8bf5c52ff691cb0cf2`.
No additional database migration or grants were needed; existing cache/view fields
support the reviewed mapping expiry. Eight theatrical mappings were reviewed on
October 16 through the Fandango detail pages; their absolute review expiry is
recorded in the embedded registry and cannot exceed 30 days.

Run **165**, **1:29:46–1:30:13 p.m. Central**, saved 34 releases, zero source issues.
Production now returns **eight direct theatrical actions**, one theatrical search
fallback (Sacrifice), and four digital search fallbacks. The unchanged frontend
`/Movies` response returned HTTP 200, rendered all eight direct Fandango buttons,
and included Street Fighter's exact `street-fighter-2026-246472/movie-overview`
href with no Street Fighter search href. No frontend image change was required.
The earlier all-search counts above describe the initial deployment only.

161 offline tests and strict Clippy passed. The restricted-account SQL cache test
passed separately, including review-expiry and RetryAfter caps. Validation database
`WebScraper_MovieCommerce_Validation_20261016_Direct` was removed. Original movie
IDs, source links, release dates/source URLs, source settings and article recovery
mapping matched before/after. Hourly runtime/security/network settings were
preserved; the next slot at verification was 2:00 p.m. Central.

Verified copy-only/checksum database backup:
`/var/opt/mssql/backup/webscraper-managed/manual/WebScraper_before_movie_direct_links_20261016.bak`.
Rollback image `web-scraper:before-movie-direct-links-20261016` and stopped container
`web-scraper-before-movie-direct-links-20261016` retain the preceding commerce build.
Host source/config/evidence backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261016-movie-direct-links/`.
Rollback may leave these valid expiring cache links intact, or the reviewed rows
can be changed back to search fallbacks explicitly; existing movie data need not
be restored merely to remove direct-link support.
