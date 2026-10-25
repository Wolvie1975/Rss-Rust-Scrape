# Opinion news

Added and verified October 24, 2026 (America/Chicago). Category **Opinion** has
production ID **6022**. Domestic means U.S. news. This combined category covers
publisher-designated opinion, columns, editorials and commentary.

| Coverage | Publisher | Source ID | Feed |
| --- | --- | --- | --- |
| U.S. sports | The Guardian — U.S. Sports Opinion | 9407 | https://www.theguardian.com/sport/us-sport+tone/comment/rss |
| Entertainment | IndieWire — Film Commentary | 9403 | https://www.indiewire.com/c/film/commentary/feed/ |
| U.S. and world | The Guardian — U.S. & World Opinion | 9404 | https://www.theguardian.com/us/commentisfree/rss |
| World | Le Monde — World Opinion | 9405 | https://www.lemonde.fr/en/opinion/rss_full.xml |
| Cross-platform video games | Eurogamer — Opinions | 9411 | https://www.eurogamer.net/feed/opinions |

These five sources are enabled for existing hourly `--from-db` collection.
Final database read-back confirmed ten articles per source (50 active articles),
all with publication dates and image URLs, and NULL LastError for each source.

## Scope and verification limits

The sports feed intersects Guardian's U.S. sports topic with its comment tone.
Sampled columns cover NFL player welfare, NBA, MLB and U.S. soccer; the publisher
is UK-based. Entertainment initially focuses on film and the film industry.
Guardian's U.S. opinion edition includes international and cultural commentary.
Eurogamer covers PC and consoles, games, studios, industry policy and game
ownership. This is general gaming opinion, with no guaranteed Steam-specific
quota. Publisher-designated opinion feeds may update less often than news feeds.
There is no new story classifier or keyword filtering.

Le Monde's RSS supplied the stored metadata. Article-page identity enrichment
was rejected for missing metadata/challenge pages; existing fallback retained
feed URLs and metadata without accepting unverified canonical changes. Full
article/subscription access was not verified.

## Registration and replacement verification

[`sql/opinion_news_seed.sql`](../sql/opinion_news_seed.sql) registers missing
sources disabled and preserves existing labels, category assignments and enabled
settings. Use the existing explicit migration profile against WebScraper.

The original sports blog (9402) leaned toward rugby/Australian sports and was
replaced with U.S. sports opinion (9407) after the user's clarification.
Initially added gaming platform feeds PC Gamer (9406), Nintendo Life (9408),
Push Square (9409) and Pure Xbox (9410) were subsequently replaced with the
single general Eurogamer opinion feed at the user's request.

Eurogamer's endpoint returned HTTP 200 with 100 RSS entries. Rust preview and
actual targeted import each reported one source success and zero failures.
The import saved ten articles and pruned zero news rows. Before activation, a
transaction verified source 9411's exact URL, Opinion category, successful
collection, NULL LastError and ten retained articles. It then enabled 9411 and
disabled the four identity-checked platform sources in one transaction. Final
seed repeat inserted zero and preserved all enablement choices.

Disabled sources 9402, 9406 and 9408–9410 retain their articles and ownership.
Readers must filter Sources.Enabled=1 to exclude those stories. No existing
source was reassigned; the broad Eurogamer source 4088 remains in Video Games.

For targeted preview/import, pass the exact desired feed URL to `web_scraper`
with `--per-source 10 --output /tmp/opinion.xml` and respectively `--dry-run` or
`--db`. Keep normal article identity and ten-per-source retention behavior.

## Frontend contract

See the [frontend integration guide](opinion-news-frontend-integration.md) for
listing/count queries, publisher filtering, rendering and acceptance checks.

Use the existing Pages.SourceId → Sources.ID and Sources.SourceCategoryId →
SourceCategories.ID joins, filter enabled sources, and resolve
`Category_Name = N'Opinion'` rather than hardcoding the numeric ID.
No schema, collector, scheduler or frontend code changes were needed for backend
registration. Frontend navigation/rendering was not verified in this task.
To stop collection, disable the intended sources; do not delete/reassign articles.
