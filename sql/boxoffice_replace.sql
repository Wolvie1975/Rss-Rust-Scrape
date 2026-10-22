SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Previous DATE=(SELECT MAX(WeekendStart) FROM dbo.BoxOfficeWeekend WITH(UPDLOCK,HOLDLOCK));
 IF @Previous>@P1 THROW 50000,'Refusing older weekend chart',1;
 DECLARE @Incoming TABLE (
  Rank INT PRIMARY KEY, MovieTitle NVARCHAR(300), WeekendGross BIGINT,
  TotalDomesticGross BIGINT, WeeklyChangePercent FLOAT, Theaters INT,
  MovieUrl NVARCHAR(2048), ResultStatus NVARCHAR(16),
  StatusSourceUrl NVARCHAR(2048), StatusAsOfUtc DATETIME2
 );
 INSERT @Incoming
 SELECT * FROM OPENJSON(@P2) WITH(
  Rank INT '$.rank',MovieTitle NVARCHAR(300) '$.title',WeekendGross BIGINT '$.weekendGross',
  TotalDomesticGross BIGINT '$.totalDomesticGross',WeeklyChangePercent FLOAT '$.weeklyChangePercent',
  Theaters INT '$.theaters',MovieUrl NVARCHAR(2048) '$.movieUrl',ResultStatus NVARCHAR(16) '$.resultStatus',
  StatusSourceUrl NVARCHAR(2048) '$.statusSourceUrl',StatusAsOfUtc DATETIME2 '$.statusAsOfUtc');
 IF (SELECT COUNT(*) FROM @Incoming)<>10 THROW 50000,'Expected ten box office rows',1;
 IF EXISTS(SELECT 1 FROM @Incoming WHERE ResultStatus IS NULL OR ResultStatus NOT IN ('estimated','final','unknown'))
  THROW 50000,'Invalid box office status',1;
 -- Match movie identity rather than mutable rank. Reject the entire snapshot on regression.
 IF @Previous=@P1 AND EXISTS(
  SELECT 1 FROM dbo.BoxOfficeWeekend old JOIN @Incoming n ON n.MovieUrl=old.MovieUrl
  WHERE (old.ResultStatus='final' AND n.ResultStatus<>'final')
     OR (old.StatusAsOfUtc IS NOT NULL AND (n.StatusAsOfUtc<old.StatusAsOfUtc OR n.StatusAsOfUtc IS NULL)))
  THROW 50000,'Refusing regressed box office status or publication time',1;
 DECLARE @Posters TABLE(MovieUrl NVARCHAR(2048),PosterUrl NVARCHAR(2048),PosterLookupStatus VARCHAR(20));
 INSERT @Posters SELECT MovieUrl,PosterUrl,PosterLookupStatus FROM dbo.BoxOfficeWeekend;
 DELETE dbo.BoxOfficeWeekend;
 DECLARE @FetchedAt DATETIME2=SYSUTCDATETIME();
 INSERT dbo.BoxOfficeWeekend(Rank,WeekendStart,MovieTitle,WeekendGross,TotalDomesticGross,WeeklyChangePercent,Theaters,MovieUrl,SourceUrl,FetchedAt,ResultStatus,StatusSourceUrl,StatusAsOfUtc,PosterUrl,PosterLookupStatus)
 SELECT n.Rank,@P1,n.MovieTitle,n.WeekendGross,n.TotalDomesticGross,n.WeeklyChangePercent,n.Theaters,n.MovieUrl,@P3,@FetchedAt,n.ResultStatus,n.StatusSourceUrl,n.StatusAsOfUtc,p.PosterUrl,p.PosterLookupStatus
 FROM @Incoming n OUTER APPLY(SELECT TOP(1) PosterUrl,PosterLookupStatus FROM @Posters p WHERE p.MovieUrl=n.MovieUrl) p;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
