-- Registers Xbox Wire disabled in Video Games for safe first collection.
-- Collect this exact URL, verify content/health, then enable only the newly registered source.
-- Repeated runs preserve existing categories, labels and enablement choices.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Video Games')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Video Games');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Video Games');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,0,@Category FROM (VALUES
  (N'https://news.xbox.com/en-us/feed/',N'Xbox Wire')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
