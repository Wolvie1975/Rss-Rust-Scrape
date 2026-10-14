IF OBJECT_ID('dbo.CinemaPosterCache','U') IS NULL
 CREATE TABLE dbo.CinemaPosterCache(
  IdentityKey NVARCHAR(400) NOT NULL PRIMARY KEY,MovieTitle NVARCHAR(300) NOT NULL,
  OriginalYear SMALLINT NULL,PosterUrl NVARCHAR(2048) NULL,SourceUrl NVARCHAR(2048) NULL,
  LookupStatus VARCHAR(20) NOT NULL,LastError NVARCHAR(1000) NULL,
  CheckedAtUtc DATETIME2 NOT NULL,RetryAfterUtc DATETIME2 NOT NULL,
  LastUsedAtUtc DATETIME2 NOT NULL
 );
IF OBJECT_ID('dbo.BoxOfficeWeekend','U') IS NOT NULL AND COL_LENGTH('dbo.BoxOfficeWeekend','PosterUrl') IS NULL
 ALTER TABLE dbo.BoxOfficeWeekend ADD PosterUrl NVARCHAR(2048) NULL,PosterLookupStatus VARCHAR(20) NULL;
IF OBJECT_ID('dbo.BoxOfficeForecasts','U') IS NOT NULL AND COL_LENGTH('dbo.BoxOfficeForecasts','PosterUrl') IS NULL
 ALTER TABLE dbo.BoxOfficeForecasts ADD PosterUrl NVARCHAR(2048) NULL,PosterLookupStatus VARCHAR(20) NULL;
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.CinemaPosterCache TO WebScraperIngest;
 IF OBJECT_ID('dbo.BoxOfficeWeekend','U') IS NOT NULL
  GRANT UPDATE(PosterUrl,PosterLookupStatus) ON dbo.BoxOfficeWeekend TO WebScraperIngest;
 IF OBJECT_ID('dbo.BoxOfficeForecasts','U') IS NOT NULL
  GRANT UPDATE(PosterUrl,PosterLookupStatus) ON dbo.BoxOfficeForecasts TO WebScraperIngest;
END;
