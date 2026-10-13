# WNBA schedule data review — October 13, 2026

## Finding

The October 14 listings are stale conditional semifinal Game 5 placeholders,
not a timezone error. The official next game is Finals Game 1 on Saturday,
October 17 at 2:30 p.m. Central (19:30 UTC): Atlanta Dream at Golden State Valkyries.

Official confirmation:
https://valkyries.wnba.com/news/golden-state-valkyries-advance-to-2026-wnba-finals-20261009
The announcement confirms a 3–0 Golden State semifinal sweep and the Finals
matchup against Atlanta, ruling out the old semifinal Game 5 fixtures.

## Stored rows and current provider response

| Public row ID | ESPN game ID | Stored date | Stored matchup |
| --- | --- | --- | --- |
| 15409 | 401918303 | October 14 | Atlanta Dream vs New York Liberty |
| 15410 | 401918304 | October 14 | Golden State Valkyries vs Las Vegas Aces |

Both retain STATUS_SCHEDULED and TimeTbd=1. Their last observation was
October 9 at approximately 10:07 p.m. Central. They were not refreshed by the
October 13 successful WNBA import. The current October ESPN scoreboard omits
both IDs; each ESPN summary endpoint returns 404.

The current ESPN scoreboard includes seven potential Finals games beginning
October 17. Games 5–7 are explicitly labeled “If Necessary.” Current stored
Finals Game 1 (401918305) has the correct date, matchup and 19:30 UTC start.

Provider endpoint inspected:
https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard?dates=202610&limit=1000

## Cause and proposed correction

The importer updates returned game IDs but retains previously imported rows when
an ID disappears. This protects history and avoids treating an incomplete provider
response as cancellation, but leaves withdrawn conditional playoff games visible.
A successful feed status therefore does not guarantee every retained upcoming
row is still present in the latest source response.

The two confirmed obsolete fixtures should be excluded from upcoming schedules
while preserving their historical rows. Durable reconciliation should distinguish
withdrawn conditional fixtures from cancellation and from a transient missing
response; do not delete historical records or infer cancellation from omission
alone. Frontend queries must honor the resulting inactive/withdrawn state.

Initial review changed no data. The user subsequently authorized the production
correction; see [the correction report](wnba-stale-correction-2026-10-13.md).
