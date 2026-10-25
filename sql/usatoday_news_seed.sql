-- USA TODAY results for a personal, noncommercial dashboard via Google News RSS.
-- Register disabled; preview/import and verify each source before enabling.
-- Existing source labels, categories and enablement are preserved on repeat.
-- No new categories, schema changes or scheduler restart are required.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Feeds TABLE(Category NVARCHAR(100), Label NVARCHAR(200), Url NVARCHAR(2048));
 INSERT @Feeds VALUES
  (N'Top Headlines', N'USA TODAY — Top Headlines (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fnews%2F+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'More News', N'USA TODAY — Money (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fmoney%2F+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'Sports', N'USA TODAY — Sports (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fsports%2F+-college+-ncaab+-motor+-%22Formula+1%22+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'Technology', N'USA TODAY — Technology (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Ftech%2F+-Wordle+-puzzle+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'Entertainment', N'USA TODAY — Entertainment (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fentertainment%2Fmusic%2F+OR+site%3Awww.usatoday.com%2Fstory%2Fentertainment%2Fcelebrities%2F+OR+site%3Awww.usatoday.com%2Fstory%2Fentertainment%2Fbooks%2F+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'TV/Film', N'USA TODAY — TV/Film (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fentertainment%2Fmovies%2F+OR+site%3Awww.usatoday.com%2Fstory%2Fentertainment%2Ftv%2F+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'Opinion', N'USA TODAY — Opinion (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fopinion%2F+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'F1 News', N'USA TODAY — F1 News (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fsports%2F+intitle%3AF1+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'Big 12 News', N'USA TODAY — Big 12 News (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fsports%2F+%22Big+12%22+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen'),
  (N'Men''s College Basketball News', N'USA TODAY — Men''s College Basketball News (Google News)', N'https://news.google.com/rss/search?q=site%3Awww.usatoday.com%2Fstory%2Fsports%2Fncaab%2F+when%3A30d&hl=en-US&gl=US&ceid=US%3Aen');
 IF EXISTS(SELECT 1 FROM @Feeds f WHERE NOT EXISTS
  (SELECT 1 FROM dbo.SourceCategories c WHERE c.Category_Name=f.Category))
  THROW 50001, 'A required USA TODAY category is missing.', 1;
 INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
 SELECT f.Url,f.Label,0,c.ID FROM @Feeds f
 JOIN dbo.SourceCategories c ON c.Category_Name=f.Category
 WHERE NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK)
  WHERE UrlHash=HASHBYTES('SHA2_256',f.Url));
 SELECT @@ROWCOUNT AS InsertedSources;
 SELECT s.ID,s.Label,s.Enabled,c.Category_Name FROM @Feeds f
 JOIN dbo.Sources s ON s.UrlHash=HASHBYTES('SHA2_256',f.Url)
 JOIN dbo.SourceCategories c ON c.ID=s.SourceCategoryId ORDER BY c.Category_Name;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
