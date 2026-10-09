SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF COL_LENGTH('dbo.Movies','PosterCheckedAt') IS NULL ALTER TABLE dbo.Movies ADD PosterCheckedAt DATETIME2 NULL;
 IF COL_LENGTH('dbo.Movies','PosterRetryAfter') IS NULL ALTER TABLE dbo.Movies ADD PosterRetryAfter DATETIME2 NULL;
 IF COL_LENGTH('dbo.Movies','PosterLookupStatus') IS NULL ALTER TABLE dbo.Movies ADD PosterLookupStatus VARCHAR(30) NULL;
 IF COL_LENGTH('dbo.Movies','PosterLookupError') IS NULL ALTER TABLE dbo.Movies ADD PosterLookupError NVARCHAR(1000) NULL;
 IF COL_LENGTH('dbo.Movies','PosterSourceUrl') IS NULL ALTER TABLE dbo.Movies ADD PosterSourceUrl NVARCHAR(2048) NULL;
 IF OBJECT_ID('dbo.MoviePosterCache','U') IS NULL
 CREATE TABLE dbo.MoviePosterCache (
  SourceLinkId INT CONSTRAINT PK_MoviePosterCache PRIMARY KEY CONSTRAINT FK_MoviePosterCache_Link REFERENCES dbo.MovieSourceLinks(ID),
  TargetIdentity NVARCHAR(1000) NOT NULL,
  Metadata NVARCHAR(MAX) NOT NULL CONSTRAINT CK_MoviePosterCache_Metadata CHECK(ISJSON(Metadata)=1),
  PosterUrl NVARCHAR(2048) NULL,
  CheckedAt DATETIME2 NOT NULL,
  RetryAfter DATETIME2 NOT NULL,
  LastError NVARCHAR(1000) NULL
 );
 COMMIT TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
