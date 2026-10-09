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
