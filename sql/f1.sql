-- F1 uses race classifications and two championships, independent of match scores.
IF OBJECT_ID('dbo.F1Source','U') IS NULL
 CREATE TABLE dbo.F1Source(
  ID INT NOT NULL PRIMARY KEY CHECK(ID=1),
  ProviderKey VARCHAR(30) NOT NULL DEFAULT 'jolpica-f1',
  Enabled BIT NOT NULL DEFAULT 0,
  SourceUrl NVARCHAR(2048) NOT NULL,
  LastAttemptAt DATETIME2 NULL, LastSuccessAt DATETIME2 NULL,
  LastError NVARCHAR(1000) NULL
 );
IF NOT EXISTS(SELECT 1 FROM dbo.F1Source WHERE ID=1)
 INSERT dbo.F1Source(ID,SourceUrl) VALUES(1,N'https://api.jolpi.ca/ergast/f1/');
IF OBJECT_ID('dbo.F1Snapshot','U') IS NULL
 CREATE TABLE dbo.F1Snapshot(
  SourceId INT NOT NULL PRIMARY KEY REFERENCES dbo.F1Source(ID),
  Season INT NOT NULL CHECK(Season BETWEEN 1950 AND 2200),
  StandingsRound INT NOT NULL CHECK(StandingsRound>=0),
  Payload NVARCHAR(MAX) NOT NULL CHECK(ISJSON(Payload)=1),
  FetchedAt DATETIME2 NOT NULL
 );
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT ON dbo.F1Source TO WebScraperIngest;
 GRANT UPDATE(LastAttemptAt,LastSuccessAt,LastError) ON dbo.F1Source TO WebScraperIngest;
 GRANT SELECT,INSERT,UPDATE ON dbo.F1Snapshot TO WebScraperIngest;
END;
