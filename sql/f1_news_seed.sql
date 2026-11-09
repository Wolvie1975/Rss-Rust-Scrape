-- Adds four enabled F1 news feeds; repeated runs preserve existing source settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'F1 News')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'F1 News');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'F1 News');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,1,@Category FROM (VALUES
  (N'https://feeds.bbci.co.uk/sport/formula1/rss.xml',N'BBC Sport — F1'),
  (N'https://www.autosport.com/rss/f1/news/',N'Autosport — F1'),
  (N'https://www.motorsport.com/rss/f1/news/',N'Motorsport.com — F1'),
  (N'https://www.theguardian.com/sport/formulaone/rss',N'The Guardian — F1')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
