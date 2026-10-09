# Big 12 expansion verification — October 9, 2026

Production import and standalone service deployment completed.

- 16 current full-member school feeds enabled; five-sport scope retained.
- 943 public SportsEvents rows and 1,444 source observations, including retained history.
- 1,431 live observations published through canonical groups; 10 retained Kansas observations; 3 source records quarantined.
- All 101 original Kansas event IDs, FirstSeenAt, primary feed, opponents, school/opponent logos preserved.
- All 16 school-logo URLs returned HTTP 200.
- 36 source records carry RSS/JSON football time-disagreement warnings. Public timing uses RSS; unknown times remain NULL/TBD.
- Concurrent full staging reruns both saved 1,431 observations and quarantined 3; event/source counts and IDs stayed unchanged. No duplicate canonical keys.
- Unit/parser tests passed; staging SQL integration passed for mirrored IDs, repeated imports, primary reschedules, secondary stale dates, disabled feeds, failure status, and preserving last-good source data.
- Final container read-only preview passed, changed no event/source/status rows, and rejected setup combined with dry-run.
- School calendar view verified: Kansas and Utah game IDs 179871/179872 resolve to one public SportsEventId, with Kansas away and Utah home, and RSS UTC start 2026-10-11 02:15.
- Neutral-site consistency query returned 0 invalid home/away rows.
- Hourly service runs independently of VS Code with its existing news/movie/YouTube flags, network, restart policy and persistent volume.

## School calendar observations

Use dbo.SportsEventSchoolCalendar for school-specific filtering. Counts include retained Kansas history and exclude quarantined observations.

| School | Calendar rows |
| --- | ---: |
| Arizona | 90 |
| Arizona State | 93 |
| Baylor | 90 |
| BYU | 92 |
| Cincinnati | 91 |
| Colorado | 88 |
| Houston | 89 |
| Iowa State | 90 |
| Kansas | 101 |
| Kansas State | 92 |
| Oklahoma State | 78 |
| TCU | 92 |
| Texas Tech | 88 |
| UCF | 91 |
| Utah | 88 |
| West Virginia | 88 |

## Quarantine

- Colorado, provider game 183088: self-opponent provider record.
- Utah, provider game 182788: member opponent has no verified game alias.
- Iowa State, provider game 183106: self-opponent provider record.

These records remain in SportsEventSources with no public event link for review.
Oklahoma State has no volleyball coverage; other target sports are available.

Operation/query guide: [big12-sports.md](../docs/big12-sports.md).
The separate .NET application was not changed; school-filtered screens should use the new view.
Rollback image: web-scraper:before-big12-20261009. Host source and pre-expansion sports snapshot:
.deployment-backups/20261009-big12/. The legacy image must use the explicit Kansas RSS
via --events-feed rather than --events-from-db, because it does not implement canonical deduplication.
