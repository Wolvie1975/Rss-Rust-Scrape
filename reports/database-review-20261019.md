# WebScraper database review — October 19, 2026

Reviewed the live WebScraper database around **10:34–10:39 AM CDT**
(15:34–15:39 UTC). The only maintenance mutation was refreshing optimizer
statistics. No application records, schema objects, indexes, credentials or
retention settings were deleted or changed. No deployment/restart was required.

## Completed maintenance

Refreshed **38 stale statistics across 13 tables**, using targeted
`UPDATE STATISTICS ... WITH FULLSCAN, MAXDOP=1` with a ten-second lock timeout.
The selected statistics had more than 1,000 modifications and were either older
than one day or had more modifications than rows in their previous snapshot.
Several sports statistics dated October 5–13 and had hundreds of thousands of
modifications recorded. Auto-created statistics can remain old when they are not
used; this maintenance is not evidence of a measured application slowdown.

| Table | Statistics refreshed |
| --- | --- |
| SportsEvents | 12 |
| SportsMatchResults | 5 |
| SportsEventSources | 4 |
| Pages | 3 |
| ResultFollows | 3 |
| SportsStandings | 3 |
| ResultGames | 2 |
| MovieReleases | 1 |
| Sources | 1 |
| SportsEventsType | 1 |
| SportsRankingPolls | 1 |
| SportsRankingSources | 1 |
| YouTubeVideos | 1 |

All refreshed statistics showed zero subsequent modifications immediately after
execution. The selection query found no remaining statistics meeting the same
threshold during read-back. This refresh updates query-planning metadata; actual
query latency improvements were not benchmarked.

Repeatable, **preview-first** maintenance:
[`sql/maintenance_refresh_stale_statistics.sql`](../sql/maintenance_refresh_stale_statistics.sql).
It defaults to `@Execute=0`; review its targets before setting it to 1 for another
authorized run. It is not installed as a scheduled task.

## Integrity and data quality

- Full `DBCC CHECKDB (N'WebScraper') WITH NO_INFOMSGS, ALL_ERRORMSGS` completed
  without reported errors. No repair option was used.
- All foreign keys and CHECK constraints were enabled and trusted. No disabled
  indexes were found.
- No article rows referenced missing sources; no news sources were unassigned.
- No orphaned movie source links, movies, poster links or untracked empty TV series
  were found.
- No invalid sports observation or box-score JSON payloads were found.
- All **1,355** optional box-score cache identities referenced completed games in
  the result models; no cache rows required orphan cleanup.
- Enabled news sources had no recorded errors or refresh timestamps older than
  one day. This is a stored-health review, not a fresh live-feed fetch of every URL.

Three sports observations have NULL SportsEventId. These are intentionally
unlinked **future** provider records, not dangling non-NULL references:

| Observation ID | Publisher school | Future event date | Existing validation issue |
| --- | --- | --- | --- |
| 497 | Colorado | November 25 | Self-opponent provider record |
| 857 | Utah | November 23 | Member opponent has no verified game alias |
| 1243 | Iowa State | December 10 | Self-opponent provider record |

All three were recently observed. They were preserved, with their existing issue
text; guessing an opponent or deleting provider evidence would weaken the current
identity rules. There were **zero genuine orphan observations** with non-NULL
SportsEventId pointing to a missing event.

## Retention review

Every existing-policy candidate count below was **zero**, both before and after
maintenance, so no application-data deletions were necessary:

- Past/canceled/unnecessary sports schedules.
- Past movie releases and past TV episodes, using America/Chicago dates.
- Finished run logs older than seven days.
- Non-current ranking polls or ranking team rows.
- Cinema posters unused for more than 90 days.
- Expired box-office forecasts.
- News articles exceeding ten per source.
- YouTube videos exceeding ten per channel.

Source configuration, retained results, article aliases/redirects and TV tracking
were preserved. Current counts included 581 articles, 200 videos, 2,912 schedule
rows, 1,495 ResultGames (including nonfinal games), 783 SportsMatchResults and
1,355 optional box-score cache rows. Existing hourly retention is keeping up.

## Storage and backups

| Item | Observed size/state |
| --- | --- |
| Data file allocated | 136 MB |
| Data file used | About 116.5 MB |
| Log file allocated | About 264 MB |
| Log used at review | About 14.7 MB / 5.6% |
| Recovery / page verification | FULL / CHECKSUM |
| Log reuse wait | NOTHING |
| Box-score cache reserved | About 21.4 MB |
| Sports source observations reserved | About 12 MB |
| Query Store storage | 59 MB, read/write |

Query Store accounts for roughly 59 MB of internal storage, with 790 captured
queries and 1,115 plans at review. Its existing 30-day cleanup policy, automatic
size cleanup, automatic capture and 1,000 MB maximum were intact; no forced plans
were present. This history was retained for troubleshooting. No Query Store
configuration change or blanket purge was warranted by the observed sizes.

The unused data/log capacity is reusable by SQL Server. No shrink, recovery-model
change, log-chain reset or blanket index rebuild was performed. Small indexes
were not rebuilt merely for cosmetic fragmentation improvements.

Backup history showed a checksummed full backup at **2:35 AM CDT** and a
checksummed log backup at **10:30 AM CDT** on October 19. This review checked SQL
backup history; it did not independently restore those backup files, re-run
RESTORE VERIFYONLY, or verify off-host copies. Existing backup operations are
documented in [the operations guide](../docs/operations.md#sql-backups).

## Follow-up requiring coordinated application work

The frontend's actual database connection resolves to **dbo** and its login has
**sysadmin** membership (`IS_SRVROLEMEMBER('sysadmin')=1`). The scraper runtime uses
its separate ingestion role. Moving the frontend to a least-privilege login is a
concrete hardening opportunity, but requires inventorying its reads, admin writes,
startup schema work and deployment configuration. No login/password or permission
changes were made as part of data cleanup.

The three unresolved future sports observations should reconcile through their
existing provider-validation path when correct data arrives. No manual merge or
fabricated game metadata was applied.

## Final verification

The scraper's explicit health check passed and scraper/frontend containers were
healthy after maintenance. Existing source code changes and the prior deployment
were preserved. This report and the preview maintenance script are recorded in
both workspace and host checkout; Git commits/pushes remain with the user.
