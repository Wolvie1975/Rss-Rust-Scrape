# Code structure cleanup deployment — October 25, 2026 CDT

Code-only release of commits `8dfe607`..`d302c7d` (library crate, split
`run_once`, `db.rs` test/SQL-const separation, refreshed Cargo.lock for
RUSTSEC-2026-0382/0383/0386). No schema, SQL text, configuration or data change.

| Service | New image | Image ID | Rollback container (stopped) |
| --- | --- | --- | --- |
| web-scoreboard | `web-scoreboard:code-cleanup-20261025` | `c5315fb01cd4` | `web-scoreboard-before-code-cleanup-20261025` (`web-scoreboard:previous-day-20261023`) |
| web-scraper | `web-scraper:code-cleanup-20261025` | `d32a5bb2b850` | `web-scraper-before-code-cleanup-20261025` (`web-scraper:theatrical-no-disc-20261022`) |

## Procedure

- Host checkout build inputs were confirmed identical to the previously deployed
  commit `71d60b6` before syncing. The 48 replaced files were saved to the protected
  `.deployment-backups/20261025-code-cleanup/` (plus the list of 3 added files); all
  51 changed files were then byte-compared with the commit.
- Images were built through the Docker API from the committed tree (no `.env`/certs
  in the context; image env is `PATH` only), capped at 2 GiB build memory.
- Each container was recreated from the live container's own configuration with
  only the image changed. Command, environment, user, health check, mounts,
  networks and all host limits/security settings were verified identical.
- The scraper was swapped at 05:56 CDT, between hourly runs.

## Verification

- Before deploy: fmt, strict Clippy, 171 tests (194 unique), `cargo audit` clean; a
  live `--dry-run` with production options matched the previous release line for
  line apart from live-data noise (235 successes, 1 transient YouTube failure each).
- web-scoreboard: healthy; first cycles read coverage and correctly found no due work.
- web-scraper: first production run 06:00–06:05 CDT: **266 source successes,
  1 failure**, identical to the previous release's 04:05 and 05:04 runs. The failure
  is pre-existing: TVmaze 68752 *High Potential* returns an episode list missing
  stored episode IDs, so the database guard preserves the last-good snapshot.
  Retention ran normally. Container healthy.

## Rollback

Stop and remove the new container, rename the `-before-code-cleanup-20261025`
container back to its original name and start it. No data or schema needs
reverting. Restore host files from `.deployment-backups/20261025-code-cleanup/`
if the host checkout should match the old release.
