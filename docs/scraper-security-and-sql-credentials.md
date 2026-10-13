# Scraper hardening and SQL credential split

The hourly collector now uses **webscraper_ingest**, a dedicated login with
**WebScraperIngest** table permissions. It has no sysadmin/db_owner membership,
no CREATE TABLE permission, and no access to result-sync tables. My-Newsfeed's
login and the shared administrator password were not changed.

## Credential files and setup

- `.env`: restricted ingestion connection, generated strong service password.
- `.env.migrations`: existing deployment/admin connection, for explicit setup only.
- Both files are mode 0600, ignored by Git and excluded from Docker build context.
  The hourly Compose service loads only `.env`. Migration credentials are not
  present in its environment or filesystem.
- `sql/runtime_permissions.sql`: reviewable grants for the ingestion role. Login
  creation/password provisioning happens separately using bound SQL parameters;
  no password is stored in this script or committed.

Normal collection connects directly to the target database and does **no DDL**.
Use administrator/deployment credentials for explicit schema changes:

```sh
# Run from the repo root, in a temporary shell; do not use this profile for the scheduler.
( set -a; . ./.env.migrations; set +a; cargo run --locked -- --initialize-db )
```

The --configure-tv/--configure-big12/--configure-nwsl/--configure-wnba/--configure-mlb/--configure-standings/--configure-movieinsider/--configure-rankings
commands also belong to the deployment/admin path. `MSSQL_MIGRATION_CONNECTION_STRING`
can optionally override the setup connection; ordinary collection never uses it.
Do not inject that administrator variable into the hourly service.

For a fresh installation, initialize the schema, provision a dedicated login,
apply the reviewed permission script, then install its private runtime profile.
Keep migration credentials separately controlled. Rotate service passwords
explicitly with a coordinated profile/container update; automated expiry is off
so the collector does not silently stop while unattended. If the old administrator
secret was exposed, rotate it in a separate coordinated change covering every
consumer, including My-Newsfeed; this remediation did not rotate a shared secret.
My-Newsfeed should have its own app-specific SQL account with permissions reviewed
against its admin and results features, rather than reuse either scraper profile.

## Scraper fixes

- Schedule collection includes a seven-day reconciliation window. Recently stored
  games receive final/postponed/canceled/moved-date observations. Completed or past
  observations can update existing IDs but cannot create new historical rows.
  Scores/results remain untouched; MLB's configured season=2026 restriction remains.
- ESPN fields are checked against SQL UTF-16 sizes before writes. A source batch
  commits atomically; any later-row failure rolls back earlier updates. Save errors
  are recorded and independent collectors continue.
- Retention limits are checked at both CLI and DB boundaries; integer wrapping
  cannot turn a large requested count into a delete-all predicate.
- Shared HTTP clients reject unsafe schemes, URL credentials, private/reserved
  addresses, unsafe redirects and DNS answers. DNS checks use the actual connection
  resolver. Ambient proxies are disabled so they cannot bypass those checks.
  News item/canonical/image links are validated before storage or fetch.
- Text responses are capped at 8 MiB, ESPN JSON at 32 MiB, poster images at 5 MB.
  Feed discovery and image enrichment have bounded work budgets.
- SQL connection/handshake/query deadlines and server lock timeouts bound stalls.
  A timed-out session is marked unusable and replaced on the next run. No additional
  statements are issued on a canceled/poisoned session.
- News source success is recorded after persistence. Partial/all-failed collections
  return nonzero and publish structured success/failure counts. Valid empty snapshots
  are handled as successful work. Run outcomes/heartbeats are stored in ScraperRuns.
- News publication times no longer move forward on repeated polls of misdated input.
  Earlier corrections and initially missing timestamps can still be populated.
- Disabled movie providers are skipped for calendar fetching and poster processing.
- The hourly scheduler considers both occurrences of ambiguous fall-back times.
- RSS publication uses same-directory atomic rename instead of overwriting in place.
- The old SQL Rustls stack was replaced with native TLS/OpenSSL. Cargo audit reports
  zero advisories and zero warnings. The runtime image includes libssl3.
- Formatting, Clippy and CI validation are configured; SQL integration tests remain
  isolated and opt-in. CI does not receive production credentials.

## Runtime monitoring and container settings

`--health-check` performs read-only schema/progress checks, without running imports
or migrations. Compose now sets 512 MiB memory, two CPUs, 128 PIDs, dropped
capabilities, no-new-privileges, read-only root, and a bounded writable /tmp.
The existing /data volume remains writable. Logs rotate at 10 MB / three files.

Docker healthchecks have a 70-minute startup allowance because the hourly collector
waits for its next aligned slot. ScraperRuns retains 30 days of run outcomes. Check
latest run Status, HeartbeatAt and ErrorSummary. Docker restart policy alone does
not restart an unhealthy-but-running process; health needs to be monitored by the
operator/orchestrator. SQL deadline recovery closes a failed session and retries
with a new connection on the next collection cycle.

## SQL certificate activation completed

After user approval, the shared SQL server was restarted with the hostname-validated
certificate for **sql2025**. The ingestion profile now uses
`TrustServerCertificateCA=certs/sql-ca.crt` and no certificate-bypass option.
The CA is mounted read-only at /data/certs/sql-ca.crt. Correct-CA health checks pass;
a deliberate unrelated-CA test fails certificate verification. My-Newsfeed /events
returned HTTP 200 after reconnection. Local migration profiles also use the CA.

Prepared leaf files (mssql-owned, private key mode 0600) are under:
`/var/opt/mssql/certs/webscraper-audit-fixes/` inside sql2025. The CA public certificate
is `certs/sql-ca.crt` in the host checkout. Signing material stays outside the image
and SQL server, in the protected deployment backup's pki folder.

For a future coordinated reapplication, run on the Docker host:

```sh
bash scripts/activate_sql_tls.sh --activate
```

This saves the previous server configuration, selects the prepared certificate,
restarts sql2025, removes certificate bypass from the ingestion profile, mounts the
CA read-only, verifies the authenticated connection and recreates the scraper.
Its password/login remains the restricted account. It does not rotate My-Newsfeed's
credentials or change server-wide force-encryption/protocol policy.

Rollback, if the authenticated connection does not pass:

```sh
bash scripts/rollback_sql_tls.sh --rollback
```

Both scripts restart the shared SQL server and need an agreed interruption window.
Rollback restores the certificate settings/runtime profile while retaining the
restricted ingestion account. See Microsoft's
[SQL Server Linux TLS configuration](https://learn.microsoft.com/en-gb/sql/linux/sql-server-linux-encrypted-connections?view=sql-server-ver16)
for certificate ownership/trust and the SQL Server 2025-specific settings.
