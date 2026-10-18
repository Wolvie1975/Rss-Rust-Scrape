-- Adds six enabled headline feeds; repeated runs preserve existing source settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Top Headlines')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Top Headlines');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Top Headlines');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,1,@Category FROM (VALUES
  (N'https://feeds.nbcnews.com/nbcnews/public/news',N'NBC News'),
  (N'https://www.cbsnews.com/latest/rss/main',N'CBS News'),
  (N'https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml',N'The New York Times'),
  (N'https://www.theguardian.com/international/rss',N'The Guardian'),
  (N'https://www.aljazeera.com/xml/rss/all.xml',N'Al Jazeera'),
  (N'https://feeds.skynews.com/feeds/rss/home.xml',N'Sky News')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
