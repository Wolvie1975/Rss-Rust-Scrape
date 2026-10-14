SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Previous DATE=(SELECT MAX(WeekendStart) FROM dbo.BoxOfficeWeekend WITH(UPDLOCK,HOLDLOCK));
 IF @Previous>@P1 THROW 50000,'Refusing older weekend chart',1;
 DELETE dbo.BoxOfficeWeekend;
 INSERT dbo.BoxOfficeWeekend(Rank,WeekendStart,MovieTitle,WeekendGross,TotalDomesticGross,WeeklyChangePercent,Theaters,MovieUrl,SourceUrl,FetchedAt)
 SELECT Rank,@P1,MovieTitle,WeekendGross,TotalDomesticGross,WeeklyChangePercent,Theaters,MovieUrl,@P3,SYSUTCDATETIME()
 FROM OPENJSON(@P2) WITH(Rank INT '$.rank',MovieTitle NVARCHAR(300) '$.title',WeekendGross BIGINT '$.weekendGross',TotalDomesticGross BIGINT '$.totalDomesticGross',WeeklyChangePercent FLOAT '$.weeklyChangePercent',Theaters INT '$.theaters',MovieUrl NVARCHAR(2048) '$.movieUrl');
 IF @@ROWCOUNT<>10 THROW 50000,'Expected ten box office rows',1;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
