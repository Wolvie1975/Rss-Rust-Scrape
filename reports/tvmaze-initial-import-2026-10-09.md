# TVmaze initial import verification — October 9, 2026

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

[Exact developer contract](../docs/tvmaze-developer-handoff.md),
[operation/licensing guide](../docs/tvmaze.md),
[TVmaze API](https://www.tvmaze.com/api).

The application must credit TVmaze, link SourceUrl and the CC BY-SA 4.0 license,
indicate normalized/adapted data and comply with ShareAlike for redistributed adaptations.

Rollback image: `web-scraper:before-tvmaze-20261009`; host source backup:
`.deployment-backups/20261009-tvmaze/source.tar`. Validation database retained:
`WebScraper_Tvmaze_Validation_20261009`.
