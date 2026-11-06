# Scraper task: shared live scoreboard data for MyDailyFeed

Prepared October 23, 2026 (America/Chicago).

## Objective

Provide a reliable, persisted scoreboard covering upcoming, live and completed games for the sports/leagues already enabled in MyDailyFeed. The frontend will use the same read contract to:

- Upgrade the homepage Today & Tonight card with live scores and final results (proposed label: Today's Games).
- Add a Scoreboard page under Sports with date and sport/league filters.
- Link games to existing cached box scores where supported.

This is a task handoff for the separate scraper project, not a report of implementation. No collector changes, schema changes or deployment have been performed for this request. The card name and exact frontend layout remain frontend decisions.

## Investigate existing ownership first

The frontend currently uses several related data paths:

- Scraper-owned `SportsEvents` for schedules, with provider identity and nullable schedule status.
- Web-owned `ResultGames` / `ResultFollows` and a background ESPN results synchronization worker.
- Scraper-owned `SportsMatchResults` for some league results.
- Scraper-owned `SportsBoxScores`, keyed by `(Sport, League, EventId)`, for optional cached statistics.
- `HomeEventResults` for existing homepage schedule/result integration.

Inspect the actual scraper schema, enabled feed configuration and collectors before choosing storage. Audit which fields are already collected and how frequently they update. Do not assume hourly schedule collection or the existing results worker provides live-game freshness.

**Avoid introducing two writers to the same data.** Prefer a dedicated scraper-owned scoreboard read model or a compatible extension of existing scraper-owned storage. Do not write to web-owned ResultGames or disable/change its worker without a coordinated ownership decision. Existing schedules, Results pages, box scores and their readers must keep working.

Resolve sources by provider/sport/league keys; never copy deployment-specific numeric source IDs. Confirm the target database and existing permissions before migrations. Keep frontend runtime access read-only.

## Initial coverage

Start with currently enabled feeds and the teams/leagues already followed. Return a coverage matrix naming each supported provider, sport, league, collection scope, live capabilities and limitations.

Audit pro football, college football, baseball, basketball, hockey, soccer, and Big 12 sports currently configured, including volleyball where a usable live provider exists. Do not promise live coverage for every schedule feed merely because it supplies fixtures. Identify missing coverage explicitly. F1 sessions need separate modeling and are outside this first matchup scoreboard unless agreed otherwise.

Do not silently expand team-specific feeds into league-wide ingestion or turn on disabled sources. Preserve existing postseason and competition eligibility policies; call out any conflict between those policies and desired scoreboard coverage for frontend review.

## Required read contract

Propose the concrete SQL table/view and mapping after the audit. A dedicated read model is acceptable; the names below describe semantics rather than mandated column names.

| Field | Requirement |
| --- | --- |
| Provider / Sport / League / EventId | Stable canonical identity. Event IDs and team IDs are strings, not assumed numeric. Document the uniqueness key and existing schedule/results/box-score mappings. |
| Source identity and enabled state | Allow readers to include only enabled, eligible coverage. Disabling a source must hide its retained scoreboard rows without deleting history. |
| StartsAtUtc | Provider event start in UTC; nullable only when genuinely unknown. Preserve and identify rescheduling. |
| Event date | Document handling for date-only and TBD fixtures without fabricating a precise kickoff. |
| Home and away participants | Stable IDs, names, safe logo URLs and home/away designation. Support unknown opponents when supplied. |
| Neutral venue | Optional provider flag; do not infer location from home designation. |
| HomeScore / AwayScore | Nullable scores; distinguish unavailable from zero. Preserve ties. Do not carry scheduled placeholder zeros into a live-score display without documenting their meaning. |
| Normalized status | Explicit upcoming, live, final, postponed, cancelled, suspended, delayed and unknown handling. Document exact values and transitions. |
| Provider status code/text | Retain enough information to explain intermission, halftime, extra time, shootouts and other sport-specific states. |
| Completed | Provider-supported completion semantics; postponed/cancelled must not become completed finals. |
| Period and clock | Nullable provider period/inning/set and display clock. Include meaningful display labels where supported. Do not derive an apparently live ticking clock from a stale sample. |
| Sport-specific live details | Nullable inning half/top/bottom, outs, set scores, overtime or shootout detail where supplied. Document supported fields per sport. Never invent missing detail. |
| Shootout / winner | Separate shootout/aggregate detail where applicable and provider-designated winner. Do not alter regulation scores or infer a winner while a match is live. |
| Venue and broadcasts | Optional venue/city and broadcast/watch metadata, either directly available or safely joined to existing schedule data. |
| Source URL | Optional safe HTTP(S) provider event URL. No secrets or embedded credentials. |
| Freshness | Last successful observation per event plus source-level last attempt, last success and sanitized availability state. Provider update time may be retained separately when available. |
| Box-score identity | Canonical mapping to existing cache keys; cache availability must not be required for a scoreboard row. |

Support sport-appropriate values without lossy casts. If scores can be non-integers or provider strings for supported competitions, document the appropriate numeric/display types. SQL BIGINT identifiers must map to frontend `long`; UTC DATETIME2 values must be documented as UTC.

Current ResultGames already stores participants, start time, nullable integer scores, State, Completed, StatusText, shootout values, WinnerSide, Venue and first/last-seen times. That is useful compatibility context, not authorization to reuse its table as scraper-owned storage. Period, clock and trustworthy successful observation semantics require audit.

## Collection and freshness

Use an adaptive cadence rather than polling every league at a live rate all day. Suggested starting targets, subject to provider constraints:

- Upcoming/inactive fixtures: existing schedule cadence, with closer checks near expected starts.
- Active games: approximately every 30–60 seconds when permitted and operationally reasonable.
- Delayed/suspended games: reduced cadence with an explicit status, not an assumption of active play.
- Newly completed games: short correction window, then taper to infrequent reconciliation.

These are targets to evaluate, not a claim of guaranteed real-time service. Report actual cadence, expected latency and rate limits. Honor provider terms, response throttling, Retry-After and bounded retries/backoff. Deduplicate requests across overlapping team and league feeds. Prevent concurrent overlapping collector runs from creating duplicate work.

Live cadence must be configurable and separately schedulable from hourly ingestion. Provide a documented collector command and deployment/scheduler approach suitable for the existing environment; do not turn the web frontend into an upstream polling service.

Capture last attempt separately from last successful observation. A failed or partial fetch must not advance freshness for untouched events. Successful empty responses must be distinguishable from failures. Preserve last-good scores on provider failure, expose that they are stale, and avoid clearing the scoreboard or marking games final to hide errors.

## Dates, identity and updates

- Store instants in UTC. Frontend default date grouping uses America/Chicago, not the collector host timezone.
- Support explicit date-range reads for the scoreboard page. Establish configurable lookback/lookahead and retention; state supported navigation limits for the frontend.
- Preserve overnight games on their documented scoreboard day and keep live games discoverable after midnight. Explain how original versus rescheduled start dates affect grouping.
- Join by canonical event identity where possible. Document a validated bridge for schedule providers with different event IDs; do not join on team-name strings alone.
- Upsert idempotently. A game present in multiple configured feeds must appear once in the effective scoreboard.
- Write a game's status, scores and live details atomically. Reject out-of-order samples where provider timestamps allow it; permit verified corrections instead of assuming scores/status only advance.
- Missing events in a response do not automatically mean cancellation, deletion or a zero score. Document any provider-supported withdrawal policy.

## Frontend behavior this must enable

The backend must provide enough data for the frontend to show:

- Upcoming: scheduled time or TBD and available broadcasts.
- Live: scores plus supplied period/clock/inning/set labels.
- Finished: final scores and provider-supported OT/shootout labels.
- Other states: explicit postponed, cancelled, suspended, delayed or unavailable labels.
- Freshness: an honest Updated timestamp and stale/unavailable message while retaining last-good data.
- Selected teams: stable participant identity for existing browser preferences.
- Box scores: existing cache identity when available, without coupling score freshness to box-score fetch success.

The frontend will refresh persisted reads while visible; it must not trigger upstream collection on every browser refresh. Browser refresh frequency and upstream collection cadence are separate settings.

## Validation

Use sanitized provider fixtures and isolated test data. Cover:

1. Upcoming → live → final for each supported sport; zero scores and missing scores.
2. Halftime/intermission, overtime, shootouts, inning halves and volleyball sets where supported.
3. Postponed, rescheduled, cancelled, delayed, suspended, TBD and unknown statuses.
4. Duplicate team/league coverage, stable joins and repeated idempotent imports.
5. Provider failure, throttling, successful empty responses, partial data and stale recovery without losing last-good scores.
6. Late corrections and out-of-order observations.
7. Central-time day boundaries, daylight saving changes and overnight games.
8. Disabled sources and existing team/competition/postseason scope.
9. Scoreboard success when optional box scores are absent or unavailable.
10. Existing schedule and final-results readers continue to work.

Provide query examples for a Central calendar day (translated to UTC bounds) and an active-games read that includes games carried over from the prior date. Use bound parameters and indexed identity/date/status access; do not require a full-table scan per browser refresh.

## Deliverables and return handoff

Return a Markdown frontend handoff containing:

- Audit findings and supported coverage matrix, including known gaps.
- Chosen owner/storage design and explicit coexistence with the web results worker.
- Exact migration/setup commands, prerequisites and read permissions; no credentials in the document.
- Concrete schema/view names, types, nullable fields, status vocabulary and canonical identity mappings.
- Example parameterized frontend reads and sanitized upcoming/live/final/stale records.
- Provider limits, actual collection cadence, scheduler configuration, stale thresholds and retention/date-navigation limits.
- Fixture/test results and dated runtime verification, clearly distinguishing implemented, deployed and proposed work.
- Deployment and rollback instructions that preserve existing schedules/results/box-score consumers.

Do not claim frontend readiness or live coverage until the documented read contract and required data exist. Coordinate any breaking reader changes before rollout. No new frontend UI is requested from the scraper project.
