-- Parameters: sport, league, event ID, normalized JSON, summary URL, box URL, error.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SportsBoxScoreCache WITH(UPDLOCK,HOLDLOCK)
  WHERE Sport=@P1 AND League=@P2 AND EventId=@P3)
  INSERT dbo.SportsBoxScoreCache(Sport,League,EventId,Availability,LastAttemptAt,NextAttemptAt)
  VALUES(@P1,@P2,@P3,'unavailable',SYSUTCDATETIME(),SYSUTCDATETIME());
 UPDATE dbo.SportsBoxScoreCache SET
  Availability=CASE WHEN @P4 IS NOT NULL THEN 'available' WHEN Payload IS NOT NULL THEN 'available'
   WHEN @P7 IS NOT NULL THEN 'error' ELSE 'unavailable' END,
  Payload=COALESCE(@P4,Payload),SummaryUrl=COALESCE(@P5,SummaryUrl),BoxScoreUrl=COALESCE(@P6,BoxScoreUrl),
  FetchedAt=CASE WHEN @P4 IS NOT NULL THEN SYSUTCDATETIME() ELSE FetchedAt END,
  LastAttemptAt=SYSUTCDATETIME(),LastError=@P7,
  NextAttemptAt=DATEADD(hour,CASE WHEN @P7 IS NOT NULL THEN 6 WHEN @P4 IS NULL THEN 24 ELSE 6 END,SYSUTCDATETIME())
 WHERE Sport=@P1 AND League=@P2 AND EventId=@P3;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
