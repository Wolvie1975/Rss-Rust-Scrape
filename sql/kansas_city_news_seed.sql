-- Registers Kansas City local news feeds disabled for safe first collection.
-- Collect these exact URLs, verify content/health, then enable only newly registered sources.
-- Repeated runs preserve existing categories, labels and enablement choices.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Kansas City News')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Kansas City News');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Kansas City News');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,0,@Category FROM (VALUES
  (N'https://www.kshb.com/news/local-news.rss',N'KSHB 41 — Local News'),
  (N'https://flatlandkc.org/feed/',N'Flatland — Kansas City PBS'),
  (N'https://www.thepitchkc.com/category/food-drink/feed/',N'The Pitch — Kansas City Food & Drink'),
  (N'https://www.kcur.org/tags/kansas-city.rss',N'KCUR — Kansas City'),
  (N'https://thebeaconnews.org/stories/category/kansas-city/feed/',N'The Beacon — Kansas City'),
  (N'https://www.kctv5.com/arc/outboundfeeds/rss/?outputType=xml',N'KCTV5'),
  (N'https://www.kmbc.com/local-news-rss',N'KMBC 9')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
