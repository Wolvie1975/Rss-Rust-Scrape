# 2026 NWSL league-wide schedule

The NWSL collector now imports every club, including Kansas City Current. The
existing production source **4191** is expanded in place; existing game IDs and
first-seen timestamps are preserved. This is schedule coverage, not a new results
collector. Existing completed-results synchronization remains independent.

## Source and scope

- Provider: `espn-nwsl`; sport: `Soccer`.
- Source URL: `https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard`.
- No `team` parameter means all teams. Explicit single-team feeds still work;
  comma-separated filters include games involving any selected team.
- Existing retention remains today/future fixtures, with a seven-day reconciliation
  window for recent reschedules/cancellations. Past games are not backfilled into
  the schedule table. A 2026 page filters local `EventDate` to that calendar year.
- Without an explicit `season`, collection scans the current reconciliation month
  through the end of next year, as before. `?season=2026` limits request years.

Live October 2026 provider data and parser fixtures cover all **16 teams**, including
Boston Legacy FC and Denver Summit FC. Official league context:
[2026 NWSL schedule footprint](https://www.nwslsoccer.com/news/nwsl-announces-2026-schedule-footprint).
Do not hardcode a Kansas City-only team list in the page.

## Identities and display

Store one row per match, not one per participating team. Existing
`CanonicalKey='espn:usa.nwsl:<GameId>'`, ID-based ESPN URLs and event IDs are
unchanged. League-wide rows use the **home team** as `SchoolName` / `SchoolId` and
away team as `Opponent` / `OpponentSchoolId`. `IsAway=false` except neutral-site
rows (`IsAway=NULL`, `IsNeutral=true`). Team logo fields follow that perspective.
Explicit single-team feeds keep their selected team's home/away perspective.

Filtering Kansas City now means checking **either** team ID for `20907`, since an
away Current game has its home opponent in `SchoolId`. This avoids hiding away
games or duplicating fixtures. Dates are Central calendar dates; starts are UTC.
Do not display a kickoff when `TimeTbd=1`.

```sql
DECLARE @Today date=CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC'
                                             AT TIME ZONE 'Central Standard Time');
SELECT e.ID,e.GameId,e.EventDate,e.StartsAtUtc,e.TimeTbd,
       e.SchoolId,e.SchoolName,e.OpponentSchoolId,e.Opponent,e.IsAway,e.IsNeutral,
       e.Location,e.Tv,e.StreamUrl,e.LiveStatsUrl,e.TeamLogoUrl,e.OpponentLogoUrl,
       e.ScheduleStatus,e.Url
FROM dbo.SportsEvents e
JOIN dbo.SportsEventsType s ON s.ID=e.SportsEventsTypeId
WHERE e.ProviderKey='espn-nwsl' AND s.ProviderKey=e.ProviderKey AND s.Enabled=1
  AND e.EventDate>=@Today AND e.EventDate>='20260101' AND e.EventDate<'20270101'
  AND (e.ScheduleStatus IS NULL OR
       (e.ScheduleStatus NOT IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_FULL_TIME')
        AND e.ScheduleStatus NOT LIKE 'STATUS_FINAL%'))
  AND (@TeamId IS NULL OR e.SchoolId=@TeamId OR e.OpponentSchoolId=@TeamId)
ORDER BY e.EventDate,e.StartsAtUtc,e.ID;
```

Use a nullable parameterized integer for `@TeamId`, or omit that predicate for an
all-team page. Keep the existing page's date-window controls; the public Events
page currently shows today through the next three days, so not all season games
appear on that view at once.

## Source setup and frontend compatibility

`--configure-nwsl` explicitly expands the old default Current source in place,
changes its URL/display name to league-wide NWSL and preserves Enabled and ID.
New installations create a disabled league source. Repeated configuration preserves
operator settings. Existing filtered custom sources are not bulk-enabled.

Production retains the legacy source's **ProviderSchoolId=20907** compatibility
tag because the deployed frontend requires that tag to recognize NWSL sources.
This tag is **not** a collector filter: the URL determines schedule scope. The
league source's display name is NWSL and SchoolName is null. No frontend source,
image, or unrelated in-progress frontend edits were changed for this expansion.

For a future frontend cleanup, accept enabled NWSL sources with ProviderSchoolId
`0` (new league installs) or `20907` (legacy source), label the source **NWSL**
in `EventsQuery.GetSchoolsAsync`, and remove the Kansas City-only preference
coverage check in `ScheduleCoverage` so other NWSL team follows count as supported.
Keep actual match/team filtering on the two event team IDs. The public league
schedule already renders other teams with the retained compatibility tag.

## Validation and rollback

162 offline tests and strict Clippy pass. The isolated NWSL SQL test verifies
legacy-source promotion without changing its ID or Enabled, 24-match league
expansion, repeated import identities, existing Current event IDs/first-seen,
reschedules and cancellation behavior. The validation database is removed after
operator verification. Repeat manually only with isolated migration credentials:

```sh
NWSL_TEST_PROFILE=.env.migrations \
NWSL_TEST_DATABASE=WebScraper_Nwsl_Validation_example \
cargo test --locked --lib db::tests::nwsl_tests -- --ignored --nocapture
```

Production imported **24 upcoming fixtures across all 16 clubs** with zero
failures, stable across a repeated import. The public NWSL Events selection
returned HTTP 200 and displayed eight current-window games, seven without Kansas
City. All three pre-existing Current event IDs and their dates/start times were
preserved. Source 4191 is the only source configuration changed.

Before rollback, retain the expanded source/event snapshot. Restore the preceding
scraper image and the source's original Current URL/display metadata if reverting
to team-only collection. Newly added league fixtures are independent rows and
should not be deleted merely to revert code; any data cleanup needs an explicit
scope. No table schema or results data was changed.

Deployed image: `web-scraper:nwsl-league-20261016`,
`sha256:d49272ed529b8917c78fea455a5aea6b7e1bf69213a77f6e4ac7151d57df7f63`.
Full backup and rollback details: [deployment evidence](../reports/sports-history.md#nwsl-league-deployment-2026-10-16).

Final-image production run **170** succeeded at **3:21 p.m. Central**, with the
same 24 unique fixtures and 16 teams. Scraper and frontend were healthy; the hourly
schedule remains unchanged.
