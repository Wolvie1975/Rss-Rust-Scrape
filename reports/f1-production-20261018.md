# F1 production deployment — October 18, 2026

Deployed `web-scraper:f1-20261018`, image
`sha256:77cdfaa0fb66812cd4623eb81d92505329ee1aa906686232f749fdbf1ae5a9c6`.
The user explicitly authorized deployment after the initial F1 data import.
The scraper and frontend containers were running and healthy after verification.
No frontend application deployment or SQL Server restart was performed by this work.

## Verification

- Host Cargo files, Dockerfile and existing source/SQL matched the workspace
  baseline. Only the expected F1 changes to main.rs/db.rs differed from HEAD.
- Saved a protected host source/configuration backup and the prior container
  inspection before synchronizing the F1 files. Credentials were not printed.
- Release Docker image built with `cargo build --locked --release`.
- Strict Clippy (`--all-targets -- -D warnings`) passed. Initial implementation
  validation: 139 tests passed, 21 opt-in tests ignored; separate F1 SQL integration
  test passed in its isolated database.
- Release image F1 dry-run passed against production feeds using the existing
  runtime environment, TLS mount, volume and network. No rows were written.
- Replaced the idle hourly container, retained its predecessor stopped for
  rollback, and successfully ran the deployed image's database health check.
- Targeted deployed-container F1 refresh **1206** succeeded with one source
  success and zero failures. Snapshot FetchedAt advanced to
  `2026-10-18T05:18:51.9111955` UTC; source LastError was absent.
- Existing frontend production database credentials successfully read the
  committed F1 views: 23 races, results for 17 Grands Prix and six sprints,
  23 drivers and 11 constructors. Season 2026, standings through round 17.
- No frontend rendering was implemented or verified. The frontend repository
  can use the [integration handoff](../docs/f1-frontend-integration.md).

Compared the new container to the pre-deployment inspection. Environment,
command, entrypoint, user, working directory, healthcheck, mounts, memory/CPU/PID
limits, dropped capabilities, security options, read-only root filesystem, tmpfs,
restart policy, logging, network and aliases were preserved. The release image
and its image metadata label changed. `web-scraper:latest` points to the release.

F1 is collected by the existing `--events-from-db` path, so hourly arguments
were not changed. Startup logged the next aligned run as **October 18, 1:00 a.m.
Central**. That future scheduled run was not awaited; validation used an explicit
F1 refresh inside the deployed container. The pre-deployment hourly log had an
unrelated YouTube channel 404; no YouTube configuration was changed.

## Rollback

Previous image retained as `web-scraper:before-f1-20261018` (the former
`web-scraper:nwsl-league-20261016` image). Previous container retained stopped as
`web-scraper-before-f1-20261018`.

Protected host backup directory:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261018-f1/`.
It includes source/configuration and before/after container inspections. Keep it
private: inspection/configuration artifacts include runtime credentials.

Disable only the F1 source before returning to the previous collector:

```sql
UPDATE dbo.F1Source SET Enabled=0 WHERE ID=1;
```

Stop and rename the F1 container aside, rename the retained old container to
`web-scraper`, restore its image as `web-scraper:latest`, and start it. Restore
host build inputs from the protected backup before rebuilding an older version.
The additive F1 tables/views and last snapshot can remain. Other sports data,
source configuration and existing scheduler settings should be preserved.

No Git commits or pushes were performed.

## F1 news configuration deployment

Later on October 18, added four enabled RSS feeds in **F1 News** using
`sql/f1_news_seed.sql`: BBC Sport 8965, Autosport 8966, Motorsport.com 8967,
and The Guardian 8968. No matching F1 feed URLs were previously configured.
The existing production image passed a four-source dry-run and an actual
news import: 40 saved articles, four source successes, zero failures.
Read-back confirmed ten rows per source, refreshed LastScrapedAt values and
no source LastError. The frontend production database identity read all 40
articles, and both production containers were healthy. Existing publisher/category
assignments were preserved.

Some optional article identity lookups received 403 or unusable BBC iPlayer
metadata; original RSS entries were still ingested. No source bypass was added.
No image rebuild or container/scheduler restart was needed for this registration;
the hourly `--from-db` collector discovers the enabled rows automatically.
The [news frontend handoff](../docs/f1-news-frontend-integration.md) supplies IDs,
URLs, category query and UI acceptance checks. This does not implement the
frontend's News tab.
