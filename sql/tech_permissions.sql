-- Permissions for Tech section tables.
-- Run with migration credentials.

-- Ingestion role permissions
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
    GRANT SELECT, UPDATE (LastCheckedAt, LastError, InYourStack) ON dbo.TechReleaseProjects TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE ON dbo.TechReleases TO [WebScraperIngest];
    GRANT SELECT, INSERT, DELETE ON dbo.TechDevReading TO [WebScraperIngest];
    GRANT SELECT ON dbo.TechStatusServices TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE ON dbo.TechServiceStatus TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.HomelabServices TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.HomelabServiceStatus TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.OllamaLibraryModels TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.OllamaLibraryTags TO [WebScraperIngest];
END;

-- Web reader role permissions
-- The web application reads these tables directly.
IF DATABASE_PRINCIPAL_ID('WebScraperReader') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.TechReleaseProjects TO [WebScraperReader];
    GRANT SELECT ON dbo.TechReleases TO [WebScraperReader];
    GRANT SELECT ON dbo.TechDevReading TO [WebScraperReader];
    GRANT SELECT ON dbo.TechStatusServices TO [WebScraperReader];
    GRANT SELECT ON dbo.TechServiceStatus TO [WebScraperReader];
    GRANT SELECT ON dbo.HomelabServices TO [WebScraperReader];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [WebScraperReader];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [WebScraperReader];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [WebScraperReader];
END;

IF DATABASE_PRINCIPAL_ID('WebScraperScoreboardReader') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.TechReleaseProjects TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.TechReleases TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.TechDevReading TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.TechStatusServices TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.TechServiceStatus TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.HomelabServices TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [WebScraperScoreboardReader];
END;

IF USER_ID('web_reader') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.TechReleaseProjects TO [web_reader];
    GRANT SELECT ON dbo.TechReleases TO [web_reader];
    GRANT SELECT ON dbo.TechDevReading TO [web_reader];
    GRANT SELECT ON dbo.TechStatusServices TO [web_reader];
    GRANT SELECT ON dbo.TechServiceStatus TO [web_reader];
    GRANT SELECT ON dbo.HomelabServices TO [web_reader];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [web_reader];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [web_reader];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [web_reader];
END;

IF USER_ID('mynewspaper') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.TechReleaseProjects TO [mynewspaper];
    GRANT SELECT ON dbo.TechReleases TO [mynewspaper];
    GRANT SELECT ON dbo.TechDevReading TO [mynewspaper];
    GRANT SELECT ON dbo.TechStatusServices TO [mynewspaper];
    GRANT SELECT ON dbo.TechServiceStatus TO [mynewspaper];
    GRANT SELECT ON dbo.HomelabServices TO [mynewspaper];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [mynewspaper];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [mynewspaper];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [mynewspaper];
END;

