# Article URL identity and deduplication

News identity is scoped to the configured `Sources` row. Titles, images,
descriptions, publication timestamps and fuzzy similarities never authorize a
merge. The existing global `Pages.UrlHash` uniqueness boundary remains intact.

## Collection and matching

`src/article_identity.rs` normalizes URLs by parsing them and removing fragments.
It preserves path case and every query parameter; it does not strip tracking or
article-identifying parameters. Lookup aliases and target URLs use the same
normalization. Existing raw URLs are also retained when verified, so historical
hashes remain discoverable. SQL hashes are SHA-256 over NVARCHAR (UTF-16), matching
the existing `Pages.UrlHash` computation.

The identity client uses the shared public DNS/IP checks, no ambient proxy, a
five-second request timeout, bounded response bodies, and manual redirects. It
rejects private/reserved destinations, redirect loops, HTTPS downgrades, unrelated
publisher hosts and generic/login/challenge/error destinations. Evidence requires
a successful HTML article response with article metadata; a 200 response alone
is insufficient. Single-segment article slugs are accepted only with verified
article metadata; generic root/news/home and error destinations remain excluded.
Same-host URLs are allowed on the same port, with standard HTTP-to-HTTPS upgrades. The explicit cross-host alias list
currently permits only `purexbox.com` ↔ `www.purexbox.com`.

A valid same-publisher article canonical is fetched and checked before it can
join identities. A failed canonical lookup does not authorize a merge; verified
redirect evidence can still be used independently. Invalid or unrelated
canonicals are ignored. Missing/failed identity evidence leaves existing articles
and useful metadata intact.

For each news source, the scraper examines up to ten recent retained articles,
ordered by publication/scrape recency, then prioritizes least-recently checked URLs.
If incoming feed URLs are already current stored URLs, it revisits two retained
URLs. When new URLs appear, it can revisit all ten and check incoming URLs, with
at most twenty jobs per source. Up to four jobs run concurrently. New batches
stop after a twelve-second source work budget or a ninety-second run work budget;
an already-running bounded request/redirect/canonical check may finish afterward.
Source processing rotates hourly so a run-wide budget does not always favor the
same sources. Collection frequency and retention limits are unchanged.

Revisiting older URLs closes the gap where the new feed URL arrived before the
old stored URL began redirecting. Confirmed redirects/canonicals reconcile older
rows before current feed metadata is saved. Retained reconciliation candidates
are not added as extra RSS entries. Evidence may be deferred by request limits or
publisher failures; the scraper does not guess identity to compensate.

## Schema and migration

Apply these files **separately and in order**, with migration credentials:

1. `sql/article_identity.sql`: creates/seeds `dbo.ArticleUrlAliases` and creates
   `dbo.ArticleIdRedirects`.
2. `sql/upsert_article_identity.sql`: creates or alters
   `dbo.UpsertArticleIdentity` in its own SQL batch.
3. `sql/article_identity_permissions.sql`: grants runtime execution, alias reads,
   check-time updates and retired-ID reads to `WebScraperIngest`.

Fresh installations also receive the objects through `--initialize-db`; normal
runtime connections do not perform DDL and require the explicit migration.
`sql/runtime_permissions.sql` includes the corresponding grants. The existing
schema snapshot predates these additive objects; apply this migration after
restoring that snapshot.

`ArticleUrlAliases` maps `(SourceId,UrlHash)` to the stable `Pages.ID`, retaining
URL, evidence kind and check timestamps. The migration seeds existing raw URLs
without claiming that their redirects/canonicals have been checked.

`ArticleIdRedirects` maps retired duplicate IDs to their survivor and archives the
retired row as `OriginalPageJson`, along with URL evidence and merge time. Both
tables cascade when the surviving article is removed by existing retention.
Saved-article mappings therefore share the article's retention lifetime; this
feature does not establish permanent saved-article storage.

## Atomic upsert and merge policy

`UpsertArticleIdentity` uses a transaction and application lock spanning the global
URL uniqueness boundary. It chooses the oldest existing ID within the same source,
archives retired IDs and metadata, redirects aliases/mappings, removes the duplicate
public rows, then updates the survivor. Unverified old aliases cannot revert an
already-confirmed current URL. Publication time follows existing ownership rules:
fill a missing value or retain an earlier supplied correction; never use scrape
time. Missing/blank descriptions and images preserve useful existing metadata.
The survivor keeps its original `ScrapedAt` and ID.

A URL already owned by another source is reported as
`skipped-cross-source-collision`; ownership is not reassigned. Unknown foreign-key
references to `Pages` produce `skipped-unhandled-foreign-reference`, leaving all
candidate rows intact. The procedure runs `EXECUTE AS OWNER` so these checks can
see all foreign keys without granting the runtime account schema ownership.
The client retries bounded SQL deadlock/lock-timeout/unique-collision errors.
Diagnostics include outcome, source, candidate IDs, survivor and alias URLs in
scraper logs, never public cards.

Use `@DryRun=1` to preview a proposed group and survivor without data writes.
Existing cleanup must first verify the actual database, recent backup, source,
current redirect/canonical evidence and intended scope. Inspect unknown references
rather than deleting around them. No title-based or whole-history purge is used.

## Frontend saved IDs

Public news queries still read `dbo.Pages` without a new filter or schema model.
A confirmed duplicate therefore becomes one public card without a frontend deploy.
Browser saved IDs cannot be enumerated by SQL. The approved policy preserves the
oldest article ID and records retired-ID redirects for frontend recovery.

A frontend implementation can resolve missing saved IDs with:

```sql
SELECT RetiredPageId,PageId,SourceId
FROM dbo.ArticleIdRedirects
WHERE RetiredPageId=@SavedPageId;
```

Then load the surviving `Pages` row and update the browser's saved ID. Grant the
web read principal SELECT on this table if its existing role does not already
permit reads. This scraper release records the mapping; it does not implement
browser-storage recovery in My-Newsfeed.

## Tests and production evidence

Offline tests cover normalization, host aliases, redirect chains, canonical
verification, malformed/generic targets, loops, rate limits, errors and timeouts.
The isolated SQL test covers repeated ingestion, identical-content distinct URLs,
new-row-first reconciliation through deterministic HTTP fixtures, metadata/ID
preservation, dry-run behavior, unknown references, cross-source collisions,
transaction rollback and concurrent sessions.

```sh
cargo test --locked --all-targets
# Explicit SQL write tests, using an isolated migration profile only:
ARTICLE_IDENTITY_TEST_DATABASE=WebScraper_ArticleIdentity_Validation_NAME \
  cargo test --locked db::tests::article_identity_sql_tests -- --ignored --nocapture
```

October 16 production verification and rollback details are recorded in
[operations history](../reports/operations-history.md#article-identity-deployment-2026-10-16).
