# SQL log maintenance — October 13, 2026

Completed approximately 2:54 p.m. Central.

Before: WebScraper_log 2,248 MB, approximately 2,201 MB used, FULL recovery,
log_reuse_wait_desc=LOG_BACKUP. SQL Agent was stopped; host crond was running.

Two CHECKSUM/compressed transaction-log backups were taken and verified. The first
released reusable log space, but active tail segments prevented file shrink.
After the second backup, one-time DBCC SHRINKFILE reclaimed the inactive tail.

After: log file 264 MB, approximately 1.2 MB used, log_reuse_wait_desc=NOTHING.
Data file remains 72 MB. File growth is fixed at 64 MB. FULL recovery is retained.
The file reduction is 1,984 MB; approximately 620 MB of new verified backups were
preserved, giving roughly 1,364 MB (1.33 GiB) net initial disk improvement.

Installed host cron: verified log backups every 15 minutes, COPY_ONLY full backups
at 2:35 a.m. Central daily. Managed full/log files rotate after eight/ten days.
Both scheduled runner commands were executed successfully and verified against
backup history. Cron is installed; no SQL Server restart or SQL Agent enablement
was required. Initial/manual backup files are excluded from managed rotation.

Policy and operations: [sql-backups.md](../docs/sql-backups.md).
No scraper/web-app rebuild, recovery-model change, commits or pushes performed.
