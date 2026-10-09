# Movie poster verification — October 9, 2026

- Enriched **28 of 33 existing movie rows**, using posters from existing sources only.
- Before/after movie, source-link and release counts: **33 / 33 / 76**, unchanged.
- Every original movie, source-link and release ID was preserved. No new source links
  were created for cross-source poster searches; provenance is stored in PosterSourceUrl.
- All 33 OriginalYear values came from movie-specific source metadata or explicit movie
  title/year, never from this week's theatrical/digital/disc release date.
- Cache validation: deployed-container rerun enriched 0, preserved 28, used 33 cache hits
  and made **0 additional source requests**. Five remain NULL with cached miss/retry status.
- Unit/parser/matching tests passed, including remake IDs, original/re-release years,
  ambiguous same-title/year candidates (including unpostered candidates), TMDB IDs,
  unrelated link IDs, generic/landscape rejection, single-result search redirects and
  sidebar exclusion. Rollback-only live SQL preservation/cache/upsert test passed.
- Runtime failure handling is separate from release upserts. Existing valid posters and
  last-good posters during transient network failure are preserved; stale writes are guarded.
- Poster-enabled standalone Docker service rebuilt/restarted; hourly flags and data volume
  unchanged. No TMDB/OMDb credentials were present in scraper or MyNewsFeed runtime.
- MyNewsFeed `/Movies` returned HTTP 200 with poster URLs in rendered HTML. Homepage `/`
  returned HTTP 200 with five Friday theater poster URLs. CSP does not restrict image hosts.

## Images visually verified

Each saved HTTPS URL returned HTTP 200 with a JPEG image. Downloaded without a Referer
using a browser-style User-Agent, then visually inspected.

| Movie | Original year | Visual check |
| --- | ---: | --- |
| Moana | 2026 | Live-action Dwayne Johnson/Catherine Laga’aia poster, not the 2016 animation |
| Angel and the Badman | 2026 | Zachary Levi/Meg Fraser remake poster, not the 1947 John Wayne movie |
| Disclosure Day | 2026 | Spielberg title and 2026 date on poster |
| Hokum | 2026 | Adam Scott / Damien McCarthy poster |
| A Merry Christmas Match | 2019 | Correct film title; original year preserved despite 2026 digital release |
| Matchbox The Movie | 2026 | John Cena / Apple Original poster; same image safely reused for its streaming row |

## Unmatched — left NULL

- Olmo (2025) — 2025: no unambiguous portrait poster in the inspected source results.
- The Birthday Party (2025) — 2025: no unambiguous portrait poster in the inspected source results.
- Animals (2026) — 2026: no unambiguous portrait poster in the inspected source results.
- Oasis: Don’t Look Back in Anger (2026) — 2026: no unambiguous portrait poster in the inspected source results.
- V/H/S Mixtape (2026) — 2026: no unambiguous portrait poster in the inspected source results.

When To Stream supplies landscape stills for these titles. No safe title/original-year
poster match was found in the available DVDs Release Dates search results. No placeholder,
site logo, different-year movie or guessed URL was stored.

## Enriched rows and provenance

| Movie | Original year | Poster | Source |
| --- | ---: | --- | --- |
| A Merry Christmas Match | 2019 | [Image](https://www.dvdsreleasedates.com/posters/800/A/A-Merry-Christmas-Match-2019-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12938/a-merry-christmas-match) |
| American Summer | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/A/American-Summer-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12413/american-summer) |
| Blades of the Guardians: Wind Rises in the Desert | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/B/Blades-of-the-Guardians-Wind-Rises-in-the-Desert-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12806/blades-of-the-guardians-wind-rises-in-the-desert) |
| Christmas Bells Are Ringing | 2018 | [Image](https://www.dvdsreleasedates.com/posters/800/C/Christmas-Bells-Are-Ringing-2018-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12933/christmas-bells-are-ringing) |
| Christmas Festival of Ice | 2017 | [Image](https://www.dvdsreleasedates.com/posters/800/C/Christmas-Festival-of-Ice-2017-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12934/christmas-festival-of-ice) |
| Colony | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/C/Colony-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12881/colony) |
| Dolph: Unbreakable | 2025 | [Image](https://www.dvdsreleasedates.com/posters/800/D/Dolph-Unbreakable-2025-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12940/dolph-unbreakable) |
| Fall 2: Deadpoint | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/F/Fall-2-Deadpoint-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12888/fall-2-deadpoint) |
| Hidden Strike | 2023 | [Image](https://www.dvdsreleasedates.com/posters/800/H/Hidden-Strike-2023-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/9133/hidden-strike) |
| Homegrown Christmas | 2018 | [Image](https://www.dvdsreleasedates.com/posters/800/H/Homegrown-Christmas-2018-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12935/homegrown-christmas) |
| Insidious: Out of the Further | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/I/Insidious-Out-of-the-Further-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12646/insidious-out-of-the-further) |
| Jackass: Best and Last | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/J/Jackass-Best-and-Last-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12808/jackass-best-and-last) |
| Jingle Around the Clock | 2018 | [Image](https://www.dvdsreleasedates.com/posters/800/J/Jingle-Around-the-Clock-2018-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12936/jingle-around-the-clock) |
| Late Fame | 2025 | [Image](https://www.dvdsreleasedates.com/posters/800/L/Late-Fame-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12732/late-fame) |
| Moana | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/M/Moana-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/11703/moana) |
| Nostalgic Christmas | 2019 | [Image](https://www.dvdsreleasedates.com/posters/800/N/Nostalgic-Christmas-2019-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12937/nostalgic-christmas) |
| Onslaught | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/O/Onslaught-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12889/onslaught) |
| Spider-Man: Brand New Day | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/S/Spider-Man-Brand-New-Day-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/11346/spider-man-brand-new-day) |
| The Salt Path | 2025 | [Image](https://www.dvdsreleasedates.com/posters/800/T/The-Salt-Path-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12932/the-salt-path) |
| Matchbox the Movie | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/M/Matchbox-the-Movie-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12321/matchbox-the-movie) |
| Angel and the Badman | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/A/Angel-and-the-Badman-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12916/angel-and-the-badman) |
| Avatar Aang: The Last Airbender | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/A/Avatar-Aang-The-Last-Airbender-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/11919/avatar-aang-the-last-airbender) |
| Disclosure Day (2026) | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/D/Disclosure-Day-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12635/disclosure-day) |
| Matchbox The Movie (2026) | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/M/Matchbox-the-Movie-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12321/matchbox-the-movie) |
| Other Mommy | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/O/Other-Mommy-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12846/other-mommy) |
| The Social Reckoning | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/T/The-Social-Reckoning-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12652/the-social-reckoning) |
| Hokum (2026) | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/H/Hokum-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12747/hokum) |
| The Beast | 2026 | [Image](https://www.dvdsreleasedates.com/posters/800/T/The-Beast-2026-movie-poster.jpg) | [Detail](https://www.dvdsreleasedates.com/movies/12943/the-beast) |

## Fallback options

TMDB is recommended if coverage for the unmatched titles is required. It needs a registered
API key/read-access token, exact movie matching and application attribution; non-commercial
use is free, while commercial use requires a commercial agreement. The source-only client
implemented here does not automatically activate TMDB when a key is added.
[TMDB requirements](https://developer.themoviedb.org/docs/faq),
[IMDb-ID lookup](https://developer.themoviedb.org/reference/find-by-id),
[HTTPS image URLs](https://developer.themoviedb.org/docs/image-basics).

OMDb is an alternative requiring a key. Its free tier advertises 1,000 requests/day; the
special high-resolution Poster API is patron-only. Public Poster URLs from normal metadata
responses avoid exposing an API key in browser requests.
[OMDb](https://www.omdbapi.com/), [keys/quota](https://www.omdbapi.com/apikey.aspx).

[Operation and matching guide](../docs/movie-posters.md).
Rollback image: web-scraper:before-posters-20261009; host source and pre-backfill movie
snapshot: `.deployment-backups/20261009-posters/`. No credentials are in this report.
