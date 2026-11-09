# Scraper handoff: deduplicate articles after URL changes

## Problem and confirmed example

The news frontend displays separate rows from `dbo.Pages`. A publisher changing a
headline and URL can cause the scraper to insert the same article twice.

Observed October 16, 2026, in `sql2025 / WebScraper`:

| Page ID | Source ID | Headline prefix | Published UTC | Scraped UTC |
| --- | --- | --- | --- | --- |
| 25604 | 1764 | Nine Xbox Games Are Out Today | 2026-10-16 10:00 | 2026-10-16 11:00 |
| 25647 | 1764 | 10 Xbox Games Are Out Today | 2026-10-16 10:00 | 2026-10-16 12:00 |

Older URL:

```text
https://www.purexbox.com/news/2026/10/nine-xbox-games-are-out-today-including-modern-warfare-4-campaign-early-access-october-16
```

Newer URL:

```text
https://www.purexbox.com/news/2026/10/10-xbox-games-are-out-today-including-modern-warfare-4-campaign-early-access-october-16
```

Following the older URL resolved to the newer URL with HTTP 200. Both stored records
belong to the same source. These IDs are diagnostic observations, not portable IDs
to hardcode into production logic.

## Requested behavior

Fix ingestion in the scraper so an article URL change updates the existing article
instead of creating another public card. Repeated collections must be idempotent.
Preserve distinct articles, including articles with identical titles or timestamps.

The frontend currently maps `Pages.ID`, `Url`, `UrlHash`, `SourceId`, `Title`,
`Description`, `Published`, `ScrapedAt`, and `ImageUrl`. It declares a unique index
on `UrlHash`. Inspect the actual scraper schema, hash implementation, upsert logic,
and existing identity handling before implementing changes.

## Identity resolution

1. Track the feed URL, redirect chain, final successful article URL, and any valid
   HTML canonical URL available during collection.
2. Use confirmed redirect/canonical identity to reconcile incoming articles with
   previously stored URLs. Recording a new article's final URL alone is insufficient:
   an older stored URL may not redirect until a later collection.
3. Resolve that gap with persisted URL aliases or a bounded reconciliation pass over
   recent articles from the same source. Reuse existing identity facilities where
   possible. Do not fetch the entire article history every hour.
4. Accept canonical URLs only after validation. Reject empty, malformed, non-HTTP(S),
   unrelated-host, homepage, login, challenge, or generic error-page targets. Define
   allowed publisher host aliases explicitly. A shared failure destination must never
   cause unrelated articles to merge.
5. Do not infer identity from title, image, description, publication date, or fuzzy
   similarity alone. Treat these as investigation clues, not merge authorization.
6. Keep normalization conservative. Do not lowercase URL paths or remove query
   parameters that could identify different articles. Apply established tracking-only
   normalization consistently to lookup and storage.
7. Keep matching within the same configured source unless cross-source identity is
   already an explicit, tested scraper feature. The existing global URL-hash uniqueness
   constraint also needs deliberate handling when another source owns a matching URL.

## Updates, collisions, and failure handling

- Prefer a stable existing article ID when identity is confirmed. Update the title,
  URL, hash, and collected metadata using existing field-ownership rules. Preserve
  publication time unless the publisher supplies a legitimate correction; never
  substitute scrape time for publication time.
- If old and new URLs already have separate rows, reconcile them atomically. Inspect
  references and retention rules before choosing the survivor or removing a duplicate.
  Account for frontend saved-article IDs, which can exist in browser storage and may
  not be discoverable through database foreign keys.
- Preserve useful metadata when an incoming scrape omits fields. Do not clear a
  valid poster/image, description, or publication date merely because lookup failed.
- Respect URL-hash uniqueness during merges and URL updates. Concurrent collection
  and retries must converge on one article without losing data or failing the batch.
- Failed requests, redirect loops, timeouts, invalid canonicals, and rate limits must
  retain last-good articles. Use bounded requests and the scraper's existing fetch
  protections, including redirects to private or otherwise disallowed destinations.
- Keep diagnostics in scraper logs/admin output, not public news cards. Record enough
  identity evidence to explain an update, collision, skipped merge, or failure.

If aliases or other schema additions are required, supply a separate additive
migration and runtime-permission requirements. Do not assume the web application
will provision scraper-owned schema.

## Existing duplicate cleanup

Start with a read-only dry run that lists source, candidate IDs, old/final URLs,
identity evidence, proposed survivor, and metadata/reference changes. Include the
Pure Xbox pair above if it still exists and redirect evidence still agrees.

Before modifying existing rows, confirm the actual target database, a recent backup,
and the intended cleanup scope. Use transactions and recheck identity/collisions at
write time. If survivor/reference handling is uncertain, report the candidate instead
of deleting it. Do not perform a broad title-based duplicate purge.

## Validation

Use deterministic HTTP fixtures and an isolated test database for write tests.
Cover:

- An unchanged feed URL collected repeatedly produces one article.
- An old URL begins redirecting to a new URL after its first ingestion.
- The new URL is discovered before the older stored URL is revisited.
- Two existing rows converge through a confirmed redirect, retaining the intended
  stable ID, metadata, and references according to the merge policy.
- Redirect chains and approved host aliases resolve consistently.
- Valid article canonicals reconcile aliases; malformed, unrelated, and generic
  canonicals do not merge records.
- Identical titles, images, or timestamps on distinct URLs remain separate stories.
- Redirect loops, timeouts, 429/5xx responses, and shared challenge/login pages cause
  no destructive merges or lost last-good data.
- Concurrent ingestion and unique-hash collisions are handled safely and repeatably.
- Re-running cleanup makes no further changes.

## Deployment and acceptance

Follow the scraper repository's own deployment instructions. Preserve its runtime
environment, mounts, network, certificate settings, scheduler arguments, and enabled
sources. Do not change collection frequency as part of this fix.

Report code changes, actual schema changes, tests, dry-run findings, applied cleanup,
remaining ambiguities, and rollback details. Verify the next collection remains
healthy and the frontend shows one card for the confirmed Pure Xbox article while
unrelated articles remain intact. A frontend deployment is unnecessary unless the
chosen identity policy explicitly requires frontend support.
