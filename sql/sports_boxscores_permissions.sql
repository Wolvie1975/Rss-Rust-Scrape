IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
 GRANT SELECT ON dbo.SportsBoxScores TO WebScraperIngest;
