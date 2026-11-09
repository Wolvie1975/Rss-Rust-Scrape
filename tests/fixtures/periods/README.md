# Period metadata fixtures

Captured October 19, 2026 from ESPN's public summary and scoreboard endpoints.
Each JSON records its exact URLs, event identity, summary line values, final status,
season type and explicit scoreboard period IDs. An additional college-football fixture records a final Baylor–TCU game omitted from its dated scoreboard and verifies the event-specific core fallback. Soccer fixtures also record core
line-score collection URLs; site scoreboards omit those arrays. Core `$ref` values
are identity evidence only and are never followed by the collector.

Fixtures are reduced to scoring/identity metadata. No provider prose or credentials
are included. `mlb` includes provider inning IDs; `nfl` is a real overtime game;
`nhl` is a real regular-season shootout; `mens-college-basketball` is a real
championship game with two halves. MLS, NWSL and Championship fixtures each have
ordinary half scores. The FIFA World Cup fixture supplies observed extra-time and
penalty period identities for soccer; it is a test fixture, not an added production
league. Men's college basketball currently has no retained production cache rows.

Clearly marked synthetic tests in `src/sports_periods.rs` cover extra innings,
shortened games, explicit unplayed markers, multiple college-football/basketball
and playoff NHL overtimes, WNBA quarters, sparse/missing IDs and values, unknown
labels, malformed identities/counts, and ambiguous shootout representation.

Scoring values are retained verbatim. Fixtures establish explicit period identity,
not an instruction to derive labels from array length or a final score. No
cumulative-to-incremental conversion is performed.
