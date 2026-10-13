-- Additive schema; run only with migration credentials.
IF OBJECT_ID('dbo.SportsRankingSources','U') IS NULL
CREATE TABLE dbo.SportsRankingSources (
 ID INT IDENTITY PRIMARY KEY,
 ProviderKey NVARCHAR(40) NOT NULL DEFAULT N'espn-rankings',
 Sport NVARCHAR(100) NOT NULL,
 LeagueId NVARCHAR(40) NOT NULL,
 PollId NVARCHAR(40) NOT NULL,
 DisplayName NVARCHAR(200) NOT NULL,
 Url NVARCHAR(2048) NOT NULL,
 Enabled BIT NOT NULL DEFAULT 0,
 LastAttemptAt DATETIME2 NULL,LastSuccessAt DATETIME2 NULL,LastError NVARCHAR(2000) NULL,
 CONSTRAINT UQ_SportsRankingSources UNIQUE(ProviderKey,Sport,PollId)
);
IF OBJECT_ID('dbo.SportsRankingPolls','U') IS NULL
CREATE TABLE dbo.SportsRankingPolls (
 ID BIGINT IDENTITY PRIMARY KEY,
 SourceId INT NOT NULL REFERENCES dbo.SportsRankingSources(ID),
 PollId NVARCHAR(40) NOT NULL,PollName NVARCHAR(200) NOT NULL,
 Season INT NOT NULL,SeasonLabel NVARCHAR(80) NOT NULL,
 PublishedAtUtc DATETIME2 NOT NULL,ProviderUpdatedAtUtc DATETIME2 NULL,
 Period NVARCHAR(100) NULL,SeasonPhase NVARCHAR(100) NULL,
 SourceUrl NVARCHAR(2048) NOT NULL,
 IsCurrent BIT NOT NULL DEFAULT 1,
 FirstSeenAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),FetchedAt DATETIME2 NOT NULL,
 CONSTRAINT UQ_SportsRankingPolls UNIQUE(SourceId,PollId,Season,PublishedAtUtc)
);
IF OBJECT_ID('dbo.SportsRankingTeams','U') IS NULL
CREATE TABLE dbo.SportsRankingTeams (
 ID BIGINT IDENTITY PRIMARY KEY,
 PollRowId BIGINT NOT NULL REFERENCES dbo.SportsRankingPolls(ID),
 TeamId NVARCHAR(80) NOT NULL,TeamName NVARCHAR(200) NOT NULL,
 Rank INT NOT NULL CHECK(Rank BETWEEN 1 AND 25),
 PreviousRank INT NULL CHECK(PreviousRank BETWEEN 1 AND 25),
 PreviouslyUnranked BIT NOT NULL,
 Points FLOAT NULL,FirstPlaceVotes INT NULL,Record NVARCHAR(100) NULL,
 LogoUrl NVARCHAR(2048) NULL,TeamUrl NVARCHAR(2048) NULL,
 Big12SchoolId INT NULL,
 IsBig12 AS CAST(CASE WHEN Big12SchoolId IS NULL THEN 0 ELSE 1 END AS BIT) PERSISTED,
 IsCurrent BIT NOT NULL DEFAULT 1,
 FirstSeenAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 CONSTRAINT UQ_SportsRankingTeams UNIQUE(PollRowId,TeamId)
);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsRankingPolls') AND name='IX_SportsRankingPolls_Current')
 CREATE INDEX IX_SportsRankingPolls_Current ON dbo.SportsRankingPolls(SourceId,IsCurrent,PublishedAtUtc);
