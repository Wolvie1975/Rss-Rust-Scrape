# Web scraper documentation

Read [HANDOFF.md](HANDOFF.md) for the current documented deployment and working conventions.

| Topic | Guide |
| --- | --- |
| Optional game box scores and team/player statistics | [Frontend handoff](<New Development/sports-boxscores-frontend-handoff.md>), [deployment and operations](reports/sports-boxscores-20261019.md) |
| F1 schedule, Grand Prix/sprint results, drivers and constructors | [Frontend integration](docs/f1-frontend-integration.md), [operations](docs/f1-operations.md) |
| Big 12 college sports news | [News frontend integration](docs/big12-news-frontend-integration.md) |
| F1 news | [News frontend integration](docs/f1-news-frontend-integration.md), [collection](docs/f1-operations.md#f1-news-registration) |
| Fixtures, NHL, Big 12, MLS, Championship/Wrexham, EPL, MLB, WNBA, NWSL | [Sports schedules and results](docs/sports-schedules.md) |
| Standings, NHL divisions, NFL divisions, MLS conferences, national Top 25 | [Sports standings and rankings](docs/sports-tables.md) |
| Movie releases, box-office news, actual box office, forecasts and posters | [Movies, box office and posters](docs/movies-and-box-office.md) |
| Top Headlines news category | [Frontend integration request](docs/top-headlines-frontend-integration.md) |
| Fandango ticket and Fandango digital links | [Purchase-link handoff](docs/movie-purchase-links.md), [frontend request](docs/movie-commerce-frontend-integration.md) |
| TVmaze tracking, series and episodes | [TV series and episodes](docs/tv.md) |
| Article URL identity, retention, SQL backups, security and performance | [Operations](docs/operations.md) |

Each guide contains a linked contents list and preserves its SQL examples and provider-specific contracts. Read the guide introduction and operations policy before applying older examples.

Historical deployment evidence is grouped into [sports](reports/sports-history.md), [movies/TV](reports/entertainment-history.md), and [operations/audits](reports/operations-history.md). These are dated observations, not current run status. The previous long handoff is archived in the operations history.

## Database structure

[WebScraper.schema.sql](sql/schema/WebScraper.schema.sql) is a validated, schema-only baseline snapshot: 31 tables and one view, with columns, identities, computed columns, keys, defaults, checks, foreign keys and indexes. It contains no table data or credentials. Shared front-end tables in the WebScraper database are included.

Use it to recreate structure in an empty database; it is not an incremental production migration. Details: [schema snapshot](docs/operations.md#database-schema-snapshot).

The October 16 article identity tables/procedure are a separate additive migration
after the schema snapshot. See [article URL identity](docs/article-url-identity.md#schema-and-migration)
for migration order and runtime permissions.

The October 18 F1 migration adds two tables and four views separately from that
baseline. Apply `--configure-f1` with migration credentials after shared schema
setup; see [F1 setup and enablement](docs/f1-operations.md#setup-and-enablement).
The deployed F1 collector and frontend read contract are documented above.

2026 NWSL schedules now cover all teams: [data and frontend notes](docs/nwsl-league-schedule.md).

The optional game-statistics cache and read view require the separate additive
`--configure-boxscores` setup with migration credentials after shared results
tables exist. [Setup, deployment and rollback](reports/sports-boxscores-20261019.md).
