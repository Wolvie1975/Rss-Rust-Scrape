-- Adds three additional enabled box-office news feeds; repeated runs preserve existing source settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Box Office')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Box Office');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Box Office');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,1,@Category FROM (VALUES
  (N'https://variety.com/v/film/box-office/feed/',N'Variety — Box Office'),
  (N'https://www.hollywoodreporter.com/t/box-office/feed/',N'The Hollywood Reporter — Box Office'),
  (N'https://www.indiewire.com/c/box-office/feed/',N'IndieWire — Box Office')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
