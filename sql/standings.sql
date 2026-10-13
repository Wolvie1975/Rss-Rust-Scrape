-- Additive migration; deployment credentials only. No schedule/result changes.
IF OBJECT_ID('dbo.SportsStandingsSources','U') IS NULL
CREATE TABLE dbo.SportsStandingsSources (
 ID INT IDENTITY PRIMARY KEY,
 ProviderKey NVARCHAR(40) NOT NULL,
 Sport NVARCHAR(100) NOT NULL,
 DisplayName NVARCHAR(200) NOT NULL,
 Url NVARCHAR(2048) NOT NULL,
 Enabled BIT NOT NULL DEFAULT 0,
 MinimumTeams INT NOT NULL CHECK(MinimumTeams BETWEEN 1 AND 256),
 LastAttemptAt DATETIME2 NULL,
 LastSuccessAt DATETIME2 NULL,
 LastError NVARCHAR(2000) NULL,
 LatestSeason NVARCHAR(80) NULL,
 LastTeamCount INT NULL,
 CONSTRAINT UQ_SportsStandingsSources UNIQUE(ProviderKey,Sport)
);
IF OBJECT_ID('dbo.SportsStandings','U') IS NULL
CREATE TABLE dbo.SportsStandings (
 ID BIGINT IDENTITY PRIMARY KEY,
 SourceId INT NOT NULL REFERENCES dbo.SportsStandingsSources(ID),
 Season NVARCHAR(80) NOT NULL,
 SeasonType NVARCHAR(80) NOT NULL,
 Title NVARCHAR(300) NOT NULL,
 SourceUrl NVARCHAR(2048) NOT NULL,
 GroupId NVARCHAR(80) NOT NULL,
 GroupName NVARCHAR(200) NOT NULL,
 TeamId NVARCHAR(80) NOT NULL,
 TeamName NVARCHAR(200) NOT NULL,
 LogoUrl NVARCHAR(2048) NULL,
 TeamUrl NVARCHAR(2048) NULL,
 Position INT NOT NULL,
 Rank INT NULL,
 StatsJson NVARCHAR(MAX) NOT NULL CHECK(ISJSON(StatsJson)=1),
 Note NVARCHAR(1000) NULL,
 IsCurrent BIT NOT NULL DEFAULT 1,
 FirstSeenAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 FetchedAt DATETIME2 NOT NULL,
 CONSTRAINT UQ_SportsStandings UNIQUE(SourceId,Season,SeasonType,GroupId,TeamId)
);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsStandings') AND name='IX_SportsStandings_Current')
 CREATE INDEX IX_SportsStandings_Current ON dbo.SportsStandings(SourceId,IsCurrent,GroupId,Position);
