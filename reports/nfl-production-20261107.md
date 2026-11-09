# NFL schedule deployment — November 7, 2026

Deployed `web-scraper:nfl-20261107`, tagged as `web-scraper:latest`.
The production `web-scraper` container is healthy.

## Production data

NFL schedule source 8503 is registered and enabled in `dbo.SportsEventsType`:
- `ProviderKey`: `espn-nfl`
- `ProviderSchoolId`: `0`
- `SchoolName`: `NULL`
- `RssUrl`: `https://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard`

| Metric | Verified Value |
| --- | ---: |
| Retained upcoming NFL games | 150 |
| TBD kickoff times (Week 18 / flexed) | 24 |
| Neutral-site games (International Series / Munich) | 3 |
| Duplicate NFL canonical fixture keys | 0 |
| Undecided postseason placeholder teams | 0 |
| Pro Bowl entries | 0 |

## Verification

- **Offline test suite**: 183 passed; 23 ignored integration tests.
- **Provider unit tests**: 6 passed in `nfl::tests`.
- **Database integration test**: `nfl_configuration_reimport_failed_fetch_and_disabled_source_are_safe` passed against isolated staging.
- **Initial production collection**: Processed 164 schedule observations (14 recent reconciliation finals + 150 upcoming games) in 2 SQL batches with 0 errors.
- **Scheduler alignment**: Container logged next aligned run for `22:00 CST`.

## Rollback

- Rollback container preserved on host: `web-scraper-before-nfl-20261107`
- In SQL Server, to disable NFL collection without reverting the binary:
  ```sql
  UPDATE dbo.SportsEventsType SET Enabled = 0 WHERE ProviderKey = 'espn-nfl';
  ```
