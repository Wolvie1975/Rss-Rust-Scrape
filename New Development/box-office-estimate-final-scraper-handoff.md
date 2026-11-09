# Scraper request: weekend box-office estimate and final status

## Purpose and current state

The My-Newsfeed homepage now has a local implementation that rotates its box-office
card by the configured display date (America/Chicago in production):

| Days | Homepage card |
| --- | --- |
| Wednesday–Saturday | Upcoming weekend preview — Predictions |
| Sunday–Tuesday | Current weekend results, first estimates and then final totals when confirmed |

The weekly rotation is implemented locally but has not been deployed. The deployed
homepage currently shows the predictions card throughout the week.

The results reader uses `dbo.BoxOfficeWeekend` from The Numbers. Its current
contract exposes no estimate/final indicator. The frontend therefore labels totals
as reported and potentially subject to revision. It must not infer final status
from Monday/Tuesday, collection time, or the presence of numeric totals.

This document requests scraper work. It does not claim that the requested fields,
migration, collector changes, or provider status detection already exist.

## Requested collection behavior

Continue collecting the domestic Friday–Sunday weekend chart through the existing
box-office collector and scheduler. Import Sunday estimates when the provider
publishes them, then replace/update them when the provider publishes actual totals.
Keep collecting revisions after actual totals appear.

Use provider evidence to identify estimates versus actuals. If the source calls its
confirmed figures “actuals,” map that to `final` for display purposes; this means
provider-reported actuals, not a guarantee that later corrections are impossible.
If reliable status cannot be determined, retain `unknown`. Report any provider
limitation instead of inventing a status.

Keep previews in `dbo.BoxOfficeForecasts` separate from collected results. Do not
copy prediction ranges into reported weekend grosses.

## Proposed additive read contract

Please expose the following fields on `dbo.BoxOfficeWeekend`, or propose an equally
explicit compatible contract before frontend integration:

| Field | Proposed type | Meaning |
| --- | --- | --- |
| `ResultStatus` | `nvarchar(16) NULL` | `estimated`, `final`, or `unknown`; NULL means unknown for legacy rows |
| `StatusSourceUrl` | `nvarchar(2048) NULL` | Provider URL supporting the status, if different from the existing chart SourceUrl |
| `StatusAsOfUtc` | `datetime2 NULL` | Provider publication/update timestamp for the status when available; do not substitute fetch time |

Status must describe the same movie, Friday-start weekend, and gross figures in
that row. If status is chart-wide, expose the same verified chart status on each
row or document the chart metadata join. If the provider mixes row statuses, retain
them individually; do not label an entire chart final because some rows are actuals.

Preserve existing fields and meanings:

```text
Rank, WeekendStart, MovieTitle, WeekendGross, TotalDomesticGross,
WeeklyChangePercent, Theaters, MovieUrl, SourceUrl, FetchedAt,
PosterUrl, PosterLookupStatus
```

`WeekendStart` remains a calendar Friday; the weekend ends Sunday (+2 days).
Dollar amounts remain integer USD. `FetchedAt` remains successful collection time
in UTC, separate from provider publication time. Preserve poster lookup behavior.

Use an additive, repeatable scraper-owned migration. Leave legacy rows unknown
unless the source establishes their status. Preserve the current frontend runtime
identity's SELECT access; no new frontend write grants are needed. Do not remove
or rename existing columns, change the existing row grain, or require the web app
to run DDL.

## Weekend selection and refresh integrity

On Sunday–Tuesday the frontend expects the immediately preceding Friday's chart.
For example, October 25–27, 2026 should display the weekend starting October 23.
If that weekend has not been published, the frontend shows a waiting message rather
than presenting the previous weekend as current. Publication timing may vary.

Publish totals, weekend identity, ranks and status atomically so readers do not see
new statuses paired with old amounts or partial chart replacement. Failed collection
must preserve the last successful snapshot and its original timestamps/status.
Avoid downgrading known actuals to estimates from an older source snapshot. A newer,
explicit provider correction may update actual totals while retaining final status.
Never carry final status from an older weekend into a new weekend's estimate rows.

## Verification requested from the scraper owner

- Parser fixtures for provider estimates, actuals, absent/ambiguous status, and mixed
  row statuses if supported. Include provenance for the provider status signal.
- Same-weekend transition from estimates to actuals updates totals and status together.
- New-weekend estimates do not inherit the previous weekend's final label.
- Older estimates cannot overwrite newer actuals; failed refresh retains good data.
- Successful actuals revisions remain collectible and update FetchedAt appropriately.
- Additive migration can be rerun, preserves existing rows and frontend read access,
  and leaves unknown publication times NULL.
- Existing preview/long-range forecasts, posters and unrelated collectors still work.
- Confirm the normal scheduler continues updating results Sunday through Tuesday;
  do not introduce a second competing collector or manual-only collection path.

## Handoff back to the frontend

Provide the final schema/read query, status values and semantics, provider detection
rules, fixture/test results, and the migration/collector deployment status. Include
read-only examples of estimates and actuals if available, with their weekend,
status provenance, publication time and fetch time. Clearly identify simulated
fixtures versus live observations. Do not include credentials or connection strings.

The frontend will then read the agreed status fields and label rows Estimated or
Final (or Reported totals when unknown), updating the homepage and box-office results
view consistently. The weekday rotation remains a frontend responsibility; status
comes from provider evidence, independently of the day of the week.
