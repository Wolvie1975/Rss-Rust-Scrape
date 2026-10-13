# WebScraper SQL backup maintenance

Configured October 13, 2026. The database remains in FULL recovery mode.

Docker host cron: /etc/cron.d/webscraper-sql-backup.
Host runner: /usr/local/sbin/webscraper-sql-backup.

- Transaction-log backups every 15 minutes.
- COPY_ONLY full backups daily at 2:35 a.m. America/Chicago.
- CHECKSUM and compression enabled; each backup is checked with RESTORE VERIFYONLY.
- Backup commands run inside sql2025 using its existing local administrative
  credentials. No credentials are embedded in cron, repository files or command
  arguments, and the scraper's restricted database account is unchanged.
- Managed backups live on the SQL volume under
  /var/opt/mssql/backup/webscraper-managed/{full,log}.
- Successful full jobs rotate managed full files aged at least eight days and log
  files aged at least ten days. Extra log retention preserves the chain for retained
  full backups. Initial/manual backups outside these directories are untouched.
- A host flock prevents overlapping managed jobs. Failures reach
  /var/log/webscraper-sql-backup.log. These are local backups, not off-host copies.

Manual checks on the Docker host:

```bash
sudo /usr/local/sbin/webscraper-sql-backup log
sudo /usr/local/sbin/webscraper-sql-backup full
sudo tail -n 40 /var/log/webscraper-sql-backup.log
```

Deployment performed a one-time log shrink after verified transaction-log backups,
reducing WebScraper_log from 2,248 MB to 264 MB. Fixed growth remains 64 MB.
No automatic shrink is scheduled. Data files were not shrunk and SQL Server was
not restarted. SQL Agent was stopped, so the existing running host cron service
schedules maintenance without changing SQL Server service settings.

The initial log backups contain approximately 610 MB of compressed recovery data
and were preserved. The first managed full backup was approximately 9.9 MB.
The file shrink reclaimed 1,984 MB; subtracting newly created backups, the initial
net space improvement was approximately 1,364 MB (1.33 GiB).

Reference: [Microsoft guidance on transaction-log file size](https://learn.microsoft.com/en-us/sql/relational-databases/logs/manage-the-size-of-the-transaction-log-file).
RESTORE VERIFYONLY checks backup readability and checksums; it does not substitute
for a complete restore drill. The daily job's 2:35 a.m. time can be skipped on the
spring DST transition; log backups continue and full retention allows for that gap.
