-- Additive scoreboard storage. Explicit migration only; no existing results/schedule writes.
SET XACT_ABORT ON;
IF OBJECT_ID('dbo.SportsScoreboardSources','U') IS NULL
CREATE TABLE dbo.SportsScoreboardSources(
 ID INT IDENTITY PRIMARY KEY,
 SourceKind VARCHAR(12) NOT NULL CHECK(SourceKind IN('events','follow')),
 OriginId INT NOT NULL,
 Sport NVARCHAR(40) NOT NULL,League NVARCHAR(60) NOT NULL,
 Label NVARCHAR(200) NOT NULL,
 LastAttemptAt DATETIME2 NULL,LastSuccessAt DATETIME2 NULL,
 Availability VARCHAR(16) NOT NULL DEFAULT 'pending',
 CONSTRAINT UQ_ScoreboardSource UNIQUE(SourceKind,OriginId)
);
IF OBJECT_ID('dbo.SportsScoreboardGames','U') IS NULL
CREATE TABLE dbo.SportsScoreboardGames(
 ID BIGINT IDENTITY PRIMARY KEY,
 Provider VARCHAR(16) NOT NULL DEFAULT 'espn',
 Sport NVARCHAR(40) NOT NULL,League NVARCHAR(60) NOT NULL,EventId NVARCHAR(80) NOT NULL,
 ProviderDate DATE NOT NULL,ScoreboardDate DATE NOT NULL,OriginalScoreboardDate DATE NOT NULL,
 StartsAtUtc DATETIME2 NULL,OriginalStartsAtUtc DATETIME2 NULL,
 TimeTbd BIT NOT NULL,WasRescheduled BIT NOT NULL DEFAULT 0,
 HomeTeamId NVARCHAR(80) NULL,HomeName NVARCHAR(200) NOT NULL,HomeLogoUrl NVARCHAR(2048) NULL,
 AwayTeamId NVARCHAR(80) NULL,AwayName NVARCHAR(200) NOT NULL,AwayLogoUrl NVARCHAR(2048) NULL,
 HomeScore DECIMAL(12,3) NULL,AwayScore DECIMAL(12,3) NULL,
 Status VARCHAR(12) NOT NULL CHECK(Status IN('upcoming','live','final','postponed','cancelled','suspended','delayed','unknown')),
 Completed BIT NOT NULL,ProviderStatus NVARCHAR(100) NOT NULL,StatusText NVARCHAR(300) NULL,
 Period INT NULL,DisplayClock NVARCHAR(80) NULL,
 NeutralSite BIT NULL,HomeShootout INT NULL,AwayShootout INT NULL,WinnerSide VARCHAR(4) NULL,
 Venue NVARCHAR(300) NULL,Broadcasts NVARCHAR(1000) NULL,SourceUrl NVARCHAR(2048) NULL,
 LiveDetailsJson NVARCHAR(MAX) NULL CHECK(LiveDetailsJson IS NULL OR ISJSON(LiveDetailsJson)=1),
 ProviderUpdatedAtUtc DATETIME2 NULL,LastObservedAtUtc DATETIME2 NOT NULL,
 FirstSeenAtUtc DATETIME2 NOT NULL,FinalFirstObservedAtUtc DATETIME2 NULL,
 CONSTRAINT UQ_ScoreboardGame UNIQUE(Provider,Sport,League,EventId)
);
IF OBJECT_ID('dbo.SportsScoreboardEventSources','U') IS NULL
CREATE TABLE dbo.SportsScoreboardEventSources(
 GameId BIGINT NOT NULL REFERENCES dbo.SportsScoreboardGames(ID),
 SourceId INT NOT NULL REFERENCES dbo.SportsScoreboardSources(ID),
 CONSTRAINT PK_ScoreboardEventSources PRIMARY KEY(GameId,SourceId)
);
IF OBJECT_ID('dbo.SportsScoreboardRequests','U') IS NULL
CREATE TABLE dbo.SportsScoreboardRequests(
 Sport NVARCHAR(40) NOT NULL,League NVARCHAR(60) NOT NULL,ProviderDate DATE NOT NULL,
 LastAttemptAt DATETIME2 NULL,LastSuccessAt DATETIME2 NULL,NextAttemptAt DATETIME2 NOT NULL,
 Availability VARCHAR(16) NOT NULL,FailureCount INT NOT NULL DEFAULT 0,
 LastEventCount INT NULL,LastIssueCount INT NULL,
 CONSTRAINT PK_ScoreboardRequests PRIMARY KEY(Sport,League,ProviderDate)
);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsScoreboardGames') AND name='IX_ScoreboardDate')
 CREATE INDEX IX_ScoreboardDate ON dbo.SportsScoreboardGames(ScoreboardDate,StartsAtUtc) INCLUDE(Status,Sport,League,LastObservedAtUtc);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsScoreboardGames') AND name='IX_ScoreboardActive')
 CREATE INDEX IX_ScoreboardActive ON dbo.SportsScoreboardGames(Status,ProviderDate) INCLUDE(StartsAtUtc,LastObservedAtUtc,Sport,League);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsScoreboardEventSources') AND name='IX_ScoreboardSourceGames')
 CREATE INDEX IX_ScoreboardSourceGames ON dbo.SportsScoreboardEventSources(SourceId,GameId);
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT ON dbo.ResultFollows TO WebScraperIngest;
 GRANT SELECT ON dbo.ResultFollowTeams TO WebScraperIngest;
 GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.SportsScoreboardSources TO WebScraperIngest;
 GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.SportsScoreboardGames TO WebScraperIngest;
 GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.SportsScoreboardEventSources TO WebScraperIngest;
 GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.SportsScoreboardRequests TO WebScraperIngest;
END;
