# Persisted scoreboard operations

The independent `scoreboard` binary writes only its `SportsScoreboard*` tables in
WebScraper. It reads enabled SportsEventsType feeds and ResultFollows, using
ResultFollowTeams for the web worker's validated conference membership. It never
changes those configurations, ResultGames, SportsMatchResults or SportsEvents.
The existing web results worker and hourly scraper remain enabled.

## Setup and operation

Prerequisites: the existing scraper schema, ResultFollows/ResultFollowTeams,
SportsBoxScoreCache, restricted ingestion credentials, and the deployed SQL CA.
Use migration credentials explicitly for these additive scripts, in this order:

```sh
cargo run --locked --example scoreboard_sql -- .env.migrations sql/scoreboard.sql
cargo run --locked --example scoreboard_sql -- .env.migrations sql/scoreboard_sources_view.sql
cargo run --locked --example scoreboard_sql -- .env.migrations sql/scoreboard_view.sql
cargo run --locked --example scoreboard_sql -- .env.migrations sql/scoreboard_permissions.sql
```

The helper always targets WebScraper; it prints query results, never connection
strings. Run it in a shell without a conflicting MSSQL_CONNECTION_STRING override.
The migration grants ingestion SELECT on follow configuration and writes on the
new scoreboard tables only. Frontend access is SELECT on SportsScoreboard and
SportsScoreboardCoverage through WebScraperScoreboardReader; add the actual
app-specific database user to that role with migration credentials if necessary.
No app user is given scoreboard write access. Existing broader readers continue
working; this task does not change their unrelated permissions.

With the restricted runtime profile:

```sh
cargo run --locked --bin scoreboard -- --dry-run
cargo run --locked --bin scoreboard
cargo run --locked --bin scoreboard -- --watch --live-minutes 10
cargo run --locked --bin scoreboard -- --health-check
```

Dry-run reads coverage/due requests without writes or upstream requests. One-shot
runs exit nonzero on partial/fetch failures; watch mode logs them and retries in a
later loop. All SQL operations have deadlines; a failed SQL run drops its session
before reconnecting. A session application lock skips overlapping collectors.

Deploy only the new service; the existing hourly container needs no restart:

```sh
docker compose build scoreboard
docker compose up -d --no-deps scoreboard
docker exec web-scoreboard scoreboard --health-check
docker exec web-scraper web_scraper --health-check
```

The verified production container was created directly through Docker with the dated
image. To adopt Compose management for that existing deployment, first stop/remove
only web-scoreboard; its SQL data persists. The commands above otherwise describe
an initial Compose deployment. Never remove the hourly scraper for this switch.

Dockerfile.scoreboard builds only the scoreboard binary with one Cargo build job.
The service uses the ingestion profile, mounted public CA, non-root user, read-only
root, dropped capabilities, bounded temporary space, memory/CPU/PID limits and
rotating logs. No migration credentials are copied into its image or service.
The production deployment can pin the image tag recorded in the release report.

## Cadence and failure behavior

The watch loop checks due work after a 60-second pause between runs. Requests are
shared per sport/league/provider day across all overlapping enabled sources.

| Persisted state for a provider day | Next request |
| --- | --- |
| Any live, delayed or suspended game | 10 minutes (configurable 10–15) |
| Upcoming game within one hour, no more than 12 hours past expected start | Same live interval |
| A final first observed within the last hour | 15 minutes |
| Past day containing only finals or no games | Daily |
| Other day / future fixtures / unknowns | Hourly |

The fastest applicable state wins for a shared request. Newly found historical
finals also receive the initial one-hour correction window. After a very old
upcoming game's scheduled time passes 12 hours without a status change, its day
returns to hourly reconciliation rather than implying active play.

A first failure retries after 15 minutes, doubling to a four-hour cap. A valid
Retry-After response on 429/503 can extend this delay up to seven days. Any pending
failed request pauses that league until its cooldown expires. Other leagues
continue. There are no immediate retries; requests are spaced by 350 ms. Partial
responses save valid games, retain untouched games and retry within 15 minutes;
they do not advance request/source full-success timestamps. Empty successful
responses have their own availability value and never clear games.

These existing public ESPN endpoints have no verified published SLA or request
quota for this integration. The intervals describe our scheduler, not guaranteed
provider latency. See [ESPN's terms reference](https://support.espn.com/hc/en-us/articles/360035445091-Terms-of-Use).
No authentication challenge or access restriction is bypassed.

LastObservedAtUtc is the successful sample's request-start timestamp, not the time
a score changed. Failed, malformed and omitted events do not advance it. Older
request samples and explicitly older provider revisions cannot replace newer
stored games. Corrections can lower scores or change a final status. The complete
game sample and its source associations commit atomically.

Source LastSuccessAt describes the most recent fully successful request for any
of its dates, not proof that every event/date is fresh. Use per-row observation
and RequestAvailability for game displays. A live/delayed/suspended game older
than 25 minutes is marked IsStale; preserve its last-good score and show a warning.
The progress health check detects enabled sources not attempted within 75 minutes;
it is not a claim that upstream requests succeeded or every event is fresh.

## Scope, dates and retention

Default request window: three provider days back and seven forward, plus one extra
forward day for the Eastern-provider/Central-display boundary. Active delayed/live/
suspended games are also rechecked outside that window until 30-day retention
expires. Flags: --lookback-days 1–7, --lookahead-days 1–14, --retention-days 8–90;
retention must exceed lookback. Day-only/TBD values do not get fabricated instants.

Rows expire after 30 days by scoreboard date. Initial population only backfills
three days; a 30-day historical navigation promise would be incorrect at launch.
No missing response implies cancellation or deletion. Source disablement hides
its retained associations immediately; another enabled association can keep the
same game visible. Event identity is (Provider, Sport, League, EventId).

Only verified ESPN coverage is collected. The Big 12 calendar has other sports
with no established live provider here. Its IDs are not ESPN IDs, and there is no
validated Big 12 schedule-to-scoreboard bridge in this release. Do not join these
by name or copy a numeric calendar GameId into an ESPN key. F1 remains separate.

## Validation and rollback

Offline fixtures cover six sport types. Run:

```sh
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --bin scoreboard scoreboard_sql_samples -- --ignored --nocapture
```

The explicit SQL test loads .env.migrations, uses unique fixture game IDs and rolls
back all writes. It checks idempotency, stale/empty preservation, out-of-order
samples, provider revision rejection, score corrections, rescheduling, optional
box-score absence and coverage visibility. It does not write web-owned results.

To roll back a first deployment, stop/remove only web-scoreboard and remove its
Compose service or restore the backed-up Compose file. Leave additive tables and
views in place so readers retain last-good samples and existing consumers continue
working. Disable/hide the frontend feature if it has already been released. Do
not stop the hourly scraper or the web results worker. To revert a later collector
release, recreate only the scoreboard service using the previously verified image
and unchanged restricted runtime configuration. Database removal is unnecessary.

Dated deployment evidence belongs in reports/live-scoreboard-20261023.md; the
frontend read contract belongs in New Development/live-scoreboard-frontend-handoff.md.
