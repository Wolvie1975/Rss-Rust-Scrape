# Sports implementation and deployment history

Historical evidence, newest dates first. Counts, image tags and rollout details describe the recorded deployment, not live state. Use the topic guides for current integration and retention.

## Contents

- [F1 production deployment — October 18, 2026](#f1-deployment-2026-10-18)

- [Brewers–Dodgers finished-series schedule cleanup — October 16, 2026](#brewers-dodgers-cleanup-2026-10-16)

- [NWSL all-team schedule expansion — October 16, 2026](#nwsl-league-deployment-2026-10-16)

- [Big 12 soccer and men’s basketball standings verification — October 15, 2026](#big12-standings-review-2026-10-15)
- [NHL schedules, standings and results deployment — October 14, 2026](#nhl-deployment-2026-10-14)
- [WNBA correction and stale schedule review — October 13, 2026](#wnba-stale-correction-2026-10-13)
- [WNBA schedule data review — October 13, 2026](#wnba-data-review-2026-10-13)
- [NFL standings deployment — October 13, 2026](#nfl-standings-deployment-2026-10-13)
- [NFL division standings — October 13, 2026](#nfl-divisions-2026-10-13)
- [MLS schedule and results deployment](#mls-deployment-2026-10-13)
- [Premier League deployment — October 13, 2026](#epl-deployment-2026-10-13)
- [EFL Championship / Wrexham deployment — October 13, 2026](#championship-deployment-2026-10-13)
- [Big 12 production feed review — October 12, 2026](#big12-feed-review-2026-10-12)
- [Sports standings implementation and validation — 2026-10-11](#sports-standings-2026-10-11)
- [Production deployment — national rankings and MLS](#rankings-mls-deployment-2026-10-11)
- [National Top 25 implementation — 2026-10-11](#national-top25-2026-10-11)
- [WNBA schedule verification — 2026-10-09](#wnba-schedule-2026-10-09)
- [Professional schedule scraper deployment — 2026-10-09](#scraper-professional-schedules-deployment-2026-10-09)
- [MLB schedule verification — 2026-10-09](#mlb-schedule-2026-10-09)
- [MLB scraper deployment — 2026-10-09](#mlb-deployment-2026-10-09)
- [Kansas team logo correction — October 9, 2026](#kansas-logo-fix-2026-10-09)
- [Kansas City Current schedule verification — 2026-10-09](#kansas-city-current-schedule-2026-10-09)
- [ESPN schedule cutoff — 2026-10-09](#espn-upcoming-only-2026-10-09)
- [Big 12 expansion verification — October 9, 2026](#big12-expansion-2026-10-09)

<a id="f1-deployment-2026-10-18"></a>

## F1 production deployment — October 18, 2026

F1 news was subsequently added through four enabled RSS sources in **F1 News**
(BBC Sport, Autosport, Motorsport.com and The Guardian). The running image
imported 40 articles successfully; hourly news collection needs no restart.
[News UI contract](../docs/f1-news-frontend-integration.md) and
[registration evidence](../docs/f1-operations.md#f1-news-registration).


Deployed `web-scraper:f1-20261018`; deployed-container run 1206 succeeded.
Season 2026 verification: 23 races, 374 Grand Prix classifications, 132 sprint
classifications, 23 drivers and 11 constructors through round 17. The frontend
production reader could query all four views. Hourly settings were preserved;
the next scheduled slot was logged but not awaited. The previous container/image
and protected host backup are retained. No frontend rendering was implemented.

Full [deployment evidence and rollback](f1-production-20261018.md),
[frontend contract](../docs/f1-frontend-integration.md) and
[operations](../docs/f1-operations.md).

<a id="brewers-dodgers-cleanup-2026-10-16"></a>

## Brewers–Dodgers finished-series schedule cleanup — October 16, 2026

User requested removal of remaining schedule games after the NLCS ended. A fresh
ESPN October scoreboard omitted the three unused games and retained the four
completed series games. SQL still held these scheduled placeholders because an
absent provider observation does not automatically delete an existing fixture.

Removed exactly three `SportsEvents` rows from MLB source 4193, guarded by row ID,
GameId, provider, source and both team names inside one transaction:

| Date (Central) | Row ID | ESPN GameId |
| --- | --- | --- |
| October 16 | 17861 | 401908034 |
| October 18 | 17862 | 401908035 |
| October 19 | 17863 | 401908036 |

Archived complete original rows at
`/var/opt/mssql/backup/webscraper-managed/manual/BrewersDodgers_schedule_cleanup_20261016.json`.
Re-imported the normal MLB feed with runtime credentials: no failures, none of the
three fixtures recreated. Remaining MLB schedule identities matched the prior
snapshot minus exactly these three rows (11 retained). ResultGames remained at
33 MLB rows; completed results were not deleted. The live MLB Events selection
returned HTTP 200 with all three removed GameIds absent. No collector image,
scheduler, source configuration or frontend code was changed.

<a id="nwsl-league-deployment-2026-10-16"></a>

## NWSL all-team schedule expansion — October 16, 2026

Expanded the existing Kansas City Current schedule to all NWSL clubs, as requested.
Final scraper image `web-scraper:nwsl-league-20261016`:
`sha256:d49272ed529b8917c78fea455a5aea6b7e1bf69213a77f6e4ac7151d57df7f63`.
The underlying shared ESPN parser no longer forces team 20907. No team parameter
uses the home-team perspective; explicit single-team NWSL filters still preserve
that selected team's perspective. Canonical event IDs/URLs remain unchanged.

Applied the explicit NWSL seed update with migration credentials: source **4191**
keeps its ID and Enabled flag, but uses the unfiltered scoreboard URL, display
name NWSL and null source SchoolName. Its legacy ProviderSchoolId=20907 tag remains
because the deployed frontend checks that tag to recognize NWSL; the collector
uses the URL to select teams. Reconfiguration recognizes that expanded source
without creating another source or changing its scope. New installs register a
disabled source with tag 0. No table schema, results collection or other source
configuration was changed.

The existing frontend query's Kansas City source-tag guard initially hid the new
source when tag 0 was tried. Retaining its original tag restored compatibility;
no frontend code/image was changed. Separate dirty frontend theme/MyTeams files
were inspected only and left untouched. Future frontend cleanup should accept
both tags, label source selectors NWSL and support follow coverage for every team;
see [frontend notes](../docs/nwsl-league-schedule.md#source-setup-and-frontend-compatibility).

Validation: **162 offline tests** passed; strict Clippy and formatting passed.
The opt-in isolated SQL test passed separately and validates legacy source ID and
Enabled preservation, 24-event expansion, stable retained Current IDs/first-seen,
repeat imports, reschedules/cancellations. Its database was removed. The October
provider fixture spans all **16 clubs**, including Boston and Denver.

Read-only new-image preview succeeded with zero failures. Initial production
imports **168 and 169**, **3:14 p.m. Central**, succeeded and retained **24 upcoming
fixtures / 24 distinct GameIds / 16 teams**. All three pre-existing Current event
IDs, first-seen timestamps, dates and kickoff instants were unchanged. Only source
4191's configuration changed. Home/opponent perspective now reflects league-wide
home-team orientation rather than always Current. No duplicate fixtures were added
on repeat. Final-image production run **170**, **3:21:41–3:21:43 p.m. Central**, also
succeeded with the same 24 fixtures/16 teams. Final scraper/frontend health checks
passed; the hourly scheduler remained set for 4:00 p.m. Central.

The live `/events?schedule=espn-nwsl%7CSoccer` page returned 200, selected NWSL, and
rendered eight known current-window matches, seven without Kansas City. This page
retains its today-through-next-three-days display; the backend keeps all remaining
upcoming 2026 schedule data. Source/team preferences must check both event team IDs
for a specific club. Existing completed results remain independent.

Verified copy-only/checksum backup:
`/var/opt/mssql/backup/webscraper-managed/manual/WebScraper_before_nwsl_league_20261016.bak`.
Protected source/config/evidence backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261016-nwsl-league/`.
Rollback image `web-scraper:before-nwsl-league-20261016` and stopped container
`web-scraper-before-nwsl-league-20261016` retain the preceding direct-movie-links
build. Rollback to Current-only also requires explicitly restoring the original
source URL/name metadata; do not delete new league game rows without a separate
cleanup scope. Runtime Env, scheduler, network, mounts, limits and security settings
were preserved. No Git commit/push was performed.

<a id="big12-standings-review-2026-10-15"></a>

## Big 12 soccer and men’s basketball standings verification — October 15, 2026

Compared the live production standings with the conference’s published tables in
response to a soccer display discrepancy and a men’s basketball screenshot.

- Soccer source **7**, season **2026**: all 16 teams, row order and 12 statistic
  columns (192 values) matched the official explicit standings page
  `https://big12sports.com/standings.aspx?standings=238&path=soc`.
  Its latest successful refresh observed was 18:00 UTC / 1:00 p.m. Central.
- The rendered app at `/standings?sourceId=7` also matched all conference records
  and points after accounting for W–L–T record strings versus W/D/L columns.
  It displayed conference GP/W/D/L/points and omitted other stored columns.
  No stale or incorrect scraper data was found.
- Men’s basketball source **5**, season **2025-26**: all 16 teams, row order and
  10 statistic columns (160 values) matched
  `https://big12sports.com/standings.aspx?path=mbball`, including the supplied
  screenshot’s conference/overall records, percentages, home/away splits, neutral
  records and streak. Latest successful refresh observed was 19:00 UTC /
  2:00 p.m. Central.
- Both sources were enabled and had no reported source error at inspection.
  No scraper fixes, database writes, configuration changes, deployment or
  scheduler restart were required. The user will adjust the soccer frontend;
  frontend changes were not implemented or verified in this review.

The consolidated [standings guide](../docs/sports-tables.md#big12-soccer-basketball-columns)
now records both complete `StatsJson` mappings. Consumers must distinguish
repeated column labels using their positional `name` within each source’s layout.
These timestamps and values are historical observations, not live status.

<a id="nhl-deployment-2026-10-14"></a>

## NHL schedules, standings and results deployment — October 14, 2026

Deployed `web-scraper:nhl-20261014`, image
`sha256:8bb3315e1a9eb0ac714f2ca425579c5f230e32df35e16351f3aa0b5feb81c645`.
The production `web-scraper` container and `mynewsfeed-web` were healthy at final
verification. No front-end deployment or SQL Server restart was performed.

### Production data

NHL schedule/results source 5637 and division standings source 13 are enabled.
ESPN season 2027 represents 2026–27.

| Data | Verified rows |
| --- | ---: |
| Current/future NHL fixtures | 1,239 |
| Final results | 170 |
| Regulation finishes | 133 |
| Overtime finishes | 23 |
| Shootout finishes | 14 |
| Current division standings teams | 32 |
| Past NHL fixture rows | 0 |
| Duplicate NHL canonical fixture keys | 0 |

The collector read 1,409 schedule observations, including completed observations
used for reconciliation. Completed games were stored as results independently of
schedule retention. NHL standings retain division order and provider statistics.

### Verification

- Full offline suite: 144 passed; 18 opt-in tests skipped.
- Formatting, strict Clippy and Cargo audit passed. Rustls was updated to 0.23.46.
- Read-only NHL live feeds and a dry run using the release image passed.
- SQL integration test passed in newly created isolated database
  `WebScraper_NHL_Validation_20261014`; the database was removed afterward.
- Targeted imports 104 and 105 succeeded. Fixtures, results and standings kept
  identical IDs and first-seen timestamps on repeat import.
- A real schedule/results import from the deployed image succeeded as run 106
  at 08:33 Central, with one source success and zero failures. Its normal retention
  cleanup succeeded. NHL IDs remained unchanged afterward.
- Existing non-NHL sports source URLs, enabled flags and limits matched the
  pre-deployment snapshot.
- Runtime environment, scheduler arguments, TLS certificate mount, data volume,
  network, restart policy, limits, security settings, logging and healthcheck
  matched the previous container.
- Startup logged the next aligned hourly collection for 09:00 Central on
  October 14. That scheduled slot was not awaited during deployment verification.

The host build inputs matched the pre-NHL source before applying the change.
Only changed code, SQL, fixtures and new NHL documentation were synchronized;
existing host documentation and credentials were preserved. No Git commits or
pushes were performed.

### Rollback

Previous image: `web-scraper:before-nhl-20261014` (also retained as
`web-scraper:cinema-posters-20261013`). Previous container is stopped and retained
as `web-scraper-before-nhl-20261014`.

Protected host backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261014-nhl/`.
It contains source/configuration and before/after verification snapshots. Keep its
permissions private because the rollback configuration includes the runtime
connection profile. No credentials are in this report.

Before returning to the old image, disable only the NHL sources so the older
collector does not attempt the new feeds:

```sql
UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ProviderKey='espn-nhl';
UPDATE dbo.SportsStandingsSources SET Enabled=0 WHERE ProviderKey='espn-nhl';
```

On the Docker host, stop the new container, rename it aside, rename the retained
old container to `web-scraper`, retag the rollback image as `web-scraper:latest`,
and start the old container. Restore changed host source from the protected backup
before rebuilding. NHL rows can remain for later reactivation.

<a id="wnba-stale-correction-2026-10-13"></a>

## WNBA correction and stale schedule review — October 13, 2026

The user authorized production updates and a quick review of other stale entries.

<a id="wnba-stale-correction-2026-10-13--production-correction"></a>

### Production correction

Public event rows 15409 and 15410 (ESPN IDs 401918303 and 401918304) are confirmed
unnecessary October 14 semifinal Game 5 fixtures. Both semifinal series ended
before those games were needed, as confirmed by the official Finals matchup.
The current ESPN scoreboard omits both IDs and their summaries return 404.

A guarded SQL transaction set only these two rows to STATUS_CANCELED, the existing
inactive status recognized by the site's upcoming-event queries. This is a local
administrative correction for unnecessary fixtures, not a new ESPN observation.
No rows were deleted; event dates, IDs, FirstSeenAt and LastSeenAt were preserved.
The existing frontend excludes both fixtures. The next active WNBA event is
October 17 at 19:30 UTC (2:30 p.m. Central), Atlanta at Golden State.
No binary, schema, permissions or web-app changes were needed.

Prior row snapshots: [backup JSON](wnba-stale-correction-before-2026-10-13.json).
Rollback, if explicitly needed: restore ScheduleStatus='STATUS_SCHEDULED' for
ProviderKey='espn-wnba', GameId IN (401918303,401918304), preserving other fields.

<a id="wnba-stale-correction-2026-10-13--quick-review-scope-and-result"></a>

### Quick review scope and result

Scanned all 981 stored sports rows for upcoming active events whose last observation
predated the feed's latest successful refresh. Only the two corrected WNBA games
matched that stale-refresh pattern.

Compared stored active events dated October 13–November 30 with all 19 live schedule
feeds: 16 Big 12 school RSS feeds, MLB and WNBA monthly ESPN scoreboards, and Kansas
City Current's NWSL scoreboard (filtered to its stored team). Every provider request
completed successfully. Only the two corrected WNBA fixtures were absent; no other
missing upcoming entries were found in this window. The comparison used the
pre-correction snapshot, so those two IDs intentionally appear in its findings.
This was a schedule review, not an audit of news, movies, TV or standings; dates
after November 30 were screened by refresh timestamps but not live-cross-checked.

The importer still retains omitted provider fixtures. This targeted data correction
does not implement automatic withdrawal reconciliation for future conditional games.
No commits or pushes performed.

<a id="wnba-data-review-2026-10-13"></a>

## WNBA schedule data review — October 13, 2026

<a id="wnba-data-review-2026-10-13--finding"></a>

### Finding

The October 14 listings are stale conditional semifinal Game 5 placeholders,
not a timezone error. The official next game is Finals Game 1 on Saturday,
October 17 at 2:30 p.m. Central (19:30 UTC): Atlanta Dream at Golden State Valkyries.

Official confirmation:
https://valkyries.wnba.com/news/golden-state-valkyries-advance-to-2026-wnba-finals-20261009
The announcement confirms a 3–0 Golden State semifinal sweep and the Finals
matchup against Atlanta, ruling out the old semifinal Game 5 fixtures.

<a id="wnba-data-review-2026-10-13--stored-rows-and-current-provider-response"></a>

### Stored rows and current provider response

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

<a id="wnba-data-review-2026-10-13--cause-and-proposed-correction"></a>

### Cause and proposed correction

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
correction; see [the correction report](sports-history.md#wnba-stale-correction-2026-10-13).

<a id="nfl-standings-deployment-2026-10-13"></a>

## NFL standings deployment — October 13, 2026

Completed approximately 8:15 a.m. Central.

- Image: web-scraper:nfl-standings-20261013 (also latest).
- Image ID: sha256:92d4734b3e4f9df381cfee99ac60b4d406dbcdc1c56092ebab1ed616c0572d86.
- Enabled NFL source ID 10: espn-nfl, Football, minimum 32 teams.
- ESPN endpoint: https://site.api.espn.com/apis/v2/sports/football/nfl/standings.
- First production standings import exited 0. NFL LastError is NULL,
  LastTeamCount=32, LatestSeason=2026, and LastSuccessAt is fresh.
- 16 AFC and 16 NFC rows stored; all 32 team IDs represented.
- Runtime health check exited 0; existing hourly standings command retained.
- /standings?sourceId=10 returns HTTP 200 and renders NFL, Kansas City,
  American Football Conference and National Football Conference.
  Existing dynamic source selection and generic statistic columns support NFL;
  no web-app rebuild was required.

Validation: 96 binary tests passed, 14 database-dependent tests skipped;
standings fixture validates 32 teams and two conference groups. Formatting and
Clippy with warnings denied passed. ESPN league identity l:28 is checked on fetch.
Wins, losses, ties, percentages, points and division records are retained as supplied.

No schema or permission changes were required. NFL was registered disabled through
the additive seed, then explicitly enabled after image activation. Existing source
URLs and enable settings were preserved. SQL Server was not restarted.

Rollback image: web-scraper:before-nfl-standings-20261013.
Previous container retained stopped: web-scraper-rollback-nfl-20261013.
Host source backup: .deployment-backups/20261013-nfl-standings/source.tar.
Disable espn-nfl when reverting to a binary without NFL support.
No commits or pushes performed.

<a id="nfl-divisions-2026-10-13"></a>

## NFL division standings — October 13, 2026

Production NFL source ID 10 now uses ESPN's standings?level=3 endpoint.
The deployed parser already supports nested groups; no binary rebuild or schema
change was necessary. Seed configuration was updated for new installations.
Existing configuration was explicitly updated with migration credentials.

Verification: production standings refresh exited 0, NFL LastError is NULL,
and the current snapshot has 32 unique teams across eight divisions with four
teams each. Old conference-group rows are historical (IsCurrent=0).
The existing /standings?sourceId=10 page renders all eight division names.
A new fixture test verifies unique membership, all eight group sizes, provider
order within divisions and Kansas City Chiefs membership in AFC West.

The frontend handoff now documents AFC/NFC parent headings, division ordering,
current-row queries, fallback handling and the compact NFL column mapping:
[sports-tables.md#my-newsfeed-nfl-standings](../docs/sports-tables.md#my-newsfeed-nfl-standings).
The frontend hierarchy change is documented for the site builder; the web app
was not modified or rebuilt in this task.

Rollback: set the NFL source URL back to the same endpoint without ?level=3
and refresh standings. Retained snapshots make the next valid import authoritative.
No commits or pushes performed.

<a id="mls-deployment-2026-10-13"></a>

## MLS schedule and results deployment

Source 5525: espn-mls / usa.1, enabled, picked up by existing hourly
--events-from-db collection. MLS standings source 9 remains unchanged.
Image web-scraper:mls-20261013, verified running hash
sha256:21a7ddb95eacea6e0596073d959255859c4a01af5cf503251c14859f3b2b965c.
Prior forecasts image/container preserved for rollback. No Git commit or push.

Production: 90 current/future fixtures (Oct 14–Nov 7), 421 final 2026 results
(Feb 21–Oct 11). Zero past/canceled fixtures after retention. Deployed targeted
import succeeded with zero failures in 3.065 seconds; runtime health exit 0,
scraper and front-end containers healthy. Results stay in their independent
current-season table; past schedule rows are removed.

Shared soccer parser/writer now supports MLS plus Championship. Nullable
HomeShootoutScore/AwayShootoutScore added to SportsMatchResults. Penalty tallies
remain separate from match goals. Source/provider guards and stable provider/game
identity prevent mixing leagues. Repeats/corrections preserve IDs and FirstSeenAt.

109 unit tests passed (16 environment-dependent tests ignored). Isolated SQL
verification passed for Championship corrections, MLS shootout persistence,
repeated IDs/timestamps, disabled-source guards and league separation; temporary
test database removed. Wrong league/season, invalid scores and missing/tied
shootout results are rejected by parsing tests. Full-season live dry run passed
before writing data, and live database counts were checked after deployment.

Initial schema setup was retried with migration credentials; runtime account
cannot perform DDL. Source activation was retried with QUOTED_IDENTIFIER enabled.
Both setup retries succeeded before rollout.

Front-end integration pending: docs/sports-schedules.md#mls-schedule-results-frontend-handoff.

<a id="epl-deployment-2026-10-13"></a>

## Premier League deployment — October 13, 2026

Completed approximately 4:10 p.m. Central.

Image web-scraper:epl-20261013 (also latest).
Image ID sha256:00cc4a3a8d111f98762b23ceac88814ffd15f6fefb9027f24fd1870ccb8d8c67.

Premier League schedule source ID 5357 and standings source ID 12 are enabled.
ProviderKey=espn-epl, competition eng.1 / ESPN league 700, current season 2026–27.
Both sources are discovered by the existing hourly events/standings collectors.

Production targeted import exited 0. Source LastError fields are NULL with fresh
success timestamps. Audit: 320 fixtures, 20 standings teams, zero past EPL
fixtures, zero historical EPL results and zero superseded EPL standings rows.

The collector begins with the current month and stops at current-season June.
Today's final observations can reconcile existing scheduled rows but are never
inserted as historical results. Existing retention removes past/canceled matches.
London TBD placeholders preserve the source calendar date; known UTC instants
use Central display dates. ESPN requests use the existing My-Newsfeed/1.0 header.

EPL standings delete only this provider's non-current rows within the successful
snapshot transaction. Other leagues' history policies are unchanged. DELETE on
SportsStandings was added to the runtime role for this operation; configuration
editing and DDL remain unavailable. No schema changes or SQL Server restart.

Validation: 100 binary tests pass; 15 integration tests skipped in the default
suite. The isolated standings SQL test passed for failure rollback, stable
repeat imports, EPL rollover leaving only 20 current rows, preservation of the
other league's historical rows, and runtime permissions. The temporary test
database was removed. Clippy with warnings denied and formatting checks pass.

Frontend: existing /standings?sourceId=12 discovers the table automatically.
Add Premier League to any hardcoded schedule selector and reuse soccer table
columns. Queries and display contract: [sports-schedules.md#my-newsfeed-epl](../docs/sports-schedules.md#my-newsfeed-epl).
No separate web-app rebuild performed.

Verified COPY_ONLY/CHECKSUM/compressed backup:
/var/opt/mssql/backup/WebScraper_before_epl_20261013.bak.
Rollback image: web-scraper:before-epl-20261013.
Previous container: web-scraper-rollback-epl-20261013 (stopped).
Host source backup: .deployment-backups/20261013-epl/source.tar.
Disable both EPL sources before reverting to a binary without EPL support.
No commits or pushes performed.

<a id="championship-deployment-2026-10-13"></a>

## EFL Championship / Wrexham deployment — October 13, 2026

Completed approximately 4:01 p.m. Central.

Image web-scraper:championship-20261013 (also latest and championship-20261013-r2).
Image ID sha256:8d58371231a536982e9bea7cfc176cb15c12d0f841f12669f4ea5cbf279b29bb.

Sources enabled:

- Schedule/results SportsEventsType ID 5326, ProviderKey espn-championship.
- Standings SportsStandingsSources ID 11, same ProviderKey, minimum 24 teams.
- ESPN competition eng.2 / league 3914; Wrexham team ID 352.

Production verification: targeted import exited 0. Both source LastError fields
are NULL with fresh success timestamps. Current 2026–27 data contains 437 future
fixtures, 24 standings teams, and 115 unique completed match results. Wrexham has
36 future fixtures and 10 completed results. Counts can change with live matches.

The additive SportsMatchResults table stores final home/away goals separately
from SportsEvents, so past schedule cleanup does not delete current-season scores.
Prior-season results are removed only after a valid new-season result import.
No writes to the separate application's ResultGames or ResultFollows tables.
Current hourly events/standings commands discover enabled sources automatically.

Schedule identity uses espn:eng.2:<gameId>. Known kickoffs retain UTC and Central
display dates; TBD midnight-London placeholders preserve their source dates.
Final goal values are validated; non-final/abandoned statuses are not results.
An initial production scoreboard fetch was rejected with 403. The collector now
uses the same My-Newsfeed/1.0 request header as existing ESPN schedules; a matching
production-client preview and the subsequent production import succeeded.

Validation: 99 binary tests passed; 15 integration tests skipped in the default
suite. The isolated Championship SQL test passed for repeated imports, score
corrections, preserved IDs/FirstSeenAt and disabled-source rollback. The temporary
test database was removed. Clippy with warnings denied and formatting checks pass.

The existing standings selector supports sourceId=11. The Wrexham controls and
results view need separate frontend implementation. Contract and queries:
[sports-schedules.md#my-newsfeed-championship-wrexham](../docs/sports-schedules.md#my-newsfeed-championship-wrexham).
No web-app rebuild was performed.

Verified COPY_ONLY/CHECKSUM/compressed database backup:
/var/opt/mssql/backup/WebScraper_before_championship_20261013.bak.
Rollback image: web-scraper:before-championship-20261013.
Original rollback container: web-scraper-rollback-championship-20261013 (stopped).
Host source backup: .deployment-backups/20261013-championship/source.tar.
Disable both new sources before running a binary without Championship support.
SQL Server was not restarted. No commits or pushes performed.

<a id="big12-feed-review-2026-10-12"></a>

## Big 12 production feed review — October 12, 2026

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

<a id="sports-standings-2026-10-11"></a>

## Sports standings implementation and validation — 2026-10-11

Implemented eight configurable sources independent of schedule and results imports.

| Source | Teams | Published season |
| --- | ---: | --- |
| MLB | 30 | 2026 |
| WNBA | 15 | 2026 |
| NWSL, all teams | 16 | 2026 |
| Big 12 football | 16 | 2026 |
| Big 12 men's basketball | 16 | 2025–26 |
| Big 12 women's basketball | 16 | 2025–26 |
| Big 12 soccer | 16 | 2026 |
| Big 12 volleyball | 15 | 2026 |

ESPN's live standings API provides complete groups, provider team identities,
names/logos/links, season labels and sport-specific statistics. Official Big 12
HTML provides records, percentages, streaks, splits and soccer points; structured
member metadata supplies stable conference IDs. The soccer menu advertises a
working explicit standings ID, which the collector discovers automatically.

New SportsStandingsSources/SportsStandings tables hold last complete snapshots.
Refreshes are atomic, preserve row identities, retain older seasons as noncurrent,
and leave last-good data intact on fetch/parse/write failure. StatsJson retains
provider values and labels rather than calculating standings from partial schedules.
ESPN identity uses numeric season year, independent of display-label changes.

Added configuration/import CLI flags, hourly compose flag, restricted SQL grants,
live fixtures for all eight sources, tests and the My-Newsfeed UI/read contract.

Validation:

- Full offline suite: 105 passing tests, no failures; 11 opt-in SQL/live tests skipped.
- Six standings fixture tests pass across all eight feeds, including bad seasons,
  incomplete/duplicate tables, unknown members and unsafe links.
- Isolated SQL test passes: repeat imports and display-label changes keep IDs,
  updated statistics persist, incomplete imports are rejected, forced insert
  failure rolls back current flags, season rollover succeeds, and runtime account
  lacks DELETE/configuration-URL UPDATE/CREATE TABLE permissions.
- Live test with webscraper_ingest against
  WebScraper_Standings_Validation_20261011: all eight sources imported,
  **140 current rows**; repeated snapshots preserve IDs and FirstSeenAt.
- Live CLI dry run: eight successes, zero failures.
- Clippy with warnings denied and formatting checks pass.

Production database, running scraper and My-Newsfeed UI were not changed.
Before deployment apply the additive migration, seed/configuration and runtime
permission updates; enable sources, then deploy the new hourly command. The
separate My-Newsfeed checkout must implement the standings UI. See
[the configuration and integration guide](../docs/sports-tables.md#sports-standings).

<a id="rankings-mls-deployment-2026-10-11"></a>

## Production deployment — national rankings and MLS

Completed 2026-10-11, approximately 09:36 Central.

- Image: web-scraper:rankings-mls-20261011 (also latest).
- Image ID: sha256:74ace0f94b42db58bf766bf095206c3e12d34525b56347ea9036ce2af0274fc2.
- Additive ranking schema and restricted grants installed; five poll sources enabled.
- MLS source registered/enabled through the existing standings configuration.
- Hourly command includes --rankings-from-db and --standings-from-db alongside
  the existing news, schedules, movies, TV and YouTube collectors.

First targeted production import: **14 source successes, zero failures**.
Five national polls each contain 25 teams (125 entries); 23 entries have current
Big 12 markers. Basketball remains correctly labeled 2025–26 with April 2026 poll
dates. Football AP is dated October 4, soccer October 6, volleyball October 5.

MLS: 30 teams, 15 Eastern / 15 Western; standings now total 170 current rows across
nine sources. Source error fields are NULL. The latest ScraperRuns record is
succeeded with no error summary. Schedules remain 981 rows and MLB has zero 2027
rows. Separate NWSL results and previously deployed movie imports were untouched.

Scraper and My-Newsfeed containers are healthy. Runtime health check passes;
/events, /movies, /standings and /standings?sourceId=9 return HTTP 200. The existing
standings frontend exposes MLS and displays both conferences, including Inter
Miami and Sporting Kansas City. The web app was not rebuilt by this deployment;
its /rankings route remains 404 and the separate frontend builder must implement
that page using docs/sports-tables.md#my-newsfeed-top25-rankings.

Runtime account still lacks CREATE TABLE and DELETE-ranking privileges. Verified
SQL TLS and runtime credential isolation are preserved; SQL Server was not restarted.

Validation before deployment: formatting, Clippy with warnings denied, 94 binary
offline tests pass; the full suite previously passed 119 offline tests. Rankings
and nine-source standings isolated SQL/live tests also passed.

Verified COPY_ONLY backup with CHECKSUM, permissions 600:
/var/opt/mssql/backup/WebScraper_before_rankings_mls_20261011.bak.

Protected source/compose backup on the Docker host:
/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261011-rankings-mls/.
Rollback image: web-scraper:before-rankings-mls-20261011. Restore the previous compose
command with that image (old binary must not receive --rankings-from-db); disable
espn-mls when reverting to a binary that lacks MLS support. Additive ranking tables
can remain during binary rollback. No commits or pushes performed.

<a id="national-top25-2026-10-11"></a>

## National Top 25 implementation — 2026-10-11

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
queries and deployment order are documented in docs/sports-tables.md#my-newsfeed-top25-rankings.

<a id="wnba-schedule-2026-10-09"></a>

## WNBA schedule verification — 2026-10-09

- Added league-wide WNBA schedule support, with optional local ESPN team-ID filters.
  Uses existing SportsEvents/Types and shares ESPN parsing/upserts with Kansas City.
- Live ESPN September scoreboard returned 38 games with both teams, home/away,
  venues, confirmed UTC starts, statuses, broadcasts and WNBA gamecast links.
- Live 2026 dry-run completed 12 monthly requests, returning 378 unique fixtures.
  Default rolling-year dry-run completed 36 requests, returning 705 unique fixtures.
- All-target tests passed: 65 main-binary tests and 19 example tests; seven DB tests
  remain opt-in. Separately ran and passed both WNBA and NWSL SQL integration tests
  with rollback-only fixture writes in isolated validation databases.
- WNBA SQL test covers disabled seed, Enabled/edited URL preservation on rerun,
  stable IDs and FirstSeenAt across repeats and postponement/rescheduling, retained
  missing games, and independent public rows for equal numeric IDs in other leagues.
- Default WNBA rows consistently represent the home team and away opponent, even
  when filtered by the away team. Provider-namespaced team IDs are stored in existing
  SchoolId/OpponentSchoolId columns for My-Newsfeed query filtering.
- Added --configure-wnba and sql/wnba_seed.sql; no new schema beyond the shared
  ScheduleStatus migration from the Kansas City importer. Results sync untouched.
- My-Newsfeed checkout absent. UI/query contract and enabling steps are documented
  in docs/sports-schedules.md#wnba-schedule. No production schema/enable/import/deployment or restart.

<a id="scraper-professional-schedules-deployment-2026-10-09"></a>

## Professional schedule scraper deployment — 2026-10-09

The updated scraper is deployed to production. Kansas City Current NWSL and
league-wide WNBA feeds are enabled and imported into the shared WebScraper DB.

<a id="scraper-professional-schedules-deployment-2026-10-09--runtime"></a>

### Runtime

- Source commit: `0230d69` (added two women's sports to schedule page).
- Host checkout: `/home/jcarpio/Projects/Rss-Rust-Scrape`; workspace source synced
  before the Compose image build, without overwriting `.env` or Git metadata.
- Image: `web-scraper:latest`, also tagged `web-scraper:pro-schedules-0230d69`.
- Image digest: `sha256:bd3b2344911aefbb9ff4eaa0793241c58af536276f9d4f246329b1e1379d0457`.
- `web-scraper` recreated and running with `unless-stopped`, zero restart failures.
  Existing hourly command preserved, including `--events-from-db`, news, YouTube,
  movies and TV. Hourly slots remain aligned to America/Chicago.
- Both configure commands ran from the new image against production. Verified
  `SportsEvents.ScheduleStatus` is nullable `NVARCHAR(100)`.
- Feed ID **4191**: `espn-nwsl`, ProviderSchoolId **20907**, enabled, Kansas City
  team parameter retained, no fixed season restriction.
- Feed ID **4192**: `espn-wnba`, ProviderSchoolId **0**, enabled, league-wide URL,
  no team or fixed season restriction.

<a id="scraper-professional-schedules-deployment-2026-10-09--validation"></a>

### Validation

- 88 regular all-target tests passed; both NWSL/WNBA rollback-only staging SQL
  integration tests passed separately. Release binary and Docker image built.
- New-image production dry-run returned **3 Kansas City** and **13 WNBA** upcoming
  fixtures. Production import and repeat import ran inside the deployed container.
- Database now has **959 public events**, **18 feeds**, and **1,444 existing school
  source observations**. Both professional feeds have successful imports, expected
  event counts and NULL LastError.
- All **943 original event IDs, URLs, GameIds, canonical keys and FirstSeenAt values**
  preserved. All 16 new event identities/FirstSeenAt preserved on repeat import;
  zero duplicate canonical keys. No historical games deleted or results sync changed.
- Every imported professional game is scheduled and dated Oct 9 or later Central.
  WNBA TBD dates Oct 11/14 retain ESPN's advertised date. Conditional playoff
  games with TBD opponents remain provider-supplied future fixtures.
- Live My-Newsfeed `/events` returned HTTP 200 and rendered the new team names.
  Its current selector still says “All schools” and lacks a WNBA option; the web
  application's new Schedule selector needs its own deployment. My-Newsfeed was
  inspected read-only and not rebuilt/restarted by this scraper deployment.

<a id="scraper-professional-schedules-deployment-2026-10-09--rollback-and-evidence"></a>

### Rollback and evidence

- Previous image: `web-scraper:before-pro-schedules-20261009`.
- Host backup folder: `.deployment-backups/20261009-professional-schedules/`:
  `source.tar`, `sports-before.json`, `sports-first-import.json`, `sports-after.json`.
- Application rollback can use the previous image. Disable the two ESPN feeds
  before running that older scraper, which does not understand these JSON feeds.
  Leave the additive nullable column and event rows in place; no database deletion
  is required. Source backup excludes credentials and Git metadata.
- Production `.env`, persistent RSS volume and existing service command retained.
  Temporary deployment helper and temporary SQL example removed after verification.

<a id="mlb-schedule-2026-10-09"></a>

## MLB schedule verification — 2026-10-09

- Added MLB league support to the shared ESPN schedule pipeline, reusing existing
  event models and locked stable-ID upserts. No results-sync changes or new schema.
- `--configure-mlb` and `sql/mlb_seed.sql` register a disabled lookup row:
  ProviderKey espn-mlb, ProviderSchoolId 0. Existing hourly --events-from-db handles
  the enabled provider after deployment; MLB namespaced IDs isolate other leagues.
- Verified ESPN MLB monthly scoreboard's teams/home-away, venues, UTC dates,
  timeValid, statuses, broadcasts/logos and game links, plus unresolved playoff
  participants with negative ESPN placeholder IDs.
- Live default dry-run against isolated staging returned 2,916 unique fixtures:
  22 remaining 2026 games and 2,894 announced 2027 games, dates Oct 10, 2026 through
  Sept 26, 2027. Asserted every preview date is >= Oct 9, 2026 Central. No past or
  completed games imported; explicit future cancellation observations remain for
  reconciliation. No production rows/feed settings were written.
- All-target tests passed: 72 main-binary plus 19 example tests; eight SQL tests
  remain opt-in. MLB SQL integration test separately passed with rollback-only
  fixtures in WebScraper_Mlb_Validation_20261009. Covers configuration preservation,
  separate doubleheader rows, repeat/reschedule/postponement identity, first-seen
  preservation, retained missing games and independent equal IDs in other leagues.
- All-target cargo check and diff whitespace checks passed.
- Configuration and My-Newsfeed query/selector handoff: docs/sports-schedules.md#mlb-schedule.
  MLB is not deployed/enabled; prior WNBA/Current production deployment unchanged.

<a id="mlb-deployment-2026-10-09"></a>

## MLB scraper deployment — 2026-10-09

MLB schedule importing is deployed and enabled in production.

- Built the workspace MLB changes on base commit `0230d69`; these changes are
  not committed by this deployment. Synced source to the host checkout at
  `/home/jcarpio/Projects/Rss-Rust-Scrape` without overwriting .env/Git metadata.
- Image: `web-scraper:latest`, also tagged `web-scraper:mlb-20261009`.
  Digest: `sha256:257a666459d34838a8431d61f9db2efbf5c5857854afabe58f7b67ca1111bcbd`.
- Recreated `web-scraper`. Verified it is running the new image with its unchanged
  hourly --events-from-db command, existing news/YouTube/movie/TV features,
  persistent RSS volume and unless-stopped restart policy.
- Ran --configure-mlb from the new image against WebScraper, then enabled feed
  **4193**: ProviderKey **espn-mlb**, ProviderSchoolId **0**. Its URL is league-wide
  with no team or fixed-season restriction. No additional schema was required.
- New-image preview, production import and repeat import returned **2,916 MLB
  fixtures**: **22 remaining 2026 games** and **2,894 announced 2027 fixtures**.
  All dates are today-forward in America/Chicago; all observations were scheduled.
- Database now contains **3,875 public sports events**, **19 feeds**, and **1,444
  school source observations**. All **959 preexisting event IDs, FirstSeenAt,
  URLs, GameIds and canonical keys** preserved, including WNBA/Current.
  Repeat import preserved all MLB identities/FirstSeenAt; zero canonical duplicates.
- 91 regular all-target tests and the separate rollback-only MLB staging SQL test
  passed before deployment. Live imports completed successfully.
- My-Newsfeed /events returned HTTP 200 and rendered MLB game links. It still has
  no dedicated MLB selector option; that UI/provider-selector change is separate.
  The web app was inspected read-only, not rebuilt or restarted.

Rollback image: **web-scraper:before-mlb-20261009**. Host backup folder:
`.deployment-backups/20261009-mlb/`, containing source.tar and before/first/after
sports snapshots. Disable only the MLB feed before running that older scraper,
which supports WNBA/Current but does not understand MLB. Retain imported events
and the shared nullable ScheduleStatus column; rollback needs no row deletion.

Temporary deployment helper and temporary SQL enable example removed after use.
Production credentials were neither printed nor committed. No Git commit/push.

<a id="mlb-deployment-2026-10-09--user-requested-season-correction"></a>

### User-requested season correction

Following the user's clarification, feed 4193 was restricted to `season=2026`.
Backed up and deleted only the 2,894 MLB rows dated 2027 or later. Reimport from
the configured URL returned 22 upcoming 2026 games. Verified database totals:
981 sports events (959 preexisting plus 22 MLB), zero 2027 MLB games. All other
sports were retained. Backup: `mlb-2027-before-removal.json` in the deployment
backup folder. Hourly collection honors the saved URL; no rebuild/restart needed.

<a id="kansas-logo-fix-2026-10-09"></a>

## Kansas team logo correction — October 9, 2026

Corrected **101 existing Kansas event rows** in WebScraper.dbo.SportsEvents.

- Existing rows before/after: 101 / 101.
- Current RSS: 91 events updated through the existing URL-keyed MERGE.
- Retained older events: 10 corrected by a TeamLogoUrl-only update scoped to their single-school feed type.
- Event IDs, event URLs, FirstSeenAt and OpponentLogoUrl were unchanged on all 101 rows. No duplicate event URLs.
- Saved school logo: [https://big12sports.com/images/logos/jhwk4C_RF_OL%20(3).png](https://big12sports.com/images/logos/jhwk4C_RF_OL%20(3).png). HTTP 200, image/png, 23269 bytes; visually confirmed to show the Kansas Jayhawk.
- Cause: every RSS `<s:teamlogo>` points to `/images/logos/site/site.png`, the conference logo.
- Fix: resolve the feed's school_id against the calendar page members component and use that school's image.url. Missing school metadata/image yields NULL. Opponent extraction is unchanged.
- Parser regression fixtures: tests/fixtures/big12-kansas-calendar.xml and big12-calendar-members.html.
- Tests: cargo test --locked --all-targets; rollback-only live SQL test school_logo_refresh_preserves_identity_opponents_and_other_feeds passed.
- Standalone Docker service rebuilt and restarted with the fix; existing hourly schedule, flags, network, credentials and persistent data volume retained.

<a id="kansas-city-current-schedule-2026-10-09"></a>

## Kansas City Current schedule verification — 2026-10-09

- Inspected repository handoff and existing Big 12/Sidearm pipeline; no AGENTS.md
  files were present in the workspace. My-Newsfeed checkout is absent here.
- ESPN team schedule returned 27 completed 2026 games, omitting upcoming fixtures.
  Monthly NWSL scoreboards returned 30 unique 2026 Kansas City fixtures, including
  Oct 17 at Utah, Oct 25 vs Washington, and Nov 1 at Racing Louisville.
- Live CLI dry-run with `season=2026` completed 12 monthly requests and reported 30
  fixtures. Default rolling-year dry-run completed 36 requests and reported 57.
- All-target tests passed (62 main-binary tests plus 19 example tests; six SQL
  integration tests opt-in). The new SQL integration test was separately run and
  passed on `WebScraper_Nwsl_Validation_20261009`, with fixture writes rolled back.
  It verifies disabled configuration preservation, repeat imports, timed
  rescheduling, cancellation, retention on absence, stable IDs and FirstSeenAt.
- Schema initialized only in the isolated staging database. No production feed
  configuration, schedule import, application deployment, or scheduler restart.
- Handoff/configuration/migration: `docs/sports-schedules.md#kansas-city-current-schedule`.

<a id="espn-upcoming-only-2026-10-09"></a>

## ESPN schedule cutoff — 2026-10-09

Both WNBA league and Kansas City Current NWSL imports now collect only today-or-later
fixtures using America/Chicago calendar dates, excluding completed games even on
today's date. Requests start in the current month and extend through next calendar
year, rather than refreshing historical seasons. Explicit season settings still
respect the cutoff. Past seasons produce no requests or observations.

Future cancellation/postponement observations remain available for stable-ID
reconciliation; the upcoming UI query excludes completed/canceled statuses and
historical rows. Existing DB rows are retained; no deletion or results-sync changes.
My-Newsfeed checkout is still absent, so its date/status query contract is documented
in both provider guides rather than implemented in that repository.

Live staging CLI dry-runs returned 3 Kansas City games (Oct 17, Oct 25, Nov 1) and
13 WNBA fixtures including current playoffs. ESPN's timeValid=false WNBA dates use
midnight Eastern placeholders: 2026-10-11T04:00Z is advertised as 10/11 TBD. The parser
now preserves that date instead of labeling it Oct 10 Central. Confirmed starts
retain UTC-to-Central conversion. Tests cover both summer/winter placeholders,
both leagues, inclusive today, past dates, completed games and empty past seasons.

All-target validation passed: 69 main-binary tests plus 19 example tests, with
seven DB integration tests opt-in. No production enabling/import/deployment or
scheduler restart was performed.

<a id="big12-expansion-2026-10-09"></a>

## Big 12 expansion verification — October 9, 2026

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

<a id="big12-expansion-2026-10-09--school-calendar-observations"></a>

### School calendar observations

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

<a id="big12-expansion-2026-10-09--quarantine"></a>

### Quarantine

- Colorado, provider game 183088: self-opponent provider record.
- Utah, provider game 182788: member opponent has no verified game alias.
- Iowa State, provider game 183106: self-opponent provider record.

These records remain in SportsEventSources with no public event link for review.
Oklahoma State has no volleyball coverage; other target sports are available.

Operation/query guide: [sports-schedules.md#big12-sports](../docs/sports-schedules.md#big12-sports).
The separate .NET application was not changed; school-filtered screens should use the new view.
Rollback image: web-scraper:before-big12-20261009. Host source and pre-expansion sports snapshot:
.deployment-backups/20261009-big12/. The legacy image must use the explicit Kansas RSS
via --events-feed rather than --events-from-db, because it does not implement canonical deduplication.

