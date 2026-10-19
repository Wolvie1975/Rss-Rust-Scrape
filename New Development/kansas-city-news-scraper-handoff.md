# Scraper handoff: populate Kansas City Local news

Requested October 19, 2026 (America/Chicago).

## Objective

Find, validate and register Kansas City-area RSS/Atom news feeds in the separate
Rust scraper project, then collect articles into the existing shared news tables.
The deployed MyNewsFeed web application already has a Kansas City Local section
at `/local`; no frontend deployment is needed for ordinary source registration.

Scope is **Kansas City only**, including the Missouri/Kansas metro: KCMO, KCK,
Johnson County, Northland, Independence, Lee's Summit and nearby communities.
Do not add a St. Louis section or broad statewide/national feeds as substitutes.
Prefer local reporting, government, education, community, business, arts and culture.
Local sports can appear, but the section should not become another sports feed.

## Exact frontend contract

- Category name: **`Kansas City News`** (preserve this exact spelling).
- Resolve/create its ID by name; do not hardcode a category or source ID.
- News sources belong in `dbo.Sources`, assigned through `SourceCategoryId`.
- Articles belong in `dbo.Pages`, with `SourceId` pointing to their owning source.
- Only enabled sources are public. The existing category has no separate enabled
  flag in the frontend model; enabled source membership controls its availability.
- The frontend resolves this category dynamically and shows its articles, newest
  first, with twelve-item cursor batches. Existing publisher labels, dates,
  images, saving and read/mute controls are reused.
- **Once this category has any enabled source, `/local` switches entirely to its
  articles. It does not supplement them with general-news keyword matches.**
  Enabling an empty source can therefore temporarily empty the Local news panel.
- Until there is an enabled source in this category, the app explicitly labels
  keyword matches for `Kansas City` from enabled collected news.

Implementation reference in this repository:
`src/MyNewsFeed.Web/Data/LocalNewsQuery.cs`.

Weather is already supplied by the web app's NWS integration. Community events
currently link to Visit KC and the city calendar. **This request concerns news
feeds; do not ingest weather responses or event calendars as news articles.**

## Feed candidates and dated checks

Direct HTTP checks below were made from the application workspace on October 19,
2026. “Entries detected” means RSS markup and item elements were detected in the
response, **not** a completed Rust parser, freshness or editorial-scope validation.
Counts are dated observations, not permanent acceptance criteria. Recheck from the
scraper's actual network/runtime before enabling anything.

| Priority | Suggested publisher label | Candidate feed URL | Observed response | Follow-up |
| --- | --- | --- | --- | --- |
| Start | KMBC 9 — Local News | <https://www.kmbc.com/local-news-rss> | HTTP 200; RSS markup; 20 entries detected | Verify local-news scope, dates, images and canonical article URLs. |
| Start | KSHB 41 — Local News | <https://www.kshb.com/news/local-news.rss> | HTTP 200; RSS markup; 50 entries detected | Check syndicated national stories and prefer a narrower local endpoint if needed. |
| Start | FOX4 Kansas City | <https://fox4kc.com/feed/> | HTTP 200; RSS markup; 7 entries detected | Root feed may mix local/national coverage; discover a publisher-supported local feed where available. |
| Start | Flatland — Kansas City PBS | <https://flatlandkc.org/feed/> | HTTP 200; RSS markup; 5 entries detected | Check publication frequency and article/video types. |
| Start | The Pitch — Kansas City | <https://www.thepitchkc.com/feed/> | HTTP 200; RSS markup; 10 entries detected | Check local arts/culture/community mix; exclude sponsored material if reliably labelled. |
| Conditional | The Beacon — Kansas City | <https://thebeaconnews.org/feed/> | HTTP 200; RSS markup; 10 entries detected | Root feed may cover multiple areas. Discover a KC-specific feed or establish reliable publisher-provided location/category filtering before enabling. |
| Investigate | KCUR — Kansas City | <https://www.kcur.org/index.rss> | HTTP 200; RSS channel; **zero item elements** | Do not count as populated coverage. Discover a working local-news/topic feed and validate actual entries. |

Publisher discovery pages:

- [KCUR](https://www.kcur.org/) and its [Feed page](https://www.kcur.org/feed).
  The latter returned HTML during the check; it is not itself an RSS endpoint.
- [KCTV5 News](https://www.kctv5.com/news/): additional local publisher to inspect
  for RSS/Atom autodiscovery. No working local-news XML feed is confirmed here.
- [The Beacon](https://thebeaconnews.org/), [Flatland](https://flatlandkc.org/),
  [The Pitch](https://www.thepitchkc.com/), [FOX4](https://fox4kc.com/),
  [KMBC](https://www.kmbc.com/) and [KSHB](https://www.kshb.com/).

Treat these as candidates, not mandatory subscriptions. Prefer a small, healthy
initial set of three to five complementary publishers over many overlapping feeds.
Do not require literal “Kansas City” in every headline: relevant reporting may
name a suburb, school district or local institution instead. For feeds with mixed
geography, use explicit publisher feed/category metadata where possible and report
any uncertain scope rather than silently treating all content as local.

## Discovery and validation

1. Read the Rust repository's instructions, current feed discovery, configuration,
   category resolution, retention and article identity code. Use its real commands
   and configuration patterns; this handoff does not prescribe new CLI flags.
2. Inventory existing sources by normalized URL, redirects and publisher identity.
   Reuse an appropriate existing registration when its category and purpose match.
3. Inspect publisher HTML for RSS/Atom `rel="alternate"` links and official feed
   directories. Verify final URLs and actual response bodies; HTTP 200 can be an
   HTML page, bot challenge, error document or empty channel.
4. Parse candidate responses with the scraper's actual RSS/Atom parser. Check
   namespaces, redirects, encoding, GUIDs, article links, dates and image fields.
   Record the canonical feed URL, response status and successful parsed-item count.
5. Sample several recent entries per publisher for metro relevance, publication
   freshness, duplicates and valid article destinations. A slower publisher is
   acceptable; an abandoned or broken feed is not healthy coverage.
6. Use the scraper's existing dry-run/preview mechanism before registration and
   collection. Document any required parser adjustment and test it appropriately.
7. Register the exact category and validated sources idempotently, preserving
   existing enablement choices on repeat setup. Coordinate first collection with
   enablement so the category switch does not leave a prolonged empty panel.
8. Add the sources to the existing normal news collection schedule. Do not create
   an independent daemon or change unrelated retention/collection settings.

## Article identity, ownership and data quality

The application currently models one category per source and globally unique
article URL hashes. Inspect the actual scraper schema and hash/upsert rules before
writing configuration or data.

- Preserve existing global article identity and URL ownership across overlapping
  feeds. Do not duplicate rows, weaken uniqueness, or move existing articles to
  inflate the Local category's counts.
- If an existing source is used in another category, do not silently move it into
  `Kansas City News` or register duplicate copies of the same feed. Report the
  conflict and choose another distinct local feed where possible. Multi-category
  membership would require a separate coordinated data/frontend change.
- Preserve stable source and article IDs. Use the existing redirect/canonical and
  retired-ID recovery behavior. See
  [article URL deduplication](article-url-deduplication.md).
- Populate the existing `Title`, `Url`, `Description`, `Published`, `ScrapedAt`,
  `ImageUrl` and `SourceId` fields according to established scraper conventions.
  Headlines, summaries and source links are sufficient; do not copy full article
  bodies or bypass subscriber access.
- Preserve actual publication timestamps in UTC where known. Do not substitute
  collection time into `Published` or invent missing timestamps/images.
- Resolve relative article/image URLs safely using publisher context. Follow the
  existing HTTP(S), redirect and image-validation rules.
- Use the existing source health fields (`LastScrapedAt`, `LastError`) and retention
  behavior. Preserve last-good content on transient collection failures; do not
  delete existing articles because a provider fails or temporarily returns empty.
- Do not include connection strings, passwords or private runtime snapshots in
  the implementation report. No new schema is expected for ordinary RSS setup.

## Acceptance checks and return handoff

Provide a short implementation report containing:

- Exact configured category name and its resolved ID as a dated observation.
- Selected publisher labels, validated feed URLs, enablement and scope notes.
- Rejected/deferred feeds and the reason (empty, mixed geography, blocked, stale,
  malformed, duplicate registration or ownership conflict).
- Actual commands/configuration changes and whether any scraper restart or
  deployment was required. Include repeat-run/idempotency results.
- Per-source successful collection times, retained article counts and health.
  Explain ownership-related count differences rather than manufacturing articles.
- Representative titles and safe source URLs proving Kansas City relevance.
- Confirmation that `/local` uses dedicated category content after enablement,
  displays attribution/dates/images or expected fallbacks, and supports Older
  stories, saving and reading controls. Check that unrelated general stories do
  not appear merely to fill empty results.
- Confirmation that weather, event links, existing categories, other feeds and
  global saved-article identity continue working.

The key completion criterion is a **successfully collected, healthy dedicated
Kansas City News category visible at `/local`**, not just a list of registered
sources. If source discovery or existing ownership blocks that outcome, report
what is populated and the specific remaining dependency.
