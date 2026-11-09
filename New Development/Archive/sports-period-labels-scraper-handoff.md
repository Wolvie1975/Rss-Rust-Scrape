# Scraper handoff: identify scoring periods in cached box scores

Requested October 19, 2026 (America/Chicago).

## Objective

Populate meaningful period labels in the existing version-1 box-score cache so
My-Newsfeed can identify innings, quarters, halves, overtime and shootouts.
Cover MLB, NFL, college football, NHL, men's college basketball, and every soccer
league already collected (including MLS, NWSL and Championship).

This is a request for the separate scraper project. This document does not mean
that collector code, stored data or frontend period presentation has been updated.
Keep the existing optional cache and result-score sources intact.

## Observed issue and investigation

The frontend reads `WebScraper.dbo.SportsBoxScores.Payload`, keyed by
`(Sport, League, EventId)`. It consumes `teams[].lineScores[].period` and
`displayValue`. An inspected MLB snapshot (`baseball`, `mlb`, `401930515`) has
nine score entries per team but a null `period` on every entry. The UI deliberately
retains the entries without inventing inning numbers. Audit other sports before
claiming they have the same cause; missing labels have not been verified for
every league.

Inspect actual provider responses and the current normalizer. Determine whether
period identity lives in a sibling field, a separate periods array, or is defined
by the provider's ordered-array contract. Do not assume each item has a `period`
property. Capture sanitized fixtures that establish the mapping.

## Existing compatible JSON contract

Continue emitting `version: 1`. Each line-score entry must have:

| Field | Type | Requirement |
| --- | --- | --- |
| `period` | string or number, nullable | Prefer a complete display label string, such as `1st inning`, `Q1`, `1st period`, `1st half`, `OT`, `2OT`, `ET1` or `SO`. Numeric values are supported but display as bare numbers today. |
| `displayValue` | string, nullable | Preserve the provider's display value, including `0`, precision and explicit unplayed markers. The current frontend retains entries only when this field is a string. |

A descriptive string in **`period` itself** is the compatible path: a new
`periodLabel` field alone will be ignored by the deployed frontend. New optional
metadata may be added without removing existing fields, but must be documented
for a coordinated frontend follow-up. Do not bump the version without agreement;
the current parser rejects unsupported versions.

Illustrative fragment only (not a complete payload):

```json
{
  "lineScores": [
    { "period": "Q1", "displayValue": "0" },
    { "period": "Q2", "displayValue": "7" },
    { "period": "Q3", "displayValue": "3" },
    { "period": "Q4", "displayValue": "0" },
    { "period": "OT", "displayValue": "6" }
  ]
}
```

Use the same label for the same period across competitors, even when one team's
value is missing. Preserve provider order and every real entry; never deduplicate
unknown labels or drop a zero. Do not add empty periods just to fill regulation.

## Sport-specific mapping requirements

| Sport / league | Required interpretation |
| --- | --- |
| Baseball / `mlb` | Identify each actual inning, including extra innings. Align the same inning across teams. Preserve explicit unplayed bottom-half markers; a missing or unplayed half is not a scored zero. Do not create a ninth inning or assume every game lasts nine innings. |
| Football / `nfl`, `college-football` | Identify regulation quarters separately from overtime. Retain distinct overtime entries if supplied and identify multiple overtime periods when supported. Do not combine overtime into Q4 or assume NFL and college overtime data have identical structure. |
| Hockey / `nhl` | Identify regulation periods, overtime periods and any explicitly supplied shootout separately. Preserve multiple overtime periods in playoff data. A shootout is not another ordinary overtime period. |
| Basketball / `mens-college-basketball` | Use regulation halves for this league, then identify overtime periods separately. Do not apply NBA/WNBA quarter labels. Verify the exact league identity against existing stored rows. |
| Soccer / existing leagues | Identify first/second halves and separately supplied extra-time periods. Distinguish a penalty shootout from match/extra-time goals. Preserve provider labels when scoring is grouped differently. Do not treat elapsed minutes or stoppage-time notation as period scores. |

These are mapping requirements, not permission to manufacture unavailable detail.
Confirm each league's provider representation using real fixtures. A provider may
supply cumulative scores rather than per-period increments: do not subtract or
reinterpret them silently. Document that case and coordinate presentation if the
existing contract cannot express it accurately.

For NHL and soccer, keep the recorded result, shootout indicators and shootout
scores in their existing sources. Never infer a shootout score from the winning
margin, add it to match goals, or sum cached periods to overwrite the result.

## Safe extraction and fallback

1. Prefer explicit provider period identifiers/labels, resolving references into
   period metadata when supplied separately.
2. Use array position only when actual provider evidence establishes a complete,
   ordered period sequence for that sport/feed. Add fixture tests for that rule.
   Live, shortened, sparse, extra-time and shootout cases must not be mislabeled.
3. If mapping is ambiguous, leave `period` null and retain the original value.
   Log a concise internal diagnostic; do not fabricate a label for visual polish.
4. Preserve cached team/player statistics, availability behavior, provider links,
   composite identities and last-good snapshot handling. A label repair must not
   wipe usable stats or mark a team-only snapshot unavailable.

Do not derive periods from a final score, assumed regulation length or the number
of entries alone. Do not introduce request-time provider scraping in My-Newsfeed.

## Backfill and verification

- Audit counts by sport/league: cached games, line entries, null/blank labels,
  resolvable labels, ambiguous labels and collection errors. Include the audit date.
- Fix normalization, then re-normalize retained source payloads if available;
  otherwise refresh existing cached identities through the normal bounded,
  rate-limited collection process. A scheduled refresh alone may not repair old
  completed games that are no longer selected, so include a historical backfill.
- Update successful snapshot timestamps according to the existing cache policy;
  preserve last-good data when provider requests fail.
- Test regulation plus extra innings, unplayed baseball half-innings, football
  overtime, NHL regular-season shootout and playoff multiple overtime, men's
  college basketball halves/multiple overtime, and soccer extra time/shootouts.
  Use recorded fixtures or clearly marked synthetic edge cases where unavailable.
- Include absent period metadata, sparse arrays, one team missing a value, `0`,
  duplicate unknown labels, shortened games and malformed provider responses.
- Verify original line values, team/player columns, totals and result rows remain
  unchanged except for intended label repairs; verify reruns are idempotent.
- Query through `dbo.SportsBoxScores` using the frontend runtime's read identity
  to confirm the repaired payloads are visible without granting new write access.

## Return handoff for the frontend

Provide the normalizer changes, sanitized before/after examples, mapping evidence,
fixture/test results, deployment/backfill status, counts and remaining limitations.
Include sample `(Sport, League, EventId)` identities for each covered sport and
edge case, plus the exact labels emitted. Never include credentials in the report.

The deployed frontend already accepts string labels. Baseball places period labels
in its summary header; other sports currently expose them under **Scoring by
period**. Repairing the cache will populate those existing views. A combined NFL,
NHL, basketball or soccer scoreboard, or new shootout/extra-time UI, is a separate
frontend change and should not be claimed as delivered by the collector repair.

Related contract: [sports box-score frontend handoff](sports-boxscores-frontend-handoff.md).
