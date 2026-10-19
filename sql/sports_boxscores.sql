-- Additive setup only. Runtime collectors perform no DDL and never alter result rows.
SET XACT_ABORT ON;
IF OBJECT_ID('dbo.SportsBoxScoreCache','U') IS NULL
 CREATE TABLE dbo.SportsBoxScoreCache(
  Sport NVARCHAR(40) NOT NULL,
  League NVARCHAR(60) NOT NULL,
  EventId NVARCHAR(20) NOT NULL,
  Availability VARCHAR(12) NOT NULL CHECK(Availability IN('available','unavailable','error')),
  Payload NVARCHAR(MAX) NULL CHECK(Payload IS NULL OR ISJSON(Payload)=1),
  SummaryUrl NVARCHAR(2048) NULL,
  BoxScoreUrl NVARCHAR(2048) NULL,
  FetchedAt DATETIME2 NULL,
  LastAttemptAt DATETIME2 NOT NULL,
  NextAttemptAt DATETIME2 NOT NULL,
  LastError NVARCHAR(1000) NULL,
  CONSTRAINT PK_SportsBoxScoreCache PRIMARY KEY(Sport,League,EventId),
  CONSTRAINT CK_SportsBoxScoreCache_Payload CHECK(
   (Availability='available' AND Payload IS NOT NULL AND FetchedAt IS NOT NULL) OR
   (Availability<>'available' AND Payload IS NULL))
 );
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT ON dbo.ResultGames TO WebScraperIngest;
 GRANT SELECT,INSERT,UPDATE ON dbo.SportsBoxScoreCache TO WebScraperIngest;
END;
