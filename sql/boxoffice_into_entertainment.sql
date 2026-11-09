-- Fold box-office news into Entertainment. Apply to WebScraper with migration credentials.
-- Sources and Pages retain their identities, settings and ownership.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Entertainment INT=(SELECT ID FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Entertainment');
 DECLARE @BoxOffice INT=(SELECT ID FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'Box Office');
 IF @Entertainment IS NULL THROW 50001,'Entertainment category missing.',1;
 UPDATE dbo.Sources SET SourceCategoryId=@Entertainment WHERE SourceCategoryId=@BoxOffice;
 DELETE dbo.SourceCategories WHERE ID=@BoxOffice;
 IF EXISTS(SELECT 1 FROM dbo.SourceCategories WHERE Category_Name=N'Box Office')
  THROW 50001,'Box Office category merge incomplete.',1;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
