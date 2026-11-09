SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.F1Source WITH(UPDLOCK,HOLDLOCK) WHERE ID=1 AND Enabled=1)
  THROW 51100,'F1 source is disabled or missing',1;
 IF EXISTS(SELECT 1 FROM dbo.F1Snapshot WITH(UPDLOCK,HOLDLOCK) WHERE SourceId=1
           AND (Season>@P1 OR (Season=@P1 AND StandingsRound>@P2)))
  THROW 51101,'F1 snapshot would regress season or standings round',1;
 UPDATE dbo.F1Snapshot SET Season=@P1,StandingsRound=@P2,Payload=@P3,FetchedAt=SYSUTCDATETIME() WHERE SourceId=1;
 IF @@ROWCOUNT=0
  INSERT dbo.F1Snapshot(SourceId,Season,StandingsRound,Payload,FetchedAt) VALUES(1,@P1,@P2,@P3,SYSUTCDATETIME());
 UPDATE dbo.F1Source SET LastSuccessAt=SYSUTCDATETIME(),LastError=NULL WHERE ID=1;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
