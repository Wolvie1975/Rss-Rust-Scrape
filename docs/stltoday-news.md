# STLtoday news

Added October 22, 2026 (America/Chicago). Production source **9401**, labelled
**St. Louis Post-Dispatch — STLtoday**, is enabled under **General** (category
2627). The feed mixes St. Louis, national, sports, opinion and lifestyle coverage;
it is not a dedicated local-news feed.

Feed: https://www.stltoday.com/search/?f=rss&t=article&l=50&s=start_time&sd=desc

The existing Rust collector parsed 50 entries. A targeted dry-run succeeded,
followed by a targeted runtime-profile `--db --per-source 10` import with one
source success and zero failures. Database read-back verified ten retained
articles, all ten with publication dates and image URLs, and NULL `LastError`.
The source was registered disabled, populated, checked, then enabled for normal
hourly `--from-db` collection. No parser, schema, scheduler or image change was
required. Frontend rendering was not checked.

[`sql/stltoday_news_seed.sql`](../sql/stltoday_news_seed.sql) is repeatable and
preserves existing labels, category assignments and enablement. It requires the
existing General category and registers missing sources disabled. Use migration
credentials and explicitly select WebScraper. A repeat after enablement succeeded.
Only enable a newly registered row after successful targeted collection and
content/health checks; preserve existing disablement decisions on repeats.

For targeted preview, run the existing collector with the exact feed URL,
`--dry-run --per-source 10 --output /tmp/stltoday-preview.xml`. For the first
import, replace `--dry-run` with `--db`. Normal ten-per-source retention and
article identity protections apply. Disable source 9401 to stop collection;
retain existing articles and their identities.
