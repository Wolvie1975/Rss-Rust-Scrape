-- Additive schema; run only with migration credentials. Channels without a category stay NULL.
IF OBJECT_ID('dbo.YoutubeCategories','U') IS NULL
CREATE TABLE dbo.YoutubeCategories (
 ID INT IDENTITY(1,1) CONSTRAINT PK_YoutubeCategories PRIMARY KEY,
 Category_Name NVARCHAR(100) NOT NULL,
 DateAdded DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 CONSTRAINT UQ_YoutubeCategories_Category_Name UNIQUE(Category_Name)
);
IF COL_LENGTH('dbo.YoutubeVideoFeed','YoutubeCategoryId') IS NULL
 ALTER TABLE dbo.YoutubeVideoFeed ADD YoutubeCategoryId INT NULL
  CONSTRAINT FK_YoutubeVideoFeed_YoutubeCategories REFERENCES dbo.YoutubeCategories(ID);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE name='IX_YoutubeVideoFeed_YoutubeCategoryId' AND object_id=OBJECT_ID('dbo.YoutubeVideoFeed'))
 EXEC(N'CREATE INDEX IX_YoutubeVideoFeed_YoutubeCategoryId ON dbo.YoutubeVideoFeed(YoutubeCategoryId)');
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
 GRANT SELECT ON dbo.YoutubeCategories TO WebScraperIngest;
