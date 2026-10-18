-- Adds five enabled Big 12 news feeds; repeated runs preserve existing source settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Big 12 News')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Big 12 News');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Big 12 News');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,1,@Category FROM (VALUES
  (N'https://big12sports.com/rss.aspx',N'Big 12 Conference — All Sports'),
  (N'https://www.heartlandcollegesports.com/feed/',N'Heartland College Sports'),
  (N'https://big12sports.com/rss.aspx?path=football',N'Big 12 Conference — Football'),
  (N'https://big12sports.com/rss.aspx?path=mbball',N'Big 12 Conference — Men''s Basketball'),
  (N'https://big12sports.com/rss.aspx?path=baseball',N'Big 12 Conference — Baseball')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
