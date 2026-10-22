# Weekend result status

The scraper adds nullable `ResultStatus nvarchar(16)`, `StatusSourceUrl nvarchar(2048)`
and `StatusAsOfUtc datetime2` to `dbo.BoxOfficeWeekend`. Existing columns, rank-based
latest-chart grain, reader grants, forecasts and Central-time hourly scheduling are preserved.

`ResultStatus` describes **WeekendGross only**, not cumulative TotalDomesticGross.
Values are `estimated`, `final`, `unknown`; legacy NULL also means unknown.
`final` means explicitly provider-labelled actuals, with later revisions still allowed.

## Provider evidence and limits

Source: https://www.the-numbers.com/weekend-box-office-chart

The October 22, 2026 UTC capture in
`tests/fixtures/boxoffice/provider-20261022.html` retains the provider desktop table
and heading. The gross cell of rank 9 (Jailer 2, October 16 weekend, $3,270,000)
has CSS class `chart_estimate`. That row is estimated. Other top-ten gross cells
have no explicit status and remain unknown. Absence of the class does **not** mean final.
The capture contains no verified publication timestamp; StatusAsOfUtc stays NULL.
StatusSourceUrl is NULL because SourceUrl is the source of the status too.

An exact gross-cell title attribute `Actual` or `Actuals` maps to final. This branch
is tested with **simulated** markup; such a label has **not** been observed live.
Conflicting estimate/actual evidence maps to unknown. There is currently no verified
live final-detection signal for unmarked grosses. Therefore the frontend must use
“Reported totals” for these rows. Do not infer final from day, collection time,
rounding, or a numeric total. A future provider markup change requires renewed review.

## Refresh integrity

The existing transaction replaces all ten rows, status, ranks, weekend and amounts
together. Movie URL identifies a movie across rank changes. For the same weekend,
a final-to-estimated or final-to-unknown transition rejects the **entire** snapshot;
an older or absent publication timestamp also rejects a row with a known newer time.
New weekends do not inherit status. New final revisions are accepted. Without provider
timestamps, relative age of two final snapshots cannot be established; revisions are
accepted in collection order. No claim of detecting all stale final snapshots is made.
A rejected collection keeps the last successful values and timestamps. Posters for
matching movie URLs survive replacement and the existing poster cache still runs.

## Read query

```sql
SELECT Rank, WeekendStart, MovieTitle, WeekendGross, TotalDomesticGross,
       COALESCE(ResultStatus, N'unknown') AS ResultStatus,
       COALESCE(StatusSourceUrl, SourceUrl) AS StatusEvidenceUrl,
       StatusAsOfUtc, FetchedAt, PosterUrl, PosterLookupStatus
FROM dbo.BoxOfficeWeekend
WHERE WeekendStart = @ExpectedFriday
ORDER BY Rank;
```

The frontend derives ExpectedFriday from its America/Chicago display date and shows
its waiting state when no matching chart exists. Weekday rotation remains frontend work.

## Setup and verification

Runtime startup remains DDL-free. `--configure-boxoffice` includes the repeatable
additive migration `sql/boxoffice_status.sql`, using migration credentials and the
WebScraper database. A targeted setup/audit utility is also available:

```sh
( set -a; . ./.env.migrations; set +a; cargo run --locked --example boxoffice_status -- --migrate )
cargo run --locked --example boxoffice_status
cargo test --locked
```

The utility exercises the replacement SQL using a session-local temporary table;
test movies never enter production tables. Its read audit displays live public chart
fields and timestamps without credentials. The migration preserves legacy NULL status
and adds no grants; existing table SELECT access continues to cover added columns.

Eight focused box-office/forecast tests passed; full suite: 162 passed, 21 ignored.
Twelve SQL checks cover estimate-to-final, revised actuals, estimate/unknown regression,
older-weekend rejection, new-weekend estimates, exact failed-snapshot preservation,
publication-time regression and poster preservation. Explicit-final parser fixtures are
simulated; the mixed estimated/unknown provider fixture is a live capture.

Deployment evidence is recorded separately in reports/boxoffice-status-20261022.md.
