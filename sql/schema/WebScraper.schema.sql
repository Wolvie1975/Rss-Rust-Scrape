-- WebScraper schema-only snapshot. Exported 2026-10-14 UTC.
-- Contains no table rows, security principals, grants, or physical database paths.
-- Run in an EMPTY database. This is a creation snapshot, not a migration.
-- Source database collation: SQL_Latin1_General_CP1_CI_AS; compatibility level: 170.

SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsEventSources](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[SportsEventId] [int] NULL,
	[SportsEventsTypeId] [int] NOT NULL,
	[ProviderGameId] [int] NOT NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[SchoolId] [int] NULL,
	[SchoolName] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[OpponentSchoolId] [int] NULL,
	[Payload] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Issue] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_SportsEventSources] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsEventSources_Game] UNIQUE NONCLUSTERED 
(
	[SportsEventsTypeId] ASC,
	[ProviderGameId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsEvents](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[UrlHash]  AS (CONVERT([binary](32),hashbytes('SHA2_256',[Url]))) PERSISTED,
	[GameId] [int] NULL,
	[Title] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Sport] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Opponent] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[IsAway] [bit] NULL,
	[Location] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[EventDate] [date] NOT NULL,
	[StartsAtUtc] [datetime2](7) NULL,
	[EndsAtUtc] [datetime2](7) NULL,
	[TimeTbd] [bit] NOT NULL,
	[Tv] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[StreamUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LiveStatsUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[TeamLogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[OpponentLogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
	[SportsEventsTypeId] [int] NULL,
	[CanonicalKey] [varchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ProviderKey] [varchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SchoolId] [int] NULL,
	[SchoolName] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[OpponentSchoolId] [int] NULL,
	[ProviderSportId] [int] NULL,
	[IsNeutral] [bit] NULL,
	[CalendarTimeZone] [nvarchar](30) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Radio] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[AudioUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ScheduleStatus] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK__SportsEv__3214EC27DE5D3483] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsEvents_UrlHash] UNIQUE NONCLUSTERED 
(
	[UrlHash] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
-- One public event per SportsEvents row. School calendars join the source perspectives.
CREATE   VIEW dbo.SportsEventSchoolCalendar AS
SELECT e.ID AS SportsEventId, s.SportsEventsTypeId, s.ProviderGameId, s.SchoolId, s.SchoolName,
 s.OpponentSchoolId, s.SourceUrl, e.CanonicalKey, e.Sport, e.EventDate,
 e.StartsAtUtc, e.EndsAtUtc, e.TimeTbd,
 JSON_VALUE(s.Payload,'$.event.title') AS Title,
 JSON_VALUE(s.Payload,'$.event.opponent') AS Opponent,
 CASE WHEN e.IsNeutral=1 THEN NULL ELSE
  CASE JSON_VALUE(s.Payload,'$.event.is_away') WHEN 'true' THEN CAST(1 AS BIT) WHEN 'false' THEN CAST(0 AS BIT) ELSE NULL END END AS IsAway,
 COALESCE(e.IsNeutral,CASE JSON_VALUE(s.Payload,'$.is_neutral') WHEN 'true' THEN CAST(1 AS BIT) WHEN 'false' THEN CAST(0 AS BIT) ELSE NULL END) AS IsNeutral,
 JSON_VALUE(s.Payload,'$.calendar_timezone') AS CalendarTimeZone,
 COALESCE(JSON_VALUE(s.Payload,'$.event.location'),e.Location) AS Location,
 JSON_VALUE(s.Payload,'$.event.team_logo_url') AS TeamLogoUrl,
 JSON_VALUE(s.Payload,'$.event.opponent_logo_url') AS OpponentLogoUrl,
 COALESCE(JSON_VALUE(s.Payload,'$.event.tv'),e.Tv) AS Tv,
 COALESCE(JSON_VALUE(s.Payload,'$.event.stream_url'),e.StreamUrl) AS StreamUrl,
 COALESCE(JSON_VALUE(s.Payload,'$.event.live_stats_url'),e.LiveStatsUrl) AS LiveStatsUrl,
 COALESCE(JSON_VALUE(s.Payload,'$.radio'),e.Radio) AS Radio,
 COALESCE(JSON_VALUE(s.Payload,'$.audio_url'),e.AudioUrl) AS AudioUrl,
 s.Issue, JSON_VALUE(s.Payload,'$.warning') AS Warning, s.LastSeenAt
FROM dbo.SportsEventSources s JOIN dbo.SportsEvents e ON e.ID=s.SportsEventId;
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[ResultFollows](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Kind] [nvarchar](10) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Sport] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[League] [nvarchar](60) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[TeamId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Name] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[LogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Enabled] [bit] NOT NULL,
	[CreatedAt] [datetime2](7) NOT NULL,
	[LastSyncedAt] [datetime2](7) NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[GroupId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK_ResultFollows] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_ResultFollows_Team] UNIQUE NONCLUSTERED 
(
	[Sport] ASC,
	[League] ASC,
	[TeamId] ASC,
	[GroupId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SourceCategories](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Category_Name] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[DateAdded] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_SourceCategories] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SourceCategories_Category_Name] UNIQUE NONCLUSTERED 
(
	[Category_Name] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[ScraperRuns](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[StartedAt] [datetime2](7) NOT NULL,
	[HeartbeatAt] [datetime2](7) NOT NULL,
	[FinishedAt] [datetime2](7) NULL,
	[Status] [varchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ErrorSummary] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK_ScraperRuns] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[ResultGames](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Sport] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[League] [nvarchar](60) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[EventId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[StartsAtUtc] [datetime2](7) NOT NULL,
	[State] [nvarchar](10) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Completed] [bit] NOT NULL,
	[StatusText] [nvarchar](60) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[HomeTeamId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[HomeName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[HomeLogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[HomeScore] [int] NULL,
	[HomeShootout] [int] NULL,
	[AwayTeamId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[AwayName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[AwayLogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[AwayScore] [int] NULL,
	[AwayShootout] [int] NULL,
	[WinnerSide] [nvarchar](4) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Venue] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_ResultGames] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_ResultGames_Event] UNIQUE NONCLUSTERED 
(
	[Sport] ASC,
	[League] ASC,
	[EventId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[Movies](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Title] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[OriginalYear] [smallint] NULL,
	[ImdbId] [varchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[TmdbId] [int] NULL,
	[Description] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[PosterUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Certification] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[RuntimeMinutes] [smallint] NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
	[PosterCheckedAt] [datetime2](7) NULL,
	[PosterRetryAfter] [datetime2](7) NULL,
	[PosterLookupStatus] [varchar](30) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[PosterLookupError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[PosterSourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK_Movies] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[MovieReleaseSources](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[SourceKey] [varchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Name] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Enabled] [bit] NOT NULL,
	[LastScrapedAt] [datetime2](7) NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[CreatedAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_MovieReleaseSources] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_MovieReleaseSources_Key] UNIQUE NONCLUSTERED 
(
	[SourceKey] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[MovieCatalogs](
	[Id] [int] NOT NULL,
	[UpdatedAt] [datetime2](7) NOT NULL,
	[ReleasesJson] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
 CONSTRAINT [PK_MovieCatalogs] PRIMARY KEY CLUSTERED 
(
	[Id] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[CinemaPosterCache](
	[IdentityKey] [nvarchar](400) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[MovieTitle] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[OriginalYear] [smallint] NULL,
	[PosterUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LookupStatus] [varchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[CheckedAtUtc] [datetime2](7) NOT NULL,
	[RetryAfterUtc] [datetime2](7) NOT NULL,
	[LastUsedAtUtc] [datetime2](7) NOT NULL,
 CONSTRAINT [PK__CinemaPo__796424B8B92DFEA6] PRIMARY KEY CLUSTERED 
(
	[IdentityKey] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[BoxOfficeWeekend](
	[Rank] [int] NOT NULL,
	[WeekendStart] [date] NOT NULL,
	[MovieTitle] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[WeekendGross] [bigint] NOT NULL,
	[TotalDomesticGross] [bigint] NOT NULL,
	[WeeklyChangePercent] [float] NULL,
	[Theaters] [int] NOT NULL,
	[MovieUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[FetchedAt] [datetime2](7) NOT NULL,
	[PosterUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[PosterLookupStatus] [varchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK__BoxOffic__DF85EC56FB531F7B] PRIMARY KEY CLUSTERED 
(
	[Rank] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[BoxOfficeForecastSources](
	[Kind] [varchar](30) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[LatestPublishedAt] [datetime2](7) NULL,
	[LastAttemptAt] [datetime2](7) NULL,
	[LastSuccessAt] [datetime2](7) NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LastMovieCount] [int] NULL,
 CONSTRAINT [PK__BoxOffic__77F0A2584D2AD4A7] PRIMARY KEY CLUSTERED 
(
	[Kind] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsStandingsSources](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[ProviderKey] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Sport] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[DisplayName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Enabled] [bit] NOT NULL,
	[MinimumTeams] [int] NOT NULL,
	[LastAttemptAt] [datetime2](7) NULL,
	[LastSuccessAt] [datetime2](7) NULL,
	[LastError] [nvarchar](2000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LatestSeason] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LastTeamCount] [int] NULL,
 CONSTRAINT [PK__SportsSt__3214EC276B60AEC8] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsStandingsSources] UNIQUE NONCLUSTERED 
(
	[ProviderKey] ASC,
	[Sport] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsRankingSources](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[ProviderKey] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Sport] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[LeagueId] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[PollId] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[DisplayName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Enabled] [bit] NOT NULL,
	[LastAttemptAt] [datetime2](7) NULL,
	[LastSuccessAt] [datetime2](7) NULL,
	[LastError] [nvarchar](2000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK__SportsRa__3214EC27FC114170] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsRankingSources] UNIQUE NONCLUSTERED 
(
	[ProviderKey] ASC,
	[Sport] ASC,
	[PollId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsEventsType](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[RssUrl] [nvarchar](450) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[EventsTypeName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[DateAdded] [datetime2](7) NOT NULL,
	[SchoolName] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ProviderKey] [varchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ProviderSchoolId] [int] NULL,
	[Enabled] [bit] NOT NULL,
	[LastAttemptAt] [datetime2](7) NULL,
	[LastSuccessAt] [datetime2](7) NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LastEventCount] [int] NULL,
	[LastIssueCount] [int] NULL,
 CONSTRAINT [PK_SportsEventsType] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsEventsType_RssUrl] UNIQUE NONCLUSTERED 
(
	[RssUrl] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[TvSeries](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[TvmazeShowId] [int] NOT NULL,
	[Title] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[NetworkId] [int] NULL,
	[NetworkName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[NetworkCountryCode] [char](2) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[NetworkTimeZone] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[WebChannelId] [int] NULL,
	[WebChannelName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[WebChannelCountryCode] [char](2) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[WebChannelTimeZone] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Status] [nvarchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[PremiereDate] [date] NULL,
	[PosterUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ImdbId] [varchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[TheTvdbId] [int] NULL,
	[TvRageId] [int] NULL,
	[RawMetadata] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_TvSeries] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_TvSeries_Show] UNIQUE NONCLUSTERED 
(
	[TvmazeShowId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[YoutubeVideoFeed](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[ChannelId] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ChannelName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[DateAdded] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_YoutubeVideoFeed] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_YoutubeVideoFeed_ChannelId] UNIQUE NONCLUSTERED 
(
	[ChannelId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[YouTubeVideos](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[VideoId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ChannelId] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ChannelName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Title] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[PublishedAt] [datetime2](7) NOT NULL,
	[UpdatedAt] [datetime2](7) NULL,
	[ThumbnailUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Description] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ViewCount] [bigint] NULL,
	[RatingCount] [int] NULL,
	[RatingAverage] [decimal](3, 2) NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
	[YoutubeVideoFeedId] [int] NULL,
 CONSTRAINT [PK__YouTubeV__3214EC273DDF15C7] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_YouTubeVideos_VideoId] UNIQUE NONCLUSTERED 
(
	[VideoId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[TvTrackedSeries](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[TvmazeShowId] [int] NOT NULL,
	[SeriesId] [int] NULL,
	[ExpectedTitle] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Enabled] [bit] NOT NULL,
	[LastAttemptAt] [datetime2](7) NULL,
	[LastSuccessAt] [datetime2](7) NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[EpisodeCount] [int] NULL,
	[LastIssueCount] [int] NULL,
	[CacheExpiresAt] [datetime2](7) NULL,
	[RetryAfter] [datetime2](7) NULL,
	[LeaseUntil] [datetime2](7) NULL,
	[LeaseToken] [uniqueidentifier] NULL,
	[CachedShow] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[CachedEpisodes] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[CreatedAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_TvTrackedSeries] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_TvTrackedSeries_Show] UNIQUE NONCLUSTERED 
(
	[TvmazeShowId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[TvEpisodes](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[TvmazeEpisodeId] [int] NOT NULL,
	[SeriesId] [int] NOT NULL,
	[SeasonNumber] [int] NULL,
	[EpisodeNumber] [int] NULL,
	[Title] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[EpisodeType] [varchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[IsSpecial] [bit] NOT NULL,
	[Airdate] [date] NULL,
	[Airtime] [time](0) NULL,
	[StartsAtUtc] [datetime2](7) NULL,
	[IsDateOnly] [bit] NOT NULL,
	[RuntimeMinutes] [int] NULL,
	[ImageUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[RawAirdate] [nvarchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[RawAirtime] [nvarchar](50) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[RawAirstamp] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[RawMetadata] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_TvEpisodes] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_TvEpisodes_Episode] UNIQUE NONCLUSTERED 
(
	[TvmazeEpisodeId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[Sources](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[UrlHash]  AS (CONVERT([binary](32),hashbytes('SHA2_256',[Url]))) PERSISTED,
	[Label] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Enabled] [bit] NOT NULL,
	[CreatedAt] [datetime2](7) NOT NULL,
	[LastScrapedAt] [datetime2](7) NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SourceCategoryId] [int] NULL,
 CONSTRAINT [PK__Sources__3214EC273E1779C8] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_Sources_UrlHash] UNIQUE NONCLUSTERED 
(
	[UrlHash] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsRankingPolls](
	[ID] [bigint] IDENTITY(1,1) NOT NULL,
	[SourceId] [int] NOT NULL,
	[PollId] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[PollName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Season] [int] NOT NULL,
	[SeasonLabel] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[PublishedAtUtc] [datetime2](7) NOT NULL,
	[ProviderUpdatedAtUtc] [datetime2](7) NULL,
	[Period] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SeasonPhase] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[IsCurrent] [bit] NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[FetchedAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK__SportsRa__3214EC273F8954B1] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsRankingPolls] UNIQUE NONCLUSTERED 
(
	[SourceId] ASC,
	[PollId] ASC,
	[Season] ASC,
	[PublishedAtUtc] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsMatchResults](
	[ID] [bigint] IDENTITY(1,1) NOT NULL,
	[SourceId] [int] NOT NULL,
	[ProviderKey] [nvarchar](40) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[GameId] [int] NOT NULL,
	[Season] [int] NOT NULL,
	[MatchDate] [date] NOT NULL,
	[StartsAtUtc] [datetime2](7) NOT NULL,
	[HomeTeamId] [int] NOT NULL,
	[HomeTeamName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[AwayTeamId] [int] NOT NULL,
	[AwayTeamName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[HomeScore] [int] NOT NULL,
	[AwayScore] [int] NOT NULL,
	[HomeLogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[AwayLogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ScheduleStatus] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
	[HomeShootoutScore] [int] NULL,
	[AwayShootoutScore] [int] NULL,
 CONSTRAINT [PK__SportsMa__3214EC27749A5BC9] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsMatchResults] UNIQUE NONCLUSTERED 
(
	[ProviderKey] ASC,
	[GameId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsStandings](
	[ID] [bigint] IDENTITY(1,1) NOT NULL,
	[SourceId] [int] NOT NULL,
	[Season] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[SeasonType] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Title] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[GroupId] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[GroupName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[TeamId] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[TeamName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[LogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[TeamUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Position] [int] NOT NULL,
	[Rank] [int] NULL,
	[StatsJson] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Note] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[IsCurrent] [bit] NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[FetchedAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK__SportsSt__3214EC27C2DA339F] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsStandings] UNIQUE NONCLUSTERED 
(
	[SourceId] ASC,
	[Season] ASC,
	[SeasonType] ASC,
	[GroupId] ASC,
	[TeamId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[BoxOfficeForecasts](
	[Kind] [varchar](30) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[TargetDate] [date] NOT NULL,
	[PeriodEnd] [date] NOT NULL,
	[MovieTitle] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Distributor] [nvarchar](300) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[PredictedRank] [int] NULL,
	[ForecastLow] [bigint] NOT NULL,
	[ForecastHigh] [bigint] NOT NULL,
	[ForecastAsOf] [date] NOT NULL,
	[PublishedAtUtc] [datetime2](7) NOT NULL,
	[UpdatedAtUtc] [datetime2](7) NOT NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[FetchedAtUtc] [datetime2](7) NOT NULL,
	[PosterUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[PosterLookupStatus] [varchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK_BoxOfficeForecasts] PRIMARY KEY CLUSTERED 
(
	[Kind] ASC,
	[TargetDate] ASC,
	[MovieTitle] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[ResultFollowTeams](
	[FollowId] [int] NOT NULL,
	[TeamId] [nvarchar](20) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
 CONSTRAINT [PK_ResultFollowTeams] PRIMARY KEY CLUSTERED 
(
	[FollowId] ASC,
	[TeamId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[MovieSourceLinks](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[MovieId] [int] NOT NULL,
	[MovieReleaseSourceId] [int] NOT NULL,
	[ExternalMovieId] [nvarchar](200) COLLATE Latin1_General_100_BIN2 NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_MovieSourceLinks] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_MovieSourceLinks_External] UNIQUE NONCLUSTERED 
(
	[MovieReleaseSourceId] ASC,
	[ExternalMovieId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[Pages](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[Url] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[UrlHash]  AS (CONVERT([binary](32),hashbytes('SHA2_256',[Url]))) PERSISTED,
	[Title] [nvarchar](500) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Description] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Published] [datetime2](7) NULL,
	[ScrapedAt] [datetime2](7) NOT NULL,
	[SourceId] [int] NOT NULL,
	[ImageUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK__Pages__3214EC277F3F483F] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_Pages_UrlHash] UNIQUE NONCLUSTERED 
(
	[UrlHash] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[MovieReleases](
	[ID] [int] IDENTITY(1,1) NOT NULL,
	[MovieSourceLinkId] [int] NOT NULL,
	[ReleaseKey] [nvarchar](200) COLLATE Latin1_General_100_BIN2 NOT NULL,
	[CountryCode] [char](2) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ReleaseType] [varchar](30) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ReleaseDate] [date] NULL,
	[DateStatus] [varchar](12) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[ReleaseStatus] [varchar](12) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Platform] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Format] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[ReleaseScope] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Distributor] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Edition] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[IsReRelease] [bit] NOT NULL,
	[SourceUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Notes] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
	[LastSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK_MovieReleases] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_MovieReleases_Key] UNIQUE NONCLUSTERED 
(
	[MovieSourceLinkId] ASC,
	[ReleaseKey] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[MoviePosterCache](
	[SourceLinkId] [int] NOT NULL,
	[TargetIdentity] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Metadata] [nvarchar](max) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[PosterUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[CheckedAt] [datetime2](7) NOT NULL,
	[RetryAfter] [datetime2](7) NOT NULL,
	[LastError] [nvarchar](1000) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
 CONSTRAINT [PK_MoviePosterCache] PRIMARY KEY CLUSTERED 
(
	[SourceLinkId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY] TEXTIMAGE_ON [PRIMARY]
GO
SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO
CREATE TABLE [dbo].[SportsRankingTeams](
	[ID] [bigint] IDENTITY(1,1) NOT NULL,
	[PollRowId] [bigint] NOT NULL,
	[TeamId] [nvarchar](80) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[TeamName] [nvarchar](200) COLLATE SQL_Latin1_General_CP1_CI_AS NOT NULL,
	[Rank] [int] NOT NULL,
	[PreviousRank] [int] NULL,
	[PreviouslyUnranked] [bit] NOT NULL,
	[Points] [float] NULL,
	[FirstPlaceVotes] [int] NULL,
	[Record] [nvarchar](100) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[LogoUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[TeamUrl] [nvarchar](2048) COLLATE SQL_Latin1_General_CP1_CI_AS NULL,
	[Big12SchoolId] [int] NULL,
	[IsBig12]  AS (CONVERT([bit],case when [Big12SchoolId] IS NULL then (0) else (1) end)) PERSISTED,
	[IsCurrent] [bit] NOT NULL,
	[FirstSeenAt] [datetime2](7) NOT NULL,
 CONSTRAINT [PK__SportsRa__3214EC275FF4F151] PRIMARY KEY CLUSTERED 
(
	[ID] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY],
 CONSTRAINT [UQ_SportsRankingTeams] UNIQUE NONCLUSTERED 
(
	[PollRowId] ASC,
	[TeamId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_ScraperRuns_Started] ON [dbo].[ScraperRuns]
(
	[StartedAt] DESC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE NONCLUSTERED INDEX [IX_ResultGames_AwayTeam] ON [dbo].[ResultGames]
(
	[Sport] ASC,
	[League] ASC,
	[AwayTeamId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE NONCLUSTERED INDEX [IX_ResultGames_HomeTeam] ON [dbo].[ResultGames]
(
	[Sport] ASC,
	[League] ASC,
	[HomeTeamId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_ResultGames_StartsAtUtc] ON [dbo].[ResultGames]
(
	[StartsAtUtc] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE UNIQUE NONCLUSTERED INDEX [UX_Movies_ImdbId] ON [dbo].[Movies]
(
	[ImdbId] ASC
)
WHERE ([ImdbId] IS NOT NULL)
WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, IGNORE_DUP_KEY = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE UNIQUE NONCLUSTERED INDEX [UX_Movies_TmdbId] ON [dbo].[Movies]
(
	[TmdbId] ASC
)
WHERE ([TmdbId] IS NOT NULL)
WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, IGNORE_DUP_KEY = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE UNIQUE NONCLUSTERED INDEX [UX_SportsEventsType_ProviderSchool] ON [dbo].[SportsEventsType]
(
	[ProviderKey] ASC,
	[ProviderSchoolId] ASC
)
WHERE ([ProviderKey] IS NOT NULL AND [ProviderSchoolId] IS NOT NULL)
WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, IGNORE_DUP_KEY = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE NONCLUSTERED INDEX [IX_YouTubeVideos_Channel_PublishedAt] ON [dbo].[YouTubeVideos]
(
	[ChannelId] ASC,
	[PublishedAt] DESC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_YouTubeVideos_YoutubeVideoFeedId] ON [dbo].[YouTubeVideos]
(
	[YoutubeVideoFeedId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE UNIQUE NONCLUSTERED INDEX [UX_TvTrackedSeries_Series] ON [dbo].[TvTrackedSeries]
(
	[SeriesId] ASC
)
WHERE ([SeriesId] IS NOT NULL)
WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, IGNORE_DUP_KEY = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_TvEpisodes_Schedule] ON [dbo].[TvEpisodes]
(
	[Airdate] ASC,
	[SeriesId] ASC
)
INCLUDE([SeasonNumber],[EpisodeNumber],[StartsAtUtc],[IsDateOnly],[IsSpecial]) WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_SportsEvents_EventDate] ON [dbo].[SportsEvents]
(
	[EventDate] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE NONCLUSTERED INDEX [IX_SportsEvents_ProviderGame] ON [dbo].[SportsEvents]
(
	[ProviderKey] ASC,
	[GameId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_SportsEvents_SportsEventsTypeId] ON [dbo].[SportsEvents]
(
	[SportsEventsTypeId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE UNIQUE NONCLUSTERED INDEX [UX_SportsEvents_CanonicalKey] ON [dbo].[SportsEvents]
(
	[CanonicalKey] ASC
)
WHERE ([CanonicalKey] IS NOT NULL)
WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, IGNORE_DUP_KEY = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_Sources_SourceCategoryId] ON [dbo].[Sources]
(
	[SourceCategoryId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_SportsRankingPolls_Current] ON [dbo].[SportsRankingPolls]
(
	[SourceId] ASC,
	[IsCurrent] ASC,
	[PublishedAtUtc] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_SportsMatchResults_Season_Date] ON [dbo].[SportsMatchResults]
(
	[SourceId] ASC,
	[Season] ASC,
	[MatchDate] DESC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE NONCLUSTERED INDEX [IX_SportsStandings_Current] ON [dbo].[SportsStandings]
(
	[SourceId] ASC,
	[IsCurrent] ASC,
	[GroupId] ASC,
	[Position] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_MovieSourceLinks_MovieId] ON [dbo].[MovieSourceLinks]
(
	[MovieId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_Pages_SourceId] ON [dbo].[Pages]
(
	[SourceId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
SET ANSI_PADDING ON
GO
CREATE NONCLUSTERED INDEX [IX_MovieReleases_Weekly] ON [dbo].[MovieReleases]
(
	[ReleaseDate] ASC,
	[ReleaseType] ASC
)
INCLUDE([MovieSourceLinkId],[CountryCode],[DateStatus],[ReleaseStatus],[Platform],[Format]) WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_SportsEventSources_Event] ON [dbo].[SportsEventSources]
(
	[SportsEventId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
CREATE NONCLUSTERED INDEX [IX_SportsEventSources_Game] ON [dbo].[SportsEventSources]
(
	[ProviderGameId] ASC,
	[SportsEventId] ASC
)WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON, OPTIMIZE_FOR_SEQUENTIAL_KEY = OFF) ON [PRIMARY]
GO
ALTER TABLE [dbo].[ResultFollows] ADD  CONSTRAINT [DF_ResultFollows_Enabled]  DEFAULT ((1)) FOR [Enabled]
GO
ALTER TABLE [dbo].[ResultFollows] ADD  CONSTRAINT [DF_ResultFollows_CreatedAt]  DEFAULT (sysutcdatetime()) FOR [CreatedAt]
GO
ALTER TABLE [dbo].[SourceCategories] ADD  CONSTRAINT [DF_SourceCategories_DateAdded]  DEFAULT (sysutcdatetime()) FOR [DateAdded]
GO
ALTER TABLE [dbo].[ScraperRuns] ADD  CONSTRAINT [DF_ScraperRuns_Start]  DEFAULT (sysutcdatetime()) FOR [StartedAt]
GO
ALTER TABLE [dbo].[ScraperRuns] ADD  CONSTRAINT [DF_ScraperRuns_Heartbeat]  DEFAULT (sysutcdatetime()) FOR [HeartbeatAt]
GO
ALTER TABLE [dbo].[ResultGames] ADD  CONSTRAINT [DF_ResultGames_FirstSeenAt]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[ResultGames] ADD  CONSTRAINT [DF_ResultGames_LastSeenAt]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[Movies] ADD  CONSTRAINT [DF_Movies_FirstSeen]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[Movies] ADD  CONSTRAINT [DF_Movies_LastSeen]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[MovieReleaseSources] ADD  CONSTRAINT [DF_MovieReleaseSources_Enabled]  DEFAULT ((0)) FOR [Enabled]
GO
ALTER TABLE [dbo].[MovieReleaseSources] ADD  CONSTRAINT [DF_MovieReleaseSources_Created]  DEFAULT (sysutcdatetime()) FOR [CreatedAt]
GO
ALTER TABLE [dbo].[SportsStandingsSources] ADD  CONSTRAINT [DF__SportsSta__Enabl__01D345B0]  DEFAULT ((0)) FOR [Enabled]
GO
ALTER TABLE [dbo].[SportsRankingSources] ADD  CONSTRAINT [DF__SportsRan__Provi__1C873BEC]  DEFAULT (N'espn-rankings') FOR [ProviderKey]
GO
ALTER TABLE [dbo].[SportsRankingSources] ADD  CONSTRAINT [DF__SportsRan__Enabl__1D7B6025]  DEFAULT ((0)) FOR [Enabled]
GO
ALTER TABLE [dbo].[SportsEventsType] ADD  CONSTRAINT [DF_SportsEventsType_DateAdded]  DEFAULT (sysutcdatetime()) FOR [DateAdded]
GO
ALTER TABLE [dbo].[SportsEventsType] ADD  CONSTRAINT [DF_SportsEventsType_Enabled]  DEFAULT ((1)) FOR [Enabled]
GO
ALTER TABLE [dbo].[TvSeries] ADD  CONSTRAINT [DF_TvSeries_First]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[TvSeries] ADD  CONSTRAINT [DF_TvSeries_Last]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[YoutubeVideoFeed] ADD  CONSTRAINT [DF_YoutubeVideoFeed_DateAdded]  DEFAULT (sysutcdatetime()) FOR [DateAdded]
GO
ALTER TABLE [dbo].[YouTubeVideos] ADD  CONSTRAINT [DF__YouTubeVi__First__5FB337D6]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[YouTubeVideos] ADD  CONSTRAINT [DF__YouTubeVi__LastS__60A75C0F]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[TvTrackedSeries] ADD  CONSTRAINT [DF_TvTrackedSeries_Enabled]  DEFAULT ((1)) FOR [Enabled]
GO
ALTER TABLE [dbo].[TvTrackedSeries] ADD  CONSTRAINT [DF_TvTrackedSeries_Created]  DEFAULT (sysutcdatetime()) FOR [CreatedAt]
GO
ALTER TABLE [dbo].[TvEpisodes] ADD  CONSTRAINT [DF_TvEpisodes_First]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[TvEpisodes] ADD  CONSTRAINT [DF_TvEpisodes_Last]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[SportsEvents] ADD  CONSTRAINT [DF__SportsEve__First__5AEE82B9]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[SportsEvents] ADD  CONSTRAINT [DF__SportsEve__LastS__5BE2A6F2]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[Sources] ADD  CONSTRAINT [DF__Sources__Enabled__4F7CD00D]  DEFAULT ((1)) FOR [Enabled]
GO
ALTER TABLE [dbo].[Sources] ADD  CONSTRAINT [DF__Sources__Created__5070F446]  DEFAULT (sysutcdatetime()) FOR [CreatedAt]
GO
ALTER TABLE [dbo].[SportsRankingPolls] ADD  CONSTRAINT [DF__SportsRan__IsCur__22401542]  DEFAULT ((1)) FOR [IsCurrent]
GO
ALTER TABLE [dbo].[SportsRankingPolls] ADD  CONSTRAINT [DF__SportsRan__First__2334397B]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[SportsMatchResults] ADD  CONSTRAINT [DF__SportsMat__First__6E8B6712]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[SportsMatchResults] ADD  CONSTRAINT [DF__SportsMat__LastS__6F7F8B4B]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[SportsStandings] ADD  CONSTRAINT [DF__SportsSta__IsCur__0880433F]  DEFAULT ((1)) FOR [IsCurrent]
GO
ALTER TABLE [dbo].[SportsStandings] ADD  CONSTRAINT [DF__SportsSta__First__09746778]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[MovieSourceLinks] ADD  CONSTRAINT [DF_MovieSourceLinks_FirstSeen]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[MovieSourceLinks] ADD  CONSTRAINT [DF_MovieSourceLinks_LastSeen]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[Pages] ADD  CONSTRAINT [DF__Pages__ScrapedAt__4BAC3F29]  DEFAULT (sysutcdatetime()) FOR [ScrapedAt]
GO
ALTER TABLE [dbo].[MovieReleases] ADD  CONSTRAINT [DF_MovieReleases_Country]  DEFAULT ('US') FOR [CountryCode]
GO
ALTER TABLE [dbo].[MovieReleases] ADD  CONSTRAINT [DF_MovieReleases_DateStatus]  DEFAULT ('tbd') FOR [DateStatus]
GO
ALTER TABLE [dbo].[MovieReleases] ADD  CONSTRAINT [DF_MovieReleases_Status]  DEFAULT ('scheduled') FOR [ReleaseStatus]
GO
ALTER TABLE [dbo].[MovieReleases] ADD  CONSTRAINT [DF_MovieReleases_ReRelease]  DEFAULT ((0)) FOR [IsReRelease]
GO
ALTER TABLE [dbo].[MovieReleases] ADD  CONSTRAINT [DF_MovieReleases_FirstSeen]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[MovieReleases] ADD  CONSTRAINT [DF_MovieReleases_LastSeen]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[SportsRankingTeams] ADD  CONSTRAINT [DF__SportsRan__IsCur__29E1370A]  DEFAULT ((1)) FOR [IsCurrent]
GO
ALTER TABLE [dbo].[SportsRankingTeams] ADD  CONSTRAINT [DF__SportsRan__First__2AD55B43]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[SportsEventSources] ADD  CONSTRAINT [DF_SportsEventSources_First]  DEFAULT (sysutcdatetime()) FOR [FirstSeenAt]
GO
ALTER TABLE [dbo].[SportsEventSources] ADD  CONSTRAINT [DF_SportsEventSources_Last]  DEFAULT (sysutcdatetime()) FOR [LastSeenAt]
GO
ALTER TABLE [dbo].[YouTubeVideos]  WITH CHECK ADD  CONSTRAINT [FK_YouTubeVideos_YoutubeVideoFeed] FOREIGN KEY([YoutubeVideoFeedId])
REFERENCES [dbo].[YoutubeVideoFeed] ([ID])
GO
ALTER TABLE [dbo].[YouTubeVideos] CHECK CONSTRAINT [FK_YouTubeVideos_YoutubeVideoFeed]
GO
ALTER TABLE [dbo].[TvTrackedSeries]  WITH CHECK ADD  CONSTRAINT [FK_TvTrackedSeries_Series] FOREIGN KEY([SeriesId])
REFERENCES [dbo].[TvSeries] ([ID])
GO
ALTER TABLE [dbo].[TvTrackedSeries] CHECK CONSTRAINT [FK_TvTrackedSeries_Series]
GO
ALTER TABLE [dbo].[TvEpisodes]  WITH CHECK ADD  CONSTRAINT [FK_TvEpisodes_Series] FOREIGN KEY([SeriesId])
REFERENCES [dbo].[TvSeries] ([ID])
GO
ALTER TABLE [dbo].[TvEpisodes] CHECK CONSTRAINT [FK_TvEpisodes_Series]
GO
ALTER TABLE [dbo].[SportsEvents]  WITH CHECK ADD  CONSTRAINT [FK_SportsEvents_SportsEventsType] FOREIGN KEY([SportsEventsTypeId])
REFERENCES [dbo].[SportsEventsType] ([ID])
GO
ALTER TABLE [dbo].[SportsEvents] CHECK CONSTRAINT [FK_SportsEvents_SportsEventsType]
GO
ALTER TABLE [dbo].[Sources]  WITH CHECK ADD  CONSTRAINT [FK_Sources_SourceCategories] FOREIGN KEY([SourceCategoryId])
REFERENCES [dbo].[SourceCategories] ([ID])
GO
ALTER TABLE [dbo].[Sources] CHECK CONSTRAINT [FK_Sources_SourceCategories]
GO
ALTER TABLE [dbo].[SportsRankingPolls]  WITH CHECK ADD  CONSTRAINT [FK__SportsRan__Sourc__214BF109] FOREIGN KEY([SourceId])
REFERENCES [dbo].[SportsRankingSources] ([ID])
GO
ALTER TABLE [dbo].[SportsRankingPolls] CHECK CONSTRAINT [FK__SportsRan__Sourc__214BF109]
GO
ALTER TABLE [dbo].[SportsMatchResults]  WITH CHECK ADD  CONSTRAINT [FK__SportsMat__Sourc__6BAEFA67] FOREIGN KEY([SourceId])
REFERENCES [dbo].[SportsEventsType] ([ID])
GO
ALTER TABLE [dbo].[SportsMatchResults] CHECK CONSTRAINT [FK__SportsMat__Sourc__6BAEFA67]
GO
ALTER TABLE [dbo].[SportsStandings]  WITH CHECK ADD  CONSTRAINT [FK__SportsSta__Sourc__0697FACD] FOREIGN KEY([SourceId])
REFERENCES [dbo].[SportsStandingsSources] ([ID])
GO
ALTER TABLE [dbo].[SportsStandings] CHECK CONSTRAINT [FK__SportsSta__Sourc__0697FACD]
GO
ALTER TABLE [dbo].[BoxOfficeForecasts]  WITH CHECK ADD  CONSTRAINT [FK__BoxOfficeF__Kind__6A85CC04] FOREIGN KEY([Kind])
REFERENCES [dbo].[BoxOfficeForecastSources] ([Kind])
GO
ALTER TABLE [dbo].[BoxOfficeForecasts] CHECK CONSTRAINT [FK__BoxOfficeF__Kind__6A85CC04]
GO
ALTER TABLE [dbo].[ResultFollowTeams]  WITH CHECK ADD  CONSTRAINT [FK_ResultFollowTeams_Follow] FOREIGN KEY([FollowId])
REFERENCES [dbo].[ResultFollows] ([ID])
ON DELETE CASCADE
GO
ALTER TABLE [dbo].[ResultFollowTeams] CHECK CONSTRAINT [FK_ResultFollowTeams_Follow]
GO
ALTER TABLE [dbo].[MovieSourceLinks]  WITH CHECK ADD  CONSTRAINT [FK_MovieSourceLinks_Movies] FOREIGN KEY([MovieId])
REFERENCES [dbo].[Movies] ([ID])
GO
ALTER TABLE [dbo].[MovieSourceLinks] CHECK CONSTRAINT [FK_MovieSourceLinks_Movies]
GO
ALTER TABLE [dbo].[MovieSourceLinks]  WITH CHECK ADD  CONSTRAINT [FK_MovieSourceLinks_Sources] FOREIGN KEY([MovieReleaseSourceId])
REFERENCES [dbo].[MovieReleaseSources] ([ID])
GO
ALTER TABLE [dbo].[MovieSourceLinks] CHECK CONSTRAINT [FK_MovieSourceLinks_Sources]
GO
ALTER TABLE [dbo].[Pages]  WITH CHECK ADD  CONSTRAINT [FK_Pages_Sources] FOREIGN KEY([SourceId])
REFERENCES [dbo].[Sources] ([ID])
GO
ALTER TABLE [dbo].[Pages] CHECK CONSTRAINT [FK_Pages_Sources]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [FK_MovieReleases_SourceLink] FOREIGN KEY([MovieSourceLinkId])
REFERENCES [dbo].[MovieSourceLinks] ([ID])
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [FK_MovieReleases_SourceLink]
GO
ALTER TABLE [dbo].[MoviePosterCache]  WITH CHECK ADD  CONSTRAINT [FK_MoviePosterCache_Link] FOREIGN KEY([SourceLinkId])
REFERENCES [dbo].[MovieSourceLinks] ([ID])
GO
ALTER TABLE [dbo].[MoviePosterCache] CHECK CONSTRAINT [FK_MoviePosterCache_Link]
GO
ALTER TABLE [dbo].[SportsRankingTeams]  WITH CHECK ADD  CONSTRAINT [FK__SportsRan__PollR__2704CA5F] FOREIGN KEY([PollRowId])
REFERENCES [dbo].[SportsRankingPolls] ([ID])
GO
ALTER TABLE [dbo].[SportsRankingTeams] CHECK CONSTRAINT [FK__SportsRan__PollR__2704CA5F]
GO
ALTER TABLE [dbo].[SportsEventSources]  WITH CHECK ADD  CONSTRAINT [FK_SportsEventSources_Event] FOREIGN KEY([SportsEventId])
REFERENCES [dbo].[SportsEvents] ([ID])
GO
ALTER TABLE [dbo].[SportsEventSources] CHECK CONSTRAINT [FK_SportsEventSources_Event]
GO
ALTER TABLE [dbo].[SportsEventSources]  WITH CHECK ADD  CONSTRAINT [FK_SportsEventSources_Type] FOREIGN KEY([SportsEventsTypeId])
REFERENCES [dbo].[SportsEventsType] ([ID])
GO
ALTER TABLE [dbo].[SportsEventSources] CHECK CONSTRAINT [FK_SportsEventSources_Type]
GO
ALTER TABLE [dbo].[ResultFollows]  WITH CHECK ADD  CONSTRAINT [CK_ResultFollows_Kind] CHECK  (([Kind]=N'team' AND [TeamId] IS NOT NULL AND [GroupId] IS NULL OR [Kind]=N'league' AND [TeamId] IS NULL AND [GroupId] IS NULL OR [Kind]=N'conference' AND [TeamId] IS NULL AND [GroupId] IS NOT NULL))
GO
ALTER TABLE [dbo].[ResultFollows] CHECK CONSTRAINT [CK_ResultFollows_Kind]
GO
ALTER TABLE [dbo].[ScraperRuns]  WITH CHECK ADD  CONSTRAINT [CK_ScraperRuns_Status] CHECK  (([Status]='failed' OR [Status]='partial' OR [Status]='succeeded' OR [Status]='running'))
GO
ALTER TABLE [dbo].[ScraperRuns] CHECK CONSTRAINT [CK_ScraperRuns_Status]
GO
ALTER TABLE [dbo].[Movies]  WITH CHECK ADD  CONSTRAINT [CK_Movies_Runtime] CHECK  (([RuntimeMinutes] IS NULL OR [RuntimeMinutes]>(0)))
GO
ALTER TABLE [dbo].[Movies] CHECK CONSTRAINT [CK_Movies_Runtime]
GO
ALTER TABLE [dbo].[Movies]  WITH CHECK ADD  CONSTRAINT [CK_Movies_Tmdb] CHECK  (([TmdbId] IS NULL OR [TmdbId]>(0)))
GO
ALTER TABLE [dbo].[Movies] CHECK CONSTRAINT [CK_Movies_Tmdb]
GO
ALTER TABLE [dbo].[Movies]  WITH CHECK ADD  CONSTRAINT [CK_Movies_Year] CHECK  (([OriginalYear] IS NULL OR [OriginalYear]>=(1800) AND [OriginalYear]<=(9999)))
GO
ALTER TABLE [dbo].[Movies] CHECK CONSTRAINT [CK_Movies_Year]
GO
ALTER TABLE [dbo].[BoxOfficeWeekend]  WITH CHECK ADD  CONSTRAINT [CK__BoxOffice__Total__3DB3258D] CHECK  (([TotalDomesticGross]>=(0)))
GO
ALTER TABLE [dbo].[BoxOfficeWeekend] CHECK CONSTRAINT [CK__BoxOffice__Total__3DB3258D]
GO
ALTER TABLE [dbo].[BoxOfficeWeekend]  WITH CHECK ADD  CONSTRAINT [CK__BoxOffice__Weeke__3CBF0154] CHECK  (([WeekendGross]>=(0)))
GO
ALTER TABLE [dbo].[BoxOfficeWeekend] CHECK CONSTRAINT [CK__BoxOffice__Weeke__3CBF0154]
GO
ALTER TABLE [dbo].[BoxOfficeWeekend]  WITH CHECK ADD  CONSTRAINT [CK__BoxOfficeW__Rank__3BCADD1B] CHECK  (([Rank]>=(1) AND [Rank]<=(10)))
GO
ALTER TABLE [dbo].[BoxOfficeWeekend] CHECK CONSTRAINT [CK__BoxOfficeW__Rank__3BCADD1B]
GO
ALTER TABLE [dbo].[SportsStandingsSources]  WITH CHECK ADD  CONSTRAINT [CK__SportsSta__Minim__02C769E9] CHECK  (([MinimumTeams]>=(1) AND [MinimumTeams]<=(256)))
GO
ALTER TABLE [dbo].[SportsStandingsSources] CHECK CONSTRAINT [CK__SportsSta__Minim__02C769E9]
GO
ALTER TABLE [dbo].[TvSeries]  WITH CHECK ADD  CONSTRAINT [CK_TvSeries_JSON] CHECK  ((isjson([RawMetadata])=(1)))
GO
ALTER TABLE [dbo].[TvSeries] CHECK CONSTRAINT [CK_TvSeries_JSON]
GO
ALTER TABLE [dbo].[TvSeries]  WITH CHECK ADD  CONSTRAINT [CK_TvSeries_Show] CHECK  (([TvmazeShowId]>(0)))
GO
ALTER TABLE [dbo].[TvSeries] CHECK CONSTRAINT [CK_TvSeries_Show]
GO
ALTER TABLE [dbo].[TvTrackedSeries]  WITH CHECK ADD  CONSTRAINT [CK_TvTrackedSeries_EpisodesJSON] CHECK  (([CachedEpisodes] IS NULL OR isjson([CachedEpisodes])=(1)))
GO
ALTER TABLE [dbo].[TvTrackedSeries] CHECK CONSTRAINT [CK_TvTrackedSeries_EpisodesJSON]
GO
ALTER TABLE [dbo].[TvTrackedSeries]  WITH CHECK ADD  CONSTRAINT [CK_TvTrackedSeries_Show] CHECK  (([TvmazeShowId]>(0)))
GO
ALTER TABLE [dbo].[TvTrackedSeries] CHECK CONSTRAINT [CK_TvTrackedSeries_Show]
GO
ALTER TABLE [dbo].[TvTrackedSeries]  WITH CHECK ADD  CONSTRAINT [CK_TvTrackedSeries_ShowJSON] CHECK  (([CachedShow] IS NULL OR isjson([CachedShow])=(1)))
GO
ALTER TABLE [dbo].[TvTrackedSeries] CHECK CONSTRAINT [CK_TvTrackedSeries_ShowJSON]
GO
ALTER TABLE [dbo].[TvEpisodes]  WITH CHECK ADD  CONSTRAINT [CK_TvEpisodes_Id] CHECK  (([TvmazeEpisodeId]>(0)))
GO
ALTER TABLE [dbo].[TvEpisodes] CHECK CONSTRAINT [CK_TvEpisodes_Id]
GO
ALTER TABLE [dbo].[TvEpisodes]  WITH CHECK ADD  CONSTRAINT [CK_TvEpisodes_JSON] CHECK  ((isjson([RawMetadata])=(1)))
GO
ALTER TABLE [dbo].[TvEpisodes] CHECK CONSTRAINT [CK_TvEpisodes_JSON]
GO
ALTER TABLE [dbo].[TvEpisodes]  WITH CHECK ADD  CONSTRAINT [CK_TvEpisodes_Time] CHECK  (([IsDateOnly]=(0) OR [Airtime] IS NULL AND [StartsAtUtc] IS NULL))
GO
ALTER TABLE [dbo].[TvEpisodes] CHECK CONSTRAINT [CK_TvEpisodes_Time]
GO
ALTER TABLE [dbo].[SportsMatchResults]  WITH CHECK ADD  CONSTRAINT [CK__SportsMat__AwayS__6D9742D9] CHECK  (([AwayScore]>=(0)))
GO
ALTER TABLE [dbo].[SportsMatchResults] CHECK CONSTRAINT [CK__SportsMat__AwayS__6D9742D9]
GO
ALTER TABLE [dbo].[SportsMatchResults]  WITH CHECK ADD  CONSTRAINT [CK__SportsMat__HomeS__6CA31EA0] CHECK  (([HomeScore]>=(0)))
GO
ALTER TABLE [dbo].[SportsMatchResults] CHECK CONSTRAINT [CK__SportsMat__HomeS__6CA31EA0]
GO
ALTER TABLE [dbo].[SportsMatchResults]  WITH CHECK ADD  CONSTRAINT [CK_SportsMatchResults_Teams] CHECK  (([HomeTeamId]<>[AwayTeamId]))
GO
ALTER TABLE [dbo].[SportsMatchResults] CHECK CONSTRAINT [CK_SportsMatchResults_Teams]
GO
ALTER TABLE [dbo].[SportsStandings]  WITH CHECK ADD  CONSTRAINT [CK__SportsSta__Stats__078C1F06] CHECK  ((isjson([StatsJson])=(1)))
GO
ALTER TABLE [dbo].[SportsStandings] CHECK CONSTRAINT [CK__SportsSta__Stats__078C1F06]
GO
ALTER TABLE [dbo].[BoxOfficeForecasts]  WITH CHECK ADD  CONSTRAINT [CK_BoxOfficeForecasts_Period] CHECK  (([PeriodEnd]>=[TargetDate]))
GO
ALTER TABLE [dbo].[BoxOfficeForecasts] CHECK CONSTRAINT [CK_BoxOfficeForecasts_Period]
GO
ALTER TABLE [dbo].[BoxOfficeForecasts]  WITH CHECK ADD  CONSTRAINT [CK_BoxOfficeForecasts_Range] CHECK  (([ForecastLow]>=(0) AND [ForecastHigh]>=[ForecastLow]))
GO
ALTER TABLE [dbo].[BoxOfficeForecasts] CHECK CONSTRAINT [CK_BoxOfficeForecasts_Range]
GO
ALTER TABLE [dbo].[MovieSourceLinks]  WITH CHECK ADD  CONSTRAINT [CK_MovieSourceLinks_External] CHECK  ((len(ltrim(rtrim([ExternalMovieId])))>(0)))
GO
ALTER TABLE [dbo].[MovieSourceLinks] CHECK CONSTRAINT [CK_MovieSourceLinks_External]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [CK_MovieReleases_Date] CHECK  (([DateStatus]='tbd' AND [ReleaseDate] IS NULL OR ([DateStatus]='estimated' OR [DateStatus]='announced') AND [ReleaseDate] IS NOT NULL))
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [CK_MovieReleases_Date]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [CK_MovieReleases_Key] CHECK  ((len(ltrim(rtrim([ReleaseKey])))>(0)))
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [CK_MovieReleases_Key]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [CK_MovieReleases_Platform] CHECK  (([ReleaseType]<>'subscription' OR [Platform] IS NOT NULL AND len(ltrim(rtrim([Platform])))>(0)))
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [CK_MovieReleases_Platform]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [CK_MovieReleases_Status] CHECK  (([ReleaseStatus]='withdrawn' OR [ReleaseStatus]='cancelled' OR [ReleaseStatus]='scheduled'))
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [CK_MovieReleases_Status]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [CK_MovieReleases_Type] CHECK  (([ReleaseType]='disc' OR [ReleaseType]='subscription' OR [ReleaseType]='digital_rental' OR [ReleaseType]='digital_purchase' OR [ReleaseType]='digital' OR [ReleaseType]='theatrical'))
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [CK_MovieReleases_Type]
GO
ALTER TABLE [dbo].[MovieReleases]  WITH CHECK ADD  CONSTRAINT [CK_MovieReleases_US] CHECK  (([CountryCode]='US'))
GO
ALTER TABLE [dbo].[MovieReleases] CHECK CONSTRAINT [CK_MovieReleases_US]
GO
ALTER TABLE [dbo].[MoviePosterCache]  WITH CHECK ADD  CONSTRAINT [CK_MoviePosterCache_Metadata] CHECK  ((isjson([Metadata])=(1)))
GO
ALTER TABLE [dbo].[MoviePosterCache] CHECK CONSTRAINT [CK_MoviePosterCache_Metadata]
GO
ALTER TABLE [dbo].[SportsRankingTeams]  WITH CHECK ADD  CONSTRAINT [CK__SportsRan__Previ__28ED12D1] CHECK  (([PreviousRank]>=(1) AND [PreviousRank]<=(25)))
GO
ALTER TABLE [dbo].[SportsRankingTeams] CHECK CONSTRAINT [CK__SportsRan__Previ__28ED12D1]
GO
ALTER TABLE [dbo].[SportsRankingTeams]  WITH CHECK ADD  CONSTRAINT [CK__SportsRank__Rank__27F8EE98] CHECK  (([Rank]>=(1) AND [Rank]<=(25)))
GO
ALTER TABLE [dbo].[SportsRankingTeams] CHECK CONSTRAINT [CK__SportsRank__Rank__27F8EE98]
GO
ALTER TABLE [dbo].[SportsEventSources]  WITH CHECK ADD  CONSTRAINT [CK_SportsEventSources_JSON] CHECK  (([Payload] IS NULL OR isjson([Payload])=(1)))
GO
ALTER TABLE [dbo].[SportsEventSources] CHECK CONSTRAINT [CK_SportsEventSources_JSON]
GO
