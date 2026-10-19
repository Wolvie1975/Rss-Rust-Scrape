# Frontend handoff: sports box-score period labels

Prepared October 19, 2026. Backend release: `web-scraper:period-labels-20261019`.

## Status and scope

The scraper changes are deployed to production. Historical repair labeled **5,356
line-score entries across 960 available box-score snapshots**. Scores, statistics,
provider links, availability, and game identities were preserved. The 395
unavailable snapshots were left unchanged.

Existing frontend pages already render the repaired string labels. The remaining
required frontend change is **NHL box-score detail visibility**, described below.
No database migration or additional provider scraping is needed for this handoff.
No frontend code changes or frontend deployment were included in the backend work.

## Data contract

Read the existing `WebScraper.dbo.SportsBoxScores` view, keyed by the exact string
tuple `(Sport, League, EventId)`. Sport and league use lower-case ESPN slugs.

The JSON payload remains **version 1**. Continue consuming
`teams[].lineScores[].period` and `teams[].lineScores[].displayValue`:

```json
{"period":"1st inning","displayValue":"0"}
```

`period` now carries descriptive strings when the provider's period identity can
be verified. Retain compatibility with numeric or null period values in older or
partially resolved snapshots. `displayValue` remains the original string or null;
the string `"0"` is a real value.

Do not switch to a new `periodLabel` field. The optional root fields
`periodLabelsVersion`, `periodLabelSource`, and `periodLabelStatus` are operational
metadata and may be ignored by the frontend. `periodLabelsVersion: 1` is the label
normalizer generation, not a change to the payload version. Status may be
`complete` or `partial`.

| Sport / league | Expected labels |
| --- | --- |
| `baseball/mlb` | `1st inning`, `2nd inning`, …; extra innings continue numerically |
| `football/nfl`, `football/college-football` | `Q1`–`Q4`, `OT`, `2OT`, … |
| `hockey/nhl` | `1st period`, `2nd period`, `3rd period`, `OT`; playoff multiple overtime labels; explicit shootout `SO` |
| `basketball/mens-college-basketball` | `1st half`, `2nd half`, `OT`, `2OT`, … |
| NBA / WNBA | `Q1`–`Q4`, `OT`, `2OT`, … |
| Soccer | `1st half`, `2nd half`, `ET1`, `ET2`; explicit penalty shootout `SO` |

Provider label strings can also be preserved as supplied. Render known labels
without replacing them with array-position labels. Preserve unknown entries and
provider order; do not collapse unknown labels or shift a sparse period into a
different column. If aligning teams into shared columns, match known period
identities and keep absent values absent.

Do not manufacture unplayed innings or periods, or replace missing/unplayed values
with zero. Do not recompute the final result by summing these entries. NHL `SO`
can represent the provider's deciding **0/1 scoreboard indicator**, not raw
shootout goals or attempts. Soccer shootout values remain separate from match
goals. Continue using the existing result model for the final score.

## Required frontend change: NHL detail visibility

Observed production behavior:

```text
/box-score/hockey/nhl/401892525 -> HTTP 404
```

The snapshot exists and the frontend database account can read its repaired
`1st period`, `2nd period`, `3rd period`, `OT`, and `SO` labels. The failure occurs
before the cache read, in the detail visibility guard.

Frontend source inspected: `src/MyNewsFeed.Web/Data/BoxScores.cs` in the
`My-Newsfeed` repository. `BoxIdentity` already maps `espn-nhl` to `hockey/nhl`.
The detail getter currently accepts completed, followed ordinary `ResultGames`,
plus explicit current-season `SportsMatchResults` branches for MLS (`usa.1`) and
Championship (`eng.2`). It lacks the equivalent NHL branch.

Implement an NHL visibility branch for `Sport == "hockey"` and `League == "nhl"`
before the guard returns null:

1. Safely parse the event ID using the existing results identity convention.
2. Resolve the enabled NHL source and current season through the same rules used
   by the NHL results page.
3. Verify that the requested game is visible in that source and season, preserving
   applicable follow/team visibility rules.
4. Allow the existing cache lookup only when that visibility check succeeds.

NHL games here are stored in `SportsMatchResults` under `ProviderKey = 'espn-nhl'`.
At verification the enabled source ID was **5637** and ESPN season was **2027**
(2026–27). These are diagnostic values: **do not hardcode them**. Reuse the NHL
results query/source resolution so the implementation follows future seasons.

Preserve the ordinary-results, MLS, and Championship paths. Cache existence alone
must not bypass the existing source/season/visibility policy. The frontend should
read cached data through its existing database access, without calling ESPN or
writing cache rows.

## Production examples

These relative routes were checked after the repair. Recheck against the target
frontend deployment; cached examples can eventually age out.

| Route | Verified result |
| --- | --- |
| `/box-score/baseball/mlb/401930515` | HTTP 200; `1st inning` through `9th inning` |
| `/box-score/football/nfl/401872988` | HTTP 200; `Q1`–`Q4`, `OT` |
| `/box-score/football/college-football/401856832` | HTTP 200; `Q1`–`Q4` |
| `/box-score/soccer/usa.1/761889` | HTTP 200; `1st half`, `2nd half` |
| `/box-score/soccer/usa.nwsl/401854026` | HTTP 200; `1st half`, `2nd half` |
| `/box-score/soccer/eng.2/401880218` | HTTP 200; `1st half`, `2nd half` |
| `/box-score/hockey/nhl/401892525` | Currently HTTP 404; labeled cache ready, frontend visibility fix pending |

Men's college basketball had 348 upcoming production games and no completed cached
box scores at verification. Its label support was tested using recorded fixture
event `401856600` and synthetic overtime cases; that event is **not a production
acceptance route**. Soccer World Cup event `633850` is likewise fixture-only and
covers extra time and penalties; this work did not register a new league.

## Acceptance checks

- The NHL example opens after the visibility fix and displays its cached labels.
- Disabled sources, games outside the allowed season, and otherwise invisible
  games remain subject to the existing visibility policy.
- Existing ordinary-results, MLS, and Championship detail routes still work.
- String period labels render correctly; numeric/null compatibility remains.
- Zero, missing, unplayed, sparse-period, overtime, and shootout examples retain
  their meaning. Final scores continue to come from the result model.
- Existing unavailable/missing-box-score fallback behavior remains usable.
- Run appropriate frontend tests and a browser check of the affected detail pages
  before deploying the frontend change.

## Backend evidence

The backend suite passed **159 tests** (21 opt-in tests ignored), plus strict
all-target Clippy. Before/after comparison verified preservation of all non-label
payload data. Production verification used SQL and HTTP; it was not an interactive
browser test.

See [deployment and repair report](../reports/sports-period-labels-20261019.md)
for the full audit, and [recorded fixture documentation](../tests/fixtures/periods/README.md)
for provider examples and synthetic-case distinctions.
