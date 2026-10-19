# Kansas City local news

Added and verified October 19, 2026 (America/Chicago).

## Sources and collection

The exact category is **Kansas City News**; its production ID was **5950** at
verification. Resolve membership by category name rather than hardcoding IDs.
All five sources below are enabled and use normal hourly `--from-db` collection.
No schema, parser, scheduler, image, frontend deployment or restart was required.

| Publisher | Source ID | Feed | Parsed entries | Retained articles |
| --- | --- | --- | --- | --- |
| KSHB 41 — Local News | 9172 | https://www.kshb.com/news/local-news.rss | 50 | 10 |
| Flatland — Kansas City PBS | 9173 | https://flatlandkc.org/feed/ | 5 | 5 |
| The Pitch — Kansas City Food & Drink | 9174 | https://www.thepitchkc.com/category/food-drink/feed/ | 10 | 10 |
| KCUR — Kansas City | 9175 | https://www.kcur.org/tags/kansas-city.rss | 10 | 10 |
| The Beacon — Kansas City | 9176 | https://thebeaconnews.org/stories/category/kansas-city/feed/ | 10 | 10 |

All endpoints returned HTTP 200 and parsed as feeds in the deployed Rust collector.
All five successful collection timestamps were October 19 at **9:35:23 AM CDT**
(14:35:23 UTC). `LastError` was NULL for every source. All 45 retained articles had
publication dates and image URLs. Flatland supplies five entries; its smaller
count is the actual feed size, not an ingestion failure. The collector reported
45 saved articles, five source successes, zero failures and zero news rows pruned.
No ownership conflicts were reported and existing article ownership was preserved.

KSHB provides fast metro reporting, including local sports and nearby Lawrence.
Flatland adds civic, housing and community reporting. The Pitch's narrower
food-and-drink feed adds local businesses and culture while avoiding its general
film reviews. KCUR and The Beacon use publisher-provided Kansas City topic/category
feeds rather than their broader roots. Those classifications can include statewide
policy affecting KC, podcasts, or occasional reporting beyond the metro (for
example, The Beacon tagged a central Missouri hospital story Kansas City). This is
publisher-defined local coverage, not a strict geographic filter. No title-keyword
filter, synthesized publication date or new content-filtering code was introduced.

## Registration and verification

[`sql/kansas_city_news_seed.sql`](../sql/kansas_city_news_seed.sql) registers missing
sources **disabled**, preserving existing enablement, category assignments and
labels on repeats. Use migration credentials and explicitly select WebScraper.
Before initial registration, inventory URLs, redirects and publishers to avoid
cross-category ownership conflicts. No existing candidate registrations or Kansas
City category were found in this production inventory.

The executed sequence was:

1. Fetch publisher feed/discovery pages and sample titles, dates, categories and
   destinations; run the real production-container parser in dry-run mode.
2. Apply the seed in WebScraper with the migration profile. Initial application
   created one category and five disabled sources. Immediate repeat inserted zero.
3. Run the deployed `web_scraper` against the five exact feed URLs above with
   `--db --per-source 10 --output /tmp/kc-news-import.xml`. For the prior preview,
   replace `--db` with `--dry-run`. Do not use `--from-db` for a targeted first
   collection, since the sources are intentionally still disabled.
4. Verify populated sources and clear errors, then enable only newly inserted IDs
   9172–9176 in a transaction guarded by category, successful collection and content
   checks. Existing sources and prior disablement choices must be preserved.
5. Repeat the seed after enablement: zero rows inserted; all five IDs, labels,
   category membership and enabled settings remained intact.

Production run **1244** finished successfully at 9:35:24 AM CDT. Dry-run and actual
import each reported five source successes and zero failures. Keep existing
article identity/canonical rules and ten-per-source retention. To stop future
collection, disable only the intended source IDs; do not delete articles or move
ownership as a rollback mechanism.

## Frontend read-back

Before enablement, HTTP `/local` displayed its explicitly labelled keyword-match
fallback and a Big 12 story. Sources were populated before enabling so the switch
did not expose an empty dedicated category.

After enablement, live `/local` returned HTTP 200 and displayed **From enabled
Kansas City News publishers.** Following its actual Older stories cursor links
returned four HTTP 200 pages with **12 / 12 / 12 / 9** cards, 45 distinct story URLs,
all attributed to these five sources. Every card included publication and image
metadata plus save and mark-read controls in the rendered HTML. No general-news
keyword fallback or unrelated publisher filled the results. All five publishers
appeared across the pages.

The NWS weather link and Visit KC/city event links remained present. The scraper
and frontend containers were healthy after verification. Browser interaction
(save persistence, mark-read/mute behavior, image downloads, weather rendering)
was not exercised; these are HTTP-rendering and database checks, not an interactive
browser acceptance test. No frontend files or saved-article identities were changed.

Representative stories:

- [Wyandotte County childcare needs](https://www.kcur.org/health/2026-10-19/only-29-of-wyandotte-countys-childcare-needs-are-met-can-a-monthly-orientation-add-providers)
- [Independence dental care and Medicaid](https://thebeaconnews.org/stories/2026/10/19/medicaid-cuts-could-put-dental-care-out-of-reach-again-in-missouri/)
- [Kansas City helping renters avoid eviction](https://flatlandkc.org/news-issues/housing/kansas-city-helping-renters-avoid-eviction/)
- [Kansas City restaurants and bars](https://www.thepitchkc.com/dish-drink-kc-sandwiches-and-world-building-at-marina-27-italian-sausage-co-and-brass-monkey/)

## Deferred candidates

- KMBC local feed: populated, but sampled out-of-area Wichita coverage and two
  future-dated items; deferred in favor of the selected five.
- FOX4 news feed: mixed national and statewide stories. Its local-news subfeed was
  empty. Neither was registered.
- KCUR `index.rss`: empty. `news.rss` worked but included wider regional stories;
  selected the working Kansas City topic feed instead.
- The Beacon root: mixed categories. Initial guessed local endpoints returned
  404; publisher category discovery identified the working selected URL.
- The Pitch root: included general movie reviews; selected Food & Drink instead.

These counts and health observations are dated verification, not live guarantees.
