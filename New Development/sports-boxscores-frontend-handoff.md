# Frontend handoff: optional sports box scores

Backend work requested October 19, 2026 (America/Chicago). Deployment evidence and
initial collection counts: [backend operations report](../reports/sports-boxscores-20261019.md).

Initial deployed backfill checked all **1,355** stored game identities:
**960** have statistics (**362** with player box scores; **598** team-only), and
**395** retain the existing fallback. There were **zero collection errors**.
These are dated counts; query per-game availability rather than hardcoding coverage.

## Requested behavior

Offer a game's box score when the scraper has usable statistics. If no detail is
available, retain the existing result display and any provider match link already
provided by that view. Do not hide a game, replace its score, fabricate a box-score
URL, or show an empty box-score panel merely because a provider advertises a link.

This handoff concerns the separate **My-Newsfeed** frontend repository. No frontend
files or image were changed by the scraper deployment. The main Results page's
current fallback is its score/status/team/venue row; it has no existing game-detail
anchor. The MLS and Championship result views already link their score to
`SportsMatchResults.SourceUrl`; preserve those links. Schedule views can retain
`LiveStatsUrl ?? Url`. These are separate fallback cases.

## Read model and joins

Read **WebScraper.dbo.SportsBoxScores**, a new view over an optional cache.
The composite identity is **(Sport, League, EventId)**. EventId alone is insufficient.
All keys are strings, using ESPN's lowercase sport and league identifiers.
Do not replace, migrate, or write `ResultGames` or `SportsMatchResults` to integrate.

| Column | SQL type | Meaning |
| --- | --- | --- |
| Sport | nvarchar(40) | e.g. `football`, `basketball`, `baseball`, `soccer`, `hockey`, `volleyball` |
| League | nvarchar(60) | e.g. `nfl`, `college-football`, `mlb`, `wnba`, `usa.1`, `usa.nwsl`, `eng.2`, `nhl` |
| EventId | nvarchar(20) | Provider event identity, matching existing results |
| HasBoxScore | bit | Usable team and/or player statistics have been cached |
| Availability | varchar(12) | `available`, `unavailable`, `error` |
| Payload | nvarchar(max), nullable | Normalized JSON version 1, described below |
| SummaryUrl | nvarchar(2048), nullable | Provider-supplied HTTPS ESPN game link, validated against event ID |
| BoxScoreUrl | nvarchar(2048), nullable | Provider-supplied explicit box-score link, when supplied |
| FetchedAt | datetime2, nullable | UTC time of last successful statistics snapshot |
| LastAttemptAt | datetime2 | UTC time of last check |
| NextAttemptAt | datetime2 | Internal refresh/retry timing; do not display as game time |
| LastError | nvarchar(1000), nullable | Operational diagnostics; do not expose raw error text publicly |

Join the ordinary Results read query with a **LEFT JOIN**, preserving every
existing filter, result row, sort order, team record and score source:

```sql
SELECT g.ID,g.Sport,g.League,g.EventId,
       b.HasBoxScore,b.Payload,b.FetchedAt,b.SummaryUrl,b.BoxScoreUrl
FROM dbo.ResultGames g
LEFT JOIN dbo.SportsBoxScores b
  ON b.Sport=g.Sport AND b.League=g.League AND b.EventId=g.EventId
WHERE g.Completed=1 AND g.State=N'post';
-- Apply the existing follows/day/window filters from ResultsQuery as well.
```

The `ResultItem` projection already carries EventId, Sport and League. Extend its
projection/DTO or fetch details in one batch for visible rows; avoid one SQL call
or a provider HTTP request per rendered card. No request-time ESPN scraping is
needed. Prefer loading the larger Payload only on the detail route or expansion,
while the results list fetches availability and links.

For `SportsMatchResults`, map provider identity explicitly:

| ProviderKey | Sport | League | EventId |
| --- | --- | --- | --- |
| `espn-mls` | `soccer` | `usa.1` | GameId converted to string |
| `espn-championship` | `soccer` | `eng.2` | GameId converted to string |
| `espn-nhl` | `hockey` | `nhl` | GameId converted to string |

Keep the existing enabled-source and season filtering on these results views.
Example join, inside the existing result query:

```sql
LEFT JOIN dbo.SportsBoxScores b
 ON b.EventId=CONVERT(nvarchar(20),r.GameId)
 AND b.Sport=CASE WHEN r.ProviderKey=N'espn-nhl' THEN N'hockey' ELSE N'soccer' END
 AND b.League=CASE r.ProviderKey
   WHEN N'espn-mls' THEN N'usa.1'
   WHEN N'espn-championship' THEN N'eng.2'
   WHEN N'espn-nhl' THEN N'nhl' END
```

F1's existing classifications are separate and do not use this home/away model.

## JSON payload version 1

The payload contains only normalized game statistics, not a full ESPN response,
article bodies, betting odds or inferred values. JSON field names are camelCase.
Treat all provider strings as plain text and HTML-encode them.

```json
{
  "version": 1,
  "sport": "football",
  "league": "nfl",
  "eventId": "401873007",
  "hasTeamStats": true,
  "hasPlayerStats": true,
  "teams": [
    {
      "id": "6",
      "name": "Dallas Cowboys",
      "homeAway": "away",
      "score": "17",
      "lineScores": [],
      "statistics": [
        {"group": null, "key": "firstDowns", "label": "1st Downs", "value": "24"}
      ]
    }
  ],
  "playerGroups": [
    {
      "teamId": "6",
      "name": "passing",
      "columns": [
        {"key": "passingYards", "label": "YDS", "description": "Yards"}
      ],
      "athletes": [
        {"id": "2577417", "name": "Dak Prescott", "position": null,
         "starter": null, "stats": ["258"]}
      ],
      "totals": ["255"]
    }
  ]
}
```

This shortened example illustrates shape; production snapshots include both teams
and the full available columns/groups. Team score strings are provider detail;
keep the existing results table as the source of the result card's score.

- **Team statistics:** group by optional `group`, then align home/away cells by
  `(group,key)`, not array index. Baseball nests batting/pitching groups; these are
  flattened with their group key retained. Missing cells display an em dash.
- **Player statistics:** one table per team/group. Match `teamId` to a team ID;
  each athlete's `stats[i]` and `totals[i]` match `columns[i]`. Use provider labels
  and descriptions rather than hardcoded sport-specific columns. Totals may be
  absent; do not recompute them (provider definitions can differ from simple sums).
- **Line scores:** `lineScores` has `{period,displayValue}` rows only when the
  summary supplies them. It can be empty; do not reconstruct innings/quarters/sets
  from a final score or assume a fixed period count. `period` can be null.
- Values are display strings or null. Preserve `0`, percentages, fractions,
  dashes and decimal precision. Never coerce missing statistics to zero.
- Player ID, position, starter, column key/description, group name and team name
  can be null. Do not treat null starter as false. Skip unusable labels safely.
- Guard against malformed JSON and unknown payload versions: preserve the current
  result fallback rather than breaking the entire results page.

## Availability and fallback UX

1. `HasBoxScore=1`, supported payload version and valid parsed data: show an
   accessible **Box score** action if player statistics exist; **Game stats** is
   an accurate label for team-statistics-only soccer coverage. An expandable panel
   or separate game detail page is suitable; keep current results readable.
2. Render only populated sections. Soccer often provides possession/shots/team
   statistics without player tables or a distinct box-score URL. Other sports can
   provide player tables; coverage varies per game, not just per league.
3. An absent row, `HasBoxScore=0`, `unavailable`, `error`, missing/malformed Payload,
   or unsupported version uses the existing fallback. Don't show a nonfunctional
   Box score button. An optional provider link may use `BoxScoreUrl ?? SummaryUrl`
   after the normal URL safety checks; the existence of that link alone does not
   prove that stored statistics exist. Preserve existing SourceUrl links.
4. `HasBoxScore=1` can coexist with `LastError`: a refresh failed and last-good
   detail was retained. Keep that usable detail, optionally show its FetchedAt as
   “Stats updated …”, and avoid publishing raw errors. FetchedAt is UTC and should
   use the app's existing date/time formatting.
5. Keep shootout totals separate from regulation/extra-time match goals, using the
   existing result model. Do not infer winners or shootout tallies from these stats.

## Refresh and limits

The scraper includes optional enrichment in its existing hourly
`--events-from-db` path, after ordinary schedule/results ingestion. It reads
completed games already in either shared results model, deduplicates their ESPN
identity, and checks up to **100 games / 180 seconds per run**, paced 250 ms apart.
Recent games take priority, followed by unchecked history. Each provider request
has a 12-second timeout. An absent snapshot can mean queued/not yet checked.

Successful snapshots are eligible for refresh after six hours; missing statistics
retry after 24 hours; transient/validation errors retry after six hours. These are
eligibility times, not guarantees of an exact refresh time. First collection was
explicitly run after deployment; the operations report records actual coverage.
New `.NET` results can appear between hourly scraper runs and receive detail later.
No data availability is inferred from `boxscoreAvailable` or an advertised link.

## Frontend acceptance checks

- Ordinary Results, MLS, Championship and any NHL view retain their existing
  result rows, scores, date/league/team filters, records, mobile layout and links.
- A populated NFL/MLB/WNBA/NHL game renders available player/team statistics using
  provider columns; soccer renders actual available team stats without inventing
  player data. Empty college-sport summaries retain the current score fallback.
- Missing/error/unavailable cache entries do not hide games or display empty
  tables. A failed refresh retains last-good detail. No API calls run per page row.
- Same-event duplicates across the two result models use one composite cache row;
  different leagues cannot leak statistics into each other.
- URL handling, encoded text, UTC display, keyboard access, narrow-screen tables,
  and missing metadata work. Test a route with a nonexistent or malformed identity.
- Frontend database identity can SELECT the view; it does not need cache writes,
  DDL permissions, or new provider secrets. Existing scores and saved-news IDs
  remain unchanged.

## Backend files

- `src/sports_boxscores.rs`: normalized parser, identity checks, bounded collector.
- `src/db.rs`, `src/main.rs`: persistence, explicit setup/refresh, hourly integration.
- `sql/sports_boxscores*.sql`: additive schema/view, permissions, candidate query,
  transactional upsert preserving last-good statistics.
- `tests/fixtures/boxscores/`: reduced real provider responses for format tests.

Deployment and exact verification evidence belong in the linked operations report;
this document does not claim the frontend UI has been implemented or tested.
