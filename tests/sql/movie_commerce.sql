-- Run only after explicit schema setup. All fixture writes roll back.
SET XACT_ABORT ON;
BEGIN TRANSACTION;
BEGIN TRY
 DECLARE @MovieId INT;
 INSERT dbo.Movies(Title,OriginalYear) VALUES(N'Commerce fixture',2026);
 SET @MovieId=SCOPE_IDENTITY();
 INSERT dbo.MovieCommerceLinks(MovieId,Provider,IdentityTitle,IdentityYear,SearchUrl)
 VALUES(@MovieId,'fandango',N'Commerce fixture',2026,'https://www.fandango.com/search?q=fixture');
 IF NOT EXISTS(SELECT 1 FROM dbo.MovieCommerce WHERE MovieId=@MovieId AND LinkKind='search' AND ActionLabel='Search Fandango') THROW 51000,'Missing search fallback',1;
 UPDATE dbo.MovieCommerceLinks SET DirectUrl='https://www.fandango.com/fixture-123/movie-overview',VerifiedUntil=DATEADD(day,1,SYSUTCDATETIME()) WHERE MovieId=@MovieId;
 IF NOT EXISTS(SELECT 1 FROM dbo.MovieCommerce WHERE MovieId=@MovieId AND LinkKind='direct' AND ActionLabel='Get tickets on Fandango') THROW 51000,'Missing direct Fandango link',1;
 UPDATE dbo.MovieCommerceLinks SET VerifiedUntil=DATEADD(day,-1,SYSUTCDATETIME()) WHERE MovieId=@MovieId;
 IF NOT EXISTS(SELECT 1 FROM dbo.MovieCommerce WHERE MovieId=@MovieId AND LinkKind='search') THROW 51000,'Expired link remained direct',1;
 UPDATE dbo.Movies SET OriginalYear=2016 WHERE ID=@MovieId;
 IF EXISTS(SELECT 1 FROM dbo.MovieCommerce WHERE MovieId=@MovieId) THROW 51000,'Stale identity remained visible',1;
 UPDATE dbo.Movies SET OriginalYear=2026 WHERE ID=@MovieId;
 INSERT dbo.MovieCommerceLinks(MovieId,Provider,IdentityTitle,IdentityYear,SearchUrl,DirectUrl,VerifiedUntil)
 VALUES(@MovieId,'fandango-at-home',N'Commerce fixture',2026,'https://athome.fandango.com/content/browse/search?searchString=fixture','https://athome.fandango.com/content/browse/details/fixture/123',DATEADD(day,1,SYSUTCDATETIME()));
 IF NOT EXISTS(SELECT 1 FROM dbo.MovieCommerce WHERE MovieId=@MovieId AND Provider='fandango-at-home' AND ActionLabel='View on Fandango at Home') THROW 51000,'Unverified purchase incorrectly labeled',1;
 UPDATE dbo.MovieCommerceLinks SET PurchaseVerified=1 WHERE MovieId=@MovieId AND Provider='fandango-at-home';
 IF NOT EXISTS(SELECT 1 FROM dbo.MovieCommerce WHERE MovieId=@MovieId AND ActionLabel='Buy on Fandango at Home') THROW 51000,'Verified purchase label missing',1;
 DELETE dbo.Movies WHERE ID=@MovieId;
 IF EXISTS(SELECT 1 FROM dbo.MovieCommerceLinks WHERE MovieId=@MovieId) THROW 51000,'Retention cascade failed',1;
 ROLLBACK TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
