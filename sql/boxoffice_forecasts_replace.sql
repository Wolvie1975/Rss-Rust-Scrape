SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Kind VARCHAR(30),@Published DATETIME2,@RowPayload NVARCHAR(MAX);
 DECLARE snapshots CURSOR LOCAL FAST_FORWARD FOR
 SELECT Kind,Published,RowPayload FROM OPENJSON(@P1) WITH
 (Kind VARCHAR(30) '$.kind',Published DATETIME2 '$.published',RowPayload NVARCHAR(MAX) '$.rows' AS JSON);
 OPEN snapshots;
 FETCH NEXT FROM snapshots INTO @Kind,@Published,@RowPayload;
 WHILE @@FETCH_STATUS=0
 BEGIN
  IF NOT EXISTS(SELECT 1 FROM dbo.BoxOfficeForecastSources WITH(UPDLOCK,HOLDLOCK) WHERE Kind=@Kind)
   THROW 50000,'Unknown forecast kind',1;
  IF EXISTS(SELECT 1 FROM dbo.BoxOfficeForecastSources WHERE Kind=@Kind AND LatestPublishedAt>@Published)
   THROW 50000,'Refusing older forecast publication',1;
  DELETE dbo.BoxOfficeForecasts WHERE Kind=@Kind;
  INSERT dbo.BoxOfficeForecasts(Kind,TargetDate,PeriodEnd,MovieTitle,Distributor,PredictedRank,ForecastLow,ForecastHigh,ForecastAsOf,PublishedAtUtc,UpdatedAtUtc,SourceUrl,FetchedAtUtc)
  SELECT @Kind,TargetDate,PeriodEnd,MovieTitle,Distributor,PredictedRank,ForecastLow,ForecastHigh,ForecastAsOf,PublishedAtUtc,UpdatedAtUtc,SourceUrl,SYSUTCDATETIME()
  FROM OPENJSON(@RowPayload) WITH (
   TargetDate DATE '$.targetDate',PeriodEnd DATE '$.endDate',MovieTitle NVARCHAR(300) '$.title',Distributor NVARCHAR(300) '$.distributor',
   PredictedRank INT '$.rank',ForecastLow BIGINT '$.low',ForecastHigh BIGINT '$.high',ForecastAsOf DATE '$.asOf',
   PublishedAtUtc DATETIME2 '$.publishedAt',UpdatedAtUtc DATETIME2 '$.updatedAt',SourceUrl NVARCHAR(2048) '$.sourceUrl'
  ) WHERE PeriodEnd>=@P2;
  DECLARE @Count INT=@@ROWCOUNT;
  UPDATE dbo.BoxOfficeForecastSources SET LatestPublishedAt=@Published,LastSuccessAt=SYSUTCDATETIME(),LastError=NULL,LastMovieCount=@Count WHERE Kind=@Kind;
  FETCH NEXT FROM snapshots INTO @Kind,@Published,@RowPayload;
 END;
 CLOSE snapshots;
 DEALLOCATE snapshots;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
