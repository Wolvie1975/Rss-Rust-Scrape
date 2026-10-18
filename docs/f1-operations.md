# F1 collector operations

## Provider and scope

[Jolpica F1](https://github.com/jolpica/jolpica-f1) provides the current season's
race/session schedule, full Grand Prix classifications, sprint classifications,
and driver/constructor championships. The collector resolves `current` once and
pins subsequent requests to that season. Provider URLs:

| Dataset | Endpoint under `https://api.jolpi.ca/ergast/f1/` |
| --- | --- |
| Calendar | `{season}/` |
| Grand Prix results | `{season}/results/` |
| Sprint results | `{season}/sprint/` |
| Drivers' championship | `{season}/driverstandings/` |
| Constructors' championship | `{season}/constructorstandings/` |

[Provider documentation](https://github.com/jolpica/jolpica-f1/blob/main/docs/README.md)
specifies pagination capped at 100 items; result pagination counts classified
drivers rather than race objects. Pages may split one race. Collection flattens
classifications with their round, checks counts/offsets and rejects duplicates,
invalid identities/dates, wrong seasons, and inconsistent championship rounds.
A custom `My-Newsfeed/0.1.0` user agent identifies requests. Requests are paced
350 ms apart and capped at 5,000 rows per endpoint. At initial verification the
collector made 10 requests per refresh. HTTP 429/other failures stop that refresh
without replacing good data; the next scheduled run can retry.

The [documented limits](https://github.com/jolpica/jolpica-f1/blob/main/docs/rate_limits.md)
at implementation time were 4 requests/second and 500/hour; limits can change.
This supplies periodic published data, not live telemetry, qualifying result
classifications, driver photos, or an official F1 cancellation/status feed.

## Setup and enablement

The shared scraper schema must already exist. Explicit migration credentials
create only the new F1 objects; runtime imports perform no DDL:

```sh
( set -a; . ./.env.migrations; set +a; cargo run --locked -- --configure-f1 )
```

This applies `sql/f1.sql` and the four `sql/f1_*_view.sql` scripts in one
transaction. `F1Source` singleton ID 1 is seeded **disabled**. Repeated setup
preserves operator enablement, existing snapshots and telemetry.
`F1Snapshot` stores one full season snapshot as validated JSON plus Season,
StandingsRound and FetchedAt. The four read views expand it into tabular feeds:
`F1Schedule`, `F1Results`, `F1DriverStandings`, `F1ConstructorStandings`.
They are separate from football-style match results and team standings.

After provider validation, enable explicitly with migration credentials:

```sql
UPDATE dbo.F1Source SET Enabled=1 WHERE ID=1;
```

## Verification and refresh

Read-only provider verification, without any database access:

```sh
cargo run --locked --example f1_preview
# Optional normalized JSON artifact:
cargo run --locked --example f1_preview -- /tmp/f1-preview.json
```

Use the existing runtime `.env` for dry-run and actual collection:

```sh
cargo run --locked -- --f1-from-db --dry-run -o /tmp/f1-dry-run.xml
cargo run --locked -- --f1-from-db -o /tmp/f1-run.xml
```

The optional XML path is the existing application output argument; F1 data is
stored in its SQL feeds, not converted into article RSS items. Dry-run performs
no snapshot, source-health or run-history writes. Actual refresh records attempt,
success and failure timestamps. All five datasets replace the snapshot atomically;
failures retain previous data. An older season or standings round cannot
supersede a newer snapshot, including concurrent overlapping refreshes.

The updated binary also collects enabled F1 data under **`--events-from-db`**,
already present in the production hourly command. No scheduler argument change
is required, and the production scraper now runs **`web-scraper:f1-20261018`**.
The user authorized deployment after initial feed implementation. The hourly
container was replaced with the updated image while preserving its existing
command and runtime configuration. No scheduler argument change was needed.

Preserve existing environment, TLS certificate mounts, volume, network, resource
limits, security flags, hourly schedule and all other collectors when deploying.
For a manual refresh before deployment, use the command above from this checkout.

## Inspect and disable

```sql
SELECT ID,ProviderKey,Enabled,LastAttemptAt,LastSuccessAt,LastError FROM dbo.F1Source;
SELECT Season,StandingsRound,FetchedAt FROM dbo.F1Snapshot;
SELECT COUNT(*) AS Races FROM dbo.F1Schedule;
SELECT SessionType,COUNT(*) AS ClassifiedDrivers FROM dbo.F1Results GROUP BY SessionType;
SELECT COUNT(*) AS Drivers FROM dbo.F1DriverStandings;
SELECT COUNT(*) AS Constructors FROM dbo.F1ConstructorStandings;

-- Stop future collection and hide the read views without deleting snapshots:
-- UPDATE dbo.F1Source SET Enabled=0 WHERE ID=1;
```

Rollback can disable the F1 source and restore the previous scraper image. The
additive tables/views can remain; old code ignores them. Do not delete other
sports data or reassign shared source IDs. There is one current-season snapshot,
not an archive; each successful refresh replaces it. F1 is independent of shared
past-schedule/news retention.

## Validation

Provider fixtures are in `tests/fixtures/f1/`. Unit tests cover real response
fields, split-race page flattening, invalid/duplicate identities, missing
classifications, unknown times, empty preseason data and fractional points.
The ignored database integration test requires migration credentials,
`F1_TEST_DATABASE` beginning with `F1_Test_`, and a normalized `F1_TEST_PAYLOAD`.
It validates all four views, idempotent setup/save, enablement, failed refresh
preservation and regression rejection in a separate database.

Frontend field mappings and acceptance checks:
[F1 frontend integration](f1-frontend-integration.md).

## Production verification — October 18, 2026

Applied the F1 migration, enabled singleton source 1, performed a read-only
provider check and runtime dry-run, then imported with runtime ingestion
credentials. Read back the committed snapshot and all four production views:

- Season 2026; championship as-of round 17.
- 23 calendar races.
- 374 Grand Prix classifications across 17 races.
- 132 sprint classifications across six sprints.
- 23 driver standings rows and 11 constructor standings rows.
- Snapshot FetchedAt: `2026-10-18T05:12:23.7640853` UTC.
- Source enabled; LastSuccessAt set; no recorded LastError.
- Provider preview, dry-run and import each made 10 HTTP requests.
- `cargo test --locked`: 139 passed, 21 ignored, no failures.
- F1 SQL integration test passed separately in `F1_Test_20261018` using the
  normalized provider payload. This isolated test database remains available.
- Formatting and diff whitespace checks passed.

The data and updated hourly collector are deployed in production. Deployed
container run **1206** succeeded with one source success and zero failures;
FetchedAt advanced to `2026-10-18T05:18:51.9111955` UTC. The existing frontend
production database identity successfully read all four views after this import.
Both scraper and frontend containers were healthy. Startup logged the next
hourly slot at **October 18, 1:00 a.m. Central**; that future slot was not awaited.
Frontend rendering has not been implemented or verified here.

Deployment evidence, configuration verification and rollback:
[F1 deployment report](../reports/f1-production-20261018.md).
