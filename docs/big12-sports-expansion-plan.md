# Big 12 sports calendar expansion proposal

Inspection date: October 9, 2026. **Implemented and deployed October 9, 2026.**
The proposal below records the pre-implementation inspection. Current operation:
[Big 12 ingestion guide](big12-sports.md). Production now has 16 enabled school
feeds, 943 public events and 1,444 source observations, with 3 quarantined records.

## Existing configuration and live schema

The only configured SportsEventsType row is ID 1, “Kansas – all sports”, SchoolName=Kansas,
with sport_id=0, school_id=3, schedule_id=0. Its 101 stored SportsEvents rows contain
Football (9), Men's Basketball (32), Women's Basketball (31), Soccer (10), and Volleyball (19).
The current live Kansas RSS contains 91 upcoming rows. Retained historical rows remain in SQL.

SportsEventsType has ID, RssUrl, EventsTypeName, DateAdded and SchoolName. RssUrl is unique.
It has no enable flag, provider school ID, polling timestamps or LastError.
SportsEvents has ID, Url, computed UrlHash, GameId, Title, Sport, Opponent, IsAway,
Location, EventDate, StartsAtUtc, EndsAtUtc, TimeTbd, Tv, StreamUrl, LiveStatsUrl,
TeamLogoUrl, OpponentLogoUrl, FirstSeenAt, LastSeenAt and SportsEventsTypeId.
UrlHash is unique; GameId is nullable and is not unique. Its only inspected foreign key
is SportsEventsTypeId -> SportsEventsType.ID. ResultGames/ResultFollows are separate
live tables; their inspected foreign keys do not reference SportsEvents.

The current MERGE matches the URL hash, then overwrites the school perspective and
SportsEventsTypeId. This cannot provide multi-feed attribution or deduplicate mirrored IDs.
The scraper logs feed errors but does not persist a per-calendar attempt/success/error status.
The existing school-logo fix resolves school_id through the calendar members component;
opponent logos still come from RSS. Preserve that behavior.

## Provider verification

- Calendar and current full-member metadata: https://big12sports.com/calendar.aspx
- RSS pattern: https://big12sports.com/services/responsive-calendar-subscription.ashx/calendar.rss?sport_id=0&school_id={ID}&schedule_id=0
- Conference-wide RSS: the same URL with school_id=0; HTTP 200, 749 items.
- School/sport availability: https://big12sports.com/services/responsive-calendar.ashx/sport_schools?sport_id={SPORT_ID}
- Structured calendar: https://big12sports.com/services/responsive-calendar.ashx?start=2026-10-01&end=2026-10-31%2023:59:59&sport_id=0&school_id=0

Current members were selected from the provider members component using school_active=true
and member_type=F, then cross-checked against sport_schools. All 16 school-specific RSS
requests returned HTTP 200. Sport IDs: Football 4, Men's Basketball 7, Women's Basketball 15,
Soccer 31 (provider gender metadata identifies women's soccer), Volleyball 37 (women's).
All 16 are listed for the first four sports; volleyball lists 15, excluding Oklahoma State.
Affiliate schools in other provider sport lists are not automatically included.

Counts below are upcoming source observations, not deduplicated events or full-season guarantees.
All feeds use the RSS pattern above with their school ID.

| School | ID | Football | Men's basketball | Women's basketball | Soccer | Volleyball |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Arizona | 34 | 7 | 32 | 30 | 6 | 15 |
| Arizona State | 35 | 8 | 32 | 32 | 6 | 15 |
| Baylor | 1 | 7 | 33 | 31 | 6 | 13 |
| BYU | 32 | 8 | 32 | 32 | 6 | 14 |
| UCF | 31 | 7 | 32 | 32 | 6 | 14 |
| Cincinnati | 30 | 7 | 32 | 31 | 6 | 15 |
| Colorado | 11 | 7 | 32 | 30 | 6 | 14 |
| Houston | 33 | 7 | 32 | 31 | 6 | 13 |
| Iowa State | 2 | 7 | 33 | 31 | 6 | 14 |
| Kansas | 3 | 8 | 32 | 31 | 6 | 14 |
| Kansas State | 4 | 8 | 32 | 32 | 6 | 14 |
| Oklahoma State | 6 | 8 | 32 | 32 | 6 | 0 |
| TCU | 7 | 7 | 33 | 32 | 6 | 14 |
| Texas Tech | 9 | 7 | 32 | 30 | 6 | 13 |
| Utah | 36 | 8 | 32 | 30 | 6 | 13 |
| West Virginia | 10 | 7 | 32 | 30 | 6 | 13 |

## Coverage and field limitations

The five-sport snapshot contains 1,434 school-feed observations. Exclude unrelated sports
(e.g. wrestling present in some all-sport feeds). Start with an explicit shared allowlist
of the five currently collected sports; review future scope changes rather than treating
an empty seasonal Kansas feed as proof that a sport should be removed.

RSS supplies opponent name/logo, vs/at perspective, local date, UTC start/end, venue,
TV text, video URL and live-stats links. Members metadata supplies official school name,
ID and logo. The JSON calendar supplies school/opponent IDs, sport ID/gender,
location_indicator (H/A/N), neutral_hometeam, explicit paired game IDs and media details.
Do not interpret every RSS “vs” as a home game: JSON identified 126 neutral-site records
in the sampled monthly five-sport metadata. For neutral events, use IsNeutral=true and
IsAway=NULL unless an explicit designated perspective is retained separately.

Missing in the five-sport RSS snapshot: 9 venues, 391 opponent logos, 1,147 TV labels,
1,223 video links and 1,252 live-stats links. 868 starts are date-only/TBD. All current
full members have school image URLs in members metadata. No member-opponent logo is
missing in this snapshot; absent opponent logos are outside that exact member-name set.
Store unknown values as NULL; do not invent times, logos, venues or broadcast availability.
Retain supplied nonempty broadcast information from either source perspective, with provenance;
50 paired games had differing TV values across the two RSS feeds. JSON also supplied
radio text on 96 and audio links on 2 sampled target-sport records; preserve these in
source metadata, with optional Radio/AudioUrl columns for public display.

Both conference-wide RSS and broad JSON queries omitted later games present in school RSS.
Conference RSS ended January 5, 2027, while the school RSS snapshot extends to March 6.
One September-April JSON query stopped January 16. Six bounded monthly JSON queries
(October-March) covered all 1,434 current target-sport source IDs via id or opp_game_id.
Request metadata in bounded months covering observed RSS dates and retained dates needed
for backfill; validate coverage and report unresolved IDs instead of assuming completeness.

JSON date_utc disagreed with timed RSS starts for 19 observed football records. For Kansas
at Utah, RSS reports October 11 02:15 UTC, while JSON reports October 10 05:00 UTC even
though its local date is October 10 21:15. Use RSS UTC/date fields as the existing timing
source; use JSON for identity/neutral/media enrichment, and log time disagreements.
Preserve supplied local calendar dates and source timezone labels; do not assume every
school is Central or substitute JSON UTC blindly. Source labels include CT, ET, MT, MST,
and missing values. NULL/TBD times must remain distinct from midnight placeholders.

## Recommended collection and identity design

1. Keep the Kansas configuration row, URL, event IDs and historical rows. Add 15 school
   configurations using verified IDs, initially disabled for validation. Use one all-sport
   RSS feed per school, with the shared five-sport allowlist. Do not replace Kansas with
   a conference-wide RSS feed or enable unrelated provider sports.
2. Fetch/cache the members metadata once per collection pass. Use each feed school_id
   for names and logos. Use structured school/opponent/sport IDs for identity; keep RSS
   opponent extraction and use verified provider identity for enrichment when needed.
3. Use JSON id/opp_game_id mappings from bounded monthly requests to link mirrors.
   Example: Kansas at Utah has Kansas ID 179871 and Utah ID 179872; the two school JSON
   responses explicitly link them reciprocally. Their RSS URLs are different, so the
   existing UrlHash unique constraint would currently allow two public events.
4. For an explicitly linked pair, use a provider-namespaced stable group identity such
   as big12:pair:179871:179872 (sorted IDs). Unpaired games use big12:game:{id}. Dates,
   title text, time and logo must not be part of identity. Validate sport and participant
   IDs before linking; do not merge by teams/date alone, which would collapse doubleheaders.
5. Preserve the existing Kansas SportsEvents.ID as the canonical row when either alias
   already matches it. Add another source attribution rather than inserting another event.
   Resolve aliases before inserting; if a formerly unpaired game later receives a pair link,
   attach it to the existing event and update its canonical key in a transaction. If two
   existing canonical rows are found, flag a reconciliation conflict rather than deleting
   records or silently rewriting references. Historical backfill must resolve existing
   GameId/event-URL IDs before any insertion; leave unresolved old rows intact.
6. Preserve each school/feed perspective in a source junction table. Do not allow the
   latest feed to flip canonical TeamLogoUrl/OpponentLogoUrl, IsAway or SportsEventsTypeId.
   Existing rows keep their current primary perspective. New events can choose the home
   source initially, with a deterministic choice for neutral events, and keep that choice.
   Public listing reads one canonical SportsEvents row. A school's calendar uses the
   source/participant links, not only the single SportsEventsTypeId foreign key.

The snapshot contains 498 explicit mirrored pairs; all 498 matched reciprocal school
names and sports. Grouping them gives 936 candidate public events from 1,434 observations.
This is a validation count, not permission to publish every record: two observations list
Colorado/Colorado and Iowa State/Iowa State, and another Utah/Iowa State observation lacks
an explicit partner ID. Retain such source records for review; do not guess a match or
publish an unresolved duplicate.

## Schema changes needed

Keep the two existing tables and public event IDs. Use an additive migration:

- SportsEventsType: ProviderKey, ProviderSchoolId, Enabled, LastAttemptAt, LastSuccessAt,
  LastError and LastEventCount. Keep SchoolName and the existing unique RssUrl. Add a
  configurable sport scope (e.g. validated SportIds JSON) if it should be admin-managed.
  Preserve Kansas's existing row and enable setting during seeding; never re-enable
  disabled feeds or create a second row due only to query-parameter ordering.
- SportsEvents: a nullable, uniquely indexed CanonicalKey for staged historical backfill;
  explicit ProviderKey, SchoolId, SchoolName, OpponentSchoolId, ProviderSportId and IsNeutral.
  Retain GameId/Url/UrlHash as the primary source identity and public compatibility fields.
  CalendarTimeZone, Radio and AudioUrl are useful additions for fields absent today.
- New SportsEventSources junction: SportsEventId FK, SportsEventsTypeId FK, ProviderGameId,
  SourceUrl, provider school/opponent IDs and FirstSeenAt/LastSeenAt. Unique constraint on
  (SportsEventsTypeId, ProviderGameId). Preserve source perspective/schedule/media fields
  in validated JSON or dedicated columns, including school/opponent names/logos, IsAway,
  neutral status and raw source times. This is required for many feeds to attribute one
  public event without abusing the current single-feed foreign key.

Use unique canonical keys plus transaction/locking protection for concurrent polling.
Feed success/failure updates are independent; a failed feed must not erase the last successful
observations. Missing RSS entries do not delete history or prove cancellation. A partial
RSS/JSON metadata failure should record an enrichment issue and keep unresolved observations
out of duplicate public publication, while other feeds continue.

Without the junction table, the alternatives are losing secondary attribution or keeping
mirrored SportsEvents rows and deduplicating only in a view. Neither is the recommended
one-event/many-sources model. The .NET front end will need participant/source-link queries
for school filtering, plus IsNeutral support; no front-end source was changed here.

## Small validation plan before rollout

1. Save representative Kansas/Utah mirrored RSS+JSON fixtures, Kansas State multi-word
   school names, Oklahoma State's absent volleyball, a neutral event, TBD/time disagreement,
   doubleheader and missing-logo cases. Validate each field and explicit pair identity.
2. On a staging copy, run the migration twice and backfill aliases for all 101 Kansas rows.
   Confirm unchanged event IDs, FirstSeenAt, logos and primary perspective; check historical
   GameId/URL matches before enabling other school rows.
3. Dry-run Kansas and Utah first. Expect one public event and two source attributions for
   179871/179872. Check merged nonempty broadcast fields, source-specific status, failed-feed
   isolation, neutral handling and atomic reschedules. Import twice and concurrently;
   event/source counts must stay stable, including retained Kansas history.
4. Dry-run all 16, compare per-sport observations against the table, validate monthly metadata
   coverage, review self-opponent/unpaired anomalies, then enable the 15 new school feeds.
   Verify public uniqueness and each school calendar before changing the production service.

The expansion is implemented and deployed. Staging repeated/concurrent imports
preserved event/source counts and all 101 original Kansas identities and logos.
The public primary source controls timing; conflicting secondary source values are
retained and reported. Setup flags cannot be combined with dry-run or scheduled mode.
