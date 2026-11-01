# Top 25 scoreboard and upcoming ranked games — frontend plan

Status: planned, waiting on the scraper work (2026-11-01). The columns and view
below do not exist until the scraper half ships; see
[top25-scoreboard-scraper.md](top25-scoreboard-scraper.md).

## What the frontend gets

Two features for **college football (FBS)** and **men's college basketball**:

1. **Top 25 scoreboard** — a filter on the existing scoreboard page showing every
   game where at least one team is ranked, Big 12 games included.
2. **Upcoming ranked games this week** — a Monday–Sunday list of games involving a
   ranked team, **excluding any game with a Big 12 team** (those already appear in
   the Big 12 schedule).

Women's basketball is out of scope.

## Rules the UI can rely on

- A game qualifies when **either** team is ranked 1–25.
- Ranks are ESPN's, as shown on ESPN's scoreboard. Late in the football season this
  may be the College Football Playoff ranking rather than AP, so a team's number can
  differ from the AP rankings page.
- A rank is the team's rank **at game time**: it can change on upcoming games when a
  new poll is released, and it never changes after the game is final.
- Unranked teams have a `NULL` rank. Never display `#99` or `#0`.

## Data contract

Access is `SELECT` through the existing `WebScraperScoreboardReader` role.

### New columns on `dbo.SportsScoreboard`

| Column | Type | Meaning |
| --- | --- | --- |
| `HomeRank` | `tinyint NULL` | Home team's rank (1–25) at game time, else `NULL` |
| `AwayRank` | `tinyint NULL` | Away team's rank (1–25) at game time, else `NULL` |

All existing scoreboard columns, status handling, `IsStale` and box score fields are
unchanged; see [live-scoreboard-frontend-handoff.md](live-scoreboard-frontend-handoff.md).

### Top 25 scoreboard query

```sql
SELECT * FROM dbo.SportsScoreboard
WHERE League IN ('college-football','mens-college-basketball')
  AND (HomeRank IS NOT NULL OR AwayRank IS NOT NULL)
  AND ScoreboardDate = @Date
ORDER BY StartsAtUtc;
```

Filter by `League` as well as rank: ranks can also be present on women's basketball
games from existing Big 12 coverage, and those should not appear here.

### New view `dbo.SportsScoreboardTop25Week`

Same columns as `dbo.SportsScoreboard`, already limited to:

- the two leagues above, with at least one ranked team;
- `ScoreboardDate` within the current Monday–Sunday week (Central time);
- no game involving a Big 12 team;
- no cancelled games.

```sql
SELECT * FROM dbo.SportsScoreboardTop25Week
WHERE League = @League
ORDER BY ScoreboardDate, StartsAtUtc;
```

Finished games from earlier in the week remain in the view (`Status='final'`).
Filter them out with `Status <> 'final'` if the list should show only games still
to be played.

## UI work

- **Scoreboard filter:** add "Top 25" alongside the existing scoreboard filters for
  football and men's basketball, using the query above.
- **Rank display:** show `#{rank}` before the team name when the rank is not `NULL`
  (for example `#5 Ohio State`), on both the Top 25 scoreboard and the weekly list.
  This can also be applied to the existing Big 12 scoreboard for free.
- **Weekly list:** group by `ScoreboardDate`; show start time (or TBD when `TimeTbd`
  is set), both teams with ranks, venue and broadcasts.
- **Empty states:** no ranked games on a date; no remaining ranked games this week;
  men's basketball before its season starts.

## Open item

- Decide whether finished games stay in the weekly list or move to the scoreboard
  only (filter `Status <> 'final'` as above).
