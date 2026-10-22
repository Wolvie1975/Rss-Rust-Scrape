# News categories and Guardian World

Implemented October 22, 2026 (America/Chicago), following approval of the recommended category structure.

| Category | Current relevant sources |
| --- | --- |
| More News (2627) | NPR 7093, CBS News 8904, St. Louis Post-Dispatch 9401 |
| Sports (9) | ESPN 297, CBS Sports 299, Redbird Rants 708, Awful Announcing 7895 |
| Video Games (11) | GamingOnLinux 19, Pure Xbox 1764, Eurogamer 4088, Indie Games Plus 9037, The Indie Informer 9038, Indie Hive 9039, Polygon 1, IGN 1768 |
| Entertainment (10) | Nerdist 1766 |
| TV/Film (5614) | Deadline 300, TVLine 301, /Film 366, THR 1765 |
| Top Headlines (5877) | BBC News 4084, PBS Newshour 7092, NBC News 8903, New York Times 8905, Guardian 8906, Al Jazeera 8907, Sky News 8908 |

More News is the renamed General News category, not a new category ID. Other categories remain as inventoried in the dated review. Guardian source 8906 now uses https://www.theguardian.com/world/rss and remains in Top Headlines. Its F1 feed is unchanged.

The repeatable [migration](../sql/news_category_cleanup_20261022.sql) was applied twice successfully using migration credentials against WebScraper. It preserves source IDs, labels, enabled settings and article ownership; category changes apply to existing articles through their source association. It guards known source identities and duplicate Guardian feed registrations. No collector rebuild or scheduler restart was needed.

The World feed returned HTTP 200 and 45 RSS entries. The Rust dry-run and actual targeted import each succeeded with one source success and zero failures. Read-back verified source 8906 enabled, NULL LastError, and ten retained articles with ten image URLs. Standard ten-per-source retention removed the prior ten Guardian stories after importing the ten World-feed stories; this is normal retention, not an article identity merge. Other moved sources' existing articles were not rewritten by the migration.

Read-back totals: More News 3 sources / 30 articles; Sports 4 / 40; Video Games 8 / 80; Entertainment 1 / 10; TV/Film 4 / 40; Top Headlines 7 / 70.

## Frontend contract

Use category IDs or current database names when joining Sources and SourceCategories. Category 2627 now displays More News. Any frontend query or navigation label hardcoded to General or General News should be updated to More News; existing source/category IDs remain stable. Frontend code and rendering were not changed or verified in this backend task.

The standalone STLtoday registration now expects More News; the headline seed uses Guardian World. The category migration is for this existing production inventory, not an empty-database bootstrap. The review report records the pre-change inventory and is intentionally historical. No commits or pushes were requested for this change.
