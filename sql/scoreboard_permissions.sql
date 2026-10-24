-- Apply after views. Frontend receives SELECT on read views only through this role.
IF DATABASE_PRINCIPAL_ID('WebScraperScoreboardReader') IS NULL
 CREATE ROLE WebScraperScoreboardReader;
GRANT SELECT ON dbo.SportsScoreboard TO WebScraperScoreboardReader;
GRANT SELECT ON dbo.SportsScoreboardCoverage TO WebScraperScoreboardReader;
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
 GRANT SELECT ON dbo.SportsScoreboard TO WebScraperIngest;
 GRANT SELECT ON dbo.SportsScoreboardCoverage TO WebScraperIngest;
END;
