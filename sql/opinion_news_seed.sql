-- Register dedicated opinion/commentary feeds disabled until first collection is verified.
-- Repeats preserve existing labels, category assignments and enablement choices.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Opinion')
  INSERT dbo.SourceCategories(Category_Name) VALUES(N'Opinion');
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Opinion');
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT s.Url,s.Label,0,@Category FROM (VALUES
  (N'https://www.theguardian.com/sport/us-sport+tone/comment/rss',N'The Guardian — U.S. Sports Opinion'),
  (N'https://www.indiewire.com/c/film/commentary/feed/',N'IndieWire — Film Commentary'),
  (N'https://www.theguardian.com/us/commentisfree/rss',N'The Guardian — U.S. & World Opinion'),
  (N'https://www.lemonde.fr/en/opinion/rss_full.xml',N'Le Monde — World Opinion'),
  (N'https://www.eurogamer.net/feed/opinions',N'Eurogamer — Opinions')
 ) s(Url,Label)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',s.Url));
 SELECT @@ROWCOUNT AS InsertedSources;
 SELECT s.ID,s.Url,s.Label,s.Enabled FROM dbo.Sources s WHERE s.SourceCategoryId=@Category ORDER BY s.ID;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
