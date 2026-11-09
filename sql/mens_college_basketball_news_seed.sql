-- Registers men's college basketball news feeds disabled for safe first collection.
-- Collect these exact URLs, verify content/health, then enable only newly registered sources.
-- Repeated runs preserve existing categories, labels and enablement choices.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Men''s College Basketball News')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Men''s College Basketball News');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Men''s College Basketball News');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,0,@Category FROM (VALUES
  (N'https://www.espn.com/espn/rss/ncb/news',N'ESPN — Men’s College Basketball'),
  (N'https://www.cbssports.com/rss/headlines/college-basketball/',N'CBS Sports — Men’s College Basketball'),
  (N'https://www.zagsblog.com/feed/',N'ZAGSBLOG — College Basketball & Recruiting'),
  (N'https://www.heartlandcollegesports.com/category/big-12-basketball/feed/',N'Heartland College Sports — Big 12 Basketball')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
