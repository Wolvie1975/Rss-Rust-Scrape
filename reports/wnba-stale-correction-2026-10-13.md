# WNBA correction and stale schedule review — October 13, 2026

The user authorized production updates and a quick review of other stale entries.

## Production correction

Public event rows 15409 and 15410 (ESPN IDs 401918303 and 401918304) are confirmed
unnecessary October 14 semifinal Game 5 fixtures. Both semifinal series ended
before those games were needed, as confirmed by the official Finals matchup.
The current ESPN scoreboard omits both IDs and their summaries return 404.

A guarded SQL transaction set only these two rows to STATUS_CANCELED, the existing
inactive status recognized by the site's upcoming-event queries. This is a local
administrative correction for unnecessary fixtures, not a new ESPN observation.
No rows were deleted; event dates, IDs, FirstSeenAt and LastSeenAt were preserved.
The existing frontend excludes both fixtures. The next active WNBA event is
October 17 at 19:30 UTC (2:30 p.m. Central), Atlanta at Golden State.
No binary, schema, permissions or web-app changes were needed.

Prior row snapshots: [backup JSON](wnba-stale-correction-before-2026-10-13.json).
Rollback, if explicitly needed: restore ScheduleStatus='STATUS_SCHEDULED' for
ProviderKey='espn-wnba', GameId IN (401918303,401918304), preserving other fields.

## Quick review scope and result

Scanned all 981 stored sports rows for upcoming active events whose last observation
predated the feed's latest successful refresh. Only the two corrected WNBA games
matched that stale-refresh pattern.

Compared stored active events dated October 13–November 30 with all 19 live schedule
feeds: 16 Big 12 school RSS feeds, MLB and WNBA monthly ESPN scoreboards, and Kansas
City Current's NWSL scoreboard (filtered to its stored team). Every provider request
completed successfully. Only the two corrected WNBA fixtures were absent; no other
missing upcoming entries were found in this window. The comparison used the
pre-correction snapshot, so those two IDs intentionally appear in its findings.
This was a schedule review, not an audit of news, movies, TV or standings; dates
after November 30 were screened by refresh timestamps but not live-cross-checked.

The importer still retains omitted provider fixtures. This targeted data correction
does not implement automatic withdrawal reconciliation for future conditional games.
No commits or pushes performed.
