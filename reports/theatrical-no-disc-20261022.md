# Restore theatrical coverage without physical discs — October 22, 2026

## Cause and change

The existing DVDs Release Dates provider (source 5) was disabled because physical
DVD releases were unwanted. Its provider-wide flag also hid six retained October
23 theatrical dates. The frontend's date window was correct.

Added the independent `MovieReleaseSources.CollectDisc` preference, preserving
provider Enabled as the overall gate. The runtime skips disc calendars, filters
physical releases on every parsed detail page (including tracked pages), and
avoids tracking disc-only records under this policy. The SQL upsert enforces the
policy independently. See [configuration and frontend contract](../docs/theatrical-without-discs.md).

The additive movie schema migration and explicit disc-disable script each applied
twice successfully. Source 5 is configured for non-disc collection. Exactly 24
retained scheduled disc rows were marked withdrawn using the established public
status filter, preserving their identities, dates and provenance. Provider detail
pages confirmed all six retained theatrical dates as October 23; activation checked
those URLs/dates against the fresh preview before enabling source 5. No old dates
were silently assumed, and no theatrical row was relabelled.

## Validation and rollout

- Rust suite: 163 passed, 22 opt-in tests ignored, zero failures.
- Two rollback-only SQL fixture tests passed serially with migration credentials:
  policy rejects new/existing disc writes, disc-only tracked records are excluded,
  non-disc channels remain writable, identity repeats and rescheduling are stable,
  and disabled sources reject writes. Runtime credentials have no source-enable grant.
- Strict Clippy and whitespace checks passed.
- Unrestricted provider preview: six theatrical, seven digital, 23 disc entries.
  Disc-disabled preview: six theatrical, seven digital, zero discs, zero issues.
- First deployed all-provider dry run hit a transient request error for Sacrifice;
  retry completed with 17 eligible releases and zero issues. Existing rows were
  preserved on the failed request; no withdrawal-on-missing rule was introduced.

Image: `web-scraper:theatrical-no-disc-20261022` (final image ID prefix
`3109f49dfbf2`). The final build recompiled the reviewed sources. The running
container's scheduler command, environment, user, working directory, health check,
entrypoint, mounts, TLS certificate, network mode, restart policy, security/resource
settings and logging were compared with the prior container and preserved.
No frontend container/image changes were made.

## Production read-back and frontend

Both real deployed `--movies` imports saved 17 releases with zero issues and zero
failures. Final source success time was October 22 at **8:58:38 AM CDT**
(13:58:38 UTC); LastError was NULL for all three movie providers.

| Provider | Scheduled theatrical | Scheduled digital | Scheduled subscription | Withdrawn disc |
| --- | ---: | ---: | ---: | ---: |
| DVDs Release Dates | 6 | 3 | 0 | 24 |
| When To Stream | 0 | 2 | 5 | 0 |
| Movie Insider | 0 | 1 | 0 | 0 |

All 40 pre-change release IDs, source-link IDs, release keys, dates, source URLs
and first-seen values were preserved; one new digital release was added. Other
providers' statuses and enablement remained unchanged. The two import snapshots
had identical release-ID sets, and zero physical releases were scheduled after
either import. Non-disc source visibility and data were preserved.

The scraper's explicit health check passed and Docker reported it healthy. The
frontend remained on `mydailyfeed-web:theatrical-window-20261022` and healthy.
Original and intermediate rollback schedulers are stopped.

Playwright Chromium checked Home, `/movies?type=theatrical` and All releases at
1440×1000 desktop and 390×844 phone viewports. All pages returned HTTP 200 and
had no horizontal overflow. The Movies view showed all six confirmed titles;
Home's Friday theater section showed five, matching its existing display limit.
All releases also included all six titles with no physical-disc badges. Existing
poster placeholders remain where four movies have no matched poster; enrichment
preserved existing posters rather than inventing matches. Browser checks cover
rendering, not ticket checkout or saved-item persistence.

## Recovery

A fresh COPY_ONLY full WebScraper backup passed RESTORE VERIFYONLY before changes.
Protected host deployment directory:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261022-theatrical-no-disc/`.
It contains runtime recovery configuration and source snapshots; `baseline-git/`
separately records the committed pre-change source baseline. Final source files
were copied to the host checkout and byte-compared with the workspace.

Original rollback container: `web-scraper-before-theatrical-no-disc-20261022`,
image `web-scraper:boxoffice-status-20261022`. An intermediate deployment container
is retained as `web-scraper-theatrical-pre-final-20261022`; use the original for
pre-change rollback. Only the final `web-scraper` scheduler should be running.

Before restoring an old image, disable source 5 because older collectors ignore
CollectDisc and may collect physical discs again. Leave the additive column and
withdrawn rows in place; do not restore the shared database. Preserve unrelated
providers and their public visibility. This restores the safe prior disabled
state while keeping the new schema available for a forward fix.
