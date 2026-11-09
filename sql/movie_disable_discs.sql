-- Explicit user preference: retain theatrical/digital coverage without physical discs.
-- Apply after movie_releases.sql. Does not change provider enablement.
-- Existing withdrawn identities remain available for audit and normal retention.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Source INT=(SELECT ID FROM dbo.MovieReleaseSources WITH(UPDLOCK,HOLDLOCK) WHERE SourceKey='dvdsreleasedates');
 IF @Source IS NULL THROW 50001,'DVDs Release Dates source is missing.',1;
 UPDATE dbo.MovieReleaseSources SET CollectDisc=0 WHERE ID=@Source;
 UPDATE r SET ReleaseStatus='withdrawn'
 FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId
 WHERE l.MovieReleaseSourceId=@Source AND r.ReleaseType='disc' AND r.ReleaseStatus='scheduled';
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
