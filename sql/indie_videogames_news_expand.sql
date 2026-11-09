-- Adds three enabled indie video game news feeds; repeated runs preserve existing source settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Video Games')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Video Games');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Video Games');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,1,@Category FROM (VALUES
  (N'https://indiegamesplus.com/feed/',N'Indie Games Plus'),
  (N'https://the-indie-in-former.com/feed/',N'The Indie Informer'),
  (N'https://indie-hive.com/feed/',N'Indie Hive')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
