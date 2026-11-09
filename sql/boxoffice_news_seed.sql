-- Configuration only: existing news tables. Registers disabled feeds and preserves
-- operator settings on repeated setup. Enable separately for production collection.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Entertainment')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Entertainment');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Entertainment');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,0,@Category
 FROM (VALUES
  (N'https://deadline.com/v/box-office/feed/',N'Deadline — Box Office'),
  (N'https://variety.com/v/film/box-office/feed/',N'Variety — Box Office')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
