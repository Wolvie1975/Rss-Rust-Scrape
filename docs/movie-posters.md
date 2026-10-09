# Movie poster enrichment

Movie posters are populated independently of release ingestion. The existing
`Movies.PosterUrl` column is consumed by MyNewsFeed; no frontend change is needed.
All assigned images use browser-loadable HTTPS URLs. Existing movie, source-link
and release identities are never merged or inserted by the poster backfill.

## Source options and selected approach

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

## Matching, validation and preservation

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

## Cache and operation

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

## Initial verification — October 9, 2026

28 of 33 existing movie rows received posters; all 33 original years were populated
from movie-specific source metadata. Movie/source-link/release counts remained
33 / 33 / 76, and every original ID was preserved. Six saved images returned HTTP
200 and were visually checked: Moana (2026 live-action), Angel and the Badman (2026),
Disclosure Day (2026), Hokum (2026), A Merry Christmas Match (2019), and Matchbox (2026).

The remaining five have no safely matched portrait poster in the inspected sources:
Olmo (2025), The Birthday Party (2025), Animals (2026), Oasis: Don’t Look Back in Anger
(2026), and V/H/S Mixtape (2026). They remain NULL and are negatively cached for retry.

See [the verification report](../reports/movie-posters-2026-10-09.md).
