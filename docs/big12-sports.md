# Big 12 calendar ingestion

The scraper collects football, men's basketball, women's basketball, soccer, and
volleyball from the provider's current full-member schools. Oklahoma State has no
volleyball coverage. Membership and school logos come from the provider calendar
members metadata; sport filtering is explicit in `src/big12.rs`.

## Setup and operation

```sh
# Apply additive schema, register verified member feeds disabled, preserve Kansas.
cargo run --locked -- --configure-big12
# Deliberately enable all registered Big 12 school feeds.
cargo run --locked -- --enable-big12
# Inspect without event/source/status writes (schema initialization still runs).
cargo run --locked -- --events-from-db --dry-run -o /tmp/calendar-preview.xml
# Import now; the existing hourly service uses this same flag.
cargo run --locked -- --events-from-db -o /tmp/calendar-preview.xml
# Read-only database audit.
cargo run --locked --example audit_sports -- WebScraper /tmp/sports-audit.json
```

`--database NAME` selects an isolated staging database. The default is WebScraper.
The setup command preserves existing feed IDs, URLs, enable settings and event IDs.
Only `--enable-big12` explicitly enables disabled member feeds. Repeated setup
recognizes school IDs despite RSS query-parameter order; no duplicate configuration
is created. Affiliates and retired members are not imported as current full members.
Existing retired configurations are reported as failures rather than silently reassigned.

## One event, multiple source perspectives

`SportsEvents` contains one public row per provider game group. A mirrored game can
have different provider IDs and URLs in each school's RSS. Structured calendar
`id`/`opp_game_id` links establish a sorted stable pair key; unpaired games use their
provider game ID. Dates, titles and team names are never deduplication keys.

`SportsEventSources` preserves each feed's provider game ID, source URL, school and
opponent IDs, JSON snapshot, warnings and timestamps. Its unique key is feed ID plus
provider game ID. Both aliases resolve to the same public row, including when an
unpaired game later gains a linked ID. Conflicting existing public identities are
quarantined for review; records are never deleted automatically.

Historical Kansas rows are backfilled as primary observations, keeping their IDs,
FirstSeenAt, school perspective, URLs and logos. For new events a home observation
is processed first; neutral games use deterministic feed ordering. The primary
perspective remains stable on subsequent imports. Only that source replaces its
public schedule; a known secondary start can fill a same-date TBD time. Secondary
stale/TBD information cannot overwrite a known primary time. Nonempty broadcast
information can fill missing public fields. Each full source snapshot remains available.

School-specific queries **must use the source links**, not just the public row's
primary SportsEventsTypeId. The SQL view provides that join:

```sql
SELECT * FROM dbo.SportsEventSchoolCalendar
WHERE SchoolId = 3 AND EventDate >= @WeekStart AND EventDate < DATEADD(day, 7, @WeekStart);
```

The normal public list can continue reading SportsEvents, which is deduplicated.
The view's `SportsEventId` is the same public event ID in both schools' calendars;
logos, opponent and home/away information reflect the selected school perspective.
Radio and AudioUrl supplement Tv, StreamUrl and LiveStatsUrl. The separate .NET
application was not changed; its school-filtered queries should adopt this view.

## Unknown data, provider errors and status

RSS is authoritative for source dates and UTC instants. Metadata adds explicit
neutral-site status, participant/sport IDs and media. Neutral events use IsAway=NULL
and IsNeutral=1. Unknown venues, logos, times and broadcast links stay NULL.
Conference/channel logos are never used as school logos. Source timezone labels are
stored as supplied; RSS UTC times are not recalculated from ambiguous abbreviations.

Conference-wide RSS and oversized JSON ranges omit some future games. The scraper
fetches school RSS and requests JSON identity metadata one observed month at a time.
Missing or inconsistent identity metadata keeps a new observation out of the public
calendar, while retaining it in SportsEventSources with SportsEventId=NULL and Issue.
Known self-opponent records and member games without explicit partner links are
quarantined. Missing records never remove historical games or imply cancellation.

RSS/JSON UTC disagreement and paired-RSS timing disagreements are recorded as warnings;
source values are retained for review. A failed refresh preserves a source's last good
payload and event link. One failed school does not stop other school feeds.
SportsEventsType records Enabled, LastAttemptAt, LastSuccessAt, LastError,
LastEventCount and LastIssueCount. LastSuccessAt means RSS observations were processed
and persisted; a nonzero LastIssueCount identifies warnings or quarantined records.
If collection or SQL persistence fails, LastSuccessAt is preserved. Disabling a feed
stops polling; disabling a feed during a run also prevents subsequent source writes.

## Validation

```sh
cargo test --locked --all-targets
# Named staging database populated from Kansas data and configured first:
cargo test --locked --bin web_scraper mirrored_imports -- --ignored
```

Fixtures cover membership, linked Kansas/Utah IDs, primary/secondary perspective,
neutral/TBD events, inconsistent UTC metadata, doubleheaders, missing identity and
self-opponents. Staging validates migration reruns, Kansas historical identities,
repeated/concurrent imports, source attribution and persisted feed status.

## Deployment verification and rollback

See [the October 9 verification report](../reports/big12-expansion-2026-10-09.md).
The staging database remains available for inspection. The host checkout has a
pre-expansion source/sports snapshot under `.deployment-backups/20261009-big12/`.
The rollback image is `web-scraper:before-big12-20261009`. A legacy image must run
with the explicit Kansas `--events-feed` URL replacing `--events-from-db`; it cannot
safely poll all newly registered feeds because it lacks canonical deduplication.
