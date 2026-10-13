-- Run with deployment/admin credentials after --initialize-db.
-- The server login is provisioned separately; never put its password in this file.
IF USER_ID(N'webscraper_ingest') IS NULL CREATE USER [webscraper_ingest] FOR LOGIN [webscraper_ingest];
IF DATABASE_PRINCIPAL_ID(N'WebScraperIngest') IS NULL CREATE ROLE [WebScraperIngest] AUTHORIZATION dbo;
IF NOT EXISTS(SELECT 1 FROM sys.database_role_members WHERE role_principal_id=DATABASE_PRINCIPAL_ID(N'WebScraperIngest') AND member_principal_id=USER_ID(N'webscraper_ingest'))
 ALTER ROLE [WebScraperIngest] ADD MEMBER [webscraper_ingest];
GRANT SELECT ON dbo.SourceCategories TO [WebScraperIngest];
GRANT SELECT,INSERT ON dbo.Sources TO [WebScraperIngest];
GRANT UPDATE (LastScrapedAt,LastError) ON dbo.Sources TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.Pages TO [WebScraperIngest];
GRANT SELECT,INSERT ON dbo.SportsEventsType TO [WebScraperIngest];
GRANT UPDATE (LastAttemptAt,LastSuccessAt,LastError,LastEventCount,LastIssueCount) ON dbo.SportsEventsType TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.SportsEvents TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.SportsEventSources TO [WebScraperIngest];
GRANT SELECT ON dbo.SportsEventSchoolCalendar TO [WebScraperIngest];
GRANT SELECT,INSERT ON dbo.YoutubeVideoFeed TO [WebScraperIngest];
GRANT UPDATE (ChannelName) ON dbo.YoutubeVideoFeed TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.YouTubeVideos TO [WebScraperIngest];
GRANT SELECT,INSERT ON dbo.MovieReleaseSources TO [WebScraperIngest];
GRANT UPDATE (LastScrapedAt,LastError) ON dbo.MovieReleaseSources TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.Movies TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.MovieSourceLinks TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.MovieReleases TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.MoviePosterCache TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.TvSeries TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.TvEpisodes TO [WebScraperIngest];
GRANT SELECT,UPDATE ON dbo.TvTrackedSeries TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.ScraperRuns TO [WebScraperIngest];
-- No result synchronization permissions, db_owner membership, server role, or DDL.
GRANT SELECT ON dbo.SportsStandingsSources TO [WebScraperIngest];
GRANT UPDATE (LastAttemptAt,LastSuccessAt,LastError,LatestSeason,LastTeamCount) ON dbo.SportsStandingsSources TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.SportsStandings TO [WebScraperIngest];
GRANT SELECT ON dbo.SportsRankingSources TO [WebScraperIngest];
GRANT UPDATE (LastAttemptAt,LastSuccessAt,LastError) ON dbo.SportsRankingSources TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.SportsRankingPolls TO [WebScraperIngest];
GRANT SELECT,INSERT,UPDATE ON dbo.SportsRankingTeams TO [WebScraperIngest];

-- Retention cleanup: dependent rows are removed before their parents.
GRANT DELETE ON dbo.SportsEvents TO [WebScraperIngest];
GRANT DELETE ON dbo.SportsEventSources TO [WebScraperIngest];
GRANT DELETE ON dbo.MovieReleases TO [WebScraperIngest];
GRANT DELETE ON dbo.MoviePosterCache TO [WebScraperIngest];
GRANT DELETE ON dbo.MovieSourceLinks TO [WebScraperIngest];
GRANT DELETE ON dbo.Movies TO [WebScraperIngest];
GRANT DELETE ON dbo.TvEpisodes TO [WebScraperIngest];
GRANT DELETE ON dbo.TvSeries TO [WebScraperIngest];
GRANT DELETE ON dbo.SportsRankingTeams TO [WebScraperIngest];
GRANT DELETE ON dbo.SportsRankingPolls TO [WebScraperIngest];

IF OBJECT_ID('dbo.SportsMatchResults','U') IS NOT NULL
 GRANT SELECT,INSERT,UPDATE,DELETE ON dbo.SportsMatchResults TO [WebScraperIngest];

-- EPL snapshot rollover removes this source's non-current standings rows.
GRANT DELETE ON dbo.SportsStandings TO [WebScraperIngest];
