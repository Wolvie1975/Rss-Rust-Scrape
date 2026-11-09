# NHL production deployment — October 14, 2026

Deployed `web-scraper:nhl-20261014`, image
`sha256:8bb3315e1a9eb0ac714f2ca425579c5f230e32df35e16351f3aa0b5feb81c645`.
The production `web-scraper` container and `mynewsfeed-web` were healthy at final
verification. No front-end deployment or SQL Server restart was performed.

## Production data

NHL schedule/results source 5637 and division standings source 13 are enabled.
ESPN season 2027 represents 2026–27.

| Data | Verified rows |
| --- | ---: |
| Current/future NHL fixtures | 1,239 |
| Final results | 170 |
| Regulation finishes | 133 |
| Overtime finishes | 23 |
| Shootout finishes | 14 |
| Current division standings teams | 32 |
| Past NHL fixture rows | 0 |
| Duplicate NHL canonical fixture keys | 0 |

The collector read 1,409 schedule observations, including completed observations
used for reconciliation. Completed games were stored as results independently of
schedule retention. NHL standings retain division order and provider statistics.

## Verification

- Full offline suite: 144 passed; 18 opt-in tests skipped.
- Formatting, strict Clippy and Cargo audit passed. Rustls was updated to 0.23.46.
- Read-only NHL live feeds and a dry run using the release image passed.
- SQL integration test passed in newly created isolated database
  `WebScraper_NHL_Validation_20261014`; the database was removed afterward.
- Targeted imports 104 and 105 succeeded. Fixtures, results and standings kept
  identical IDs and first-seen timestamps on repeat import.
- A real schedule/results import from the deployed image succeeded as run 106
  at 08:33 Central, with one source success and zero failures. Its normal retention
  cleanup succeeded. NHL IDs remained unchanged afterward.
- Existing non-NHL sports source URLs, enabled flags and limits matched the
  pre-deployment snapshot.
- Runtime environment, scheduler arguments, TLS certificate mount, data volume,
  network, restart policy, limits, security settings, logging and healthcheck
  matched the previous container.
- Startup logged the next aligned hourly collection for 09:00 Central on
  October 14. That scheduled slot was not awaited during deployment verification.

The host build inputs matched the pre-NHL source before applying the change.
Only changed code, SQL, fixtures and new NHL documentation were synchronized;
existing host documentation and credentials were preserved. No Git commits or
pushes were performed.

## Rollback

Previous image: `web-scraper:before-nhl-20261014` (also retained as
`web-scraper:cinema-posters-20261013`). Previous container is stopped and retained
as `web-scraper-before-nhl-20261014`.

Protected host backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261014-nhl/`.
It contains source/configuration and before/after verification snapshots. Keep its
permissions private because the rollback configuration includes the runtime
connection profile. No credentials are in this report.

Before returning to the old image, disable only the NHL sources so the older
collector does not attempt the new feeds:

```sql
UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ProviderKey='espn-nhl';
UPDATE dbo.SportsStandingsSources SET Enabled=0 WHERE ProviderKey='espn-nhl';
```

On the Docker host, stop the new container, rename it aside, rename the retained
old container to `web-scraper`, retag the rollback image as `web-scraper:latest`,
and start the old container. Restore changed host source from the protected backup
before rebuilding. NHL rows can remain for later reactivation.
