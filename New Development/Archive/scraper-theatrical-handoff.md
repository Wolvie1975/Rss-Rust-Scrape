# Scraper handoff: restore theatrical coverage without DVD/disc releases

Prepared October 22, 2026 (Central) for the separate Rss-Rust-Scrape project.

## User intent

The user disabled **DVDs Release Dates because they do not buy DVDs**. They
intended to exclude DVD/disc releases, not theatrical dates supplied by that same
provider. Preserve theatrical coverage and existing digital/streaming coverage
while excluding physical-disc collection. Do not infer that a provider's name
determines the release types it supplies.

The user requested this handoff for the scraper session. No scraper configuration,
provider enablement or database data was changed by the frontend investigation.
Deployment permission recorded elsewhere applies to the current frontend session;
it is not blanket authorization for a future scraper session.

## Verified diagnosis

A read-only query through the deployed web application's SQL connection found six
records dated **Friday, October 23, 2026**, all with:

- ReleaseType = `theatrical`
- CountryCode = `US`
- ReleaseStatus = `scheduled`
- Provider Name = `DVDs Release Dates`
- Provider Enabled = `false`
- LastSeenAt = October 21, 2026, 13:35:03 (raw stored timestamp)

Titles:

- A Talent for Murder
- Ali G: Who Iz I?
- Clayface
- Klara and the Sun
- Wildwood
- You Can See Everything

These are observations of stored records, not independent verification of real
release dates or titles. Recheck current data; do not treat this dated snapshot as
evidence that these releases remain accurate.

The frontend correctly excludes disabled sources. Both the general Movies query
and homepage theatrical query join MovieReleases → MovieSourceLinks → Movies /
MovieReleaseSources and require `source.Enabled`, US, scheduled status and the
requested release type/date. The Movies page returned zero theatrical releases
for the current month because these retained records belonged to a disabled source.
The October 23 date itself was inside the page's date window.

## Requested scraper work

1. Inspect the provider implementation, existing configuration and schema before
   choosing a change. Confirm how provider enablement, per-type collection and
   retained rows interact. Do not assume per-type switches already exist.
2. Restore trustworthy US theatrical coverage. If retaining DVDs Release Dates,
   support collecting theatrical data independently of DVD/Blu-ray/other physical
   formats. If existing enabled providers can supply equivalent coverage, assess
   that path before adding another source or duplicate records.
3. Keep physical-disc collection disabled according to the user's preference.
   Preserve enabled providers, digital/rental/purchase and subscription behavior.
   Audit the collector's physical-format mappings instead of assuming every
   physical format is already represented by the same release-type value.
4. Verify the six stored dates against current provider evidence before refreshing
   or exposing them. Preserve source links and accurate status/date provenance.
   Correct stale mappings or dates through the established ingestion process.
5. Address retained physical-disc rows explicitly. Merely skipping future disc
   inserts does not hide historical scheduled disc rows when a source is enabled.
   Inspect existing mechanisms for type eligibility/withdrawal and report the
   selected approach. Do not delete unrelated records or relabel theatrical rows
   to bypass filters.
6. Re-enable the existing provider only after collection restrictions and retained
   row behavior meet the requested scope. Preserve source/movie identities,
   deduplication, unrelated enablement choices and last-good data on failures.

## Frontend contract and coordination

- `MovieReleaseSources.Enabled` is currently a provider-wide public visibility
  gate. The frontend has no per-source/per-type enablement gate.
- Public release types include `theatrical`, `subscription`, `digital`,
  `digital_rental`, `digital_purchase` and `disc`. Confirm collector mappings.
- The Movies page's All releases query can show scheduled disc rows from an
  enabled source. The user wants discs excluded: if the scraper solution needs a
  new eligibility field or view, coordinate a frontend reader change before
  enabling that source. Do not silently change this shared contract.
- The homepage theatrical subsection now shows only the current Friday's releases
  Thursday–Sunday inclusive in the configured display zone (Central in production).
  Saturday/Sunday retain Friday; Monday–Wednesday hide the subsection. It shows
  up to five deduplicated movies. Six eligible records therefore need not all
  appear on Home; the full Movies page should expose all eligible records.
- `/movies?type=theatrical` shows scheduled US theatrical releases from today
  through month end. This is separate from the homepage's Thursday–Sunday rule.
- Digital / VOD retains its own enabled-provider selection and date filters.

Frontend references in My-Newsfeed:

- `src/MyDailyFeed.Web/Data/MoviesQuery.cs`
- `src/MyDailyFeed.Web/Data/DigitalReleasesQuery.cs`
- `src/MyDailyFeed.Web/Data/Models/Movie.cs`
- `src/MyDailyFeed.Web/Components/Pages/Movies.razor`
- `src/MyDailyFeed.Web/Components/Pages/Home.razor`
- `docs/entertainment.md`

## Validation and completion evidence

Use isolated fixtures/staging for ingestion writes and regression checks. Verify
repeat collection, source/movie deduplication, disabled-provider behavior,
theatrical-only selection, retained disc handling and failure isolation. Confirm
digital and subscription observations are preserved. Follow the scraper project's
own migration, backup and deployment requirements if changes require them.

After rollout, verify current source/type/date/status counts and check both
`/movies?type=theatrical` and Home, including phone/desktop layout. An enabled
provider with zero successfully collected eligible rows is not restored coverage.
Report which titles/dates were confirmed, corrected or excluded and why; do not
require all six historical rows to survive verification. Update the frontend
handoff with the implemented configuration/data contract, checks, deployment state,
rollback details and any required frontend changes. Keep secrets out of reports.


## Scraper completion — October 22, 2026, 8:58 AM CDT

Implemented and deployed **web-scraper:theatrical-no-disc-20261022**. Existing
source 5 is enabled with new `MovieReleaseSources.CollectDisc=0`. The collector
skips disc discovery and disc-only tracking, filters physical releases from all
detail pages, and the SQL upsert independently rejects physical-disc writes.
Twenty-four retained disc rows are withdrawn, so existing frontend scheduled-only
queries exclude them. No frontend schema/query or deployment change is required.
Provider Enabled continues to be the overall public visibility gate.

All six listed October 23 dates were confirmed against current provider detail
pages; none needed correction or exclusion. Two deployed imports saved 17 non-disc
releases each with zero issues and stable release IDs. All 40 pre-change release
IDs, dates, release keys and source URLs were preserved; one new digital release
was added. When To Stream and Movie Insider remain enabled, with digital and
subscription observations preserved. Zero scheduled physical-disc rows remain.

Playwright checked Home, theatrical Movies and All releases on desktop and phone:
HTTP 200, six titles on Movies, five on Home (existing cap), no horizontal overflow
or physical-disc badges. Existing unmatched-poster placeholders remain; checkout
and saved-item interactions were not tested. Scraper and frontend are healthy.

The full Rust suite passed 163 tests; two additional rollback-only SQL fixture
tests and strict Clippy passed. A verified COPY_ONLY database backup and protected
host source/configuration recovery snapshots were retained. Rollback must disable
source 5 before restoring the previous image, since that collector ignores the
new disc preference; leave the additive schema and withdrawn rows in place.

Canonical details: [configuration](../docs/theatrical-without-discs.md),
[deployment and rollback](../reports/theatrical-no-disc-20261022.md),
[disc policy script](../sql/movie_disable_discs.sql).
