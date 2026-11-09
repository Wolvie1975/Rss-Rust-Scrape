# New-chat handoff — October 19, 2026

## Start here

This is the Rust scraper repository at
`/workspaces/Rust_Projects/Rss-Rust-Scrape`. Read this file, then
[HANDOFF.md](HANDOFF.md) and [README.md](README.md). The user maintains a separate
My-Newsfeed frontend and handles its UI work in another chat/repository.
The latest requested backend changes are complete: expand Box Office news beyond
Deadline. No deployment or source addition from this conversation remains pending.

The user wants useful additions implemented and verified, not merely proposed.
User-facing dates/times use America/Chicago. The active IDE file is a SQL CA
certificate; it does not imply that certificate changes are requested.

## Production and access

- Last verified running scraper image: `web-scraper:f1-20261018`; container
  `web-scraper`. Scraper and frontend were healthy during October 19 verification.
  These are recorded observations, not a fresh health check for the next chat.
- Host checkout: `/home/jcarpio/Projects/Rss-Rust-Scrape`. Workspace and host
  checkout are separate; authorized deployment changes must reach the host too.
- Docker daemon is accessible through `/var/run/docker.sock` in the dev container.
  The Docker CLI was absent. Docker Engine HTTP API over the Unix socket worked;
  use curl/Python if needed. Do not assume temporary scripts under `/tmp` persist.
- Database: **WebScraper**, SQL Server `sql2025` on the Docker network. `.env` is
  the runtime ingestion profile; `.env.migrations` is for explicit migrations.
  Explicitly select WebScraper: the migration connection may default to master.
- Never print connection strings, tokens, environment contents, or private
  container inspections. Read existing instructions before deploying.
- Preserve scheduler arguments, runtime environment, TLS mount, persistent
  volume, network, resource/security settings, logging and restart policy.
- The user explicitly authorized the F1 image deployment/restart in this session.
  This is not blanket authorization for future unrelated production restarts.
  Source registration uses existing hourly collectors and needs no restart.
- The user commits and pushes Git changes themselves. Git was clean immediately
  before creating this handoff; this handoff itself will be a new untracked file.
  Recheck status and preserve concurrent work.

## Recent news additions

All source IDs below are `dbo.Sources.ID`, not sports source IDs. They were
registered enabled in production and use the normal hourly `--from-db` collector.
No extra scheduler flags or image rebuilds were needed for news additions.

| Category | Sources |
| --- | --- |
| Top Headlines | NBC 8903; CBS 8904; New York Times 8905; Guardian 8906; Al Jazeera 8907; Sky News 8908 |
| F1 News | BBC Sport 8965; Autosport 8966; Motorsport.com 8967; Guardian 8968 |
| Big 12 News | Official all-sports 8997; Heartland College Sports 8998; official football 8999; men's basketball 9000; baseball 9001 |
| Box Office | Existing Deadline 7094; new Variety 9034; Hollywood Reporter 9035; IndieWire 9036 |

BBC News 4084 and NPR 7093 remain in General.

The user requested omission of the dedicated Big 12 women's basketball feed.
It was not registered; the all-sports feed can still include women's sports.

Recorded import verification:

- F1 News: 40 articles, four successful sources, zero failures.
- Big 12 News: 46 articles, five successful sources, zero failures. Four duplicate
  article URLs across official feeds were skipped by existing identity protection.
- Box Office expansion on October 19: 30 new articles, three successful sources,
  zero failures. Read-back showed ten articles for each new source and Deadline's
  ten retained articles, all enabled with no LastError. Both containers healthy.
- Top Headlines registration was verified; individual initial collection outcomes
  were not separately verified during that registration task.

News is stored in `Pages`, joined to `Sources` and `SourceCategories`. Query
category membership dynamically, filter enabled sources, use Pages.ID for saved
identity and Pages.Url for article destinations. Keep the existing ten-per-source
retention and cross-source URL ownership rules. Do not reassign duplicate article
ownership or weaken canonical/redirect validation to inflate source counts.

## Latest Box Office work

Dedicated RSS feeds added:

- Variety: https://variety.com/v/film/box-office/feed/
- The Hollywood Reporter: https://www.hollywoodreporter.com/t/box-office/feed/
- IndieWire: https://www.indiewire.com/c/box-office/feed/

Existing Deadline: https://deadline.com/v/box-office/feed/.
Boxoffice Pro's news RSS returned 403, The Numbers news feed candidates failed,
and TheWrap's box-office feed was stale; none was added.

Repeatable registration is [sql/boxoffice_news_expand.sql](sql/boxoffice_news_expand.sql).
The older `boxoffice_news_seed.sql` seeds disabled sources and is historical;
it is not the current inventory. Current documentation supersedes old statements
that Deadline/Variety news is retired. News coexists with structured
`BoxOfficeWeekend` charts and Boxoffice Pro forecasts, which were preserved.
The production script and documentation were synced to the host checkout.

## F1 structured data

F1 is deployed and included by the existing hourly `--events-from-db` path.
There is also `--f1-from-db` for a targeted refresh and `--configure-f1` for
explicit migration setup. Runtime imports perform no DDL.

Jolpica provides race/weekend schedules, Grand Prix and sprint classifications,
and driver/constructor championships. `F1Source` singleton ID 1 and `F1Snapshot`
are separate from shared team-versus-team sports tables. Frontend read views:
`F1Schedule`, `F1Results`, `F1DriverStandings`, `F1ConstructorStandings`.
All five datasets replace one current-season snapshot atomically; failures retain
previous data. This is periodic published data, not live timing.

October 18 deployed-container run 1206 succeeded: season 2026, 23 races,
374 Grand Prix classification rows across 17 races, 132 sprint rows across six
sprints, 23 drivers, 11 constructors, standings round 17. These counts are dated.
The frontend production database identity successfully queried the views.
Initial validation: 139 automated tests passed; 21 opt-in tests ignored; separate
F1 SQL integration test and strict Clippy passed. Frontend rendering was not
implemented or verified by this scraper work.

## Documentation map

- [Box Office news and movie data](docs/movies-and-box-office.md#boxoffice-news)
- [Top Headlines frontend handoff](docs/top-headlines-frontend-integration.md)
- [Big 12 news frontend handoff](docs/big12-news-frontend-integration.md)
- [F1 news frontend handoff](docs/f1-news-frontend-integration.md)
- [F1 structured-data frontend handoff](docs/f1-frontend-integration.md)
- [F1 setup, refresh and production evidence](docs/f1-operations.md)
- [F1 deployment and rollback report](reports/f1-production-20261018.md)
- [General operations](docs/operations.md)
- [Sports schedules/results](docs/sports-schedules.md)
- [Sports standings/rankings](docs/sports-tables.md)
- [Entertainment history](reports/entertainment-history.md)

The schema-only baseline snapshot predates article identity and F1 additions;
apply their separate migrations when initializing a new installation.

Protected host deployment backups are under `.deployment-backups/`, including
`20261018-f1/`, `20261018-f1-news/`, `20261018-big12-news/`, and
`20261019-box-news/`. Keep them private. The pre-F1 image/container are retained
for rollback; see the deployment report before using them. Do not delete or
restore production data merely to repeat verification.
