-- Additive scraper-owned migration. Apply before deploying the new ingestion binary.
IF OBJECT_ID('dbo.ArticleUrlAliases','U') IS NULL
CREATE TABLE dbo.ArticleUrlAliases (
 SourceId INT NOT NULL REFERENCES dbo.Sources(ID),
 Url NVARCHAR(2048) NOT NULL,
 UrlHash AS CAST(HASHBYTES('SHA2_256',Url) AS BINARY(32)) PERSISTED NOT NULL,
 PageId INT NOT NULL REFERENCES dbo.Pages(ID) ON DELETE CASCADE,
 Evidence NVARCHAR(80) NOT NULL,
 LastCheckedAt DATETIME2 NULL,
 FirstSeenAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 CONSTRAINT PK_ArticleUrlAliases PRIMARY KEY(SourceId,UrlHash)
);
IF OBJECT_ID('dbo.ArticleIdRedirects','U') IS NULL
CREATE TABLE dbo.ArticleIdRedirects (
 RetiredPageId INT NOT NULL PRIMARY KEY,
 PageId INT NOT NULL REFERENCES dbo.Pages(ID) ON DELETE CASCADE,
 SourceId INT NOT NULL,
 OriginalPageJson NVARCHAR(MAX) NOT NULL CHECK(ISJSON(OriginalPageJson)=1),
 EvidenceJson NVARCHAR(MAX) NOT NULL CHECK(ISJSON(EvidenceJson)=1),
 MergedAt DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME(),
 CONSTRAINT CK_ArticleIdRedirects_Different CHECK(RetiredPageId<>PageId)
);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.ArticleUrlAliases') AND name='IX_ArticleUrlAliases_Page')
 CREATE INDEX IX_ArticleUrlAliases_Page ON dbo.ArticleUrlAliases(PageId);
IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.ArticleIdRedirects') AND name='IX_ArticleIdRedirects_Page')
 CREATE INDEX IX_ArticleIdRedirects_Page ON dbo.ArticleIdRedirects(PageId);
INSERT dbo.ArticleUrlAliases(SourceId,Url,PageId,Evidence)
 SELECT p.SourceId,p.Url,p.ID,N'stored-url' FROM dbo.Pages p
 WHERE NOT EXISTS(SELECT 1 FROM dbo.ArticleUrlAliases a WHERE a.SourceId=p.SourceId AND a.UrlHash=p.UrlHash);
