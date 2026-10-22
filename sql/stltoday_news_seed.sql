-- Register the verified mixed local/national feed under General, disabled first.
-- Collect the exact URL, verify content and source health, then enable the new row.
-- Repeated applications preserve existing labels, categories and enablement.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Category INT=(SELECT ID FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'General');
 IF @Category IS NULL THROW 50001, 'General source category is required.', 1;
 DECLARE @Url NVARCHAR(2048)=N'https://www.stltoday.com/search/?f=rss&t=article&l=50&s=start_time&sd=desc';
 IF NOT EXISTS(SELECT 1 FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',@Url))
  INSERT dbo.Sources(Url,Label,Enabled,SourceCategoryId)
  VALUES(@Url,N'St. Louis Post-Dispatch — STLtoday',0,@Category);
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
