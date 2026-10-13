-- Results are independent of SportsEvents, whose past rows are removed by retention.
IF OBJECT_ID('dbo.SportsMatchResults','U') IS NULL
CREATE TABLE dbo.SportsMatchResults (
 ID BIGINT IDENTITY PRIMARY KEY,
 SourceId INT NOT NULL REFERENCES dbo.SportsEventsType(ID),
 ProviderKey NVARCHAR(40) NOT NULL,
 GameId INT NOT NULL,
 Season INT NOT NULL,
 MatchDate DATE NOT NULL,
 StartsAtUtc DATETIME2 NOT NULL,
 HomeTeamId INT NOT NULL, HomeTeamName NVARCHAR(200) NOT NULL,
 AwayTeamId INT NOT NULL, AwayTeamName NVARCHAR(200) NOT NULL,
 HomeScore INT NOT NULL CHECK(HomeScore>=0), AwayScore INT NOT NULL CHECK(AwayScore>=0),
 HomeLogoUrl NVARCHAR(2048) NULL, AwayLogoUrl NVARCHAR(2048) NULL,
 ScheduleStatus NVARCHAR(100) NOT NULL,
 SourceUrl NVARCHAR(2048) NOT NULL,
 FirstSeenAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 LastSeenAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 CONSTRAINT UQ_SportsMatchResults UNIQUE(ProviderKey,GameId),
 CONSTRAINT CK_SportsMatchResults_Teams CHECK(HomeTeamId<>AwayTeamId)
);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsMatchResults') AND name='IX_SportsMatchResults_Season_Date')
 CREATE INDEX IX_SportsMatchResults_Season_Date ON dbo.SportsMatchResults(SourceId,Season,MatchDate DESC);
