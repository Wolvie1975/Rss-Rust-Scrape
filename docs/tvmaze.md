# TVmaze original episode scheduling

The collector uses explicit TVmaze show IDs. It never searches by title and never
uses nextepisode as an episode list. Initial tracking: 83073 Avatar: Seven Havens,
64950 VisionQuest, 45039 Slow Horses, 33352 The Rings of Power, and 90632 Line of
Fire (2026, NBC). The last is not the unrelated 2003 show.

## Schema handoff

[Exact developer definitions](tvmaze-developer-handoff.md) and `sql/tvmaze.sql`
were provided before production schema deployment. The migration is additive,
creates only TvSeries/TvEpisodes/TvTrackedSeries, and does not alter existing tables.
`sql/tvmaze_seed.sql` inserts missing settings only, preserving disabled entries.
The MyNewsFeed project receives copies of the migration, seed and this handoff.

## Operation

```sh
cargo run --locked -- --configure-tv
cargo run --locked -- --tv-from-db -o /tmp/tv-preview.xml
cargo run --locked -- --tv-from-db --dry-run -o /tmp/tv-preview.xml
# Explicit manual refresh ignores cache/backoff, but not active leases/disabled settings.
cargo run --locked -- --tv-from-db --tv-force-refresh -o /tmp/tv-preview.xml
cargo run --locked --example audit_tv -- WebScraper /tmp/tv-audit.json
cargo test --locked --all-targets
# Requires the populated TV staging database; this test alters only staging fixtures/status.
cargo test --locked --bin web_scraper tv_snapshots -- --ignored
```

The hourly service adds `--tv-from-db`. Enabling/disabling is managed through
TvTrackedSeries.Enabled; setup never re-enables an existing disabled entry.
Disabled rows are not fetched, and disabling during a fetch prevents its snapshot
from being persisted. Lease tokens prevent concurrent runs from writing superseded
snapshots. The last successful collection/episode count remains intact on failures.

## Dates and completeness

Blank/null airtime means date-only, even if airstamp is a noon placeholder. Raw
Airdate/Airtime/Airstamp and provider JSON are preserved exactly, including empty
strings and nulls. Airtime and StartsAtUtc are then NULL and IsDateOnly=1.
A nonblank local time requires a valid date and stamp before storing a UTC instant;
missing UTC information is reported without inventing a conversion.

Episode identity is TvmazeEpisodeId, not date/season/title. Same-day batches and
specials are preserved; fetch `/shows/{id}/episodes?specials=1` in full. The entire
show/list is validated before an atomic upsert. Invalid IDs, wrong show links,
duplicate IDs, missing required fields and non-list responses fail without changes.
Lists omitting previously stored episode IDs or losing a known airdate are rejected
as incomplete. Missing episodes/history are never deleted. Legitimate known-date
reschedules update the same episode. Raw JSON retains source changes separately.

These tables describe original broadcast/release scheduling. NetworkCountryCode
and WebChannelCountryCode describe that provider/network context, not streaming
availability. No Peacock availability or release dates are inferred from NBC.
A future country-specific availability feature must have separate attribution and
records, rather than rewriting these original schedules.

## Caching, rate limiting and failures

TVmaze caches normal API responses for 60 minutes. This collector skips HTTP when
a successful per-series cache is younger than one hour. It stores validated show
and full-list JSON in tracking settings. Per-series leases last 10 minutes; abandoned
claims expire, and disabled/superseded claims cannot write data.

The public API permits at least 20 calls per 10 seconds/IP. Requests are sequential,
reuse one client and pause at least 600 ms between starts. HTTP 408/429/5xx and
transport errors receive up to three attempts. Retry-After seconds/HTTP-date values
are honored; delays over 60 seconds are persisted for a later run instead of holding
up other shows. Other errors/malformed snapshots retain the last-good data and set
a retry window. Failure is isolated per series; no API key is needed.
[Official endpoint/caching/rate documentation](https://www.tvmaze.com/api).

## Web app licensing and attribution

The API is licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
The app should credit **TVmaze**, link episode/series cards to their stored SourceUrl,
link the license and indicate processing/adaptations such as date-only normalization.
When distributing adapted data, comply with ShareAlike. This does not automatically
require licensing unrelated application code under CC BY-SA. Images may be hotlinked
according to the API documentation; their stable URLs can be cached. No paid enterprise
license or separate premium user API is used.

[Official TVmaze licensing statement](https://www.tvmaze.com/api#licensing).

## Validation

Live identities and representative full-list fixtures cover all five shows, streaming
noon placeholders, timed NBC episodes, batch releases and both kinds of specials.
Mock HTTP tests cover Retry-After, transient retry and failure isolation. SQL staging
checks cover idempotent seeds with disabled entries, stable IDs/FirstSeenAt on reschedule,
repeated imports, atomic rejection of incomplete lists, last-success preservation
and another series succeeding after a failure.
