# Optional sports box scores — deployment October 19, 2026

## Outcome

Deployed **web-scraper:boxscores-20261019** at approximately **10:23 AM CDT**
(15:23 UTC). Image:
`sha256:b94880c203af826652bbe685df790f034c192bb62a26746252cf79625285f64f`.
The previous image/container are retained for rollback. The frontend image was
not changed; [frontend integration remains](<../New Development/sports-boxscores-frontend-handoff.md>).

The backend now fetches optional ESPN game detail for completed records already
stored in `ResultGames` and enabled NHL/MLS/Championship `SportsMatchResults`.
It writes `SportsBoxScoreCache`; `SportsBoxScores` provides the frontend read
contract. Both result models keep their existing identities and score ownership.
F1 remains on its separate classifications model.

The existing hourly `--events-from-db` collector includes enrichment without new
scheduler arguments. It checks up to 100 eligible identities or 180 seconds per
run, with a 12-second HTTP timeout and 250 ms between games. An explicit
`--boxscores-from-db --boxscores-limit N` run supports a bounded backfill (maximum
N=500; the same time budget still applies). Candidates deduplicate by
Sport/League/EventId, prioritize recent games and then unchecked history.

Usable team/player statistics are normalized into JSON version 1. A box-score
link or empty team/player shell alone does not qualify as usable detail. Available
team-only soccer stats do qualify; the payload flags distinguish them from player
box scores. Missing data retains the existing result fallback. Fetch, parse or
identity-validation failures record errors without replacing last-good statistics.
A successful snapshot becomes eligible for another check after six hours; empty
results retry after 24 hours and errors after six hours. Queue/request limits can
delay those checks beyond their eligibility time. Runtime performs no DDL.

## Validation

- `cargo test --locked`: **145 passed, 21 opt-in tests ignored**.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- Six focused parser tests also passed after reducing the real-response fixtures.
  Coverage includes NFL, nested MLB stats, team-only NWSL, empty volleyball shells,
  wrong game/team identities, nonfinal games, shifted player columns, unsafe or
  wrong-event links, missing values and legitimate zeroes.
- An isolated SQL database tested repeated schema/view setup, a runtime-role user,
  final/enabled-source/retry candidate filtering, idempotent cache keys, unavailable
  and error states, next-attempt scheduling and last-good data/link/FetchedAt
  preservation after error and empty refresh. The test database was removed.
- Local and release-image live dry-runs each checked 30 stored games: **26 with
  detail, four without detail, zero errors**. The release image used the production
  runtime profile, TLS/network settings and mounts.
- Production migration added only the optional cache/view and necessary ingest
  read/write grants. The actual frontend database identity successfully selected
  the new view and its populated JSON. No new provider credentials were needed.
- Deployed `--health-check` passed and configuration comparison passed. Both
  scraper and frontend containers were healthy during backfill verification.

## Production backfill

Initial inventory contained **1,355 distinct completed game identities** across
both result models. Bounded deployed-container imports populated the optional
cache; final counts and run IDs are recorded below.

Backfill completed at **10:33:06 AM CDT** (15:33:06 UTC). Read-back verified
**1,355 cached identities, zero unchecked games, zero error states**:

| League | Available stats | Unavailable / fallback | Player box scores |
| --- | --- | --- | --- |
| MLB | 34 | 0 | 34 |
| WNBA | 7 | 0 | 7 |
| College football | 75 | 0 | 75 |
| NFL | 45 | 0 | 45 |
| NHL | 201 | 0 | 201 |
| EFL Championship | 131 | 0 | 0 |
| MLS | 451 | 0 | 0 |
| NWSL | 16 | 0 | 0 |
| NCAA women's soccer | 0 | 175 | 0 |
| Women's college volleyball | 0 | 220 | 0 |
| **Total** | **960** | **395** | **362** |

All 960 available snapshots have team statistics; 362 also have player statistics.
The remaining 598 are team-statistics-only soccer games. These counts describe
actual provider coverage, not inferred completeness or a guarantee for future games.
College soccer/volleyball responses had no usable statistics; their existing scores
remain the fallback. No external school-provider scraping was added.

| Production run | Available | Unavailable | Errors | Status |
| --- | --- | --- | --- | --- |
| 1246 | 343 | 83 | 0 | succeeded |
| 1247 | 237 | 183 | 0 | succeeded |
| 1248 | 277 | 129 | 0 | succeeded |
| 1249 | 103 | 0 | 0 | succeeded |
| 1250 (queue read-back) | 0 | 0 | 0 | succeeded |

The runtime profile wrote all snapshots successfully. The frontend database
identity read all 960 available JSON payloads. Integrity checks found zero payloads
with a mismatched sport, league, event identity or version. Existing game identity
and score checksums matched the verification baseline; row counts remained 1,495
ResultGames and 783 SportsMatchResults (the former also includes nonfinal games).
The latest live Results page returned HTTP 200 and rendered its 21 existing rows.
Both scraper and frontend were healthy after collection.

The collector does not write `ResultGames` or `SportsMatchResults`. Their source
applications can continue updating refresh timestamps independently during this
operation. Existing article sources, saved-news identities, game scores, scheduler
settings and frontend fallback links were not changed by the box-score migration.
The regular run wrapper still applies the repository's existing retention policy.
No new retention/deletion policy was added for optional game detail.

The deployed frontend still renders its existing results UI. HTTP checks of the
existing Results route confirm it remains available; this release does not claim
that inline box-score panels or detail routes have already been implemented.

## Setup and manual operations

Use migration credentials, explicitly targeting WebScraper:

```sh
(set -a; . ./.env.migrations; set +a; \
  cargo run --locked -- --database WebScraper --configure-boxscores)
```

This applies, in order:

1. `sql/sports_boxscores.sql`
2. `sql/sports_boxscores_view.sql`
3. `sql/sports_boxscores_permissions.sql`

Ordinary refresh uses runtime credentials:

```sh
cargo run --locked -- --boxscores-from-db --dry-run --boxscores-limit 30 -o /tmp/boxscores-preview.xml
cargo run --locked -- --boxscores-from-db --boxscores-limit 100 -o /tmp/boxscores-import.xml
```

Dry-run honors cache retry times and writes no database state. It may therefore
report zero candidates just after a completed import. Do not clear a working cache
merely to repeat a preview. Check availability/health using the view:

```sql
SELECT Sport,League,Availability,COUNT(*) Games
FROM dbo.SportsBoxScores
GROUP BY Sport,League,Availability;
```

Sources with no statistics need no destructive cleanup: their normal results stay
visible and future eligibility allows another check. Existing available data is
retained on provider errors or later empty responses. Display FetchedAt when data
freshness matters. The ESPN site API is public but undocumented; per-game coverage
and format availability are not guaranteed.

## Deployment preservation and rollback

Protected host backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261019-boxscores/`.
It contains the previous host source/configuration archive and private container
configuration. Keep it private. Host source and workspace are separate; changed
implementation, SQL, fixtures and documentation are synchronized and read back.

Rollback container: **web-scraper-before-boxscores-20261019**.
Rollback image tag: **web-scraper:before-boxscores-20261019**.
The scheduler command, runtime environment, entrypoint, user, working directory,
health configuration, mounts, networks/aliases, resource/security restrictions,
logging and restart policy were preserved and compared after replacement.

If rollback is authorized, stop the new scraper, retain it under a diagnostic
name, rename/start the preserved previous container as `web-scraper`, then verify
health and the scheduler. Leave the additive optional schema/data in place; the
old release does not consume it. No score rollback, article restore or deletion
is needed. Do not restart or replace the separately managed frontend to roll back
this scraper change. The user handles Git commits and pushes.
