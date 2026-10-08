-- Additive, repeatable migration. Execute against WebScraper; no GO separators.
SET XACT_ABORT ON;
BEGIN TRY
    BEGIN TRANSACTION;

    IF OBJECT_ID('dbo.MovieReleaseSources', 'U') IS NULL
    CREATE TABLE dbo.MovieReleaseSources (
        ID INT IDENTITY(1,1) CONSTRAINT PK_MovieReleaseSources PRIMARY KEY,
        SourceKey VARCHAR(50) NOT NULL CONSTRAINT UQ_MovieReleaseSources_Key UNIQUE,
        Name NVARCHAR(200) NOT NULL,
        Url NVARCHAR(2048) NOT NULL,
        Enabled BIT NOT NULL CONSTRAINT DF_MovieReleaseSources_Enabled DEFAULT 0,
        LastScrapedAt DATETIME2 NULL,
        LastError NVARCHAR(1000) NULL,
        CreatedAt DATETIME2 NOT NULL CONSTRAINT DF_MovieReleaseSources_Created DEFAULT SYSUTCDATETIME()
    );

    IF OBJECT_ID('dbo.Movies', 'U') IS NULL
    CREATE TABLE dbo.Movies (
        ID INT IDENTITY(1,1) CONSTRAINT PK_Movies PRIMARY KEY,
        Title NVARCHAR(500) NOT NULL,
        OriginalYear SMALLINT NULL,
        ImdbId VARCHAR(20) NULL,
        TmdbId INT NULL,
        Description NVARCHAR(MAX) NULL,
        PosterUrl NVARCHAR(2048) NULL,
        Certification NVARCHAR(20) NULL,
        RuntimeMinutes SMALLINT NULL,
        FirstSeenAt DATETIME2 NOT NULL CONSTRAINT DF_Movies_FirstSeen DEFAULT SYSUTCDATETIME(),
        LastSeenAt DATETIME2 NOT NULL CONSTRAINT DF_Movies_LastSeen DEFAULT SYSUTCDATETIME(),
        CONSTRAINT CK_Movies_Year CHECK (OriginalYear IS NULL OR OriginalYear BETWEEN 1800 AND 9999),
        CONSTRAINT CK_Movies_Runtime CHECK (RuntimeMinutes IS NULL OR RuntimeMinutes > 0),
        CONSTRAINT CK_Movies_Tmdb CHECK (TmdbId IS NULL OR TmdbId > 0)
    );

    IF OBJECT_ID('dbo.MovieSourceLinks', 'U') IS NULL
    CREATE TABLE dbo.MovieSourceLinks (
        ID INT IDENTITY(1,1) CONSTRAINT PK_MovieSourceLinks PRIMARY KEY,
        MovieId INT NOT NULL CONSTRAINT FK_MovieSourceLinks_Movies REFERENCES dbo.Movies(ID),
        MovieReleaseSourceId INT NOT NULL CONSTRAINT FK_MovieSourceLinks_Sources REFERENCES dbo.MovieReleaseSources(ID),
        ExternalMovieId NVARCHAR(200) COLLATE Latin1_General_100_BIN2 NOT NULL,
        Url NVARCHAR(2048) NOT NULL,
        FirstSeenAt DATETIME2 NOT NULL CONSTRAINT DF_MovieSourceLinks_FirstSeen DEFAULT SYSUTCDATETIME(),
        LastSeenAt DATETIME2 NOT NULL CONSTRAINT DF_MovieSourceLinks_LastSeen DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_MovieSourceLinks_External UNIQUE (MovieReleaseSourceId, ExternalMovieId),
        CONSTRAINT CK_MovieSourceLinks_External CHECK (LEN(LTRIM(RTRIM(ExternalMovieId))) > 0)
    );

    IF OBJECT_ID('dbo.MovieReleases', 'U') IS NULL
    CREATE TABLE dbo.MovieReleases (
        ID INT IDENTITY(1,1) CONSTRAINT PK_MovieReleases PRIMARY KEY,
        MovieSourceLinkId INT NOT NULL CONSTRAINT FK_MovieReleases_SourceLink REFERENCES dbo.MovieSourceLinks(ID),
        -- Stable within a source movie; excludes date so rescheduling is an UPDATE.
        ReleaseKey NVARCHAR(200) COLLATE Latin1_General_100_BIN2 NOT NULL,
        CountryCode CHAR(2) NOT NULL CONSTRAINT DF_MovieReleases_Country DEFAULT 'US',
        ReleaseType VARCHAR(30) NOT NULL,
        ReleaseDate DATE NULL,
        DateStatus VARCHAR(12) NOT NULL CONSTRAINT DF_MovieReleases_DateStatus DEFAULT 'tbd',
        ReleaseStatus VARCHAR(12) NOT NULL CONSTRAINT DF_MovieReleases_Status DEFAULT 'scheduled',
        Platform NVARCHAR(100) NULL,
        Format NVARCHAR(100) NULL,
        ReleaseScope NVARCHAR(100) NULL,
        Distributor NVARCHAR(200) NULL,
        Edition NVARCHAR(200) NULL,
        IsReRelease BIT NOT NULL CONSTRAINT DF_MovieReleases_ReRelease DEFAULT 0,
        SourceUrl NVARCHAR(2048) NOT NULL,
        Notes NVARCHAR(1000) NULL,
        FirstSeenAt DATETIME2 NOT NULL CONSTRAINT DF_MovieReleases_FirstSeen DEFAULT SYSUTCDATETIME(),
        LastSeenAt DATETIME2 NOT NULL CONSTRAINT DF_MovieReleases_LastSeen DEFAULT SYSUTCDATETIME(),
        CONSTRAINT UQ_MovieReleases_Key UNIQUE (MovieSourceLinkId, ReleaseKey),
        CONSTRAINT CK_MovieReleases_Key CHECK (LEN(LTRIM(RTRIM(ReleaseKey))) > 0),
        CONSTRAINT CK_MovieReleases_US CHECK (CountryCode = 'US'),
        CONSTRAINT CK_MovieReleases_Type CHECK (ReleaseType IN
            ('theatrical', 'digital', 'digital_purchase', 'digital_rental', 'subscription', 'disc')),
        CONSTRAINT CK_MovieReleases_Date CHECK (
            (DateStatus = 'tbd' AND ReleaseDate IS NULL) OR
            (DateStatus IN ('announced', 'estimated') AND ReleaseDate IS NOT NULL)),
        CONSTRAINT CK_MovieReleases_Status CHECK (ReleaseStatus IN ('scheduled', 'cancelled', 'withdrawn')),
        CONSTRAINT CK_MovieReleases_Platform CHECK (
            ReleaseType <> 'subscription' OR
            (Platform IS NOT NULL AND LEN(LTRIM(RTRIM(Platform))) > 0))
    );

    IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE object_id = OBJECT_ID('dbo.Movies') AND name = 'UX_Movies_ImdbId')
        CREATE UNIQUE INDEX UX_Movies_ImdbId ON dbo.Movies(ImdbId) WHERE ImdbId IS NOT NULL;
    IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE object_id = OBJECT_ID('dbo.Movies') AND name = 'UX_Movies_TmdbId')
        CREATE UNIQUE INDEX UX_Movies_TmdbId ON dbo.Movies(TmdbId) WHERE TmdbId IS NOT NULL;
    IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE object_id = OBJECT_ID('dbo.MovieSourceLinks') AND name = 'IX_MovieSourceLinks_MovieId')
        CREATE INDEX IX_MovieSourceLinks_MovieId ON dbo.MovieSourceLinks(MovieId);
    IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE object_id = OBJECT_ID('dbo.MovieReleases') AND name = 'IX_MovieReleases_Weekly')
        CREATE INDEX IX_MovieReleases_Weekly ON dbo.MovieReleases(ReleaseDate, ReleaseType)
            INCLUDE (MovieSourceLinkId, CountryCode, DateStatus, ReleaseStatus, Platform, Format);

    COMMIT TRANSACTION;
END TRY
BEGIN CATCH
    IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION;
    THROW;
END CATCH;
