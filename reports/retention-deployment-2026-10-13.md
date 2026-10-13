# Retention deployment — October 13, 2026

Deployed approximately 2:46 p.m. Central as web-scraper:retention-20261013
(also latest), image sha256:8f581316006d45354ea03c354f8781c2d26b2b0262b5d07fad7d23623d615884.

Policies are documented in [data-retention.md](../docs/data-retention.md).
Runtime cleanup now removes past sports and canceled games, past movie releases
and episodes, orphan movie metadata, old ranking polls and finished run logs
older than seven days. Today is retained using America/Chicago; undated/TBD
records remain. TV tracking settings survive. News remains 10/source. YouTube
CLI default and hourly command now keep 10/channel.

Rankings remove prior polls within the replacement transaction, after a successful
save. Fetch/validation/save failures retain the last good current poll. Scoped
DELETE permissions were granted on the affected existing tables; no schema or
server-role changes were made. Cleanup is transactional and skips dry runs.

Verification:

- 97 binary tests passed; 14 integration tests skipped in the default suite.
- Isolated ranking SQL test passed, including failed replacement rollback,
  successful rollover deleting old polls, and rejection of stale responses.
- Retention SQL ran inside an outer rollback transaction under both migration
  and restricted runtime credentials. Current/future records, undated records
  and tracking configuration survived; old records were removed in the transaction.
- Clippy with warnings denied and formatting checks passed.
- Production targeted ranking/YouTube collection and cleanup succeeded (run 73).
- Counts: sports 981 → 937; movie releases 140 → 50; episodes 90 → 31;
  ranking polls 8 → 5. Every one of 15 YouTube channels has 10 videos.
- Audit reports zero expired sports, releases, episodes, old polls or old finished
  run logs. Scraper is healthy; restart count is zero. No SQL Server restart.

Verified COPY_ONLY backup with CHECKSUM:
/var/opt/mssql/backup/WebScraper_before_retention_20261013.bak.
Rollback image: web-scraper:before-retention-20261013.
Previous container: web-scraper-rollback-retention-20261013 (stopped).
Host source backup: .deployment-backups/20261013-retention/source.tar.
Binary rollback stops the new retention behavior; deleted data requires restoring
from the database backup if recovery is needed. The temporary isolated ranking
validation database was removed after testing. No commits or pushes performed.
