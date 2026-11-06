# Scraper report: compatibility with myNewspaper profiles

Answers [profiles-scraper-handoff.md](profiles-scraper-handoff.md). Checked October 27, 2026 (America/Chicago)
against Rss-Rust-Scrape `main` at `fde6d4c` and production **WebScraper**.

## Result

**All six checks pass. No scraper changes are needed**, and 020 can keep `ON DELETE CASCADE` on all three foreign
keys. Nothing in the scraper deletes and reinserts either table on purpose. No scraper code or database objects were
changed for this review; the production queries were read-only catalog checks.

## Checks

| # | Check | Result | Where we looked |
|---|---|---|---|
| 1 | No `TRUNCATE` on `TvTrackedSeries` or `YoutubeVideoFeed` | **Pass** | Whole repository: `src/`, `sql/`, `scripts/` (including the SQL backup cron), `examples/`, `tests/`. There is no `TRUNCATE` statement on any table. |
| 2 | No delete-and-reinsert | **Pass** | No `DELETE` targets either table anywhere. **TV:** the import only updates rows, keyed by `TvmazeShowId`: the claim and failure statements in [src/db.rs](../src/db.rs) and the success path in [sql/upsert_tvmaze_snapshot.sql](../sql/upsert_tvmaze_snapshot.sql). **YouTube:** `ENSURE_YOUTUBE_FEED_SQL` in [src/db.rs](../src/db.rs) looks the channel up by `ChannelId`, inserts only when it is missing, and otherwise only fills in a missing `ChannelName`. [sql/youtube_categories_seed.sql](../sql/youtube_categories_seed.sql) updates in place. Nothing replaces a feed when its title changes. |
| 3 | No `MERGE … WHEN NOT MATCHED BY SOURCE THEN DELETE` | **Pass** | The only `MERGE` is in [sql/tvmaze_seed.sql](../sql/tvmaze_seed.sql). It is still `MERGE dbo.TvTrackedSeries WITH(HOLDLOCK)`, has only `WHEN NOT MATCHED THEN INSERT`, and leaves existing rows untouched, including `Enabled`. |
| 4 | Explicit column lists; no `SELECT *` | **Pass** | The one `INSERT` into `YoutubeVideoFeed` lists `(ChannelId, ChannelName, Url)`. Every read names its columns (`SELECT ID, Url`, `SELECT TvmazeShowId, ExpectedTitle`, the TV audit tool's list). The `SELECT *` uses in the repository are on other tables, temp tables or `OPENJSON`. There is no bulk insert or bcp. |
| 5 | Schema scripts leave web additions alone | **Pass** | Both `CREATE TABLE` statements are guarded by `IF OBJECT_ID(...) IS NULL` ([src/db.rs](../src/db.rs) for `YoutubeVideoFeed`, [sql/tvmaze.sql](../sql/tvmaze.sql) for `TvTrackedSeries`). The only `ALTER` on either table adds `YoutubeCategoryId`, guarded by `COL_LENGTH` ([sql/youtube_categories.sql](../sql/youtube_categories.sql)). There is no `DROP`, `sp_rename`, copy-table rebuild, or removal of columns or constraints the scraper does not own. |
| 6 | Key column is `ID` | **Pass** | Production catalog: `PK_TvTrackedSeries` and `PK_YoutubeVideoFeed` are both on `ID` (`INT IDENTITY`). |

## Behavior the web relies on

All of it is in place today.

- **`TvTrackedSeries.Enabled = 0` stops collection and keeps history.** Claiming a show requires `Enabled = 1`. A show
  disabled mid-run finishes as `disabled_or_superseded` without writing anything. Retention
  ([sql/retention.sql](../sql/retention.sql)) removes only `TvSeries` rows that no `TvTrackedSeries` row references,
  and disabled rows still reference theirs. Retention's normal removal of past episodes is unchanged.
- **The lease.** A claim sets `LeaseUntil` to now plus 10 minutes and a new `LeaseToken`. Success and failure both
  clear them.
- **New rows are picked up on the next run.** TV reads every `Enabled = 1` row each run, YouTube reads every
  `YoutubeVideoFeed` row, and news reads every enabled `Sources` row.

## Other notes

- **Tests.** Live database tests are `#[ignore]` opt-ins. The two that connect to `WebScraper` by name touch only
  movie tables, inside rolled-back transactions. The TV tests use an isolated `WebScraper_Tvmaze_Validation_*`
  database.
- **Production state before 020.** `IsHouseholdPick` doesn't exist yet. Neither table has triggers. `YoutubeVideoFeed`
  already has one incoming key, `FK_YouTubeVideos_YoutubeVideoFeed` (`NO ACTION`), so deleting a feed that has stored
  videos already fails today.
- **Schema snapshot.** [sql/schema/WebScraper.schema.sql](../sql/schema/WebScraper.schema.sql) is a creation snapshot
  for new installs and is never run against production. If it is regenerated from live metadata after 020, it will
  include the web's `MyDailyFeed*` tables and `IsHouseholdPick`. Whoever regenerates it should decide whether to keep
  or remove those web objects.
