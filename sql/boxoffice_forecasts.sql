IF OBJECT_ID('dbo.BoxOfficeForecastSources','U') IS NULL
 CREATE TABLE dbo.BoxOfficeForecastSources(
  Kind VARCHAR(30) NOT NULL PRIMARY KEY,
  LatestPublishedAt DATETIME2 NULL, LastAttemptAt DATETIME2 NULL,
  LastSuccessAt DATETIME2 NULL, LastError NVARCHAR(1000) NULL,
  LastMovieCount INT NULL
 );
INSERT dbo.BoxOfficeForecastSources(Kind)
 SELECT Kind FROM (VALUES('weekend-preview'),('long-range')) s(Kind)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.BoxOfficeForecastSources t WHERE t.Kind=s.Kind);
IF OBJECT_ID('dbo.BoxOfficeForecasts','U') IS NULL
 CREATE TABLE dbo.BoxOfficeForecasts(
  Kind VARCHAR(30) NOT NULL REFERENCES dbo.BoxOfficeForecastSources(Kind),
  TargetDate DATE NOT NULL, PeriodEnd DATE NOT NULL,
  MovieTitle NVARCHAR(300) NOT NULL, Distributor NVARCHAR(300) NOT NULL,
  PredictedRank INT NULL, ForecastLow BIGINT NOT NULL, ForecastHigh BIGINT NOT NULL,
  ForecastAsOf DATE NOT NULL, PublishedAtUtc DATETIME2 NOT NULL,
  UpdatedAtUtc DATETIME2 NOT NULL, SourceUrl NVARCHAR(2048) NOT NULL,
  FetchedAtUtc DATETIME2 NOT NULL,
  CONSTRAINT PK_BoxOfficeForecasts PRIMARY KEY(Kind,TargetDate,MovieTitle),
  CONSTRAINT CK_BoxOfficeForecasts_Range CHECK(ForecastLow>=0 AND ForecastHigh>=ForecastLow),
  CONSTRAINT CK_BoxOfficeForecasts_Period CHECK(PeriodEnd>=TargetDate)
 );
-- Holiday openings (e.g. Thanksgiving) also carry a longer 4-7 day frame beside the 3-day weekend.
IF COL_LENGTH('dbo.BoxOfficeForecasts','ExtendedForecastDays') IS NULL
 ALTER TABLE dbo.BoxOfficeForecasts ADD
  ExtendedForecastDays TINYINT NULL, ExtendedForecastLow BIGINT NULL, ExtendedForecastHigh BIGINT NULL,
  CONSTRAINT CK_BoxOfficeForecasts_Extended CHECK(
   (ExtendedForecastDays IS NULL AND ExtendedForecastLow IS NULL AND ExtendedForecastHigh IS NULL)
   OR (ExtendedForecastDays IS NOT NULL AND ExtendedForecastLow IS NOT NULL AND ExtendedForecastHigh IS NOT NULL
       AND ExtendedForecastDays BETWEEN 4 AND 7 AND ExtendedForecastLow>=0 AND ExtendedForecastHigh>=ExtendedForecastLow));
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT,INSERT,DELETE ON dbo.BoxOfficeForecasts TO WebScraperIngest;
 GRANT SELECT,UPDATE ON dbo.BoxOfficeForecastSources TO WebScraperIngest;
END;
