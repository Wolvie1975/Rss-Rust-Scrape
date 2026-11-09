# Digital / VOD provider handoff — October 21, 2026

The user requested that we skip **DVDs Release Dates** and focus on digital releases. The production scraper configuration has been updated. The frontend should respect the enabled provider list when choosing sources and displaying release data.

## Completed backend change

Verified production state in `WebScraper.dbo.MovieReleaseSources`:

| SourceKey | Provider | Enabled |
| --- | --- | --- |
| `dvdsreleasedates` | DVDs Release Dates | No |
| `movieinsider` | Movie Insider | Yes |
| `whentostream` | When To Stream | Yes |

Applied this configuration update:

```sql
UPDATE dbo.MovieReleaseSources
SET Enabled=0
WHERE SourceKey='dvdsreleasedates';
```

This disables the entire DVDs Release Dates provider, including its digital, theatrical and disc observations. Existing stored movies, source links and release records were preserved. No schema migration, scraper deployment or scheduler restart was performed. Existing hourly `--movies` collection reads enabled providers and excludes disabled providers' tracked URLs.

## Frontend behavior

- Keep **Movie Insider** as the preferred Digital / VOD provider when enabled. Keep **When To Stream** available as the alternate.
- Populate the provider selector from enabled sources. Remove DVDs Release Dates from active choices while it is disabled.
- Filter release queries by `s.Enabled=1` and the selected `s.SourceKey`; historical DVDs Release Dates records must not reappear through a fallback or combined query.
- If a saved URL or preference selects a disabled provider, resolve to an enabled provider, preferring Movie Insider, and keep the visible label and URL selection consistent. Show an empty state if no enabled provider exists.
- Preserve the current **from today onward** date window seen on the live Digital / VOD page. This task does not request a date-window change.
- Keep digital rental/purchase separate from subscription streaming. Restrict this view to `digital`, `digital_purchase` and `digital_rental` releases.
- Base refresh warnings on the selected enabled provider's `LastError`. An error belonging to a different or disabled provider should not mark Movie Insider as failed.

The earlier [movie guide](../docs/movies-and-box-office.md#my-newsfeed-digital-releases--frontend-implementation-request) includes the table joins and field contract. Its suggestion to offer DVDs Release Dates and its older current-week date window are superseded by this handoff for this view.

## Warning investigation and verification

The October 21, approximately 8:03 AM America/Chicago Movie Insider collection attempt failed to fetch its digital calendar. A manual retry subsequently fetched Movie Insider successfully and saved successful movie results. The separate DVDs Release Dates request for an Abbott Elementary detail page still failed during that retry, so the overall manual movie command returned a partial-failure status even though Movie Insider succeeded.

After the retry, the live homepage and `/movies/digital` no longer contained the “Latest collection failed” warning. The digital page showed **Last checked Oct 21, 2026, 8:35 AM (UTC-5)** and one upcoming Movie Insider release, **Junk**, dated October 23. These are observations from that check, not permanent expected counts.

The provider disablement was applied after that retry and verified by reading back all three provider flags. A post-disablement movie collection and frontend selector change have not been performed in this task.

## Frontend acceptance checks

1. Digital / VOD defaults to enabled Movie Insider and offers enabled When To Stream.
2. DVDs Release Dates is absent from active provider choices and release results.
3. An old selection of the disabled provider resolves consistently to an enabled provider.
4. A disabled provider's old `LastError` does not trigger a Movie Insider warning.
5. Movie Insider with no matching upcoming rows shows an honest empty state; retained historical rows are not silently mixed into its results.
6. Upcoming dates use the site's America/Chicago calendar date. SQL `DATE` release values are displayed without UTC conversion; UTC collection timestamps are converted for display.
