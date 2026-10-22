# Frontend handoff: weekend box-office result labels

## Backend availability

The scraper migration and collector were deployed and verified October 21, 2026,
23:35 CDT (October 22 UTC), as `web-scraper:boxoffice-status-20261022`.
This is deployment evidence, not a fresh health check. The existing hourly collector
continues refreshing results throughout the week. No frontend DDL, new write grants,
or additional collector is required.

This file describes frontend work to perform; it does not claim the frontend
changes have been implemented or deployed.

## Read contract

Read `dbo.BoxOfficeWeekend` in the existing WebScraper database using the existing
frontend reader identity. Existing fields and meanings are preserved. Add these
nullable properties to the query/model/API response:

| Field | SQL type | Meaning |
| --- | --- | --- |
| `ResultStatus` | `nvarchar(16) NULL` | `estimated`, `final`, or `unknown`; legacy NULL means unknown |
| `StatusSourceUrl` | `nvarchar(2048) NULL` | Status evidence URL; fall back to `SourceUrl` when NULL |
| `StatusAsOfUtc` | `datetime2 NULL` | Provider publication/update time when established; NULL means unavailable |

`ResultStatus` applies to **WeekendGross only**. Do not apply it to
`TotalDomesticGross`. Amounts are integer USD. `WeekendStart` is a calendar Friday
and the displayed weekend ends two calendar days later. `FetchedAt` is the
successful collection time in UTC, separate from provider publication time.
Treat the UTC datetime2 fields as UTC when serializing them to the browser.

Use a parameterized query and bind `@ExpectedFriday` as a SQL `date`:

```sql
SELECT Rank, WeekendStart, MovieTitle, WeekendGross, TotalDomesticGross,
       WeeklyChangePercent, Theaters, MovieUrl, SourceUrl,
       COALESCE(ResultStatus, N'unknown') AS ResultStatus,
       COALESCE(StatusSourceUrl, SourceUrl) AS StatusEvidenceUrl,
       StatusAsOfUtc, FetchedAt, PosterUrl, PosterLookupStatus
FROM dbo.BoxOfficeWeekend
WHERE WeekendStart = @ExpectedFriday
ORDER BY Rank;
```

The table holds the latest ten-row chart, not historical charts. Fetch amounts,
status and weekend together and keep them in the same cached response; avoid
combining separately cached status and gross values.

## Display rules

Use the same mapping on homepage results cards and the box-office results view:

| Stored status | Weekend gross label |
| --- | --- |
| `estimated` | Estimated |
| `final` | Final |
| `unknown`, NULL, or an unrecognized value | Reported totals |

A Final label means provider-reported actuals; later corrections remain possible.
Status is per movie. A mixed chart must not receive a blanket Final label because
some rows are final. Preserve existing poster and missing-poster behavior.

Use `StatusEvidenceUrl` for an optional source link. Only show a provider update
time when `StatusAsOfUtc` exists. If showing `FetchedAt`, label it as the collection
or refresh time. Never substitute it for the provider publication time.

## Weekday rotation and weekend selection

Retain the intended homepage rotation using the configured display date in
**America/Chicago**, including any existing date-preview mechanism:

- Wednesday–Saturday: upcoming weekend preview from `dbo.BoxOfficeForecasts`.
- Sunday–Tuesday: results from `dbo.BoxOfficeWeekend` for the immediately preceding Friday.

For the results window, derive the date using calendar arithmetic in the display
timezone: Sunday minus 2 days, Monday minus 3, Tuesday minus 4. Pass that Friday
to the query above. Do not derive the date from the browser timezone or UTC date.

For October 25–27, 2026, the expected WeekendStart is October 23, 2026, and the
weekend label covers October 23–25. If no matching chart is available, show a
waiting message such as “Weekend results have not been published yet.” Do not
present the previous weekend as current. A separate historical/latest-reported
view may show older data only with its actual weekend clearly identified.

Keep forecast ranges separate from reported grosses. Do not infer result status
from weekdays, fetch time, numeric totals, or the age of a chart.

## Provider limits and examples

The live source is The Numbers. The scraper recognizes an individual gross cell's
`chart_estimate` marker as estimated. Unmarked gross cells remain unknown;
absence of an estimate marker is not proof of actuals.

At deployment verification, the October 16, 2026 chart contained one estimated row
(Jailer 2, WeekendGross 3270000) and nine unknown rows. No live final row was
verified. StatusAsOfUtc was NULL for all ten rows; FetchedAt was
`2026-10-22T04:35:39.4754301` UTC. This is a historical observation, not a statement
about the current database.

The scraper's explicit Actual/Actuals label branch has simulated parser coverage
but no verified live example. Test the frontend Final state with a clearly identified
mock; do not relabel an unknown live row for testing or infer final status yourself.

## Frontend acceptance checks

- Both views map estimated/final/unknown/NULL consistently; unexpected values fall back safely.
- A mixed-status chart labels individual weekend grosses and leaves cumulative totals separate.
- Sunday estimates can become Final on a later refresh without a frontend redeployment.
- Wednesday–Saturday previews remain separate and retain their existing forecast behavior.
- Sunday–Tuesday selects the correct Friday across month/year boundaries and Central-time midnight.
- Missing current-weekend data shows the waiting state even if an older chart exists.
- NULL provider timestamps do not render a fabricated publication time.
- Amounts, ranks, status and weekend update together in the frontend cache.
- Existing source links, posters and empty/error states continue to work.

Backend verification already completed: 162 Rust tests passed (21 ignored), plus
12 isolated SQL checks covering transitions, revisions, downgrade rejection,
publication-time guards, failed-refresh preservation and poster preservation.
Frontend verification and deployment remain the frontend owner's responsibility.

## References

- [Backend contract and provider evidence](../docs/box-office-result-status.md)
- [Deployment and rollback evidence](../reports/boxoffice-status-20261022.md)
- [Original scraper request](box-office-estimate-final-scraper-handoff.md)
