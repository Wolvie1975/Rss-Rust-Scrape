-- Approved category cleanup. Apply in WebScraper with migration credentials.
-- Preserves source IDs, article ownership, labels and enablement.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @General INT=(SELECT ID FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'General News');
 DECLARE @More INT=(SELECT ID FROM dbo.SourceCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=N'More News');
 IF @General IS NOT NULL AND @More IS NOT NULL THROW 50001,'Both General News and More News exist; review before merging.',1;
 IF @General IS NOT NULL UPDATE dbo.SourceCategories SET Category_Name=N'More News' WHERE ID=@General;
 IF @General IS NULL AND @More IS NULL THROW 50001,'Expected catch-all category missing.',1;
 DECLARE @Sports INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Sports');
 DECLARE @Games INT=(SELECT ID FROM dbo.SourceCategories WHERE Category_Name=N'Video Games');
 IF @Sports IS NULL OR @Games IS NULL THROW 50001,'Expected destination categories missing.',1;
 IF (SELECT COUNT(*) FROM dbo.Sources WHERE (ID=7895 AND Url=N'https://awfulannouncing.com/') OR (ID=1 AND Url=N'https://www.polygon.com/') OR (ID=1768 AND Url=N'https://www.ign.com') OR (ID=8906 AND Url IN(N'https://www.theguardian.com/international/rss',N'https://www.theguardian.com/world/rss')))<>4
  THROW 50001,'Source identities differ from approved inventory.',1;
 IF EXISTS(SELECT 1 FROM dbo.Sources WHERE ID<>8906 AND Url=N'https://www.theguardian.com/world/rss')
  THROW 50001,'Guardian World feed already belongs to another source.',1;
 UPDATE dbo.Sources SET SourceCategoryId=@Sports WHERE ID=7895;
 UPDATE dbo.Sources SET SourceCategoryId=@Games WHERE ID IN(1,1768);
 UPDATE dbo.Sources SET Url=N'https://www.theguardian.com/world/rss' WHERE ID=8906;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
