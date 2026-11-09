# Rankings context: frontend implementation handoff

Prepared October 22, 2026 (America/Chicago).

## Requested change

Enhance the existing national rankings page with poll date/week, an explicit
Previous rank column, and a compact Big 12 summary above the table. Keep the
full national poll, existing sport selection, records, points, votes, logos,
movement indicators and Big 12 highlighting.

The scraper already stores the required fields. No new feed, database migration,
collector deployment or historical backfill is needed. This file specifies frontend
work; the frontend application has not been modified or deployed in this checkout.
Canonical backend reference: [sports rankings contract](sports-tables.md#my-newsfeed-top25-rankings).

## Header and poll context

Show the selected sport, actual `PollName`, and `SeasonLabel`. Below the title,
show **Poll dated [date]** and the provider's `Period` (week) and `SeasonPhase`
when present. Omit missing optional text and avoid repeating identical labels.
Use each sport's actual poll name; soccer and volleyball are not AP polls.

Show **Last refreshed [time]** from `FetchedAt`, separately from the poll date,
and link **Source** to `SourceUrl`. Interpret SQL DATETIME2 timestamps as UTC
before converting to America/Chicago. Poll date describes the provider's poll,
not when the collector fetched it.

If publication is over 14 days old, use **Latest available poll — dated [date]**
and keep the season visible. Do not infer Final or Preseason from the date.
If `LastSuccessAt` is over two hours old or `LastError` exists, retain the poll
and show a short refresh availability message. Never expose raw database errors.

## Previous rank column

Insert **Previous** immediately before the existing Trend column:

**Rank · Team · Record · Points · Previous · Trend**

Keep first-place votes in their existing parentheses beside team names, with a
visible legend or accessible explanation: **Parentheses indicate first-place votes**.

| Stored values | Previous | Trend |
| --- | --- | --- |
| PreviousRank = 8, Rank = 2 | 8 | ↑6 |
| PreviousRank = 2, Rank = 4 | 2 | ↓2 |
| PreviousRank = Rank | supplied rank | — (Unchanged) |
| PreviousRank is NULL, PreviouslyUnranked = 1 | NR | New |
| PreviousRank is NULL, PreviouslyUnranked = 0 | — | — (Unavailable) |

Calculate movement as `PreviousRank - Rank` only when PreviousRank is supplied.
Give arrows accessible text such as “Up 6 places.” Preserve tied ranks and nullable
values. Do not calculate a previous rank from retained snapshots: retention removes
old polls, while `PreviousRank` is supplied directly in the current poll.

## Big 12 summary

Place three compact cards between the poll context and national table. Derive them
from the full selected current poll before UI pagination or source/team filters.
Use stored `IsBig12` / `Big12SchoolId`; do not match school names in the frontend.

1. **Big 12 teams ranked:** count distinct TeamId where IsBig12 = 1.
2. **Highest-ranked Big 12:** minimum Rank among those teams, displaying the rank
   and team name. List all tied teams at that rank.
3. **Big 12 movement:** biggest riser and biggest faller among Big 12 rows with
   PreviousRank present. Riser is the largest positive delta; faller is the most
   negative delta. Show team names and movement; retain all ties. When tied names
   are lengthy, wrap them or show an expandable list without silently dropping teams.

If there are no ranked Big 12 teams, show 0 and “None ranked” for the other cards.
If teams are ranked but no positive/negative movement exists, show “No risers” /
“No fallers”; if all previous ranks are missing, show “Movement unavailable.”
Newly ranked teams are not numeric movers. Show an optional “New: [teams]” line
inside the movement card only for PreviouslyUnranked = 1 and PreviousRank NULL.

Cards should wrap on small screens. Keep the table horizontally scrollable and
preserve the existing national rankings view and Big 12 legend.

## Data query

Use a bound `@SourceId` selected from enabled SportsRankingSources. This returns
all rows needed for the header, table and summary in one query:

```sql
SELECT s.ID AS SourceId, s.Sport, s.LastSuccessAt,
       CAST(CASE WHEN s.LastError IS NULL THEN 0 ELSE 1 END AS bit) AS HasRefreshError,
       p.ID AS PollRowId, p.PollName, p.SeasonLabel,
       p.PublishedAtUtc, p.Period, p.SeasonPhase, p.SourceUrl, p.FetchedAt,
       t.ID AS RankingRowId, t.TeamId, t.TeamName, t.Rank,
       t.PreviousRank, t.PreviouslyUnranked, t.Points, t.FirstPlaceVotes,
       t.Record, t.LogoUrl, t.TeamUrl, t.Big12SchoolId, t.IsBig12
FROM dbo.SportsRankingSources s
JOIN dbo.SportsRankingPolls p ON p.SourceId=s.ID AND p.IsCurrent=1
JOIN dbo.SportsRankingTeams t ON t.PollRowId=p.ID AND t.IsCurrent=1
WHERE s.Enabled=1 AND s.ID=@SourceId
ORDER BY t.Rank, t.TeamName, t.ID;
```

Both IsCurrent predicates are required. Treat SQL BIGINT IDs as `long`, TeamId as
string, ranks and school IDs as nullable integers where applicable, and Points as
nullable double. Compute summaries once from these rows, not once per rendered row.
Do not add summary queries against schedules or standings.

Keep enabled sources selectable even when they have no current poll. If the query
returns no rows, show “Rankings not available yet” and hide summary values; an
unavailable poll must not appear as zero ranked teams. On a request failure, show
a retryable availability state. Encode provider text and allow only safe HTTP(S)
links/images using the frontend's existing URL validation.

## Acceptance checks

- Selected sport changes the header, poll date/week, rows and summary together.
- Previous 8 / current 2 renders Previous 8 and Up 6; previous 2 / current 4 renders Down 2.
- New, unknown and unchanged ranks remain distinct and accessible without color.
- Big 12 summary uses the complete current national poll, including tied ranks.
- No Big 12 teams, missing movement, tied leaders/movers and new entrants display correctly.
- Optional missing week/phase fields leave no empty separators or fabricated labels.
- An old poll with a recent fetch still shows its actual old publication date.
- Empty/failed queries do not display misleading zero-count cards.
- Desktop and phone layouts retain readable cards, table scrolling and keyboard access.

Validate the frontend build and these behaviors in the frontend repository before
deployment. The backend field availability and calculation contract were checked
against this checkout's schema, parser and retention SQL; live frontend rendering
is outside this handoff's validation.
