# National Top 25 implementation — 2026-10-11

User requested the full national polls with Big 12 teams highlighted for all five
tracked college sports. Implemented separate configurable polls, preserving the
publisher's ranks, dates, season labels, previous ranks, records and available
points/first-place votes. ESPN transports AP, United Soccer Coaches/IWSOC and AVCA
polls. Sources are selected by exact league/poll ID, not array position.

| Sport | Poll | Poll date | Season label | Ranked teams | Big 12 |
| --- | --- | --- | --- | ---: | ---: |
| Football | AP Top 25 | 2026-10-04 | 2026 | 25 | 5 |
| Men's basketball | AP Top 25 | 2026-04-07 | 2025–26 | 25 | 5 |
| Women's basketball | AP Top 25 | 2026-04-06 | 2025–26 | 25 | 3 |
| Women's soccer | United Soccer Coaches/IWSOC | 2026-10-06 | 2026 NCAA Division I Women's Soccer | 25 | 5 |
| Women's volleyball | AVCA Division I Women's Top 25 | 2026-10-05 | 2026 | 25 | 5 |

Live validation imported 125 ranking entries across five polls, including 23
Big 12 entries (not 23 unique schools). Current official Big 12 member metadata
supplies a separate Big12SchoolId through exact normalized school-name matching.
ESPN IDs remain scoped to the sport/source; former members are not highlighted.

New additive tables: SportsRankingSources, SportsRankingPolls, SportsRankingTeams.
--configure-rankings registers five disabled sources. --rankings-from-db imports
enabled sources and is added to hourly compose. Runtime grants are table-scoped;
schema setup remains restricted to migration credentials.

Each poll and its teams commit atomically. Repeated imports and corrected ranks
preserve IDs/FirstSeenAt. New poll dates archive older snapshots. Earlier poll
responses/revisions are rejected rather than replacing newer data. Empty, truncated
and invalid polls retain last-good data. Ties and unavailable values are preserved.

Verification:

- 119 offline tests pass, no failures; 14 opt-in database/live tests skipped.
- Six new offline tests cover all five source fixtures, distinct provider IDs,
  current membership, missing statistics, new/unknown previous ranks, ties,
  incomplete/duplicate teams, wrong league/poll/season and invalid links/dates.
- Isolated SQL test passes: corrections persist, repeat IDs remain stable, incomplete
  and older responses cannot replace data, forced insert failure rolls back, and
  publication rollover succeeds.
- All five live polls import with restricted runtime credentials into
  WebScraper_Rankings_Validation_20261011; production was not used for fixtures.
- Live CLI dry run: five successes, zero failures.
- Clippy with warnings denied and formatting checks pass.

Production migration/enabling/deployment has not been performed for rankings.
The previously deployed standings/digital features remain unchanged. My-Newsfeed
must implement the new UI; source dates, older basketball labels, Big 12 badges,
queries and deployment order are documented in docs/my-newsfeed-top25-rankings.md.
