# Scraper spec: Breaking news (corroborated stories across topics)

Requested November 10, 2026 (America/Chicago). Target: the Rust scraper repository.
Status: **design only. Nothing in this document has been implemented, migrated or deployed.**

## Goal

Give myNewspaper a **Breaking** feed that spans four topics: **News, Sports, Entertainment and Tech**.

A story is breaking only when it passes a strict signal: **several different publishers report the same story
within a short window**. A single publisher posting a new article is not breaking, however recent it is.

This is a detection layer over articles the scraper already collects. It adds no new news sources and does not
change how `dbo.Pages` is filled.

## Decisions already made

| Question | Decision |
| --- | --- |
| One breaking category, or cross-topic? | Cross-topic: News, Sports, Entertainment, Tech |
| "Everything recent" or a stricter signal? | Stricter: corroboration by multiple publishers |
| Where does it live? | New additive tables and one view; `Pages`, `Sources` and `SourceCategories` are not altered |
| Video Games topic? | Tech |
| Kansas City News in the pool? | Yes, under News |
| Fast-lane cadence? | Every 10 minutes (accepted November 10, 2026); built as phase 2, after phase 1 thresholds are tuned |
| Minimum publishers? | Start at 3 for every topic; make it settable per topic so Tech or Entertainment can drop to 2 if the dry run shows too few stories |

## Current state (live database, November 10, 2026)

Read-only query with the runtime profile: 16 categories, 87 enabled sources, 573 articles published or scraped in
the previous 48 hours.

| ID | Category | Enabled sources | Articles in 48h |
| --- | --- | --- | --- |
| 5877 | Top Headlines | 9 | 88 |
| 2627 | U.S. News | 3 | 30 |
| 5950 | Kansas City News | 9 | 46 |
| 9 | Sports | 5 | 49 |
| 5892 | F1 News | 5 | 29 |
| 5900 | Big 12 News | 5 | 16 |
| 6007 | Men's College Basketball News | 4 | 21 |
| 7628 | Sports Media | 1 | 10 |
| 10 | Entertainment | 3 | 22 |
| 5614 | TV/Film | 5 | 32 |
| 6443 | Box Office | 4 | 8 |
| 11 | Video Games | 10 | 58 |
| 13 | Technology | 12 | 106 |
| 8 | Linux | 5 | 36 |
| 6022 | Opinion | 7 | 22 |
| 6444 | Business | 0 | 0 |

Category names and membership have drifted from the older guides (for example `U.S. News`, a separate `Box Office`
category, `Linux`, `Sports Media`). Resolve IDs from the database, not from older documents.

Facts about the existing collector that shape this design:

- News is collected hourly by `--from-db` with `--per-source 10`. Busy feeds push an article out of `Pages` within
  hours, so a breaking story must keep its own copy of the articles that formed it.
- An article URL belongs to exactly one source (`UQ_Pages_UrlHash`). Two feeds from the same publisher never both
  hold the same article, so counting publishers from `Pages` is not inflated by duplicates.
- Eleven enabled sources are Google News searches (nine USA TODAY, Lawrence Journal World, Kansas City Star). Their
  article URLs are on `news.google.com` unless a redirect was verified, so the URL host does not identify the
  publisher for these.
- `--health-check` reads only the latest `dbo.ScraperRuns` row. This matters for the optional fast lane in phase 2.

## 1. Topic mapping

Add a small mapping table so topics survive category renames and new sources are covered automatically.

`dbo.BreakingTopicCategories`

| Column | Type | Notes |
| --- | --- | --- |
| `SourceCategoryId` | `INT` PK | FK to `dbo.SourceCategories(ID)` |
| `Topic` | `VARCHAR(20)` | `CHECK` in `News`, `Sports`, `Entertainment`, `Tech` |

Proposed seed, matched by category name and applied only when the row is missing:

| Topic | Categories |
| --- | --- |
| News | Top Headlines, U.S. News, Kansas City News |
| Sports | Sports, F1 News, Big 12 News, Men's College Basketball News, Sports Media |
| Entertainment | Entertainment, TV/Film, Box Office |
| Tech | Technology, Linux, Video Games |
| Not mapped | Opinion, Business |

A category with no row is outside the breaking pool. Opinion is excluded because commentary on an existing story
would count as corroboration.

## 2. Storage

`dbo.BreakingStories`

| Column | Type | Notes |
| --- | --- | --- |
| `ID` | `INT IDENTITY` PK | Stable story identity for the frontend |
| `Topic` | `VARCHAR(20)` | Majority topic of the member articles; ties go to `News` |
| `Headline` | `NVARCHAR(500)` | Title of the earliest member article, publisher suffix removed |
| `LeadPageId` | `INT NULL` | `Pages.ID` of that article; no FK, the page may be pruned |
| `LeadUrl` | `NVARCHAR(2048)` | |
| `LeadImageUrl` | `NVARCHAR(2048) NULL` | First member image available |
| `FirstPublishedAt` | `DATETIME2` | Earliest member article |
| `LastPublishedAt` | `DATETIME2` | Latest member article |
| `PublisherCount` | `INT` | Distinct publishers |
| `ArticleCount` | `INT` | |
| `DetectedAt` | `DATETIME2` | When the story first passed the threshold; never updated |
| `UpdatedAt` | `DATETIME2` | |

`dbo.BreakingStoryArticles`

| Column | Type | Notes |
| --- | --- | --- |
| `StoryId` | `INT` | FK to `BreakingStories(ID)` with `ON DELETE CASCADE` |
| `PageId` | `INT` | `Pages.ID` at detection; no FK. Unique across the table, so one article is in one story |
| `SourceId` | `INT` | |
| `Publisher` | `NVARCHAR(200)` | Display name, see section 3 |
| `PublisherKey` | `NVARCHAR(200)` | Normalised key used for counting |
| `Url`, `Title`, `ImageUrl`, `Published` | | Snapshot copied from `Pages` |

Primary key `(StoryId, PageId)`, unique index on `PageId`.

`dbo.BreakingNews` (view, the frontend read contract): one row per story whose `LastPublishedAt` is within the last
6 hours, with the story columns above plus `Articles`, a JSON array of the member articles ordered by `Published`.

Stories are additive. Once detected, a story keeps its articles even after `Pages` prunes them, gains articles as
more publishers report, and leaves the view when it goes quiet. Retention deletes stories whose `LastPublishedAt`
is older than 48 hours.

## 3. Detection

Add `src/breaking.rs` as pure functions over in-memory articles, with no HTTP and no SQL, so it can be unit
tested from fixtures.

**Pool.** Enabled sources whose category is mapped; articles with `COALESCE(Published, ScrapedAt)` in the last
24 hours. Articles with no `Published` use `ScrapedAt`.

**Publisher identity.**

1. Take the article URL host, drop `www.`, reduce to the registrable domain (`feeds.bbci.co.uk` and `bbc.co.uk`
   forms must agree).
2. If the host is `news.google.com`, use the source label text before ` — ` instead (`USA TODAY — Sports (Google
   News)` gives `USA TODAY`).

This makes The Guardian's main feed and its F1 feed one publisher, and all nine USA TODAY searches one publisher.

**Title normalisation.** Lowercase; remove a trailing publisher suffix (` - USA TODAY`, ` | Deadline`); split on
non-alphanumerics; drop stopwords and one-character tokens; strip possessive and plural `s`. Keep numbers.

**Similarity.** Weight each token by inverse document frequency across the pool, so names and rare words count
more than common news words. Two articles link when all of these hold:

- weighted Jaccard similarity of their token sets is at least **0.5**;
- they share at least **two** tokens;
- they are from **different publishers**;
- their publication times are within **6 hours** of each other.

**Clustering.** Cluster across the whole pool, not per topic, so a sports story on a general-news front page still
corroborates a sports outlet. To avoid chaining unrelated stories through a shared name, an article joins a
cluster only if it links to at least half of the cluster's existing members.

**Threshold.** A cluster becomes a story when it has at least **3 distinct publishers** and its earliest article is
less than **12 hours** old.

The numeric values above are starting points and have not been run against real titles. Tune them with the dry
run in section 5 before enabling writes; expect the publisher threshold to be the main control. Keep the minimum
in a `MinPublishers` column per topic (a four-row `dbo.BreakingTopics` table, default 3) rather than a constant,
because Tech and Entertainment have fewer overlapping publishers than News and may need 2.

**Reconciling with stored stories.** Each run recomputes clusters, then:

- a cluster that shares any `PageId` with a stored story updates that story (lowest `ID` wins);
- if it overlaps several stored stories, the others are merged into the winner and deleted;
- a cluster with no overlap that passes the threshold inserts a new story;
- stored stories with no matching cluster are left alone to age out.

Apply one run's changes in a single transaction from a JSON payload (`sql/breaking_upsert.sql`), the same pattern
as `sql/scoreboard_upsert.sql`. A failed run leaves the previous stories intact.

**This is grouping, not merging.** The article-identity rule in `HANDOFF.md` forbids merging `Pages` rows by title
similarity. This feature never updates or deletes `Pages`; it only reads them.

## 4. Commands and scheduling

- `--configure-breaking`: explicit setup with migration credentials. Applies `sql/breaking.sql` (tables, view),
  `sql/breaking_seed.sql` (topic mapping) and `sql/breaking_permissions.sql`. Repeatable; preserves edited
  mappings. Add it to `Cli::is_setup` and `Cli::uses_db`. Runtime does no DDL.
- `--breaking`: run detection from the database (implies `--db`). Add a `collect_breaking` stage to `Run::collect`
  in `src/main.rs`, placed **after** `collect_news` so it sees the articles just saved. Time it with
  `metrics::Stage::new("breaking")`. A detection error is pushed to `failures` and does not stop the run.
- `--breaking --dry-run`: print each candidate story with topic, publishers, similarity and member titles, and
  write nothing.

Permissions: `WebScraperIngest` needs `SELECT` on the mapping table and `SELECT, INSERT, UPDATE, DELETE` on the
two story tables. Grant `SELECT` on the story tables and the view to the same reader principals that
`sql/tech_permissions.sql` covers.

Retention: add the 48-hour story cleanup to `sql/retention.sql`, or a separate `sql/breaking_retention.sql` run
from the same stage.

**Phase 1 (this spec):** append `--breaking` to the `scraper` command in `docker-compose.yml`. Detection then runs
hourly with the existing collector. No new service.

**Phase 2 (accepted, follows phase 1): fast lane.** Hourly detection means a story can be up to an hour late.
A faster lane would be a third compose service that collects only mapped sources and then runs detection.

Cadence: **every 10 minutes**, aligned to the clock like the hourly run. The mapped pool is 80 of the 87
enabled sources, so this is about 480 feed requests an hour instead of 80, and six requests an hour to any one
feed. Five minutes doubles that and puts 132 requests an hour on Google News across its eleven searches, for
little gain: the story still waits for a third publisher, and publisher feeds are themselves cached for several
minutes. A fast run must finish well inside its interval; read `metrics: stage=news elapsed_ms` from the
production log to confirm before choosing the cadence. That figure has not been checked.

It needs, at minimum:

- an `--every-minutes` schedule and a source filter on `--from-db`;
- a way to keep its runs from hiding a stalled hourly run in `--health-check`;
- no global retention pass on each fast run;
- a check that two collectors writing `Pages` and article identities at once is safe. This has not been reviewed.

Phase 1 is useful without phase 2 and is the place to tune the thresholds.

## 5. Tests and verification

Unit tests in `src/breaking.rs`:

- the same event reported by three publishers with different wording forms one story;
- two stories sharing a prominent name (same person, different events) stay separate;
- three articles from one publisher, or from three feeds of one publisher, do not qualify;
- Google News sources resolve to the labelled publisher;
- articles more than 6 hours apart do not link;
- reconciliation: a growing cluster keeps its `ID` and `DetectedAt`; overlapping stories merge; one article never
  appears in two stories.

Opt-in SQL integration test (rolled back, like the others in `src/db/tests.rs`): upsert twice is idempotent,
merge removes the absorbed story, a failed payload leaves prior stories unchanged, retention removes only old
stories.

Before enabling in production:

1. `--breaking --dry-run` against the live database on several different hours, including a busy news period.
2. Read the output by hand. Record false groupings and missed stories, adjust thresholds, repeat.
3. Apply `--configure-breaking` with migration credentials, then one real `--breaking` run, then read back
   `dbo.BreakingNews`.
4. Only then add the flag to the hourly command. Deployment and restart need the user's approval.

## 6. Frontend contract (for the separate myNewspaper repository)

Read `dbo.BreakingNews`; do not query the two tables directly.

```sql
SELECT ID, Topic, Headline, LeadUrl, LeadImageUrl,
       FirstPublishedAt, LastPublishedAt, DetectedAt,
       PublisherCount, ArticleCount, Articles
FROM dbo.BreakingNews
WHERE (@Topic IS NULL OR Topic = @Topic)
ORDER BY DetectedAt DESC, ID DESC;
```

- `ID` is stable for the life of a story and can key a "seen" or dismissed state.
- `Articles` is JSON: `PageId`, `Publisher`, `Title`, `Url`, `ImageUrl`, `Published`.
- `PageId` can be used for saved articles only while that row still exists in `dbo.Pages`; treat it as optional.
- The view can be empty. That is the normal state on a quiet day and should not render as an error.
- Timestamps are UTC; display in America/Chicago.

## Open questions

None. Still to confirm during implementation: the similarity thresholds (section 3) and how long the production
news stage takes (phase 2).
