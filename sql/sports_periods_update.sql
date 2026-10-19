-- Compare-and-swap prevents a label repair from overwriting a concurrent full refresh.
UPDATE dbo.SportsBoxScoreCache SET Payload=@P5,FetchedAt=SYSUTCDATETIME(),
 LastAttemptAt=SYSUTCDATETIME(),NextAttemptAt=DATEADD(hour,6,SYSUTCDATETIME()),LastError=NULL
WHERE Sport=@P1 AND League=@P2 AND EventId=@P3 AND Availability='available'
 AND HASHBYTES('SHA2_256',Payload)=HASHBYTES('SHA2_256',@P4);
