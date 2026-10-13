IF OBJECT_ID('dbo.ScraperRuns','U') IS NULL
BEGIN
 CREATE TABLE dbo.ScraperRuns(
  ID INT IDENTITY(1,1) CONSTRAINT PK_ScraperRuns PRIMARY KEY,
  StartedAt DATETIME2 NOT NULL CONSTRAINT DF_ScraperRuns_Start DEFAULT SYSUTCDATETIME(),
  HeartbeatAt DATETIME2 NOT NULL CONSTRAINT DF_ScraperRuns_Heartbeat DEFAULT SYSUTCDATETIME(),
  FinishedAt DATETIME2 NULL,
  Status VARCHAR(20) NOT NULL,
  ErrorSummary NVARCHAR(1000) NULL,
  CONSTRAINT CK_ScraperRuns_Status CHECK(Status IN('running','succeeded','partial','failed'))
 );
 CREATE INDEX IX_ScraperRuns_Started ON dbo.ScraperRuns(StartedAt DESC);
END;
