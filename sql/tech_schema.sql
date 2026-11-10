-- Tech section tables migration: repeatable and idempotent
IF OBJECT_ID('dbo.TechReleaseProjects', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.TechReleaseProjects (
        Id             int IDENTITY(1,1) NOT NULL CONSTRAINT PK_TechReleaseProjects PRIMARY KEY,
        Slug           nvarchar(40)   NOT NULL CONSTRAINT UQ_TechReleaseProjects_Slug UNIQUE,   -- "linux-mainline"
        Name           nvarchar(80)   NOT NULL,                                                 -- "Linux kernel"
        Kind           nvarchar(16)   NOT NULL,      -- kernel | distro | desktop | graphics | gaming | app
        SourceKind     nvarchar(20)   NOT NULL,      -- kernel_org | github_releases | gitlab_tags | rss
        SourceUrl      nvarchar(400)  NOT NULL,
        SourceFilter   nvarchar(200)  NULL,          -- kernel_org: moniker (mainline | stable | longterm); rss/gitlab_tags: version regex with one capture group
        NotesLabel     nvarchar(30)   NOT NULL CONSTRAINT DF_TechReleaseProjects_NotesLabel DEFAULT N'Release notes',
        IncludePrerelease bit         NOT NULL CONSTRAINT DF_TechReleaseProjects_Pre DEFAULT 0,
        Enabled        bit            NOT NULL CONSTRAINT DF_TechReleaseProjects_Enabled DEFAULT 1,
        SortOrder      int            NOT NULL CONSTRAINT DF_TechReleaseProjects_Sort DEFAULT 100,
        InYourStack    bit            NOT NULL CONSTRAINT DF_TechReleaseProjects_InYourStack DEFAULT 0,
        LastCheckedAt  datetime2      NULL,
        LastError      nvarchar(400)  NULL,
        CONSTRAINT CK_TechReleaseProjects_Kind CHECK (Kind IN (N'kernel', N'distro', N'desktop', N'graphics', N'gaming', N'app', N'sdk', N'toolchain', N'framework', N'runtime', N'compiler')),
        CONSTRAINT CK_TechReleaseProjects_Source CHECK (SourceKind IN (N'kernel_org', N'github_releases', N'gitlab_tags', N'rss', N'dotnet_releases'))
    );
END;

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

IF OBJECT_ID('dbo.TechReleases', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.TechReleases (
        Id            int IDENTITY(1,1) NOT NULL CONSTRAINT PK_TechReleases PRIMARY KEY,
        ProjectId     int            NOT NULL CONSTRAINT FK_TechReleases_Project REFERENCES dbo.TechReleaseProjects(Id) ON DELETE CASCADE,
        Version       nvarchar(64)   NOT NULL,       -- "6.18", "44", "6.5.2" (no "v" prefix)
        ReleasedAt    datetime2      NOT NULL,       -- UTC; the source's date, else FirstSeenAt
        NotesUrl      nvarchar(400)  NULL,           -- changelog / announcement / release page
        Title         nvarchar(300)  NULL,           -- the source's own title, for reference
        IsPrerelease  bit            NOT NULL CONSTRAINT DF_TechReleases_Pre DEFAULT 0,
        FirstSeenAt   datetime2      NOT NULL CONSTRAINT DF_TechReleases_Seen DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_TechReleases_Project_Version UNIQUE (ProjectId, Version)
    );
END;

IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_TechReleases_ReleasedAt' AND object_id = OBJECT_ID('dbo.TechReleases'))
BEGIN
    CREATE INDEX IX_TechReleases_ReleasedAt ON dbo.TechReleases (ReleasedAt DESC) INCLUDE (ProjectId, Version);
END;

IF OBJECT_ID('dbo.TechDevReading', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.TechDevReading (
        Site          nvarchar(12)   NOT NULL,       -- hn | lobsters
        Rank          int            NOT NULL,       -- 1 = top
        ExternalId    nvarchar(40)   NOT NULL,
        Title         nvarchar(400)  NOT NULL,
        Url           nvarchar(800)  NULL,           -- the linked article; NULL for text posts (Ask HN)
        Domain        nvarchar(120)  NULL,           -- "github.com" (from Url); NULL for text posts
        Points        int            NOT NULL,
        Comments      int            NOT NULL,
        DiscussionUrl nvarchar(400)  NOT NULL,       -- the HN item / Lobsters comments page
        PostedAt      datetime2      NULL,           -- UTC
        SnapshotAt    datetime2      NOT NULL,
        CONSTRAINT PK_TechDevReading PRIMARY KEY (Site, Rank),
        CONSTRAINT CK_TechDevReading_Site CHECK (Site IN (N'hn', N'lobsters'))
    );
END;

IF OBJECT_ID('dbo.TechStatusServices', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.TechStatusServices (
        Id         int IDENTITY(1,1) NOT NULL CONSTRAINT PK_TechStatusServices PRIMARY KEY,
        Slug       nvarchar(30)  NOT NULL CONSTRAINT UQ_TechStatusServices_Slug UNIQUE,
        Name       nvarchar(60)  NOT NULL,
        Kind       nvarchar(16)  NOT NULL,          -- statuspage (Atlassian Statuspage JSON)
        ApiUrl     nvarchar(300) NOT NULL,          -- e.g. https://www.githubstatus.com/api/v2/summary.json
        PageUrl    nvarchar(300) NOT NULL,          -- the human status page, linked from the strip
        Enabled    bit           NOT NULL CONSTRAINT DF_TechStatusServices_Enabled DEFAULT 1,
        SortOrder  int           NOT NULL CONSTRAINT DF_TechStatusServices_Sort DEFAULT 100,
        CONSTRAINT CK_TechStatusServices_Kind CHECK (Kind IN (N'statuspage'))
    );
END;

IF OBJECT_ID('dbo.TechServiceStatus', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.TechServiceStatus (
        ServiceId         int           NOT NULL CONSTRAINT PK_TechServiceStatus PRIMARY KEY
                                        CONSTRAINT FK_TechServiceStatus_Service REFERENCES dbo.TechStatusServices(Id) ON DELETE CASCADE,
        Indicator         nvarchar(12)  NOT NULL,   -- none | minor | major | critical | maintenance | unknown
        Description       nvarchar(200) NULL,       -- "All Systems Operational" / "Partial System Outage"
        IncidentName      nvarchar(300) NULL,       -- the newest unresolved incident, if any
        IncidentComponent nvarchar(120) NULL,       -- the first affected component ("Actions")
        IncidentStartedAt datetime2     NULL,       -- UTC
        IncidentUrl       nvarchar(400) NULL,
        CheckedAt         datetime2     NOT NULL,
        LastError         nvarchar(400) NULL
    );
END;

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

