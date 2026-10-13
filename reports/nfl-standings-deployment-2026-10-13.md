# NFL standings deployment — October 13, 2026

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
