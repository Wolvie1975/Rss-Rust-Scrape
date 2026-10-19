# Movie, box-office and TV history

Historical evidence, newest dates first. The old box-office-news frontend instructions are preserved for provenance only; the October 19, 2026 news expansion supersedes the earlier disabled-source state. News and structured charts/forecasts now coexist. Use the movie and TV topic guides for current integration.

## Contents

- [Box Office news expansion — October 19, 2026](#boxoffice-news-2026-10-19)

- [Poster enrichment across movie sections](#cinema-posters-deployment-2026-10-13)
- [Boxoffice Pro domestic predictions](#boxoffice-forecasts-deployment-2026-10-13)
- [Latest weekend box-office chart](#boxoffice-chart-deployment-2026-10-13)
- [Box-office news deployment — 2026-10-12](#boxoffice-news-deployment-2026-10-12)
- [Production deployment — standings and digital releases](#standings-digital-deployment-2026-10-11)
- [Digital release coverage correction — 2026-10-11](#digital-release-coverage-2026-10-11)
- [TVmaze initial import verification — October 9, 2026](#tvmaze-initial-import-2026-10-09)
- [Movie poster verification — October 9, 2026](#movie-posters-2026-10-09)
- [U.S. movie release preview: October 5–11, 2026](#movie-preview-2026-10-05)
- [My-Newsfeed box-office news](#my-newsfeed-boxoffice-news)

<a id="boxoffice-news-2026-10-19"></a>

## Box Office news expansion — October 19, 2026

Added three enabled, dedicated box-office RSS sources to the existing category:
Variety 9034, The Hollywood Reporter 9035 and IndieWire 9036. Deadline 7094 was
already enabled and was preserved. The deployed production image passed a
three-source dry-run with 30 articles and zero failures. A subsequent production
import saved 30 articles, with three source successes and zero failures. Read-back
confirmed ten articles per new source, refreshed timestamps and no LastError.
Deadline retained its ten existing articles. Both production containers were healthy.
Existing hourly news
collection handles the additions without a rebuild or scheduler restart.
[Current source inventory and frontend contract](../docs/movies-and-box-office.md#boxoffice-news).

<a id="cinema-posters-deployment-2026-10-13"></a>

## Poster enrichment across movie sections

Added PosterUrl/PosterLookupStatus to BoxOfficeWeekend and BoxOfficeForecasts.
Shared CinemaPosterCache persists through chart/forecast replacement, using
normalized title and original year. Reuses validated release posters; actuals
use their linked The Numbers movie page and existing DVD Release Dates search.
Release backfill also tries The Numbers for verified title/year identities.
Portrait JPEG/PNG responses are validated before publishing image URLs.

Live backfills: 10/10 actual chart films, 12/12 future forecasts, 24/34 retained
release movies. One additional release poster found by the new fallback. Ten
release movies remain unmatched; ambiguous/missing original-year identities
are not guessed. Current preview dataset is empty; newly published previews
use the same lookup automatically.

111 tests passed, 16 environment-dependent tests ignored. Re-release original
year and conflicting detail identity tested. Existing image-size, dimensions,
portrait and metadata tests passed. Live missing release images were explicitly
retried; a second cinema enrichment reused all 22 cache entries with zero
additional lookup errors.

Additive schema applied with migration credentials; runtime column-update/cache
permissions granted. Successful cache entries refresh after 30 days, misses
retry after seven days, errors after six hours. Unused cache entries expire
90 days after LastUsedAtUtc. Valid cached URLs survive lookup errors for the
same movie identity. Poster failure does not erase release/chart data.

Image web-scraper:cinema-posters-20261013, previous MLS container/image retained
for rollback. Existing hourly --boxoffice/--boxoffice-forecasts performs automatic
poster attachment. No Git commit or push. Front-end rendering is a separate
change: docs/movies-and-box-office.md#movie-section-posters-frontend-handoff.

Deployed verification: two collector successes, zero failures, 798 ms total.
Poster stage reused 22 cache entries, performed zero HTTP requests and completed
in 133 ms after chart/forecast replacement. Runtime health exit 0; both scraper
and front-end healthy. Verified running image:
sha256:ed10bb9113d5e57e30abb12c2aec6a224487fb9a19b8b81a19edbb500cd09c14.

<a id="boxoffice-forecasts-deployment-2026-10-13"></a>

## Boxoffice Pro domestic predictions

The collector discovers the latest domestic weekend preview and long-range
forecast on the Forecasts & Tracking category page. It stores only factual film,
date, rank and USD range fields plus source attribution. Article prose is not
copied. U.K./Ireland predictions and general box-office stories are excluded.

Forecasts live in dbo.BoxOfficeForecasts, separate from the actual top-ten chart.
Source state is dbo.BoxOfficeForecastSources. Both snapshots replace atomically;
failed or malformed imports preserve current data. Old publications are rejected.
Expired previews and past release dates are removed by regular retention.

Live dry run and runtime-account import succeeded: 12 future-film forecasts,
zero current previews (latest article covers October 9–11), zero expired rows.
Existing actual-results chart still contains ten movies. Full unit suite:
107 passed, 16 environment-dependent tests ignored. An isolated SQL database
verified repeat imports, rollback of the first snapshot when the second snapshot
fails, and expiration. Test database removed afterward.

A SQL variable-name mismatch was found during initial verification and fixed
before rollout. The failed transaction left forecast data unchanged. The live
repeat succeeded and cleared source errors.

Image web-scraper:forecasts-20261013; hourly command --boxoffice-forecasts.
Prior scraper image/container preserved for rollback. No Git commit or push.
Front-end view implementation remains separate; see
`docs/movies-and-box-office.md#boxoffice-forecasts-frontend-handoff`.

Final deployed verification: both collectors succeeded, zero failures, 882 ms;
10 actual-results rows, 12 future forecasts, no current preview. Runtime health
check exit 0 and scraper/front-end containers healthy. Running image verified as
sha256:30e6fa092b6ebd401715c7ca54d3f5f74974f9f6a029a22cf4f24ede4a9a21f8.
The initial rollout selected an earlier image before the rebuild tag completed;
corrected image replaced it, and the superseded candidate was removed after
verification. Prior good box-office deployment remains available for rollback.

<a id="boxoffice-chart-deployment-2026-10-13"></a>

## Latest weekend box-office chart

The Numbers HTML collector validates ten movies, chart date, column structure,
movie links and descending weekend gross. Database replacement is transactional
and refuses older charts. Partial or stale responses preserve the previous chart.
Source attribution and front-end query: docs/movies-and-box-office.md#boxoffice-top-10-frontend-handoff.

Verification: 103 tests passed, 16 environment-dependent tests ignored. Live dry
run and runtime-account database import each succeeded with 10 movies for the
October 9–11 weekend. Production query confirmed ranks 1–10. Dedicated Deadline
and Variety box-office sources disabled, their saved articles removed. Hourly
command includes --boxoffice; image web-scraper:boxoffice-20261013. Prior image
preserved as a stopped rollback container. No front-end code changed.

<a id="boxoffice-news-deployment-2026-10-12"></a>

## Box-office news deployment — 2026-10-12

User selected free Deadline/Variety news coverage, including Sunday estimates and
Monday/Tuesday follow-ups. Deployed as configuration in the existing news pipeline;
no binary, schema, permission, container or web-app change was necessary.

- Category: Box Office (SourceCategories ID 5615).
- Deadline source ID 7094: https://deadline.com/v/box-office/feed/.
- Variety source ID 7095: https://variety.com/v/film/box-office/feed/.
- Both enabled, labeled and categorized; both LastError fields NULL.
- Initial import: 20 articles, ten per provider; two source successes, zero failures.
- Repeat import: 20 articles; IDs and original publication dates preserved,
  zero duplicate additions and zero old-page pruning.
- Latest ScraperRuns record succeeded with no error summary.
- Scraper and My-Newsfeed containers healthy; runtime health check passes;
  website root returns HTTP 200.
- Existing hourly --from-db collection discovers enabled sources dynamically and
  runs seven days a week. Sunday, Monday and Tuesday require no special scheduler.

These are news observations, not structured domestic/worldwide box-office totals.
No monetary fields or Estimate/Actual flags were inferred from headlines or weekdays.
Publisher RSS may expose revisions of existing stories; an update absent from the
RSS cannot be guaranteed to appear. Related box-office analysis is included.

Configuration: sql/boxoffice_news_seed.sql registers disabled feeds and preserves
operator settings on repeat setup. Explicit enabling was performed for this deploy.
Frontend/display guide: reports/entertainment-history.md#my-newsfeed-boxoffice-news. A site that hardcodes
categories must add Box Office separately; no web-app rebuild was performed here.

Verified COPY_ONLY SQL backup with CHECKSUM, permissions 600:
/var/opt/mssql/backup/WebScraper_before_boxoffice_news_20261012.bak.
Rollback is configuration-only: disable source IDs 7094/7095. Leave imported rows
for history unless cleanup is separately requested. SQL Server was not restarted;
credentials, verified TLS and existing sports/movie sources remain unchanged.

No commits or pushes performed.

<a id="standings-digital-deployment-2026-10-11"></a>

## Production deployment — standings and digital releases

Deployed 2026-10-11, approximately 08:52 Central.

<a id="standings-digital-deployment-2026-10-11--deployed"></a>

### Deployed

- Scraper image `web-scraper:standings-digital-20261011`, also tagged latest.
- Image ID: `sha256:9bca0f33d513b62609fcc56f3f8c42abbdf3cd94cedbec4880ccbc643c472f97`.
- Additive SportsStandingsSources/SportsStandings schema and restricted runtime grants.
- Eight standings sources registered and enabled; Movie Insider registered/enabled.
- Updated hourly container includes --standings-from-db alongside all existing
  collectors. Movie Insider and separate When To Stream VOD parsing use --movies.
- Frontend/configuration handoff documents synchronized to the deployed scraper checkout.

<a id="standings-digital-deployment-2026-10-11--production-verification"></a>

### Production verification

First production collection succeeded (ScraperRuns ID 11), nine collector successes,
zero failures, zero movie parser issues. All refresh metadata shows no errors.

| Standings source | Current teams | Season |
| --- | ---: | --- |
| MLB | 30 | 2026 |
| WNBA | 15 | 2026 |
| NWSL | 16 | 2026 |
| Big 12 football | 16 | 2026 |
| Big 12 men's basketball | 16 | 2025–26 |
| Big 12 women's basketball | 16 | 2025–26 |
| Big 12 soccer | 16 | 2026 |
| Big 12 volleyball | 15 | 2026 |

140 current standings rows. Movie import saved 94 release observations; production
now holds 48 movies, 48 source links, 94 release rows and 30 verified posters.
18 movies still have no matched poster; the frontend must show placeholders.
All 76 pre-deployment movie release IDs and FirstSeenAt timestamps were preserved.

For the current October 5–11 week: Movie Insider has 10 digital observations,
When To Stream has 5, DVDs Release Dates has 16. Providers overlap; these counts
are not unique movie totals. The frontend handoff recommends explicit source
selection with Movie Insider preferred for the Digital/VOD view.

All 19 schedule feed configurations remain unchanged. SportsEvents remains 981
rows, zero duplicate canonical keys and zero 2027 MLB games. The 2026 MLB
restriction and separate My-Newsfeed NWSL results sync remain intact.

Runtime health check passes; scraper and My-Newsfeed containers are healthy.
/events and /movies return HTTP 200. Runtime account can insert standings, cannot
CREATE TABLE or DELETE standings, and retains verified SQL TLS.
SQL Server was not restarted for this deployment.

Pre-deploy verification: 113 offline tests, Clippy with warnings denied, formatting
checks and isolated digital SQL validation passed; standings SQL/live validation
had already passed before deployment.

<a id="standings-digital-deployment-2026-10-11--frontend-boundary"></a>

### Frontend boundary

Only scraper implementation, database/configuration and handoff documents changed
in this repository. The My-Newsfeed web application was not rebuilt. Its new
/standings route still returns 404. The existing /movies page remains available;
preferred digital source selection is a separate frontend change. Data is ready;
the frontend builder still needs to apply:

- docs/sports-tables.md#my-newsfeed-standings-frontend
- docs/movies-and-box-office.md#my-newsfeed-digital-releases

<a id="standings-digital-deployment-2026-10-11--backup-and-rollback"></a>

### Backup and rollback

Verified COPY_ONLY SQL backup with CHECKSUM:
`/var/opt/mssql/backup/WebScraper_before_standings_digital_20261011.bak` in sql2025's
persistent SQL storage (600 permissions).

Protected host source/configuration backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261011-standings-digital/`.
Includes source-before.tar and compose-before.yml. Runtime credentials and CA mount
were preserved during source synchronization.

Rollback image: `web-scraper:before-today-standings-digital-20261011`.
Restore the prior compose command together with that image: the previous binary
must not receive --standings-from-db. Additive tables can remain during a binary
rollback; restore the database only for a separately justified data recovery.

No commits or pushes performed.

<a id="digital-release-coverage-2026-10-11"></a>

## Digital release coverage correction — 2026-10-11

The user selected the current Central Monday–Sunday week with improved VOD/digital
coverage, using Movie Insider's 2026 digital list as the reference.

Read-only production audit found 33 movies/33 source links/76 release records.
The existing digital source was DVDs Release Dates; its coverage included older
catalog/holiday films that did not appear in the reference week's Movie Insider
list. This difference alone does not establish that those availability dates are
wrong. The When To Stream parser had a definite gap: it ignored explicit VOD dates.

Implemented:

- Movie Insider digital provider using the accessible public VOD calendar, all
  pagination, current-week candidate selection and explicit detail-page digital
  dates. Calendar cards can be out of order; no date-based early stopping.
- Numeric movie IDs and digital:first release keys preserve identity across date
  and slug changes. Announced digital dates stay separate from theaters/discs/SVOD.
- Existing poster validation accepts the movie's own labeled poster, with source
  identity checks. No original-year or external-ID guesses from release dates.
- --configure-movieinsider / sql/movieinsider_seed.sql register disabled, preserving
  existing configuration. Existing --movies imports enabled providers.
- When To Stream explicit VOD dates now import as digital, alongside existing
  separate subscription dates and services.
- Source-specific preview flags, source error tracking and frontend handoff.

Live preview for October 5–11:

| Provider | Digital observations | Subscription observations | Issues |
| --- | ---: | ---: | ---: |
| Movie Insider | 10 | 0 | 0 |
| When To Stream | 5 | 9 | 0 |

These are provider observations, with overlap; they must not be summed as unique
movies. Both previews were read-only. Existing production rows were retained.

Validation: 113 offline tests pass; isolated SQL test checks disabled/enabled
settings, repeat imports, reschedules, changed slugs and preserved IDs/FirstSeenAt.
Clippy with warnings denied and formatting checks pass.

Production deployment, source enabling and My-Newsfeed frontend changes remain
pending. No new movie schema columns/tables or runtime privileges are required.
The frontend handoff recommends Movie Insider as the preferred digital source,
with explicit provider selection rather than silently combining catalog and
premiere observations. See docs/movies-and-box-office.md#my-newsfeed-digital-releases.

<a id="tvmaze-initial-import-2026-10-09"></a>

## TVmaze initial import verification — October 9, 2026

Migration and exact definitions were provided in chat and delivered to the MyNewsFeed
checkout **before applying production schema changes**:
`db/tvmaze.sql`, `db/tvmaze_seed.sql`, `deployment/tvmaze-developer-handoff.md`.

Production import: **5 series, 90 unique episodes, 5 tracking settings**.
No collection errors. All tracking rows have last-attempt/success timestamps and
correct episode counts. A deployed cache rerun issued no HTTP collections and
left series/episodes/tracking rows unchanged.

| Series | TVmaze ID | Episodes | Date-only | Specials |
| --- | ---: | ---: | ---: | ---: |
| Avatar: Seven Havens | 83073 | 13 | 13 | 0 |
| VisionQuest | 64950 | 8 | 8 | 0 |
| Slow Horses | 45039 | 36 | 36 | 0 |
| The Lord of the Rings: The Rings of Power | 33352 | 26 | 26 | 2 |
| Line of Fire | 90632 | 7 | 0 | 0 |

- All 83 blank-airtime episodes have NULL Airtime/StartsAtUtc and IsDateOnly=1,
  while retaining raw noon airstamps exactly. The 7 NBC episodes have explicit
  local airtime and meaningful normalized UTC timestamps.
- Avatar's first date retains three distinct episodes. Same-day batches for the
  other shows and both Rings of Power specials are preserved by episode ID.
- Line of Fire is provider ID 90632, NBC, premiered 2026-09-21, IMDb tt42496001;
  no 2003 title match or Peacock release dates were introduced.
- Staging migration/seed reruns and full repeated imports preserved keys/counts.
  SQL tests verified reschedules without duplicate IDs/FirstSeenAt changes,
  disabled tracking preserved through reseeding, atomic rejection of incomplete
  lists, retained last-good history/status and another show's success after failure.
- Mock HTTP tests cover 429 Retry-After, 503 retry and failed-request isolation.
- Source gaps: 30 runtimes, 36 images, 8 TBA titles and 15 generic “Episode N” titles.
  Dates/titles are provider-reported; blank airtime is not a precise release time.
- Requests use explicit show IDs and full `/episodes?specials=1` lists; validated
  snapshots are cached for one hour. No regional streaming availability is inferred.
- The standalone hourly service now includes `--tv-from-db` alongside existing
  news/sports/movie/YouTube flags. The MyNewsFeed UI was not modified for TV tracking.

[Exact developer contract](../docs/tv.md#tvmaze-developer-handoff),
[operation/licensing guide](../docs/tv.md#tvmaze),
[TVmaze API](https://www.tvmaze.com/api).

The application must credit TVmaze, link SourceUrl and the CC BY-SA 4.0 license,
indicate normalized/adapted data and comply with ShareAlike for redistributed adaptations.

Rollback image: `web-scraper:before-tvmaze-20261009`; host source backup:
`.deployment-backups/20261009-tvmaze/source.tar`. Validation database retained:
`WebScraper_Tvmaze_Validation_20261009`.

<a id="movie-posters-2026-10-09"></a>

## Movie poster verification — October 9, 2026

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

<a id="movie-posters-2026-10-09--images-visually-verified"></a>

### Images visually verified

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

<a id="movie-posters-2026-10-09--unmatched--left-null"></a>

### Unmatched — left NULL

- Olmo (2025) — 2025: no unambiguous portrait poster in the inspected source results.
- The Birthday Party (2025) — 2025: no unambiguous portrait poster in the inspected source results.
- Animals (2026) — 2026: no unambiguous portrait poster in the inspected source results.
- Oasis: Don’t Look Back in Anger (2026) — 2026: no unambiguous portrait poster in the inspected source results.
- V/H/S Mixtape (2026) — 2026: no unambiguous portrait poster in the inspected source results.

When To Stream supplies landscape stills for these titles. No safe title/original-year
poster match was found in the available DVDs Release Dates search results. No placeholder,
site logo, different-year movie or guessed URL was stored.

<a id="movie-posters-2026-10-09--enriched-rows-and-provenance"></a>

### Enriched rows and provenance

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

<a id="movie-posters-2026-10-09--fallback-options"></a>

### Fallback options

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

[Operation and matching guide](../docs/movies-and-box-office.md#movie-posters).
Rollback image: web-scraper:before-posters-20261009; host source and pre-backfill movie
snapshot: `.deployment-backups/20261009-posters/`. No credentials are in this report.

<a id="movie-preview-2026-10-05"></a>

## U.S. movie release preview: October 5–11, 2026

Live preview from DVDs Release Dates and When To Stream. No database writes.

Dates are source-reported, not independently verified. Disc editions and streaming services are separate release records.

| Category | Titles | Release records |
| --- | ---: | ---: |
| theatrical | 5 | 5 |
| digital | 16 | 16 |
| subscription | 8 | 9 |
| disc | 4 | 9 |

Checked 56 unique detail pages; 0 HTTP/parser issues; excluded 4 apparent TV series.

| Date | Movie | Category | Format / Service | Source |
| --- | --- | --- | --- | --- |
| 2026-10-05 | Olmo (2025) | subscription | Hulu | [Source](https://whentostream.com/olmo-2025/) |
| 2026-10-06 | A Merry Christmas Match | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12938/a-merry-christmas-match) |
| 2026-10-06 | American Summer | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12413/american-summer) |
| 2026-10-06 | Blades of the Guardians: Wind Rises in the Desert | disc | DVD | [Source](https://www.dvdsreleasedates.com/movies/12806/blades-of-the-guardians-wind-rises-in-the-desert) |
| 2026-10-06 | Blades of the Guardians: Wind Rises in the Desert | disc | 4K UHD + Blu-ray | [Source](https://www.dvdsreleasedates.com/movies/12806/blades-of-the-guardians-wind-rises-in-the-desert) |
| 2026-10-06 | Christmas Bells Are Ringing | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12933/christmas-bells-are-ringing) |
| 2026-10-06 | Christmas Festival of Ice | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12934/christmas-festival-of-ice) |
| 2026-10-06 | Colony | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12881/colony) |
| 2026-10-06 | Dolph: Unbreakable | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12940/dolph-unbreakable) |
| 2026-10-06 | Fall 2: Deadpoint | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12888/fall-2-deadpoint) |
| 2026-10-06 | Hidden Strike | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/9133/hidden-strike) |
| 2026-10-06 | Homegrown Christmas | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12935/homegrown-christmas) |
| 2026-10-06 | Insidious: Out of the Further | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12646/insidious-out-of-the-further) |
| 2026-10-06 | Jackass: Best and Last | disc | DVD | [Source](https://www.dvdsreleasedates.com/movies/12808/jackass-best-and-last) |
| 2026-10-06 | Jackass: Best and Last | disc | Blu-ray | [Source](https://www.dvdsreleasedates.com/movies/12808/jackass-best-and-last) |
| 2026-10-06 | Jingle Around the Clock | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12936/jingle-around-the-clock) |
| 2026-10-06 | Late Fame | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12732/late-fame) |
| 2026-10-06 | Late Fame | disc | DVD | [Source](https://www.dvdsreleasedates.com/movies/12732/late-fame) |
| 2026-10-06 | Late Fame | disc | Blu-ray | [Source](https://www.dvdsreleasedates.com/movies/12732/late-fame) |
| 2026-10-06 | Moana | disc | DVD | [Source](https://www.dvdsreleasedates.com/movies/11703/moana) |
| 2026-10-06 | Moana | disc | DVD + Digital | [Source](https://www.dvdsreleasedates.com/movies/11703/moana) |
| 2026-10-06 | Moana | disc | 4K UHD + Blu-ray + Digital | [Source](https://www.dvdsreleasedates.com/movies/11703/moana) |
| 2026-10-06 | Nostalgic Christmas | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12937/nostalgic-christmas) |
| 2026-10-06 | Onslaught | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12889/onslaught) |
| 2026-10-06 | Spider-Man: Brand New Day | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/11346/spider-man-brand-new-day) |
| 2026-10-06 | The Salt Path | digital | Digital HD | [Source](https://www.dvdsreleasedates.com/movies/12932/the-salt-path) |
| 2026-10-08 | Matchbox the Movie | theatrical |  | [Source](https://www.dvdsreleasedates.com/movies/12321/matchbox-the-movie) |
| 2026-10-08 | The Birthday Party (2025) | subscription | Lionsgate+ on Prime Video | [Source](https://whentostream.com/the-birthday-party-2025/) |
| 2026-10-09 | Angel and the Badman | theatrical |  | [Source](https://www.dvdsreleasedates.com/movies/12916/angel-and-the-badman) |
| 2026-10-09 | Animals (2026) | subscription | Netflix | [Source](https://whentostream.com/animals-2026/) |
| 2026-10-09 | Avatar Aang: The Last Airbender | theatrical |  | [Source](https://www.dvdsreleasedates.com/movies/11919/avatar-aang-the-last-airbender) |
| 2026-10-09 | Disclosure Day (2026) | subscription | Peacock | [Source](https://whentostream.com/disclosure-day-2026/) |
| 2026-10-09 | Matchbox The Movie (2026) | subscription | Apple TV | [Source](https://whentostream.com/matchbox-the-movie-2026/) |
| 2026-10-09 | Oasis: Don’t Look Back in Anger (2026) | subscription | Disney+ | [Source](https://whentostream.com/dont-look-back-in-anger-2026/) |
| 2026-10-09 | Oasis: Don’t Look Back in Anger (2026) | subscription | Hulu | [Source](https://whentostream.com/dont-look-back-in-anger-2026/) |
| 2026-10-09 | Other Mommy | theatrical |  | [Source](https://www.dvdsreleasedates.com/movies/12846/other-mommy) |
| 2026-10-09 | The Social Reckoning | theatrical |  | [Source](https://www.dvdsreleasedates.com/movies/12652/the-social-reckoning) |
| 2026-10-09 | V/H/S Mixtape (2026) | subscription | Shudder | [Source](https://whentostream.com/v-h-s-mixtape-2026/) |
| 2026-10-10 | Hokum (2026) | subscription | Hulu | [Source](https://whentostream.com/hokum-2026/) |

<a id="movie-preview-2026-10-05--excluded-television-titles"></a>

### Excluded television titles

- excluded apparent TV title/season edition: The Madison ([source](https://www.dvdsreleasedates.com/movies/12847/the-madison))
- excluded apparent TV title/season edition: Chicago Fire ([source](https://www.dvdsreleasedates.com/movies/6458/Chicago-Fire-TV-Series-2012.html))
- excluded apparent TV title/season edition: Chicago P.D. ([source](https://www.dvdsreleasedates.com/movies/6933/chicago-p.d.))
- excluded apparent TV title/season edition: Chicago Med ([source](https://www.dvdsreleasedates.com/movies/8037/chicago-med))

<a id="movie-preview-2026-10-05--candidates-without-a-matching-announced-date"></a>

### Candidates without a matching announced date

These include adjacent theatrical weeks and VOD-only entries without an announced subscription date.

- [https://whentostream.com/black-zombie-2026/](https://whentostream.com/black-zombie-2026/) — subscription
- [https://whentostream.com/colony-2026/](https://whentostream.com/colony-2026/) — subscription
- [https://whentostream.com/fall-2-deadpoint-2026/](https://whentostream.com/fall-2-deadpoint-2026/) — subscription
- [https://whentostream.com/insidious-out-of-the-further-2026/](https://whentostream.com/insidious-out-of-the-further-2026/) — subscription
- [https://whentostream.com/late-fame-2025/](https://whentostream.com/late-fame-2025/) — subscription
- [https://whentostream.com/onslaught-2026/](https://whentostream.com/onslaught-2026/) — subscription
- [https://whentostream.com/parasomnia-2025/](https://whentostream.com/parasomnia-2025/) — subscription
- [https://whentostream.com/spider-man-brand-new-day-2026/](https://whentostream.com/spider-man-brand-new-day-2026/) — subscription
- [https://whentostream.com/the-librarians-2025/](https://whentostream.com/the-librarians-2025/) — subscription
- [https://whentostream.com/the-salt-path-2024/](https://whentostream.com/the-salt-path-2024/) — subscription
- [https://www.dvdsreleasedates.com/movies/12158/sacrifice](https://www.dvdsreleasedates.com/movies/12158/sacrifice) — theatrical
- [https://www.dvdsreleasedates.com/movies/12309/verity](https://www.dvdsreleasedates.com/movies/12309/verity) — theatrical
- [https://www.dvdsreleasedates.com/movies/12597/street-fighter](https://www.dvdsreleasedates.com/movies/12597/street-fighter) — theatrical
- [https://www.dvdsreleasedates.com/movies/12650/digger](https://www.dvdsreleasedates.com/movies/12650/digger) — theatrical
- [https://www.dvdsreleasedates.com/movies/12827/whalefall](https://www.dvdsreleasedates.com/movies/12827/whalefall) — theatrical
- [https://www.dvdsreleasedates.com/movies/12913/rolling-loud-the-movie](https://www.dvdsreleasedates.com/movies/12913/rolling-loud-the-movie) — theatrical
- [https://www.dvdsreleasedates.com/movies/12923/your-mother-your-mother-your-mother](https://www.dvdsreleasedates.com/movies/12923/your-mother-your-mother-your-mother) — theatrical
- [https://www.dvdsreleasedates.com/movies/12928/trust-me,-im-a-doctor](https://www.dvdsreleasedates.com/movies/12928/trust-me,-im-a-doctor) — theatrical
- [https://www.dvdsreleasedates.com/movies/12929/you-can-see-everything](https://www.dvdsreleasedates.com/movies/12929/you-can-see-everything) — theatrical
- [https://www.dvdsreleasedates.com/movies/12930/beware-boi%C3%BAna](https://www.dvdsreleasedates.com/movies/12930/beware-boi%C3%BAna) — theatrical

<a id="my-newsfeed-boxoffice-news"></a>

## My-Newsfeed box-office news

Retired integration. Do not enable these sources or implement the article cards; use the structured charts and forecasts in the movie guide.

The user chose free Deadline/Variety **news feeds**, including Sunday estimates
and Monday/Tuesday follow-ups. This uses the existing Sources/SourceCategories/
Pages news pipeline, not structured box-office tables or a licensed API.

Feeds:

- https://deadline.com/v/box-office/feed/
- https://variety.com/v/film/box-office/feed/

Both were verified as active RSS on 2026-10-12. Deadline's /tag/box-office/feed/
was sparsely updated, while the selected /v/box-office/feed/ is current. Avoid
Box Office Mojo's old news feed (latest entries January 2023) and The Numbers'
removed /rss-feed endpoint.

<a id="my-newsfeed-boxoffice-news--display"></a>

### Display

Expose the **Box Office** category through the existing news category filter.
Show headlines, bounded excerpts, images when supplied, publication dates in
Central time, provider labels and links to the original articles. Use ordinary
HTML encoding/sanitization and the site's established safe link/image behavior.

Coverage includes domestic, overseas-market and worldwide reporting, plus related
box-office analysis. The publishers choose their article dates and can update an
existing article. Do not claim every article contains both domestic and worldwide
figures or that every Sunday/Monday/Tuesday guarantees a new story.

These are news articles. Do not infer an Estimate/Actual status from weekday,
create dollar totals by parsing headlines, or combine weekend/cumulative figures.
If adding a status badge later, require explicit verified article evidence; a
Sunday article can already contain some actual figures and a Monday one can
still use estimates. Publisher attribution remains visible.

<a id="my-newsfeed-boxoffice-news--collection-and-configuration"></a>

### Collection and configuration

Existing hourly --from-db collection runs all seven days, including Sundays,
Mondays and Tuesdays; no extra schedule flag or container restart is required.
The deployed command caps each source at its ten newest articles per collection.
Existing URL-based upserts update repeated articles rather than duplicating them.
Published dates retain the original earliest known publication time; LastScrapedAt
means last checked, not necessarily when a publisher revised an article.
An article correction must be present in the publisher's RSS to be observed.

`sql/boxoffice_news_seed.sql` creates the Box Office category if missing and
registers the two feeds **disabled**, preserving existing settings on repeat setup.
Apply with migration/admin credentials, then enable explicitly:

```sql
UPDATE dbo.Sources SET Enabled=1
WHERE Url IN (
 'https://deadline.com/v/box-office/feed/',
 'https://variety.com/v/film/box-office/feed/'
);
```

No schema, credentials, runtime grants, sports/movie imports or web-app rebuild
are required for ingestion. A database-driven news selector can expose the new
category automatically; frontend changes are separate if it hardcodes categories.
Read-only preview:

```bash
web_scraper --dry-run --per-source 10 https://deadline.com/v/box-office/feed/ https://variety.com/v/film/box-office/feed/ -o /tmp/boxoffice-preview.xml
```

Import with runtime credentials using the same URLs and --db, omitting --dry-run.
Use a temporary output file for targeted imports so the main combined RSS remains
intact. The normal hourly run updates the combined feed with all enabled sources.

<a id="my-newsfeed-boxoffice-news--verification"></a>

### Verification

Check both source records are enabled, categorized as Box Office, and have NULL
LastError after collection. Confirm publication dates, provider URLs, excerpts and
repeat-import identity with the existing news importer. No historical/news rows
should be deleted as part of this configuration change.

<a id="my-newsfeed-boxoffice-news--frontend-build-handoff"></a>

### Frontend build handoff

<a id="my-newsfeed-boxoffice-news--request-to-give-the-my-newsfeed-builder"></a>

#### Request to give the My-Newsfeed builder

“Integrate the deployed Box Office news category into the existing news page and
category selector. Reuse the site's data-access, article-card, authorization and
pagination patterns. Read existing WebScraper news tables; do not fetch publisher
RSS from the browser. Display Deadline/Variety headlines, excerpts, publication
dates, attribution and article links. Preserve category/source selection in URLs,
handle unavailable images and refresh failures, and do not present the articles
as structured box-office totals. Follow the queries and acceptance checks below.”

<a id="my-newsfeed-boxoffice-news--production-status-and-identity"></a>

#### Production status and identity

Deployed 2026-10-12: category ID 5615, Deadline source ID 7094, Variety source ID
7095. Initial collection imported 20 articles, with stable IDs on repeat import.
These IDs identify this deployment; discover categories/sources from the database
rather than hardcoding those numbers in application logic.

The frontend needs SELECT on dbo.SourceCategories, dbo.Sources and dbo.Pages,
using its existing server-side database connection. No new frontend table or movie
model is needed. Credentials stay on the server; no ingestion/admin privileges
are required for rendering.

<a id="my-newsfeed-boxoffice-news--category-and-source-selectors"></a>

#### Category and source selectors

Populate news categories from enabled sources, so a newly registered category
appears without a hardcoded category enum:

```sql
SELECT c.ID, c.Category_Name
FROM dbo.SourceCategories c
WHERE EXISTS (
 SELECT 1 FROM dbo.Sources s WHERE s.SourceCategoryId=c.ID AND s.Enabled=1
)
ORDER BY c.Category_Name, c.ID;
```

For an optional provider selector, retain enabled sources even when no article
has been collected yet:

```sql
SELECT ID, Label, Url, LastScrapedAt, LastError
FROM dbo.Sources
WHERE Enabled=1 AND SourceCategoryId=@CategoryId
ORDER BY Label, ID;
```

Reuse existing category/source query parameters. Validate a selected source belongs
to the selected category and is enabled. Resolve Box Office by its category name
for a dedicated navigation link if needed; use the resolved ID for parameterized
queries. Do not accept disabled source IDs to reveal their articles.

<a id="my-newsfeed-boxoffice-news--article-query"></a>

#### Article query

Use the site's existing page size, with a bounded positive @PageSize and nonnegative
@Offset. Bind values as SQL parameters, including the nullable @SourceId:

```sql
SELECT p.ID AS ArticleId, p.Title, p.Description, p.Url AS ArticleUrl,
       p.ImageUrl, p.Published, p.ScrapedAt,
       s.ID AS SourceId, s.Label AS SourceLabel, s.Url AS FeedUrl,
       s.LastScrapedAt, s.LastError,
       c.ID AS CategoryId, c.Category_Name AS CategoryName
FROM dbo.Pages p
JOIN dbo.Sources s ON s.ID=p.SourceId
JOIN dbo.SourceCategories c ON c.ID=s.SourceCategoryId
WHERE s.Enabled=1
  AND c.ID=@CategoryId
  AND (@SourceId IS NULL OR s.ID=@SourceId)
ORDER BY COALESCE(p.Published,p.ScrapedAt) DESC, p.ID DESC
OFFSET @Offset ROWS FETCH NEXT @PageSize ROWS ONLY;
```

Use the same filters for a separate COUNT query if the existing paginator needs a
total. Do not join to Movies or derive money values from movie-release records.
If adding date filters, compute Central calendar boundaries, convert them to UTC
on the server, and use an inclusive lower/exclusive upper timestamp bound. No
Sunday-only, Monday-only, current-week or sports today-forward restriction is
needed for the default news category.

Map ArticleId/SourceId/CategoryId to .NET int; Published and LastScrapedAt to
DateTime?; Description/ImageUrl/SourceLabel/LastError are nullable strings.
ScrapedAt is nonnullable, but represents initial storage time and must not be
advertised as an article revision timestamp.

<a id="my-newsfeed-boxoffice-news--article-card-and-timestamps"></a>

#### Article card and timestamps

Reuse the existing news card. Show headline, short excerpt when available, image
when available, provider label, publication date and “Read at Deadline/Variety”.
Link to ArticleUrl, not FeedUrl. If SourceLabel is absent, show the validated
article/provider hostname rather than an empty attribution.

All stored DATETIME2 values represent UTC. Explicitly mark them UTC in .NET before
using the site's Central-time conversion; handle CST/CDT using the timezone helper.
Published is the publisher's earliest known article publication, not the scraper's
refresh time. If NULL, display “Publication date unavailable”; ScrapedAt may be
shown separately as “First collected”, not substituted as a claimed publication.

The collector already bounds excerpts. Render Description as encoded text unless
the existing component safely sanitizes it; do not use raw HTML rendering on feed
content. Only accept HTTP(S) article/image URLs. Missing/failed images should
leave a readable text card. Follow existing external-link conventions and use
noopener noreferrer for links opening a new tab.

<a id="my-newsfeed-boxoffice-news--loading-empty-and-refresh-states"></a>

#### Loading, empty and refresh states

| Condition | Display behavior |
| --- | --- |
| Articles available | Render newest first with stable ArticleId keys. |
| Enabled sources, no articles | “No box-office stories available yet.” Keep selectors usable. |
| No enabled sources | “No box-office news sources are enabled.” |
| LastError set for a source | Keep previously stored articles; show a generic refresh message. |
| LastScrapedAt older than two hours | Indicate collection may be delayed; do not delete articles. |
| Database request fails | Show a retryable availability message; log details server-side. |
| Client request loading, if applicable | Use a loading state rather than briefly showing an empty list. |

LastScrapedAt records an attempt; it is not a separate last-success field. Combine
it with LastError when determining collection health. Public pages must not expose
raw LastError, connection details or SQL exceptions. Lack of a new Sunday or
Monday story alone is not a collection failure: publishers control their coverage.

Keep filters, page selection and links keyboard-accessible. Use semantic headlines
and source/date text, responsive cards, and consistent image dimensions. Any page
cache should include category/source/page in its key and be short relative to the
hourly collector (for example five minutes).

<a id="my-newsfeed-boxoffice-news--frontend-acceptance-checks"></a>

#### Frontend acceptance checks

- Box Office is selectable through the existing news category filter.
- Both enabled publishers appear, with optional provider filtering and URL state.
- The deployed category initially has 20 article records; do not hardcode that count.
- Article links open publisher stories, not RSS endpoints.
- Cards retain attribution, correct Central publication dates and readable text
  when descriptions/images/publication dates are missing.
- Repeated ingestion updates existing ArticleId cards rather than duplicating them.
- Sunday estimates and Monday/Tuesday follow-ups remain ordinary attributed news;
  no weekday-derived status badges or headline-derived dollar totals appear.
- Disabled sources, empty/loading/error/stale states, mobile layout and keyboard
  interaction behave correctly.
- Other news categories, movie releases and sports pages retain their behavior.

Use existing frontend tests for filtering, pagination and card rendering; use
stored/fixture articles rather than live publisher requests. Deployment evidence:
[box-office deployment report](entertainment-history.md#boxoffice-news-deployment-2026-10-12).

Superseded by `docs/movies-and-box-office.md#boxoffice-top-10-frontend-handoff` on October 13, 2026.
The dedicated Deadline/Variety box-office sources are disabled and their saved
articles removed after the first successful top-ten chart import.

