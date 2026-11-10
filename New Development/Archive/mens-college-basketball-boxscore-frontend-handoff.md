# Men's college basketball: championship box-score template

Prepared October 19, 2026. This is a local development fixture and frontend sample;
it has not inserted a historical game into production or deployed new code.

## Reference game

- April 6, 2026 NCAA men's national championship: Michigan 69, UConn 63.
- ESPN event: `401856600`; home Michigan `130`, away UConn `41`.
- [ESPN box score](https://www.espn.com/mens-college-basketball/boxscore/_/gameId/401856600).
- [ESPN summary JSON](https://site.api.espn.com/apis/site/v2/sports/basketball/mens-college-basketball/summary?event=401856600).
- [ESPN dated scoreboard JSON](https://site.api.espn.com/apis/site/v2/sports/basketball/mens-college-basketball/scoreboard?dates=20260406&limit=1000).

The summary endpoint was fetched successfully on the preparation date. The browser
box-score page can present a JavaScript/robot check; the collector uses JSON, not
HTML scraping.

## Ready-to-use frontend sample

Use [mens-college-basketball-boxscore-example.json](mens-college-basketball-boxscore-example.json)
as the **contents of the existing cache Payload**, not as an outer database row or
new API response. It is checked against the actual Rust parser plus period-label
normalizer by an offline regression test.

Identity: `basketball / mens-college-basketball / 401856600`. The sample uses the
existing version-1 contract:

- Two teams, with final scores and 25 team statistics per team.
- `1st half` and `2nd half`: Michigan `33`, `36`; UConn `29`, `34`.
- Two player groups, with nine UConn and eight Michigan players who have stats.
- Thirteen columns, in provider order: MIN, PTS, FG, 3PT, FT, REB, AST, TO,
  STL, BLK, OREB, DREB, PF.
- Provider column keys, descriptions, starter flags, positions, athlete IDs and
  team totals remain available. DNP source rows without stats are omitted from
  normalized player groups; no zero-filled statistics are manufactured.

Render columns dynamically from `playerGroups[].columns` and match
`athletes[].stats` and `totals` by index. Do not hardcode basketball values into
football or baseball layouts. Team matching uses `teamId` and `teams[].id`, not
array position. Shooting values such as `21-68` are strings. Missing total minutes
are null; a genuine string `"0"` stays visible.

Keep the sample in development/demo tooling. A production detail route still
requires an eligible, visible result and a cache row; the sample does not make
`/box-score/basketball/mens-college-basketball/401856600` a live production route.
No provider requests or cache writes belong in the frontend.

## Collector readiness

The existing generic collector already parses the real championship team and
player tables without a basketball-specific parser. This work adds a full reduced
provider fixture and regression coverage; it does not change runtime extraction.
The previously deployed period normalizer supplies half/overtime labels. Existing
hourly `--events-from-db` collection will enrich eligible completed basketball games
through its usual follows, request limits, retry and last-good-cache rules.

The championship demonstrates regulation halves. Multiple overtimes are covered
by existing synthetic period tests, not by this regulation-only game. Final results
remain owned by the results model; do not calculate them by summing line scores.
Preserve the existing unavailable-box-score fallback.

## Validation

Regression test: `sports_boxscores::tests::championship_basketball_matches_complete_frontend_example`.
It parses the captured summary, applies verified scoreboard period identities,
compares the entire normalized payload to the frontend sample, and checks player
counts, provider box-score link, missing minutes, and DNP handling.

Provider fixture: [mens-college-basketball.json](../tests/fixtures/boxscores/mens-college-basketball.json).
The reduced fixture keeps factual identity/statistics and excludes recap prose,
images and unrelated provider content. Labels use the existing
[period fixture](../tests/fixtures/periods/mens-college-basketball.json).

See the [sports period labels frontend handoff](sports-period-labels-frontend-handoff.md)
for the common contract and the separate pending NHL visibility fix.
