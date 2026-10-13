# Rust scraper application audit — October 10, 2026 (America/Chicago)
Remediation status: all thirteen findings have since been addressed and deployed.
See [remediation and TLS verification](audit-remediation-2026-10-10.md). Findings
below preserve the original pre-remediation audit evidence.


The collector is operating and its stored identities are sound, but two issues
need attention first: completed professional games do not receive their final
schedule status, and the runtime SQL account has server-wide administrator rights.

This audit reviewed the Rust scraper workspace, SQL migrations/upserts, all
collector modules, dependencies, deployment configuration, and the running Docker
collector. Production SQL checks were SELECT-only, using the same credentials as
the deployed container. My-Newsfeed received a read-only /events spot check; its
application source and authorization controls were not audited. No production
configuration, schema, event data, image or service was changed. Existing uncommitted
work was preserved. Temporary audit examples were removed.

## Findings, in priority order

P1 means address promptly; P2 means address in the next hardening/correctness work;
P3 means lower-priority reliability work. Evidence distinguishes live confirmation,
deterministic reproduction, and source review. None of these claims an observed
server compromise.

| ID | Priority | Finding | Evidence |
|---|---|---|---|
| F01 | P1 | Finished games stop receiving schedule-status updates | Live DB/API mismatch and visible finished MLB game |
| F02 | P1 | Hourly collector uses SQL Server sysadmin credentials | Live role query, deployed credential match verified privately |
| F03 | P2 | Untrusted feed URLs can contain script schemes or internal destinations | Real parser accepted both adversarial fixtures |
| F04 | P2 | SQL certificate authentication is disabled | Deployed TrustServerCertificate=true |
| F05 | P2 | Old SQL TLS dependency has three RustSec advisory alerts | cargo audit and dependency tree; reachability caveats below |
| F06 | P2 | Oversized retention values wrap negative and can delete every row | CLI acceptance reproduced; destructive SQL not executed |
| F07 | P2 | Most downloaded bodies have no size limit | Source review; runtime has no memory cap |
| F08 | P2 | SQL operations have no application deadlines or progress watchdog | Source review and live missing healthcheck |
| F09 | P2 | Failed collections can report success | All-news-failed CLI run returned exit code zero |
| F10 | P2 | ESPN source writes are partial on failure and can abort unrelated collectors | Per-row autocommit and propagated error paths |
| F11 | P2 | Future-dated news can continually move its stored publication time | Parser clamp plus unconditional non-null publication update |
| F12 | P2 | Disabling a movie source does not stop its fetching/poster processing | Calendar collector and poster job query ignore Enabled |
| F13 | P3 | Hourly schedule skips the repeated fall-back 01:00 occurrence | Deterministic execution of the actual next_slot implementation |

### F01 — Completed-game status reconciliation is missing

[espn_schedule.rs](../src/espn_schedule.rs), lines 133–141, returns only
`event_date >= today && !completed`. Once a stored game finishes, its final
observation is discarded before `save_espn_schedule` can update ScheduleStatus.
Earlier in-progress or scheduled values remain indefinitely. The Central-date
page cutoff eventually hides older games, but cannot hide a finished game today.
This flaw is in the today-forward filtering added during the schedule work.

Live example: MLB ESPN game **401907994**, Chicago White Sox at Cleveland Guardians,
EventDate **2026-10-10**. ESPN reports **STATUS_FINAL**; the DB retains
**STATUS_IN_PROGRESS**. The live /events HTML still contains its game link and
7:00 P.M. CT scheduled card. Two October 9 WNBA games, 401918299 and 401918300,
also retain IN_PROGRESS although ESPN reports FINAL; those are now hidden by date.

**Fix:** distinguish new upcoming imports from reconciliation of already-stored
IDs. Allow final observations to update existing rows' schedule status without
inserting completed historical games or importing scores/results. Consider
reconciliation of recently stored IDs that moved to an earlier date/month as well.
Add the missing end-to-end scheduled → in-progress → final regression test and
verify same-day finished cards disappear without losing IDs/FirstSeenAt.

### F02 — Runtime database privileges are excessive

[db.rs](../src/db.rs), lines 46–59 and subsequent DDL, opens master and initializes
schema during normal collection. The live audit returned **IsSysadmin=1** and
**IsDbOwner=1**, using credentials privately confirmed identical to the running
collector. Compromise or misuse of those credentials therefore affects the SQL
server beyond the scraper's owned tables.

**Fix:** separate schema/deployment credentials from hourly ingestion credentials.
Use a dedicated least-privilege account for ordinary reads/writes to the required
scraper tables. Run migrations/configuration with a separately controlled account;
remove routine startup's need for master/CREATE DATABASE/ALTER VIEW. Do not simply
revoke rights before separating initialization, because current startup needs DDL.

### F03 — Upstream URL validation does not cover feed item links

[feed.rs](../src/feed.rs), lines 79–87, checks only link length. Its actual parser
accepted `javascript:alert(1)` and `http://127.0.0.1:8080/admin` in safe local RSS
fixtures. [main.rs](../src/main.rs), lines 104–111, subsequently fetches item URLs
when looking for missing images. Page canonical URLs are also joined/stored without
a final http(s) scheme check. Image validation elsewhere does not protect these
article links. A malicious or compromised upstream feed can therefore introduce
an internal-network request target or an unsafe link for a downstream renderer.

**Fix:** validate article/canonical/feed URLs before storage or fetch; allow only
approved schemes, block credentials and unintended internal/private destinations,
and apply the same checks after redirects and DNS resolution. Explicitly distinguish
trusted administrator source configuration from untrusted upstream-discovered links.
Validate link schemes in My-Newsfeed as defense in depth. No currently stored
non-http(s) news URLs were found. Browser script execution and internal endpoint
exploitation were not attempted or proven by this audit.

### F04 — SQL TLS does not authenticate the server

The deployed connection has **TrustServerCertificate=true**. Tiberius's enabled
TLS feature defaults to required encryption, so this is not a finding of cleartext
transport; it is a finding that certificate authentication is bypassed.

**Fix:** configure a trusted server certificate/CA and hostname validation, then
remove the bypass. Coordinate SQL Server certificate configuration and container
trust settings so the existing collector remains available.

### F05 — Dependency advisory alerts require TLS-path remediation

[Cargo.lock](../Cargo.lock), line 1697, retains rustls-webpki **0.101.7** through:
`tiberius 0.12.3 → tokio-rustls 0.24.1 → rustls 0.21.12 → rustls-webpki 0.101.7`.
The HTTP client's separate rustls-webpki **0.103.15** is newer; updating only that
copy will not address the SQL TLS path. cargo audit v0.22.2 flagged:

- [RUSTSEC-2026-0098](https://rustsec.org/advisories/RUSTSEC-2026-0098.html): URI name constraints.
- [RUSTSEC-2026-0099](https://rustsec.org/advisories/RUSTSEC-2026-0099.html): wildcard name constraints.
- [RUSTSEC-2026-0104](https://rustsec.org/advisories/RUSTSEC-2026-0104.html): CRL parsing panic.

The two name-constraint issues require certificate misissuance. The CRL advisory
explicitly excludes applications that do not use CRLs; this application does not
configure CRL use, so that alert is not evidence of a reachable current DoS.
Current SQL certificate bypass also limits the applicability of validation-path
findings, while independently causing F04. These are dependency-version alerts,
not three demonstrated application exploits.

Additional warnings: unmaintained rustls-pemfile **1.0.4** and yanked yoke-derive
**0.8.3**. Review an upgraded/replaced Tiberius TLS path and lockfile refresh with
SQL compatibility tests. Do not force an incompatible transitive semver update.
Add a repeatable dependency audit check to the project's validation workflow.

### F06 — Retention counts can become destructive negative values

CLI limits accept positive u32 values up to 4,294,967,295. [db.rs](../src/db.rs),
lines 398, 422 and 818, casts keep to i32 before `DELETE ... WHERE rn > @P1`.
**2,147,483,648 becomes -2,147,483,648**, making every positive row number qualify
for deletion. Affects global news, per-source news, and per-channel video retention.

CLI acceptance was reproduced with oversized --keep/--per-source values and an
intentionally nonexistent URLs file; it reached the file-read error, not argument
validation. No destructive SQL was executed. Current production defaults of 10
news stories and 5 videos are safe.

**Fix:** enforce i32-safe bounds at the CLI and DB API boundaries, using checked
conversion, or bind counts as SQL BIGINT. Test boundary values without deletion.

### F07 — HTTP timeout is not a response-size limit

Full-body `.text()` reads in [feed.rs](../src/feed.rs):16,
[main.rs](../src/main.rs):61/111, [espn_schedule.rs](../src/espn_schedule.rs):127,
[movies.rs](../src/movies.rs):323/366, [tvmaze.rs](../src/tvmaze.rs):203, and other
collectors have no byte cap. A fast large response can consume substantial memory
before the 15/20-second timeout. Parsing/checking event count happens afterward.
The running container also has **Memory=0 / NanoCpus=0** (no configured limits).
Poster image reads already have a useful 5 MB cap.

**Fix:** introduce shared bounded HTTP readers with suitable per-resource limits,
reject over-limit responses before full allocation/parsing, and set reviewed
container memory/CPU limits. Do not rely on Content-Length alone. No OOM test was
performed against production.

### F08 — A stalled SQL operation can freeze an apparently running collector

[db.rs](../src/db.rs):20 calls TcpStream::connect without an application timeout;
connect/query futures throughout are awaited without deadlines. HTTP timeouts do
not cover those operations. One stalled DB connection/lock/read can block every
later collector and the hourly loop. The running container has no healthcheck;
restart policy cannot detect a process that remains alive but makes no progress.

**Fix:** add bounded connection/query deadlines with safe cancellation/reconnection,
and a persisted run heartbeat/last-success monitor. Keep useful per-source progress
when individual providers fail. The observed runtime is currently making progress:
its latest captured cycle started at 23:00 and finished at 23:06 Central.

### F09 — Collection success is reported inconsistently

A safe dry-run against localhost port 1 failed all news fetch attempts, logged the
failure, produced zero pages, and still returned **exit code 0**. The fetch-error
paths in [main.rs](../src/main.rs):371/614/742/748 log and continue, while run_once
ultimately returns Ok. Partial source failures can be acceptable, but there is no
aggregate successful/partial/failed outcome for external monitoring. News source
success is recorded at lines 348–351 before persistence, so subsequent save failures
can leave fresh success-looking source metadata.

**Fix:** collect structured per-source outcomes, mark success after persistence,
return failure when all required work fails, and expose partial failures without
preventing independent providers from completing. Preserve the existing failure
isolation intent rather than terminating on every harmless source warning.

### F10 — ESPN persistence has no snapshot transaction or save-error isolation

[db.rs](../src/db.rs):516–560 saves each fixture through a separate MERGE. HOLDLOCK
protects that event's identity, not the entire source snapshot. A later SQL error
can leave earlier rows committed. [main.rs](../src/main.rs):610 propagates a save
error before event_result records it; later independent feeds/movies/TV are skipped.
Unbounded venue/name fields and UTF-16-vs-character limits can trigger SQL rejection.
This is source-confirmed behavior, not an observed failed production import.

**Fix:** choose explicit source-level transactional persistence or validated
per-row quarantine with an accurate partial outcome, record save errors, and
continue unrelated collectors. Reuse the Big 12 per-observation error isolation
and TV's atomic snapshot pattern where appropriate. Add a forced middle-row failure
regression test against staging and verify LastError and unaffected providers.

### F11 — News publication dates can drift forward during repeated polling

[feed.rs](../src/feed.rs):123 clamps future published dates to the current poll
instant. [db.rs](../src/db.rs):382 overwrites an existing Published whenever the
new value is non-null. The same URL with a misdated future pubDate consequently
moves forward on each poll and can displace genuinely newer stories during pruning.
This is also documented as an existing open issue in HANDOFF.md.

**Fix:** retain raw/provider date provenance, preserve the first valid stored
publication time under the intended correction rules, and test repeated imports
of a future-misdated story. No publication timestamps were modified by this audit.

### F12 — Disabled movie providers still get fetched and used for posters

[movies.rs](../src/movies.rs):287–339 builds and requests both providers' calendars
without reading Enabled. [db.rs](../src/db.rs):210 selects poster jobs across all
MovieReleaseSources, also without Enabled. Release upserts correctly check Enabled,
but disablement does not stop calendar traffic and does not stop poster cache/movie
metadata updates based on that source. Both current movie providers are enabled,
so this is not an observed current disabled-provider incident.

**Fix:** pass enabled-provider configuration into collection and filter poster jobs;
make disablement semantics consistent for fetching, releases, posters and statuses.
Test a disabled provider with a request-count mock and staging assertions.

### F13 — Hourly fall-back scheduling skips the second 01:00

[main.rs](../src/main.rs):792 uses LocalResult.earliest(), discarding the later
valid occurrence of an ambiguous local time. The actual next_slot function,
executed with now=**2026-11-01T06:30:00Z** (01:30 CDT) and hours=1, returns
**08:00Z** (02:00 CST); the next eligible hourly clock slot is **07:00Z** (01:00 CST).
The deployed one-hour collector therefore has a two-hour gap across fall-back.
Existing tests cover three-hour schedules, which do not expose this case.

**Fix:** consider both occurrences of ambiguous local slots before filtering
strictly-future instants. Add one-hour fall-back regression coverage. No production
clock or scheduler was changed to reproduce this.

## Validation and healthy properties

- `cargo test --locked --all-targets`: **91 passed**, eight DB tests opt-in.
- NWSL, WNBA and MLB SQL integration tests: **three additional passes**, with
  rollback-only fixtures in their isolated validation databases. Five remaining
  opt-in DB tests were not executed because this audit avoided production writes
  and broader staging changes.
- `cargo clippy --locked --all-targets -- -D warnings`: **failed**. Bin: 20 lints;
  bin test: 21; movie_preview example: seven. Primarily collapsible branches,
  map-entry/sort idioms and an argument-count lint; do not confuse these with
  confirmed security defects. `cargo fmt --all -- --check` also fails baseline
  formatting. No automated CI workflow was found in this workspace.
- `cargo audit --json`: three version advisories and the two warnings noted above;
  RustSec database revision 7eebec69c352c7191b1f13eb95dd510eeca5d1de.
- Production: **981 sports events**, **19 feeds**, **1,444 school observations**;
  zero duplicate canonical keys; all foreign/check constraints enabled and trusted.
- MLB feed 4193 retains **season=2026**; **22 stored MLB games**, zero 2027 rows.
  Default seeding still has no season parameter: preserve/verify this deployment's
  explicit URL during recreation or restore to respect the user's scope.
- News: 28 sources, 280 pages, 10 per source; no stored non-http(s) page links and
  no current source errors. YouTube: 11 feeds and 55 videos.
- Movies: 33 movies, 33 links, 76 releases; 28 matched posters and five unmatched.
  Both movie source statuses healthy. Existing unmatched posters are not new defects.
- TV: five tracked shows, 90 episodes, no current errors; zero timed episodes
  missing UTC timestamps in the production snapshot. ID validation, validated
  snapshot persistence, cache/retry state and leases are sound foundations.
- Three Big 12 provider records remain intentionally quarantined (two self-opponent,
  one missing alias). Timing disagreements are retained as warnings; public game
  identities remain unique. They are upstream quality issues, not 16 broken imports.
- Runtime: expected image 257a666459d34838a8431d61f9db2efbf5c5857854afabe58f7b67ca1111bcbd,
  running with zero restarts, existing hourly flags and non-root scraper user.
  Not privileged. Actual Docker logging is bounded to three 10 MB files.
- SQL values are parameterized; database-name interpolation is restricted to
  ASCII letters/digits/underscore. No .env file was found in tracked Git history;
  workspace .env mode is 0600. No credential values were printed or stored in this report.

## Recommended work order and limits

Fix F01 first, then separate runtime SQL privileges (F02) while planning certificate
and dependency remediation together (F04/F05). Next address URL/body boundaries,
retention bounds, truthful run outcomes and save-error isolation. Follow with news
publication rules, disabled-provider semantics, DST handling, and formatting/CI.
Keep results import behavior separate from schedule-status reconciliation.

This was a source/runtime/data audit, not a penetration test, formal correctness
proof, load test, or container OS-package vulnerability scan. My-Newsfeed source,
its CSP/authentication/link handling, and backup restore procedures were not fully
tested. No exploit of the dependency alerts was attempted. The JSON companion
contains sanitized evidence and counts; raw production snapshots remain in /tmp.
