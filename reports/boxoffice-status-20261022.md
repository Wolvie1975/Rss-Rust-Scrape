# Weekend result status — October 22, 2026 UTC

## Change and contract

Adds three nullable status columns using a repeatable scraper-owned migration;
legacy rows remain unknown. See [the frontend read contract and provider limits](../docs/box-office-result-status.md).
The existing hourly `--boxoffice` collector remains responsible for updates all week.
Forecasts stay separate. No frontend files, credentials, certificate or schedule changes.

The observed provider signal is `chart_estimate` on an individual weekend gross
cell. Unmarked grosses remain unknown. Exact gross-cell title `Actual`/`Actuals`
is supported with simulated tests but has not been observed live. No verified
provider publication timestamp exists in the captured chart; these times remain NULL.
No live final status is claimed.

## Validation

- Full Rust suite: 162 passed, 21 ignored, zero failures.
- Focused box-office and forecast suite: eight passed.
- Twelve isolated SQL checks using a temporary table: estimates to actuals, actuals
  revisions, estimate/unknown downgrade rejection, older-weekend rejection, new-weekend
  estimates, invalid/partial payload rejection, publication-time regression and poster
  preservation. Failed refresh tests compare the complete snapshot including timestamps.
- Live migration applied twice with migration credentials in WebScraper. Production
  legacy rows retained their original values and NULL status before first collection.
- Runtime-credential SQL checks passed. Existing frontend table permissions were not
  altered; no frontend write grants were added.
- Local live dry run: ten movies for weekend October 16, zero failures.
- `git diff --check` passed. No Git commit or push.

## Deployment and rollback

Image: `web-scraper:boxoffice-status-20261022`.
Protected host backup: `/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261022-boxoffice-status/`.
Changed host source files are copied and byte-compared against the workspace.
Runtime configuration is saved with restricted permissions; it must not be published.
The additive migration is compatible with the previous image; rollback should keep
new columns and restore the prior container rather than restore the shared database.

Deployed and verified at approximately October 21, 2026, 23:35 CDT
(October 22, 04:35 UTC). Container `web-scraper` is running and Docker reports
**healthy**; the explicit health probe also passed. Deployed dry run and real
`--boxoffice` collection each succeeded with ten movies and zero failures.
Poster enrichment returned 23 cache hits and zero errors.

Live read-back: weekend October 16, 2026; one estimated row (Jailer 2,
$3,270,000), nine unknown rows, zero final rows. All StatusAsOfUtc values remain
NULL. Final collection FetchedAt is `2026-10-22T04:35:39.4754301` UTC.

The container command, environment, user, working directory, health check, entrypoint,
mounts, network mode, restart policy, resource/security limits and logging settings
were compared with the previous container and match. Container output confirms the
next normal hourly run at October 22, 00:00 CDT; that future slot was not awaited.
No second scheduler was introduced. The targeted verification invocations exit after
one collection. Rollback container:
`web-scraper-before-boxoffice-status-20261022`, image `web-scraper:period-labels-20261019`.
The host source backup includes the prior files and protected runtime configuration.
No frontend deployment was performed by this scraper task.
