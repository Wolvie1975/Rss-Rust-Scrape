# Production data retention

Effective October 13, 2026. Cleanup runs after every non-dry-run collection,
including runs with individual provider failures. A SQL transaction removes
child records before parent records. Today is determined in America/Chicago.

| Data | Rule |
| --- | --- |
| News articles | Newest 10 per source; older rows deleted (unchanged) |
| YouTube videos | Newest 10 per channel; older rows deleted |
| Sports events | Delete dates before today and canceled/unnecessary games, with their source observations |
| Movie releases | Delete release dates before today; remove orphan poster caches, source links and movies |
| TV episodes | Delete dates before today; timestamped episodes use their Central date |
| Rankings | Keep only each source's current poll and current team rows; replace atomically after successful import |
| Scraper runs | Delete finished runs started more than 7 days ago |
| Standings | EPL retains only current rows; other leagues retain existing history |
| Championship results | Current season, separately from schedules; old seasons removed after a successful new-season result import |

Today's records remain even if their start time has passed. Unknown/TBD movie
release dates and TV episode dates remain because they cannot be classified as
past. TV tracking settings and their series metadata remain so future episodes
can continue to be discovered. Source/feed configuration is not removed.

Rankings retain the last good poll if fetching, validation or saving its replacement
fails. Corrections to the same poll update existing records. These rules delete
records rather than moving them into an archive. Database backups are separate.

Implementation: sql/retention.sql, Db::apply_retention, and end-of-run cleanup.
Ranking rollover also deletes obsolete polls in its existing save transaction.
The hourly command explicitly sets --youtube-latest 10; the CLI default is also 10.
Runtime DELETE grants cover only the affected tables. Retention skips dry runs.

Validation: tests/sql/retention.sql runs cleanup inside an outer transaction,
asserts current/future and tracking preservation, then rolls back. Run with the
appropriate connection profile; it is not an instruction to commit test deletions.
