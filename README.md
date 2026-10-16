# Web scraper documentation

Read [HANDOFF.md](HANDOFF.md) for the current documented deployment and working conventions.

| Topic | Guide |
| --- | --- |
| Fixtures, NHL, Big 12, MLS, Championship/Wrexham, EPL, MLB, WNBA, NWSL | [Sports schedules and results](docs/sports-schedules.md) |
| Standings, NHL divisions, NFL divisions, MLS conferences, national Top 25 | [Sports standings and rankings](docs/sports-tables.md) |
| Movie releases, actual box office, forecasts and posters | [Movies, box office and posters](docs/movies-and-box-office.md) |
| Fandango ticket and Fandango digital links | [Purchase-link handoff](docs/movie-purchase-links.md), [frontend request](docs/movie-commerce-frontend-integration.md) |
| TVmaze tracking, series and episodes | [TV series and episodes](docs/tv.md) |
| Article URL identity, retention, SQL backups, security and performance | [Operations](docs/operations.md) |

Each guide contains a linked contents list and preserves its SQL examples and provider-specific contracts. Read the guide introduction and operations policy before applying older examples.

Historical deployment evidence is grouped into [sports](reports/sports-history.md), [movies/TV](reports/entertainment-history.md), and [operations/audits](reports/operations-history.md). These are dated observations, not current run status. The previous long handoff is archived in the operations history.

## Database structure

[WebScraper.schema.sql](sql/schema/WebScraper.schema.sql) is a validated, schema-only snapshot of the live database: 31 tables and one view, with columns, identities, computed columns, keys, defaults, checks, foreign keys and indexes. It contains no table data or credentials. Shared front-end tables in the WebScraper database are included.

Use it to recreate structure in an empty database; it is not an incremental production migration. Details: [schema snapshot](docs/operations.md#database-schema-snapshot).

The October 16 article identity tables/procedure are a separate additive migration
after the schema snapshot. See [article URL identity](docs/article-url-identity.md#schema-and-migration)
for migration order and runtime permissions.
