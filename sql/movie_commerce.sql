-- Additive movie retailer cache. Run explicitly with migration credentials.
SET XACT_ABORT ON;
BEGIN TRANSACTION;
IF OBJECT_ID('dbo.MovieCommerceLinks','U') IS NULL
CREATE TABLE dbo.MovieCommerceLinks (
    MovieId INT NOT NULL CONSTRAINT FK_MovieCommerceLinks_Movie REFERENCES dbo.Movies(ID) ON DELETE CASCADE,
    Provider VARCHAR(30) NOT NULL,
    IdentityTitle NVARCHAR(500) NOT NULL,
    IdentityYear SMALLINT NULL,
    IdentityImdbId VARCHAR(20) NULL,
    SearchUrl NVARCHAR(MAX) NOT NULL,
    DirectUrl NVARCHAR(2048) NULL,
    PurchaseVerified BIT NOT NULL CONSTRAINT DF_MovieCommerceLinks_Purchase DEFAULT 0,
    VerifiedUntil DATETIME2 NULL,
    CheckedAt DATETIME2 NULL,
    RetryAfter DATETIME2 NULL,
    LastError NVARCHAR(1000) NULL,
    CONSTRAINT PK_MovieCommerceLinks PRIMARY KEY(MovieId,Provider),
    CONSTRAINT CK_MovieCommerceLinks_Provider CHECK(Provider IN ('fandango','fandango-at-home')),
    CONSTRAINT CK_MovieCommerceLinks_Direct CHECK(DirectUrl IS NULL OR
      (Provider='fandango' AND (DirectUrl LIKE 'https://www.fandango.com/%/movie-overview' OR DirectUrl LIKE 'https://fandango.com/%/movie-overview')) OR
      (Provider='fandango-at-home' AND DirectUrl LIKE 'https://athome.fandango.com/content/browse/details/%'))
);
COMMIT TRANSACTION;
