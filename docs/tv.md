# TV series and episodes

TVmaze provides series metadata and current/future episode schedules. Keep tracking configuration and undated/TBD episodes. Past episode retention is governed by the operations guide; earlier deployment counts are historical.

## Contents

- [MyNewsFeed developer handoff: TVmaze series and episodes](#tvmaze-developer-handoff)
- [TVmaze original episode scheduling](#tvmaze)

<a id="tvmaze-developer-handoff"></a>

## MyNewsFeed developer handoff: TVmaze series and episodes

Prepared October 9, 2026, **before any production schema deployment**. The live
WebScraper inspection found no TV series/episode tables. This additive migration
creates only the three tables below; existing movie/sports/news schemas are unchanged.

<a id="tvmaze-developer-handoff--query-contract"></a>

### Query contract

- `TvSeries.ID` is the internal key; `TvmazeShowId` is the unique public provider ID.
- `TvEpisodes.SeriesId` references TvSeries.ID. `TvmazeEpisodeId` is globally unique.
  Never deduplicate by date, title, season/number or show name: batches and specials
  can share dates, and special episode numbers can be NULL.
- `TvTrackedSeries` is an admin lookup/status table. It can exist before a successful
  series fetch, so SeriesId is initially NULL. New seeds are enabled; rerunning seed
  SQL does not modify/re-enable existing tracking entries.
- `Airdate` is the provider's original local calendar date. `Airtime` is its supplied
  local clock time, not a timezone conversion. `StartsAtUtc` is meaningful only when
  the provider supplied an airdate, nonblank airtime and a parseable airstamp.
- **IsDateOnly=1 means no confirmed release time.** Never display RawAirstamp as a
  release time. In particular, the API's noon stamps with blank airtime are placeholders.
- Network/web-channel fields describe original scheduling. They do not establish
  country-specific playback availability. Line of Fire is TVmaze 90632 (NBC, 2026),
  not the unrelated 2003 show. No Peacock dates are derived or stored.
- Raw JSON and raw scheduling strings retain provider nulls/empty values, timestamps,
  summaries and any future external IDs. Treat HTML in summaries as untrusted if rendered.
- Dates/times/source fields are last-known facts. Missing episodes are never deleted.
  Shorter identity sets or structurally incomplete responses fail atomically and
  preserve the last-good series/episodes/cache/status success timestamp.

<a id="tvmaze-developer-handoff--exact-sql-column-definitions-defaults-constraints-and-indexes"></a>

### Exact SQL column definitions, defaults, constraints and indexes

The following is the final migration artifact; it is also in `sql/tvmaze.sql`.
All collection timestamps are UTC DATETIME2. All foreign keys use NO ACTION.

```sql
-- Additive, repeatable TVmaze schema. No existing tables are altered.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF OBJECT_ID('dbo.TvSeries','U') IS NULL
 CREATE TABLE dbo.TvSeries (
  ID INT IDENTITY(1,1) CONSTRAINT PK_TvSeries PRIMARY KEY,
  TvmazeShowId INT NOT NULL CONSTRAINT UQ_TvSeries_Show UNIQUE,
  Title NVARCHAR(500) NOT NULL,
  NetworkId INT NULL,
  NetworkName NVARCHAR(200) NULL,
  NetworkCountryCode CHAR(2) NULL,
  NetworkTimeZone NVARCHAR(100) NULL,
  WebChannelId INT NULL,
  WebChannelName NVARCHAR(200) NULL,
  WebChannelCountryCode CHAR(2) NULL,
  WebChannelTimeZone NVARCHAR(100) NULL,
  Status NVARCHAR(50) NULL,
  PremiereDate DATE NULL,
  PosterUrl NVARCHAR(2048) NULL,
  SourceUrl NVARCHAR(2048) NOT NULL,
  ImdbId VARCHAR(20) NULL,
  TheTvdbId INT NULL,
  TvRageId INT NULL,
  RawMetadata NVARCHAR(MAX) NOT NULL CONSTRAINT CK_TvSeries_JSON CHECK(ISJSON(RawMetadata)=1),
  FirstSeenAt DATETIME2 NOT NULL CONSTRAINT DF_TvSeries_First DEFAULT SYSUTCDATETIME(),
  LastSeenAt DATETIME2 NOT NULL CONSTRAINT DF_TvSeries_Last DEFAULT SYSUTCDATETIME(),
  CONSTRAINT CK_TvSeries_Show CHECK(TvmazeShowId>0)
 );
 IF OBJECT_ID('dbo.TvEpisodes','U') IS NULL
 CREATE TABLE dbo.TvEpisodes (
  ID INT IDENTITY(1,1) CONSTRAINT PK_TvEpisodes PRIMARY KEY,
  TvmazeEpisodeId INT NOT NULL CONSTRAINT UQ_TvEpisodes_Episode UNIQUE,
  SeriesId INT NOT NULL CONSTRAINT FK_TvEpisodes_Series REFERENCES dbo.TvSeries(ID),
  SeasonNumber INT NULL,
  EpisodeNumber INT NULL,
  Title NVARCHAR(500) NULL,
  EpisodeType VARCHAR(50) NULL,
  IsSpecial BIT NOT NULL,
  Airdate DATE NULL,
  Airtime TIME(0) NULL,
  StartsAtUtc DATETIME2 NULL,
  IsDateOnly BIT NOT NULL,
  RuntimeMinutes INT NULL,
  ImageUrl NVARCHAR(2048) NULL,
  SourceUrl NVARCHAR(2048) NOT NULL,
  RawAirdate NVARCHAR(50) NULL,
  RawAirtime NVARCHAR(50) NULL,
  RawAirstamp NVARCHAR(100) NULL,
  RawMetadata NVARCHAR(MAX) NOT NULL CONSTRAINT CK_TvEpisodes_JSON CHECK(ISJSON(RawMetadata)=1),
  FirstSeenAt DATETIME2 NOT NULL CONSTRAINT DF_TvEpisodes_First DEFAULT SYSUTCDATETIME(),
  LastSeenAt DATETIME2 NOT NULL CONSTRAINT DF_TvEpisodes_Last DEFAULT SYSUTCDATETIME(),
  CONSTRAINT CK_TvEpisodes_Id CHECK(TvmazeEpisodeId>0),
  CONSTRAINT CK_TvEpisodes_Time CHECK(IsDateOnly=0 OR (Airtime IS NULL AND StartsAtUtc IS NULL))
 );
 IF OBJECT_ID('dbo.TvTrackedSeries','U') IS NULL
 CREATE TABLE dbo.TvTrackedSeries (
  ID INT IDENTITY(1,1) CONSTRAINT PK_TvTrackedSeries PRIMARY KEY,
  TvmazeShowId INT NOT NULL CONSTRAINT UQ_TvTrackedSeries_Show UNIQUE,
  SeriesId INT NULL CONSTRAINT FK_TvTrackedSeries_Series REFERENCES dbo.TvSeries(ID),
  ExpectedTitle NVARCHAR(500) NOT NULL,
  Enabled BIT NOT NULL CONSTRAINT DF_TvTrackedSeries_Enabled DEFAULT 1,
  LastAttemptAt DATETIME2 NULL,
  LastSuccessAt DATETIME2 NULL,
  LastError NVARCHAR(1000) NULL,
  EpisodeCount INT NULL,
  LastIssueCount INT NULL,
  CacheExpiresAt DATETIME2 NULL,
  RetryAfter DATETIME2 NULL,
  LeaseUntil DATETIME2 NULL,
  LeaseToken UNIQUEIDENTIFIER NULL,
  CachedShow NVARCHAR(MAX) NULL CONSTRAINT CK_TvTrackedSeries_ShowJSON CHECK(CachedShow IS NULL OR ISJSON(CachedShow)=1),
  CachedEpisodes NVARCHAR(MAX) NULL CONSTRAINT CK_TvTrackedSeries_EpisodesJSON CHECK(CachedEpisodes IS NULL OR ISJSON(CachedEpisodes)=1),
  CreatedAt DATETIME2 NOT NULL CONSTRAINT DF_TvTrackedSeries_Created DEFAULT SYSUTCDATETIME(),
  CONSTRAINT CK_TvTrackedSeries_Show CHECK(TvmazeShowId>0)
 );
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.TvEpisodes') AND name='IX_TvEpisodes_Schedule')
  CREATE INDEX IX_TvEpisodes_Schedule ON dbo.TvEpisodes(Airdate,SeriesId) INCLUDE(SeasonNumber,EpisodeNumber,StartsAtUtc,IsDateOnly,IsSpecial);
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.TvTrackedSeries') AND name='UX_TvTrackedSeries_Series')
  CREATE UNIQUE INDEX UX_TvTrackedSeries_Series ON dbo.TvTrackedSeries(SeriesId) WHERE SeriesId IS NOT NULL;
 COMMIT TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
```

<a id="tvmaze-developer-handoff--exact-seed-script"></a>

### Exact seed script

```sql
-- Explicit IDs only. Matched tracking rows retain Enabled and all existing settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 MERGE dbo.TvTrackedSeries WITH(HOLDLOCK) AS t
 USING (VALUES
  (83073,N'Avatar: Seven Havens'),
  (64950,N'VisionQuest'),
  (45039,N'Slow Horses'),
  (33352,N'The Lord of the Rings: The Rings of Power'),
  (90632,N'Line of Fire (2026, NBC)')
 ) AS s(TvmazeShowId,ExpectedTitle) ON t.TvmazeShowId=s.TvmazeShowId
 WHEN NOT MATCHED THEN INSERT(TvmazeShowId,ExpectedTitle) VALUES(s.TvmazeShowId,s.ExpectedTitle);
 COMMIT TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
```

<a id="tvmaze-developer-handoff--suggested-app-query"></a>

### Suggested app query

```sql
SELECT s.TvmazeShowId,s.Title AS SeriesTitle,s.NetworkName,s.WebChannelName,
       s.PosterUrl,s.SourceUrl AS SeriesSourceUrl,
       e.TvmazeEpisodeId,e.SeasonNumber,e.EpisodeNumber,e.Title AS EpisodeTitle,
       e.Airdate,e.Airtime,e.StartsAtUtc,e.IsDateOnly,e.IsSpecial,
       e.RuntimeMinutes,e.ImageUrl,e.SourceUrl AS EpisodeSourceUrl
FROM dbo.TvEpisodes e
JOIN dbo.TvSeries s ON s.ID=e.SeriesId
JOIN dbo.TvTrackedSeries t ON t.SeriesId=s.ID
WHERE t.Enabled=1 AND e.Airdate>=@WeekStart AND e.Airdate<DATEADD(day,7,@WeekStart)
ORDER BY e.Airdate,s.Title,e.SeasonNumber,e.EpisodeNumber,e.TvmazeEpisodeId;
```

<a id="tvmaze-developer-handoff--attribution-and-licensing"></a>

### Attribution and licensing

TVmaze's public API needs no key. Its documentation licenses API data under CC BY-SA;
credit TVmaze in the app and link each series/episode to its stored SourceUrl. Follow
ShareAlike when redistributing adapted data; do not describe this as a public-domain
feed. Image hotlinking is permitted by the API documentation. Keep attribution when
using those images. The license is [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/); link it and indicate date-only normalization/adaptations.
[TVmaze API licensing/rate limits](https://www.tvmaze.com/api).

<a id="tvmaze-developer-handoff--deployment-and-compatibility"></a>

### Deployment and compatibility

The collector uses `/shows/{id}` and `/shows/{id}/episodes?specials=1`, a one-hour
validated cache, enabled flags, last-attempt/success/error/count status, and atomic
snapshot upserts. Scheduling and availability are separate concepts. No application
UI changes are included in this scraper migration. The original show/episode links
are the attribution URLs; API request URLs are used only by the collector.

The scraper will validate this exact migration and collector in a separate database,
then provide import counts and any unresolved data issues. Changing Enabled is an
admin action; do not delete tracking rows to disable a show.

<a id="tvmaze"></a>

## TVmaze original episode scheduling

The collector uses explicit TVmaze show IDs. It never searches by title and never
uses nextepisode as an episode list. Initial tracking: 83073 Avatar: Seven Havens,
64950 VisionQuest, 45039 Slow Horses, 33352 The Rings of Power, and 90632 Line of
Fire (2026, NBC). The last is not the unrelated 2003 show.

<a id="tvmaze--schema-handoff"></a>

### Schema handoff

[Exact developer definitions](tv.md#tvmaze-developer-handoff) and `sql/tvmaze.sql`
were provided before production schema deployment. The migration is additive,
creates only TvSeries/TvEpisodes/TvTrackedSeries, and does not alter existing tables.
`sql/tvmaze_seed.sql` inserts missing settings only, preserving disabled entries.
The MyNewsFeed project receives copies of the migration, seed and this handoff.

<a id="tvmaze--operation"></a>

### Operation

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

<a id="tvmaze--dates-and-completeness"></a>

### Dates and completeness

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

<a id="tvmaze--caching-rate-limiting-and-failures"></a>

### Caching, rate limiting and failures

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

<a id="tvmaze--web-app-licensing-and-attribution"></a>

### Web app licensing and attribution

The API is licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
The app should credit **TVmaze**, link episode/series cards to their stored SourceUrl,
link the license and indicate processing/adaptations such as date-only normalization.
When distributing adapted data, comply with ShareAlike. This does not automatically
require licensing unrelated application code under CC BY-SA. Images may be hotlinked
according to the API documentation; their stable URLs can be cached. No paid enterprise
license or separate premium user API is used.

[Official TVmaze licensing statement](https://www.tvmaze.com/api#licensing).

<a id="tvmaze--validation"></a>

### Validation

Live identities and representative full-list fixtures cover all five shows, streaming
noon placeholders, timed NBC episodes, batch releases and both kinds of specials.
Mock HTTP tests cover Retry-After, transient retry and failure isolation. SQL staging
checks cover idempotent seeds with disabled entries, stable IDs/FirstSeenAt on reschedule,
repeated imports, atomic rejection of incomplete lists, last-success preservation
and another series succeeding after a failure.

