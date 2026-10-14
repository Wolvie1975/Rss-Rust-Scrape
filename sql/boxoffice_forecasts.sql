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
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT,INSERT,DELETE ON dbo.BoxOfficeForecasts TO WebScraperIngest;
 GRANT SELECT,UPDATE ON dbo.BoxOfficeForecastSources TO WebScraperIngest;
END;
