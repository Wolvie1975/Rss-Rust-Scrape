# Sports period labels — October 19, 2026

## Outcome

Final release **web-scraper:period-labels-20261019** was deployed at approximately
**2:58 PM CDT** (19:58 UTC), with image:
`sha256:689abd076ab5c7cbbf2bc0953b3be133b8fbb240feae098ab05d234e80372e17`.
The deployed explicit health probe and runtime configuration comparisons passed.
Both scraper and frontend containers were healthy after the completed repair.
The existing hourly command, environment, TLS mount, persistent volume, networks,
resource/security settings, logging and restart policy were preserved.

The JSON payload remains **version 1**. Descriptive strings now populate the existing
`teams[].lineScores[].period` field, which the deployed frontend already reads.
No schema migration or frontend deployment is required. Result scores and their
ownership remain in the existing result models.

## Initial production audit

The cache contained 1,355 identities: 960 usable statistics snapshots and 395
unavailable snapshots. All **5,356** retained line entries lacked period labels.

| Sport/league | Cached games | Line entries | Missing labels before |
| --- | --- | --- | --- |
| MLB | 34 | 598 | 598 |
| WNBA | 7 | 60 | 60 |
| College football | 75 | 600 | 600 |
| NFL | 45 | 370 | 370 |
| NHL | 201 | 1,336 | 1,336 |
| Championship | 131 | 524 | 524 |
| MLS | 451 | 1,804 | 1,804 |
| NWSL | 16 | 64 | 64 |
| NCAA women's soccer | 175 | 0 | 0 |
| Women's college volleyball | 220 | 0 | 0 |

The stored results already include 348 upcoming men's college basketball games
under the exact league key `mens-college-basketball`, with enabled follows, but
none were completed. No men's college basketball snapshots were retained at this audit. The normalizer
supports the configured ESPN league `mens-college-basketball`, tested against a
recorded April championship game and synthetic overtime cases; no historical
basketball game was inserted into production merely to demonstrate labels.

## Provider evidence and mapping

ESPN summary competitors supply line-score display values but omit the numeric
period identities in the inspected payloads. The previous normalizer copied that
missing `period` field. Site scoreboards provide explicit period IDs for baseball,
football, hockey and basketball. Their dates use US Eastern calendar days; candidate
queries derive that date from the stored UTC start using `Eastern Standard Time`.

Soccer site scoreboards omit line-score arrays. For those competitors, the collector
retrieves the public core line-score collection using fixed, validated sport/league,
event and team identities. It checks completeness and `$ref` identity paths without
following provider references. This supplies explicit half/extra-time/penalty IDs.

Some college games are absent from dated scoreboards even when their final event
summary is published. The collector can anchor core metadata in that event's
summary after verifying the requested event, competition, final status and both
teams. It still obtains explicit period IDs from core and checks every cached
value. A recorded Baylor–TCU fixture demonstrates this omission and fallback.

Exact provider URLs and reduced source data are in
[`tests/fixtures/periods/`](../tests/fixtures/periods/README.md). Source event and team
identities, final state, array lengths and every existing display value must agree
before a cached array can be labeled. Sparse arrays use their explicit period IDs;
no complete `1..N` sequence is synthesized. Ambiguous IDs remain null and are logged.
Malformed/mismatched metadata fails safely and preserves the last-good cache.

| Sport/league | Labels emitted for supported numeric identities |
| --- | --- |
| MLB | `1st inning`, `2nd inning`, …, `10th inning`, `11th inning`, … |
| NFL / college football | `Q1`–`Q4`, `OT`, `2OT`, … |
| NHL | `1st period`–`3rd period`, `OT`; playoff season metadata permits `2OT`, `3OT`, …; an explicitly identified final shootout row is `SO` |
| Men's college basketball | `1st half`, `2nd half`, `OT`, `2OT`, … |
| WNBA / NBA | `Q1`–`Q4`, then `OT`, `2OT`, … |
| Soccer | `1st half`, `2nd half`, `ET1`, `ET2`; an explicitly identified penalty row is `SO` |

Provider label strings are preserved. No zeros are substituted for missing values;
explicit unplayed markers remain as supplied. No ninth inning, regulation period,
overtime or shootout row is manufactured. A penalty-only third soccer entry without
sufficient period identity remains ambiguous rather than being called ET1 or SO.
Regular-season NHL period 5 without shootout identification is likewise ambiguous.

The same explicit identity produces the same label for either team. Numeric entries
are not deduplicated or shifted into gaps. Scoring values are never subtracted or
reinterpreted as increments; the recorded samples use provider period values. This
change makes no claim that an unknown future provider format is per-period scoring.
Do not sum cached periods to overwrite a result. NHL's SO line can be its deciding
0/1 scoreboard indicator; it is not inferred to be the number of shootout attempts
or raw shootout goals. Soccer shootout values remain separate from match goals.

## Preservation and historical repair

Normal optional enrichment now requests/cache-reuses scoreboard metadata by league
and Eastern date; missing arrays are supplemented from core. Existing HTTP limits,
12-second timeouts, pacing, game caps and the 180-second stage budget remain.
There can be additional metadata requests: one scoreboard per league/date per run
and up to two core calls per game when site line arrays are absent. All HTTP calls
are paced. A metadata error preserves an existing last-good snapshot. A first-time
fetch still retains independently validated, usable team/player statistics with
unknown period labels if metadata is temporarily unavailable; a label lookup never
turns valid first-fetch statistics into an unavailable box score.

Explicit historical repair:

```sh
web_scraper --repair-boxscore-periods --dry-run --boxscores-limit 40 -o /tmp/periods-preview.xml
web_scraper --repair-boxscore-periods --boxscores-limit 500 -o /tmp/periods-repair.xml
```

This separate path ignores ordinary refresh eligibility for previously unprocessed
available snapshots, so old retained games are included. It reads existing cached
statistics and copies only period fields plus three optional root metadata fields:

- `periodLabelsVersion: 1` — the period-normalizer generation, not the payload version.
- `periodLabelSource` — `espn-scoreboard`, `espn-scoreboard+core`, or `espn-summary+core`.
- `periodLabelStatus` — `complete` or `partial` if identifiers remain ambiguous.

These optional fields are not required by the frontend. Successfully repaired
snapshots receive normal successful-fetch/attempt timestamps and six-hour refresh
eligibility. Failed requests retain payload, links, availability and successful
snapshot timestamp, with operational error/retry tracking. An SQL compare-and-swap
checks the original payload hash so repair cannot overwrite a concurrent full refresh.

Repair reruns skip processed snapshots. Partial snapshots keep original values and
can receive new labels through a later ordinary provider refresh; no labels are
fabricated to satisfy a target count. Unavailable snapshots are preserved unchanged.
The existing scheduler arguments and ingestion credentials are unchanged.

## Validation and production backfill

- `cargo test --locked`: **159 passed, 21 opt-in tests ignored**.
- Strict all-target Clippy passed; formatting and diff whitespace checks passed.
- Recorded fixtures cover MLB, NFL overtime, college football, NHL shootout, men's
  college basketball halves, MLS, NWSL, Championship, and soccer ET/penalties.
- Marked synthetic cases cover extra innings, shortened/unplayed baseball, multiple
  college-football/college-basketball and NHL playoff overtimes, WNBA quarters,
  sparse/null/zero values, missing and duplicate/unknown IDs, malformed metadata,
  and shootout ambiguity.
- Label-only/idempotence tests prove all non-label content is preserved semantically.
- An isolated SQL test verified runtime update permissions, unchanged links and
  availability, compare-and-swap success and rejection of stale concurrent writes.
  Its temporary database was removed afterward.
- A local live dry-run validated **40 games with zero errors**.

Historical repair completed at **2:59:28 PM CDT** (19:59:28 UTC). Final read-back:

- **960 available snapshots processed**, preserving all 1,355 cache identities.
- **5,356 line entries resolved**, **zero missing/blank labels**, **zero ambiguous
  entries** in the retained data, and **zero remaining cache errors**.
- All 395 unavailable snapshots remain unchanged; no unavailable statistics or
  periods were manufactured.
- Per-league repaired entry counts match the initial table: MLB 598, WNBA 60,
  college football 600, NFL 370, NHL 1,336, Championship 524, MLS 1,804, NWSL 64.

| Run | Validated/updated | Metadata failures | Outcome |
| --- | --- | --- | --- |
| 1256 | 480 | 20 | Partial, failed snapshots preserved |
| 1257 | 273 | 8 | Partial, failed snapshots preserved |
| 1258 | 179 | 0 | Succeeded |
| 1259 | 0 | 0 | Initial queue check succeeded |
| 1260 | 28 | 0 | Fallback retry succeeded |
| 1261 | 0 | 0 | Idempotent final queue check succeeded |

The initial 28 failures were college games omitted from dated scoreboards. A
verified event-summary/core fallback was added and recorded as a fixture. The
final release image dry-run validated all 28 with zero errors before retrying
production. Their retry eligibility was advanced only for the matching initial
metadata-error rows; payloads and scores were preserved until successful repair.

A protected pre-deployment cache JSON baseline was compared to final read-back.
After removing only period fields and the three documented optional root metadata
fields, **every payload matched semantically**. Cache identity sets, availability,
summary/box-score links, line counts, all display values, team/player statistics,
column definitions, totals and other payload fields were unchanged. Existing game
identity and score checksums also matched their pre-deployment baseline.

The frontend runtime's database identity read repaired labels through
`dbo.SportsBoxScores` without permission changes, including NHL's `SO` row.
Seven live box-score routes returned HTTP 200 and displayed the expected strings:
MLB innings, two NFL overtime examples, the previously omitted college-football
game, and all three retained soccer leagues. The existing frontend rendered these
labels without a deployment. This was HTTP/SQL verification, not an interactive
browser test of unrelated UI controls. The final empty repair run confirms
idempotence on processed data.

**Existing NHL frontend limitation:** `/box-score/hockey/nhl/401892525` returns 404
although its cache row is available and its labels are verified through the
frontend database identity. Read-only inspection of frontend `Data/BoxScores.cs`
shows that detail visibility accepts ordinary followed `ResultGames`, or current
season `SportsMatchResults` through explicit MLS and Championship branches. It
has no equivalent NHL season-visibility branch. NHL results here are in
`SportsMatchResults`, so the guard rejects the page before reading its snapshot.
The frontend should add equivalent enabled/current-season NHL visibility before
claiming its detail route is exposed. No frontend code or visibility policy was
changed in this collector repair.

## Frontend examples and return handoff

The existing frontend accepts string `period` labels. Baseball's summary header
and the other sports' **Scoring by period** sections can consume this repair directly.
No new periodLabel-only field or payload version was introduced.

| Sport / league / event | Labels / evidence |
| --- | --- |
| baseball / mlb / 401930515 | `1st inning` through `9th inning` |
| football / nfl / 401872988 | `Q1`, `Q2`, `Q3`, `Q4`, `OT` |
| football / college-football / 401856833 | `Q1` through `Q4` |
| hockey / nhl / 401892525 | `1st period`, `2nd period`, `3rd period`, `OT`, `SO` |
| soccer / usa.1 / 761889 | `1st half`, `2nd half` |
| soccer / usa.nwsl / 401854026 | `1st half`, `2nd half` |
| soccer / eng.2 / 401880218 | `1st half`, `2nd half` |
| basketball / mens-college-basketball / 401856600 | Recorded fixture only: `1st half`, `2nd half` |
| soccer / fifa.world / 633850 | Recorded fixture only: `1st half`, `2nd half`, `ET1`, `ET2`, `SO` |

Before: `{"period":null,"displayValue":"0"}`.
After, for an explicitly identified first inning:
`{"period":"1st inning","displayValue":"0"}`.
Player tables, totals, final scores, provider links and field names remain compatible.
New combined scoreboards, shootout presentation changes, and support for the optional
metadata fields are separate frontend work and are not claimed here.

## Deployment, backups and rollback

Protected host backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261019-period-labels/`.
It preserves the pre-change source/configuration and cache baseline. The host
checkout receives the changed implementation, SQL, fixtures and documentation;
read-back verifies that files match the workspace.

Verified copy-only database backup:
`/var/opt/mssql/backup/WebScraper_period_labels_20261019_1945.bak`.

Primary pre-change rollback container:
**web-scraper-before-period-labels-20261019**, using the previous box-score release.
Rollback image tag: **web-scraper:before-period-labels-20261019**.
The initial deployment container is additionally retained as
**web-scraper-period-labels-initial-20261019**; it predates the omitted-game fallback.
Use the primary pre-change rollback rather than confusing those two releases.

The copy-only SQL backup does not reset the existing log chain. Verification used
RESTORE VERIFYONLY WITH CHECKSUM; an actual restore was not performed. A protected
cache JSON baseline enables per-game semantic comparisons after repair.

If rollback is authorized, restore the preserved previous scraper container and
verify its health and scheduler. The version-1 labeled snapshots are compatible
with the prior frontend. Do not restore the entire shared database merely to
remove label metadata; preserve current scores, articles and other concurrent data.
No schema rollback or frontend restart is required. Git commits/pushes remain
separate from this deployment authorization.
