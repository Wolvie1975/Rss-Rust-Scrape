# Big 12 production feed review — October 12, 2026

Completed approximately 11:03 p.m. Central using the deployed
`web-scraper:parser-fixes-20261012` image. All 16 enabled school RSS feeds were
refreshed together through the normal production importer.

Result: 16 source successes, zero failures, exit code 0. All current-feed RSS/JSON
UTC warnings cleared; 13 feeds have zero issues. Three invalid or unverifiable
provider records remain quarantined. All feeds recorded a fresh LastSuccessAt.

| School | Issues before | Issues after | Remaining issue |
| --- | ---: | ---: | --- |
| Kansas | 1 | 0 | Clear |
| Arizona | 1 | 0 | Clear |
| Arizona State | 0 | 0 | Clear |
| Baylor | 2 | 0 | Clear |
| BYU | 1 | 0 | Clear |
| UCF | 2 | 0 | Clear |
| Cincinnati | 1 | 0 | Clear |
| Colorado | 3 | 1 | 183088: self-opponent provider record |
| Houston | 2 | 0 | Clear |
| Iowa State | 2 | 1 | 183106: self-opponent provider record |
| Kansas State | 1 | 0 | Clear |
| Oklahoma State | 1 | 0 | Clear |
| TCU | 4 | 0 | Clear |
| Texas Tech | 2 | 0 | Clear |
| Utah | 3 | 1 | 182788: member opponent has no verified game alias |
| West Virginia | 3 | 0 | Clear |

Remaining source records:

- Colorado game 183088: November 25 men's basketball lists Colorado vs Colorado
  in both RSS and JSON. The provider self-opponent record is quarantined.
- Iowa State game 183106: December 10 men's basketball lists Iowa State at Iowa
  State in both RSS and JSON. The provider self-opponent record is quarantined.
- Utah game 182788: November 23 women's basketball against Iowa State lacks
  opp_game_id in the JSON metadata. It remains quarantined because the importer
  cannot verify the paired identity for a conference-member opponent.

The timing warnings checked in stored source payloads used midnight Central
placeholders, with the JSON local kickoff agreeing with the RSS UTC instant.
This includes November dates after the daylight-saving change; the deployed
America/Chicago comparison handles both offsets. Older retained source snapshots
can still contain historical warnings, but current per-feed status is refreshed.

The audit shows 981 public sports events, 1,444 retained Big 12 source observations,
and zero duplicate canonical keys. No further code changes or deployment were
needed. No commits, pushes, schema changes, or manual timing overrides performed.
