# USA TODAY via Google News

Added October 24, 2026 (America/Chicago) for the user's personal, noncommercial
dashboard. This uses Google News search RSS, not an official USA TODAY feed.
The feed's copyright element restricts use to a personal, noncommercial feed
reader. This registration is not permission to republish full articles or photos.

## Category mapping

Existing categories are resolved by name; none are created or renamed.
Each section is a separate source under the existing source/category model.
Money coverage goes in More News, music/books/celebrities in Entertainment,
and TV/movies in TV/Film. Google search matching is approximate and results
can overlap; existing global article-URL ownership and deduplication apply.

| Category | Source ID | Search query |
| --- | --- | --- |
| Top Headlines | 9412 | `site:www.usatoday.com/story/news/ when:30d` |
| More News | 9413 | `site:www.usatoday.com/story/money/ when:30d` |
| Sports | 9414 | `site:www.usatoday.com/story/sports/ -college -ncaab -motor -"Formula 1" when:30d` |
| Technology | 9415 | `site:www.usatoday.com/story/tech/ -Wordle -puzzle when:30d` |
| Entertainment | 9416 | `site:www.usatoday.com/story/entertainment/music/ OR site:www.usatoday.com/story/entertainment/celebrities/ OR site:www.usatoday.com/story/entertainment/books/ when:30d` |
| TV/Film | 9417 | `site:www.usatoday.com/story/entertainment/movies/ OR site:www.usatoday.com/story/entertainment/tv/ when:30d` |
| Opinion | 9418 | `site:www.usatoday.com/story/opinion/ when:30d` |
| F1 News | 9419 | `site:www.usatoday.com/story/sports/ intitle:F1 when:30d` |
| Big 12 News | 9420 | `site:www.usatoday.com/story/sports/ "Big 12" when:30d` |
| Men's College Basketball News | 9421 | `site:www.usatoday.com/story/sports/ncaab/ when:30d` |

All searches request U.S. English results (`hl=en-US&gl=US&ceid=US:en`) within
30 days. Google supplies publication dates and Google News article links.
Links remain Google News URLs unless normal, verified redirect/canonical
resolution supplies a publisher URL. Do not decode opaque article tokens or
weaken URL validation. Google does not supply reliable article thumbnails;
missing images must use the dashboard's existing placeholder.

The tested Video Games queries returned no recent entries, so no source was
registered for that category. No Linux or Kansas City feed was added because
no appropriately scoped source was verified. Other publishers remain intact.

## Registration and verification

[`sql/usatoday_news_seed.sql`](../sql/usatoday_news_seed.sql) registers ten missing
sources disabled using the explicit migration profile against WebScraper.
Repeat applications preserve existing category, label and enablement choices.
Use the exact URLs in that script for targeted `web_scraper --dry-run` previews,
then `--db --per-source 10 --output /tmp/usatoday.xml` using runtime credentials.
Activate only after checking source identity, category, successful collection,
NULL LastError and retained articles. The existing hourly `--from-db` command
then collects them. No parser/schema/image change or scheduler restart is needed.

Rollback: disable only sources 9412–9421 after verifying their exact labels/URLs;
retain articles and identity mappings. Reapplying the seed preserves disablement.

## Frontend contract

Use the existing Pages.SourceId → Sources.ID → SourceCategories.ID joins and
filter Sources.Enabled=1. Render Sources.Label for attribution, Pages.Title,
Published and Url; descriptions/images may be absent. The source label explicitly
identifies USA TODAY and Google News. No frontend code change is required to
query these categories dynamically. Frontend rendering was not checked here.

## Production verification

The existing deployed `web-scraper:theatrical-no-disc-20261022` binary performed
both targeted preview and runtime-profile import: ten source successes, zero
failures in each run. Import saved 92 pages and pruned zero news pages.
Read-back verified all ten sources enabled with NULL LastError and 92 dated
articles: ten each except Sports (nine, one cross-source duplicate skipped)
and F1 (three available). All 92 image URLs were absent.

Activation checked the ten exact source IDs/URLs/category assignments, successful
collection and stored articles in a transaction. Repeating the registration
inserted zero rows and preserved all ten enabled settings. No source ID or
existing article ownership was reassigned. The normal hourly schedule was
preserved; a scheduled run was not awaited. Frontend rendering was not verified.
