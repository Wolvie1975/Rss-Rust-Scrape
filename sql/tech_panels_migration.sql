-- Tech front panels migration: Your stack, Your servers, New in Ollama
-- Repeatable and idempotent

-- 1. Your stack
IF COL_LENGTH(N'dbo.TechReleaseProjects', N'InYourStack') IS NULL
BEGIN
    ALTER TABLE dbo.TechReleaseProjects ADD InYourStack bit NOT NULL
        CONSTRAINT DF_TechReleaseProjects_InYourStack DEFAULT (0);
END;

IF EXISTS (SELECT 1 FROM sys.check_constraints WHERE name = 'CK_TechReleaseProjects_Kind' AND parent_object_id = OBJECT_ID('dbo.TechReleaseProjects'))
    ALTER TABLE dbo.TechReleaseProjects DROP CONSTRAINT CK_TechReleaseProjects_Kind;

ALTER TABLE dbo.TechReleaseProjects ADD CONSTRAINT CK_TechReleaseProjects_Kind
    CHECK (Kind IN (N'kernel', N'distro', N'desktop', N'graphics', N'gaming', N'app', N'sdk', N'toolchain', N'framework', N'runtime', N'compiler'));

IF EXISTS (SELECT 1 FROM sys.check_constraints WHERE name = 'CK_TechReleaseProjects_Source' AND parent_object_id = OBJECT_ID('dbo.TechReleaseProjects'))
    ALTER TABLE dbo.TechReleaseProjects DROP CONSTRAINT CK_TechReleaseProjects_Source;

ALTER TABLE dbo.TechReleaseProjects ADD CONSTRAINT CK_TechReleaseProjects_Source
    CHECK (SourceKind IN (N'kernel_org', N'github_releases', N'gitlab_tags', N'rss', N'dotnet_releases'));

-- Flag existing stack projects
EXEC(N'UPDATE dbo.TechReleaseProjects
SET InYourStack = 1
WHERE Slug IN (N''linux-mainline'', N''fedora'', N''mesa'', N''proton'');');

-- Seed additional stack projects
EXEC(N'MERGE dbo.TechReleaseProjects AS target
USING (VALUES
    (N''dotnet'', N''.NET'', N''sdk'', N''dotnet_releases'', N''https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/releases-index.json'', NULL, N''Release notes'', 0, 1, 100, 1),
    (N''rust'', N''Rust'', N''toolchain'', N''github_releases'', N''https://api.github.com/repos/rust-lang/rust/releases'', NULL, N''Release notes'', 0, 1, 110, 1),
    (N''qt'', N''Qt'', N''framework'', N''rss'', N''https://github.com/qt/qtbase/tags.atom'', N''^(?:v|qtbase-)?(6\.\d+\.\d+)$'', N''Changelog'', 0, 1, 120, 1),
    (N''nodejs'', N''Node.js'', N''runtime'', N''github_releases'', N''https://api.github.com/repos/nodejs/node/releases'', NULL, N''Changelog'', 0, 1, 130, 1),
    (N''typescript'', N''TypeScript'', N''compiler'', N''github_releases'', N''https://api.github.com/repos/microsoft/TypeScript/releases'', NULL, N''Release notes'', 0, 1, 140, 1)
) AS source (Slug, Name, Kind, SourceKind, SourceUrl, SourceFilter, NotesLabel, IncludePrerelease, Enabled, SortOrder, InYourStack)
ON target.Slug = source.Slug
WHEN MATCHED THEN
    UPDATE SET InYourStack = source.InYourStack
WHEN NOT MATCHED THEN
    INSERT (Slug, Name, Kind, SourceKind, SourceUrl, SourceFilter, NotesLabel, IncludePrerelease, Enabled, SortOrder, InYourStack)
    VALUES (source.Slug, source.Name, source.Kind, source.SourceKind, source.SourceUrl, source.SourceFilter, source.NotesLabel, source.IncludePrerelease, source.Enabled, source.SortOrder, source.InYourStack);');

-- 2. Your servers (Homelab)
IF OBJECT_ID('dbo.HomelabServices', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.HomelabServices (
        ID          int IDENTITY PRIMARY KEY,
        Slug        nvarchar(40)  NOT NULL UNIQUE,
        Name        nvarchar(80)  NOT NULL,
        CheckKind   nvarchar(20)  NOT NULL,          -- http | http-body | tautulli | self
        CheckUrl    nvarchar(400) NULL,              -- internal address; never contains a secret
        ExpectBody  nvarchar(80)  NULL,              -- for http-body, e.g. 'Healthy'
        SecretEnv   nvarchar(60)  NULL,              -- name of an env var holding an API key, e.g. 'TAUTULLI_API_KEY'
        SlowMs      int           NOT NULL DEFAULT (1500),
        LinkUrl     nvarchar(400) NULL,              -- where the web dot links (the service's own UI)
        Enabled     bit           NOT NULL DEFAULT (1),
        SortOrder   int           NOT NULL DEFAULT (0)
    );
END;

IF OBJECT_ID('dbo.HomelabServiceStatus', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.HomelabServiceStatus (
        ServiceId   int           NOT NULL PRIMARY KEY REFERENCES dbo.HomelabServices(ID),
        State       nvarchar(10)  NOT NULL,          -- ok | slow | down | unknown
        LatencyMs   int           NULL,
        Detail      nvarchar(120) NULL,
        StateSince  datetime2     NOT NULL,          -- UTC, when State last changed
        CheckedAt   datetime2     NOT NULL,          -- UTC, last check
        FailCount   int           NOT NULL DEFAULT (0)
    );
END;

-- Seed Homelab services
MERGE dbo.HomelabServices AS target
USING (VALUES
    (N'plex', N'Plex', N'tautulli', N'http://tautulli:8181/api/v2?cmd=get_activity', NULL, N'TAUTULLI_API_KEY', 1500, N'http://host.docker.internal:32400/web', 1, 10),
    (N'jellyfin', N'Jellyfin', N'http-body', N'http://host.docker.internal:8096/health', N'Healthy', NULL, 1500, N'http://host.docker.internal:8096', 1, 20),
    (N'tautulli', N'Tautulli', N'tautulli', N'http://tautulli:8181/api/v2?cmd=get_activity', NULL, N'TAUTULLI_API_KEY', 1500, N'http://tautulli:8181', 1, 30),
    (N'ollama', N'Ollama', N'http', N'http://ai-machine.local:11434/api/version', NULL, NULL, 1500, NULL, 1, 40),
    (N'open-webui', N'Open WebUI', N'http', N'http://ai-machine.local:8080/health', NULL, NULL, 1500, N'http://ai-machine.local:8080', 1, 50),
    (N'portainer', N'Portainer', N'http', N'http://portainer:9000/api/system/status', NULL, NULL, 1500, N'http://portainer:9000', 1, 60),
    (N'scraper', N'myNewspaper scraper', N'self', NULL, NULL, NULL, 1500, NULL, 1, 70)
) AS source (Slug, Name, CheckKind, CheckUrl, ExpectBody, SecretEnv, SlowMs, LinkUrl, Enabled, SortOrder)
ON target.Slug = source.Slug
WHEN NOT MATCHED THEN
    INSERT (Slug, Name, CheckKind, CheckUrl, ExpectBody, SecretEnv, SlowMs, LinkUrl, Enabled, SortOrder)
    VALUES (source.Slug, source.Name, source.CheckKind, source.CheckUrl, source.ExpectBody, source.SecretEnv, source.SlowMs, source.LinkUrl, source.Enabled, source.SortOrder);

-- 3. New in Ollama
IF OBJECT_ID('dbo.OllamaLibraryModels', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.OllamaLibraryModels (
        ID           int IDENTITY PRIMARY KEY,
        Name         nvarchar(80)  NOT NULL UNIQUE,
        Description  nvarchar(300) NULL,
        Change       nvarchar(10)  NOT NULL,         -- new | updated
        ChangedAt    datetime2     NOT NULL,         -- UTC
        UpdatedText  nvarchar(40)  NULL,             -- the library's own "Updated 2 days ago", kept to detect updates
        LibraryUrl   nvarchar(300) NOT NULL,
        FirstSeenAt  datetime2     NOT NULL,
        SnapshotAt   datetime2     NOT NULL
    );
END;

IF OBJECT_ID('dbo.OllamaLibraryTags', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.OllamaLibraryTags (
        ID         int IDENTITY PRIMARY KEY,
        ModelId    int          NOT NULL REFERENCES dbo.OllamaLibraryModels(ID) ON DELETE CASCADE,
        Tag        nvarchar(40) NOT NULL,            -- '8b', '70b', 'e4b'
        SizeGb     decimal(6,1) NULL,
        Fits       bit          NOT NULL DEFAULT (0),
        Installed  bit          NOT NULL DEFAULT (0),
        SortOrder  int          NOT NULL DEFAULT (0),
        CONSTRAINT UQ_OllamaLibraryTags UNIQUE (ModelId, Tag)
    );
END;

-- 4. Permissions
IF DATABASE_PRINCIPAL_ID('WebScraperReader') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.HomelabServices TO [WebScraperReader];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [WebScraperReader];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [WebScraperReader];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [WebScraperReader];
END;

IF DATABASE_PRINCIPAL_ID('WebScraperScoreboardReader') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.HomelabServices TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [WebScraperScoreboardReader];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [WebScraperScoreboardReader];
END;

IF USER_ID('web_reader') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.HomelabServices TO [web_reader];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [web_reader];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [web_reader];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [web_reader];
END;

IF USER_ID('mynewspaper') IS NOT NULL
BEGIN
    GRANT SELECT ON dbo.HomelabServices TO [mynewspaper];
    GRANT SELECT ON dbo.HomelabServiceStatus TO [mynewspaper];
    GRANT SELECT ON dbo.OllamaLibraryModels TO [mynewspaper];
    GRANT SELECT ON dbo.OllamaLibraryTags TO [mynewspaper];
END;

IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
BEGIN
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.HomelabServices TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.HomelabServiceStatus TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.OllamaLibraryModels TO [WebScraperIngest];
    GRANT SELECT, INSERT, UPDATE, DELETE ON dbo.OllamaLibraryTags TO [WebScraperIngest];
    GRANT UPDATE (InYourStack) ON dbo.TechReleaseProjects TO [WebScraperIngest];
END;
