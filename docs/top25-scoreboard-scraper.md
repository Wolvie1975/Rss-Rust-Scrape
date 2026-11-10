# Top 25 scoreboard and upcoming ranked games — scraper plan

Status: agreed scope, not started (2026-11-01). Estimated effort: 2–3 days
including tests, docs and deployment. The frontend half is in
[top25-scoreboard-frontend.md](top25-scoreboard-frontend.md).

## Decisions

| Topic | Decision |
| --- | --- |
| Sports | College football (FBS) and men's college basketball only. Women's basketball is out of scope. |
| Which games | A game qualifies when **either** team is ranked. |
| Rank source | ESPN's rank as shown on its scoreboard (`competitors[].curatedRank.current`), used as-is. Late in the football season this may follow the CFP rankings rather than AP; that is accepted. |
| Rank timing | Rank as of game time. The stored rank keeps updating while the game is upcoming/live and is frozen once the game is final. |
| Upcoming games | Option A: reuse the scoreboard's existing lookahead. No new provider or `SportsEvents` ingestion. |
| Big 12 in upcoming list | Exclude any game involving a Big 12 team (ranked or not); the Big 12 calendar already covers those. |
| Big 12 on Top 25 scoreboard | Included. The scoreboard shows every ranked game. |
| Upcoming window | Monday–Sunday (Central), matching the weekly poll cadence. |

## Why this is cheap

- The scoreboard already requests every FBS game (`groups=80`) and every Division I
  men's game (`groups=50`) per day — see `request_url` in `src/scoreboard.rs`.
  Coverage only filters which games are kept, so Top 25 coverage adds **no new ESPN
  requests**.
- ESPN includes each team's rank on every scoreboard game, so qualification and
  rank-at-game-time come from the same response. No join to `SportsRankingTeams`
  is required.
- The default 7-day lookahead (plus the extra day already in the request range)
  always reaches the coming Sunday, so `--lookahead-days` stays at 7.

## 1. Store ranks on scoreboard games

- `sql/scoreboard.sql` (additive migration): add `HomeRank TINYINT NULL` and
  `AwayRank TINYINT NULL` to `dbo.SportsScoreboardGames`, each
  `CHECK (... IS NULL OR ... BETWEEN 1 AND 25)`.
- `parse_event` in `src/scoreboard.rs`: read `curatedRank.current` for each side.
  Store it only when 1–25; ESPN uses `99` for unranked, which becomes `NULL`.
  Emit `homeRank`/`awayRank` in the row JSON. This applies to all leagues;
  non-college leagues will be `NULL`, and women's basketball ranks may be stored
  for existing Big 12 coverage but are not used by this feature.
- `sql/scoreboard_upsert.sql`: add both columns to the `OPENJSON` schema. On update,
  keep the stored rank when the existing row is already `final`:
  `HomeRank = CASE WHEN g.Status='final' THEN g.HomeRank ELSE i.HomeRank END`
  (same for away). A game first observed as final takes the provider's rank at
  that moment.

## 2. Top 25 coverage source

Top 25 coverage is scraper-owned, so it does not touch the web app's
`ResultFollows` configuration or its `CK_ResultFollows_Kind` constraint.

- `sql/scoreboard_config.sql`: add a third `UNION ALL` branch with
  `SourceKind='rankings'`, `Scope='top25'`, sourced from enabled
  `dbo.SportsRankingSources` rows for `Football` → `football/college-football` and
  `Men's Basketball` → `basketball/mens-college-basketball` (AP poll, `PollId='1'`).
  Enabling the ranking source therefore enables its Top 25 scoreboard coverage.
- `sql/scoreboard.sql`: extend the `SportsScoreboardSources.SourceKind` check to
  include `'rankings'`.
- `sql/scoreboard_sources_view.sql`: include the new rankings rows in
  `dbo.SportsScoreboardCoverage`.
- `src/bin/scoreboard.rs` `load()`: accept `SourceKind='rankings'` with
  `Scope='top25'` and no team list; reject any league other than the two above.
- `Coverage::accepts` in `src/scoreboard.rs`: for `top25`, keep regular season and
  postseason only (same policy as team/conference scopes), and accept the game when
  `homeRank` or `awayRank` is present.

## 3. Read views

- `sql/scoreboard_view.sql` (`dbo.SportsScoreboard`): add the branch
  `s.SourceKind='rankings' AND (g.HomeRank IS NOT NULL OR g.AwayRank IS NOT NULL)`.
  Filtering on stored ranks (not event-source links alone) keeps frozen final games
  visible even if a later correction fetch reports a different rank. Expose
  `HomeRank` and `AwayRank` (already covered by `g.*`).
- New view `dbo.SportsScoreboardTop25Week` for the upcoming list:
  - rows from `dbo.SportsScoreboard` for `college-football` and
    `mens-college-basketball` with `HomeRank` or `AwayRank` present;
  - `ScoreboardDate` between Monday and Sunday of the current Central week;
  - `Status` not `cancelled` (whether today's finished games stay in the list is a
    frontend decision; keep `final` rows in the view and let the frontend filter);
  - excludes games where `HomeTeamId` or `AwayTeamId` is in the Big 12 membership
    for that sport, taken from `ResultFollowTeams` of the enabled Big 12
    `conference` follow (ESPN group IDs believed to be 4 for football and 8 for
    men's basketball — verify against the existing follow rows).
- `sql/scoreboard_permissions.sql`: grant `SELECT` on the new view to
  `WebScraperScoreboardReader`.

## 4. Tests

- Parser: ranked/unranked (`99`) sides, missing `curatedRank`, out-of-range values.
- Upsert: rank updates while upcoming/live, frozen after final.
- Coverage: `top25` accepts a game with one ranked side, rejects an unranked game,
  rejects preseason.
- Use captured FBS and men's basketball scoreboard fixtures containing ranked games.

## 5. Docs and deployment

- Update `docs/live-scoreboard-operations.md` (migration order, new source kind) and
  `docs/live-scoreboard-frontend-handoff.md` (rank columns, Top 25 filter, weekly
  view, Big 12 exclusion rule).
- Apply the migrations with migration credentials, run `scoreboard --dry-run` to
  confirm the two new coverage rows, then redeploy only the `scoreboard` service.
  The hourly scraper is unaffected.

## Notes and risks

- Poll release days differ (AP football on Sunday, CFP on Tuesday, AP men's
  basketball on Monday). Because ranks come from ESPN as-is, upcoming games pick up
  new ranks whenever ESPN updates, regardless of release day.
- Whether ESPN's football `curatedRank` switches to CFP rankings in November should
  be confirmed against live data; it is accepted either way.
- The existing scoreboard retention (1 day of history by default) also limits how
  far back the Top 25 scoreboard goes.
