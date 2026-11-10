# Scraper task: check compatibility with myNewspaper profiles

Prepared October 27, 2026 (America/Chicago). Companion to `new development/profiles-handoff.md`.

## Objective

The web app is adding household profiles. Its database script `db/020_profiles.sql` creates web-owned tables that
**point at two scraper-owned tables** with foreign keys, and adds one web-owned column to a scraper-owned table. Before
020 runs, confirm the scraper won't break those links, and won't wipe out data through them.

**Most likely no scraper code changes are needed.** This is mainly a set of checks. Change scraper code only where a
check fails, and report every result either way.

This is a task handoff for the separate scraper project (Rss-Rust-Scrape), not a report of implementation. Nothing in
the scraper has been inspected or changed for this request.

## What 020 changes in the shared database

| Object | Owner | Change |
|---|---|---|
| `dbo.TvTrackedSeries` | scraper | New web table `MyDailyFeedProfileTvFollows` references `TvTrackedSeries(ID)` with `ON DELETE CASCADE` |
| `dbo.YoutubeVideoFeed` | scraper | New web tables `MyDailyFeedProfileChannelFollows` and `MyDailyFeedChannelRollouts` reference `YoutubeVideoFeed(ID)` with `ON DELETE CASCADE` |
| `dbo.YoutubeVideoFeed` | web-owned column | New column `IsHouseholdPick bit NOT NULL DEFAULT 0` (like `SourceCategories.NewsHub`) |
| `dbo.MyDailyFeed*` | web | New tables. The scraper never reads or writes them |

**What follows from the cascades:** deleting a row from `TvTrackedSeries` or `YoutubeVideoFeed` now also deletes
every profile's follow of it. A delete that's really a "replace" (delete, then insert the same thing again) silently
unfollows that show or channel for everyone, because the new row gets a new `ID`.

## Required checks

Do all of these before 020 runs (web release 1).

1. **No `TRUNCATE`** on `TvTrackedSeries` or `YoutubeVideoFeed`, anywhere: collectors, maintenance jobs, scripts in
   `sql/`, tests that run against the shared database. SQL Server refuses to truncate a table that a foreign key points
   at, so a `TRUNCATE` would start failing once 020 runs.
2. **No delete-and-reinsert.** Updates to existing rows must be `UPDATE` or `MERGE … WHEN MATCHED THEN UPDATE`, so
   `ID` stays the same. Look in particular for "sync" code that clears the table and reloads it, or replaces a feed
   when its channel ID or title changes.
3. **No `MERGE … WHEN NOT MATCHED BY SOURCE THEN DELETE`** on either table. `docs/entertainment.md` in the web repo shows
   a `MERGE dbo.TvTrackedSeries WITH(HOLDLOCK)`; confirm whether that's still how the scraper writes it, and that it has
   no delete clause.
4. **Explicit column lists** on every `INSERT` into `YoutubeVideoFeed`, and no `SELECT *` read by column position.
   Bulk inserts must map columns by name. The new `IsHouseholdPick` column has a default, so explicit column lists need
   no change.
5. **Schema scripts leave the web additions alone.** Nothing in the scraper's `sql/` folder or migrations drops and
   recreates `TvTrackedSeries` or `YoutubeVideoFeed`, rebuilds them through a copy table, or removes columns and
   constraints it doesn't own (`IsHouseholdPick`, `DF_YoutubeVideoFeed_HouseholdPick`, and the incoming foreign keys
   `FK_MyDailyFeedProfileTvFollows_Series`, `FK_MyDailyFeedProfileChannelFollows_Feed` and
   `FK_MyDailyFeedChannelRollouts_Feed`).
6. **Key column names.** The web models map both tables' keys as `ID`. Confirm the actual column name is `ID` on both,
   since 020's foreign keys reference it.

## Behavior the scraper already has that must stay the same

- **`TvTrackedSeries.Enabled = 0` means stop collecting and keep the history.** The web will now toggle `Enabled` more
  often: when a profile's last follower unfollows a show, the web sets it to 0, and a new follow sets it back to 1.
- **The lease.** The web's admin Delete only removes a `TvTrackedSeries` row whose `LeaseUntil` is null or past. Keep
  setting `LeaseUntil` and `LeaseToken` while importing.
- **New rows the web inserts are picked up on the next run**, as they are today when the admin adds them:
  - `TvTrackedSeries`, when someone follows a new show
  - `YoutubeVideoFeed`, when the admin approves a channel request
  - `Sources` with `Enabled = 1`, when the admin approves a news-source request
- **Feeds the scraper adds to `YoutubeVideoFeed` by itself are fine.** The web now turns them on for every profile
  automatically. Keep their `ID` stable afterwards (check 2).

## Not needed

- No new scraper tables, columns or collectors.
- The scraper doesn't need to know about profiles, follows or requests.
- No permission changes for the scraper's login. It never touches the `MyDailyFeed*` tables directly.

## Report back

For each of checks 1 to 6, give: pass or fail, and where you looked (files, SQL scripts, jobs). If something failed,
say what you changed, how it was tested, and the scraper version or commit that has the fix. If anything does
delete-and-reinsert on purpose and can't easily change, say so before the web runs 020, so the plan can switch those
foreign keys to `ON DELETE NO ACTION` or drop them.
