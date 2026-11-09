CREATE OR ALTER PROCEDURE dbo.UpsertArticleIdentity
 @SourceUrl NVARCHAR(2048), @TargetUrl NVARCHAR(2048), @Title NVARCHAR(500),
 @Description NVARCHAR(MAX), @Published DATETIME2, @ImageUrl NVARCHAR(2048),
 @Aliases NVARCHAR(MAX), @Verified BIT, @DryRun BIT=0
WITH EXECUTE AS OWNER
AS
BEGIN
 SET NOCOUNT ON; SET XACT_ABORT ON;
 SET @Description=NULLIF(LTRIM(RTRIM(@Description)),N'');
 SET @ImageUrl=NULLIF(LTRIM(RTRIM(@ImageUrl)),N'');
 BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @lock INT;
 -- Serialize source lookup and the global Pages URL uniqueness boundary together.
 EXEC @lock=sys.sp_getapplock @Resource=N'WebScraper.ArticleIdentity',@LockMode='Exclusive',@LockOwner='Transaction',@LockTimeout=15000;
 IF @lock<0 THROW 50000,'Article identity lock timed out',1;
 DECLARE @sid INT=(SELECT ID FROM dbo.Sources WITH(UPDLOCK,HOLDLOCK) WHERE UrlHash=HASHBYTES('SHA2_256',@SourceUrl));
 IF @sid IS NULL AND @DryRun=0 BEGIN
  INSERT dbo.Sources(Url) VALUES(@SourceUrl); SET @sid=SCOPE_IDENTITY();
 END;
 DECLARE @urls TABLE(Url NVARCHAR(2048),Hash BINARY(32) PRIMARY KEY);
 INSERT @urls(Url,Hash) SELECT @TargetUrl,HASHBYTES('SHA2_256',@TargetUrl);
 IF @Verified=1
  INSERT @urls(Url,Hash)
  SELECT MIN(CONVERT(NVARCHAR(2048),value)),HASHBYTES('SHA2_256',CONVERT(NVARCHAR(2048),value))
  FROM OPENJSON(@Aliases)
  WHERE type=1 AND DATALENGTH(value) BETWEEN 2 AND 4096
    AND HASHBYTES('SHA2_256',CONVERT(NVARCHAR(2048),value))<>HASHBYTES('SHA2_256',@TargetUrl)
  GROUP BY HASHBYTES('SHA2_256',CONVERT(NVARCHAR(2048),value));
 DECLARE @ids TABLE(ID INT PRIMARY KEY);
 INSERT @ids SELECT p.ID FROM dbo.Pages p WITH(UPDLOCK,HOLDLOCK)
 WHERE p.SourceId=@sid AND (p.UrlHash IN(SELECT Hash FROM @urls)
  OR p.ID IN(SELECT PageId FROM dbo.ArticleUrlAliases WITH(UPDLOCK,HOLDLOCK) WHERE SourceId=@sid AND UrlHash IN(SELECT Hash FROM @urls)));
 DECLARE @survivor INT=(SELECT MIN(ID) FROM @ids),@matches INT=(SELECT COUNT(*) FROM @ids),@outcome NVARCHAR(100)=N'updated';
 -- Global hash ownership must never silently reassign an article to another source.
 IF EXISTS(SELECT 1 FROM dbo.Pages WITH(UPDLOCK,HOLDLOCK) WHERE SourceId<>@sid AND UrlHash IN(SELECT Hash FROM @urls))
  SET @outcome=N'skipped-cross-source-collision';
 IF @matches>1 AND EXISTS(SELECT 1 FROM sys.foreign_keys
    WHERE referenced_object_id=OBJECT_ID('dbo.Pages')
      AND parent_object_id NOT IN(OBJECT_ID('dbo.ArticleUrlAliases'),OBJECT_ID('dbo.ArticleIdRedirects')))
  SET @outcome=N'skipped-unhandled-foreign-reference';
 DECLARE @candidateIds NVARCHAR(MAX)=(SELECT ID FROM @ids ORDER BY ID FOR JSON PATH);
 IF @DryRun=1 OR @outcome LIKE N'skipped-%'
 BEGIN
  SELECT CASE WHEN @DryRun=1 AND @outcome NOT LIKE N'skipped-%' THEN N'preview' ELSE @outcome END AS Outcome,
    @survivor AS PageId,@candidateIds AS Candidates;
  COMMIT; RETURN;
 END;
 IF @matches>1 AND @Verified=0 THROW 50000,'Unverified article merge rejected',1;
 IF @matches>1 BEGIN
  -- Archive every retired ID and metadata before removing its public card.
  INSERT dbo.ArticleIdRedirects(RetiredPageId,PageId,SourceId,OriginalPageJson,EvidenceJson)
   SELECT p.ID,@survivor,@sid,(SELECT p.* FOR JSON PATH,WITHOUT_ARRAY_WRAPPER),@Aliases
   FROM dbo.Pages p WHERE p.ID IN(SELECT ID FROM @ids) AND p.ID<>@survivor;
  UPDATE dbo.ArticleIdRedirects SET PageId=@survivor WHERE PageId IN(SELECT ID FROM @ids WHERE ID<>@survivor);
  UPDATE dbo.ArticleUrlAliases SET PageId=@survivor WHERE SourceId=@sid AND PageId IN(SELECT ID FROM @ids);
  UPDATE p SET
   Description=COALESCE(p.Description,(SELECT TOP(1) Description FROM dbo.Pages WHERE ID IN(SELECT ID FROM @ids) AND Description IS NOT NULL ORDER BY ID)),
   ImageUrl=COALESCE(p.ImageUrl,(SELECT TOP(1) ImageUrl FROM dbo.Pages WHERE ID IN(SELECT ID FROM @ids) AND ImageUrl IS NOT NULL ORDER BY ID)),
   Published=(SELECT MIN(Published) FROM dbo.Pages WHERE ID IN(SELECT ID FROM @ids))
  FROM dbo.Pages p WHERE p.ID=@survivor;
  DELETE dbo.Pages WHERE ID IN(SELECT ID FROM @ids WHERE ID<>@survivor);
  SET @outcome=N'merged';
 END;
 IF @survivor IS NULL BEGIN
  INSERT dbo.Pages(Url,Title,Description,Published,ImageUrl,SourceId)
   VALUES(@TargetUrl,@Title,@Description,@Published,@ImageUrl,@sid);
  SET @survivor=SCOPE_IDENTITY(); SET @outcome=N'inserted';
 END ELSE BEGIN
  UPDATE dbo.Pages SET Url=CASE WHEN @Verified=1 THEN @TargetUrl ELSE Url END,Title=@Title,
   Description=COALESCE(@Description,Description),ImageUrl=COALESCE(@ImageUrl,ImageUrl),
   Published=CASE WHEN Published IS NULL OR @Published<Published THEN COALESCE(@Published,Published) ELSE Published END
  WHERE ID=@survivor AND SourceId=@sid;
 END;
 UPDATE a SET PageId=@survivor,Evidence=CASE WHEN @Verified=1 THEN N'redirect/canonical' ELSE a.Evidence END,
   LastCheckedAt=CASE WHEN @Verified=1 THEN SYSUTCDATETIME() ELSE a.LastCheckedAt END
 FROM dbo.ArticleUrlAliases a JOIN @urls u ON a.UrlHash=u.Hash WHERE a.SourceId=@sid;
 INSERT dbo.ArticleUrlAliases(SourceId,Url,PageId,Evidence,LastCheckedAt)
  SELECT @sid,u.Url,@survivor,CASE WHEN @Verified=1 THEN N'redirect/canonical' ELSE N'collected-url' END,
    CASE WHEN @Verified=1 THEN SYSUTCDATETIME() END
  FROM @urls u WHERE NOT EXISTS(SELECT 1 FROM dbo.ArticleUrlAliases a WHERE a.SourceId=@sid AND a.UrlHash=u.Hash);
 SELECT @outcome AS Outcome,@survivor AS PageId,@candidateIds AS Candidates;
 COMMIT;
 END TRY
 BEGIN CATCH
  IF @@TRANCOUNT>0 ROLLBACK;
  THROW;
 END CATCH;
END;
