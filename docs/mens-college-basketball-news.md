# Men's college basketball news

Added to production October 19, 2026 (America/Chicago). The selected sources were
ESPN, CBS Sports, ZAGSBLOG and Heartland's basketball feed; NCAA.com and the
existing official Big 12 feed were not selected for this category.

## Production inventory

Exact category: **Men's College Basketball News**. Resolved production category ID
at verification: **6007**. Resolve categories dynamically by name in integrations.

| Publisher | Source ID | Feed | Retained articles |
| --- | --- | --- | --- |
| ESPN — Men’s College Basketball | 9353 | https://www.espn.com/espn/rss/ncb/news | 8 |
| CBS Sports — Men’s College Basketball | 9354 | https://www.cbssports.com/rss/headlines/college-basketball/ | 9 |
| ZAGSBLOG — College Basketball & Recruiting | 9355 | https://www.zagsblog.com/feed/ | 10 |
| Heartland College Sports — Big 12 Basketball | 9356 | https://www.heartlandcollegesports.com/category/big-12-basketball/feed/ | 10 |

All four are enabled, with successful collection timestamps of **1:29:04 PM CDT**
(18:29:04 UTC), NULL LastError and publication dates/image URLs on all **37** rows.
These are dated checks, not continuous health guarantees.

ESPN/CBS supply national news and analysis, ZAGSBLOG emphasizes recruiting
(including high-school prospects), and Heartland adds Big 12 basketball reporting.
Publisher-selected feeds can also include related NBA/draft, eligibility, alumni
or broader basketball stories. No title-keyword filter or parser change was added.

## Registration and production verification

The repeatable [seed](../sql/mens_college_basketball_news_seed.sql) registers new
sources **disabled** and preserves existing settings on repeated runs. Use the
migration profile and explicitly target **WebScraper**. First collect the four
exact URLs with the runtime profile before enabling their newly inserted IDs.

The actual deployed collector commands were equivalent to:

```sh
web_scraper --dry-run --per-source 10 \
  https://www.espn.com/espn/rss/ncb/news \
  https://www.cbssports.com/rss/headlines/college-basketball/ \
  https://www.zagsblog.com/feed/ \
  https://www.heartlandcollegesports.com/category/big-12-basketball/feed/ \
  --output /tmp/mbb-news-dry.xml
# Actual import: replace --dry-run with --db.
```

Production dry-run: **four source successes, zero failures**. The initial seed
created one category and four disabled sources; immediate repeat inserted zero.
Production import **1254** succeeded with **37 saved articles**, four source
successes and zero failures. After population/read-back, a guarded transaction
enabled only IDs 9353–9356. A further seed repeat inserted zero and preserved
enabled settings and stable identities.

Two ESPN URLs and one CBS URL already belonged to existing sources. Existing
cross-source URL ownership protection skipped them. No article was duplicated or
reassigned, and no validation or retention rules were weakened to fill the cap.
Heartland's root feed **8998** and official Big 12 men's basketball **9000** remain
under **Big 12 News**; neither was moved or registered a second time.

The existing hourly `--from-db` collector automatically includes the new feeds.
No image build, runtime configuration, schema change, scheduler change or service
restart was required. The running scraper remains the box-score release,
`web-scraper:boxscores-20261019`. Both scraper and frontend were healthy afterward.

## Frontend read-back

The live category route `/news?category=6007` returned HTTP 200. Following its
actual Older stories cursor links returned **three pages**, with **13 / 12 / 12**
rendered article cards: **37 unique article URLs**, all four selected publishers,
and no publisher outside the selected category. Each cursor preserved the category
parameter. This verifies existing category rendering and pagination via HTTP;
interactive save/read/mute behavior was not re-tested.

Use the existing `Pages` / `Sources` / `SourceCategories` contract, filter enabled
sources and resolve category IDs by name. Keep Pages.ID for saved identity and
Pages.Url for article destinations. No frontend files or image were changed.

To stop future collection, disable only the intended source IDs. Preserve existing
articles, aliases, categories and shared-source ownership. The seed deliberately
does not re-enable an operator-disabled registration.
