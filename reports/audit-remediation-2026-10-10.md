# Audit remediation — October 10, 2026 (Central)

All thirteen audit findings are now addressed and deployed. After the user approved
the shared interruption, sql2025 restarted with the validated server certificate
and the scraper switched to CA verification. Website credentials were not changed.

| Finding | Remediation |
|---|---|
| F01 | Seven-day existing-ID reconciliation; final/past observations never insert historical games |
| F02 | Dedicated webscraper_ingest account, table-scoped grants; migration credentials outside scheduler |
| F03 | Public URL/IP/DNS/redirect boundary and article/canonical/image checks |
| F04 | Hostname-validated server certificate active; CA verification required; wrong CA rejected |
| F05 | Native SQL TLS backend and dependency refresh; audit clean |
| F06 | Checked retention bounds and UTF-16-aware SQL title limits |
| F07 | Bounded readers and container resource limits |
| F08 | Connection/operation deadlines, poisoned-session guard, heartbeat and read-only healthcheck |
| F09 | Accurate persistence outcomes, nonzero failure/partial exit, per-source summaries |
| F10 | Atomic ESPN snapshots, prevalidation and save-error isolation |
| F11 | Monotonic publication timestamps under correction rules |
| F12 | Enabled movie-provider filtering for calendars and poster jobs |
| F13 | Both DST fall-back clock occurrences considered |

Validation: 99 regular tests passed; NWSL/WNBA/MLB SQL tests and a new hardening SQL
regression passed in isolated staging. The new regression verifies scheduled →
in-progress → final on the same ID/FirstSeenAt, excludes newly observed completed
rows, and forces a later MERGE failure to prove earlier snapshot writes roll back.
Formatting and Clippy with warnings denied pass. Dependency audit has zero advisories
and zero warnings. Schema addition is dbo.ScraperRuns; setup now requires an explicit
migration path rather than performing DDL during hourly collection.

The restricted account was tested live: sysadmin=0, db_owner=0, CREATE TABLE=0;
reads of ResultGames are denied. A full read-only production preview completed,
then a full live collection completed with **64 source successes, zero failures**.
Read-only health check passes. MLB game 401907994 now has STATUS_FINAL, with its
original database ID **17849**. Sports remain **981 rows**, with no 2027 MLB rows.

Final deployed image:
`sha256:15fb2ef4dc56ec94a522d0083a16e051b97558d8f744edb31b37e84b4ae46211`.
Service remains hourly with news, school/ESPN sports, YouTube, movies and TV.
The hardened container uses the restricted runtime profile only. Its first
replacement attempt hit a Compose tmpfs-list formatting error; the previous
collector remained running. Corrected the configuration and verified the intended
image and credentials before live collection.

Rollback image: `web-scraper:before-audit-fixes-20261011`.
Protected host backups: `.deployment-backups/20261011-audit-fixes/` (UTC-labelled
artifact folder), source.tar, previous private profile and pki material. Do not
restore old administrator runtime credentials except as a controlled emergency;
return to the restricted profile once the corrected image is restored.

Guidance: docs/scraper-security-and-sql-credentials.md. The shared TLS cutover
completed after approval; correct-CA authentication and wrong-CA rejection are
verified. Post-TLS professional schedule refresh completed with three source
successes and zero failures. No Git commit/push; existing work retained, plus
remediation code/docs/tests.

## TLS cutover completed after user approval

- SQL certificate is active; shared sql2025 restarted once. Scraper uses the trusted
  CA with certificate bypass removed, retains webscraper_ingest and encrypted SQL.
- Authenticated health check passes. A deliberate unrelated CA fails with
  certificate verify failed (unable to get local issuer certificate), exit code 1.
- My-Newsfeed remains healthy; /events returns HTTP 200. Read-only sports audit
  over verified TLS returns 981 events / 19 feeds / 1,444 school observations and
  zero canonical duplicates.
- The initial helper execution resolved relative mount paths against its virtual
  checkout location, creating an empty CA directory. SQL remained running; the
  old collector could still connect. Removed the empty directory, supplied the
  actual host project path and recreated the scraper successfully. Scripts now
  accept SCRAPER_HOST_PROJECT_DIR for helper use.
- CA is read-only mounted at /data/certs/sql-ca.crt. Local runtime/migration profiles
  use the same CA-relative path from the repository root. No plaintext secrets are
  printed. Protected pre-TLS profiles/configuration remain available for rollback.
- Rollback uses the cached Rust helper image with --volumes-from so it can restore
  server configuration even if SQL's container cannot start.
