-- Atomic samples; @P1 Sport, @P2 League, @P3 JSON, @P4 request-start UTC, @P5 provider day.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 SELECT * INTO #ScoreboardIncoming FROM OPENJSON(@P3) WITH(
  EventId nvarchar(80) '$.eventId',
  ProviderDate date '$.providerDate',
  ScoreboardDate date '$.scoreboardDate',
  StartsAtUtc datetime2 '$.startsAtUtc',
  TimeTbd bit '$.timeTbd',
  HomeTeamId nvarchar(80) '$.homeTeamId',
  HomeName nvarchar(200) '$.homeName',
  HomeLogoUrl nvarchar(2048) '$.homeLogoUrl',
  AwayTeamId nvarchar(80) '$.awayTeamId',
  AwayName nvarchar(200) '$.awayName',
  AwayLogoUrl nvarchar(2048) '$.awayLogoUrl',
  HomeScore decimal(12,3) '$.homeScore',
  AwayScore decimal(12,3) '$.awayScore',
  Status varchar(12) '$.status',
  Completed bit '$.completed',
  ProviderStatus nvarchar(100) '$.providerStatus',
  StatusText nvarchar(300) '$.statusText',
  Period int '$.period',
  DisplayClock nvarchar(80) '$.displayClock',
  NeutralSite bit '$.neutralSite',
  HomeShootout int '$.homeShootout',
  AwayShootout int '$.awayShootout',
  WinnerSide varchar(4) '$.winnerSide',
  Venue nvarchar(300) '$.venue',
  Broadcasts nvarchar(1000) '$.broadcasts',
  SourceUrl nvarchar(2048) '$.sourceUrl',
  LiveDetailsJson nvarchar(max) '$.liveDetails' AS JSON,
  ProviderUpdatedAtUtc datetime2 '$.providerUpdatedAtUtc',
  Sources nvarchar(max) '$.sources' AS JSON
 );
 -- A provider revision or earlier-started request cannot overwrite a newer observation.
 DELETE i FROM #ScoreboardIncoming i JOIN dbo.SportsScoreboardGames g
 ON g.Provider='espn' AND g.Sport=@P1 AND g.League=@P2 AND g.EventId=i.EventId
 WHERE g.LastObservedAtUtc>@P4 OR (g.ProviderUpdatedAtUtc IS NOT NULL
 AND i.ProviderUpdatedAtUtc IS NOT NULL AND i.ProviderUpdatedAtUtc<g.ProviderUpdatedAtUtc);
 UPDATE g SET
  WasRescheduled=CASE WHEN (g.StartsAtUtc<>i.StartsAtUtc OR g.ScoreboardDate<>i.ScoreboardDate)
    THEN 1 ELSE g.WasRescheduled END,
  FinalFirstObservedAtUtc=CASE WHEN i.Status='final' THEN COALESCE(g.FinalFirstObservedAtUtc,@P4) ELSE NULL END,
  LastObservedAtUtc=@P4,
  ProviderDate=i.ProviderDate,
  ScoreboardDate=CASE WHEN g.Status IN('live','suspended','delayed') AND g.StartsAtUtc=i.StartsAtUtc THEN g.ScoreboardDate ELSE i.ScoreboardDate END,
  StartsAtUtc=i.StartsAtUtc,
  TimeTbd=i.TimeTbd,
  HomeTeamId=i.HomeTeamId,
  HomeName=i.HomeName,
  HomeLogoUrl=i.HomeLogoUrl,
  AwayTeamId=i.AwayTeamId,
  AwayName=i.AwayName,
  AwayLogoUrl=i.AwayLogoUrl,
  HomeScore=CASE WHEN i.Status IN('postponed','suspended','delayed','unknown') THEN COALESCE(i.HomeScore,g.HomeScore) ELSE i.HomeScore END,
  AwayScore=CASE WHEN i.Status IN('postponed','suspended','delayed','unknown') THEN COALESCE(i.AwayScore,g.AwayScore) ELSE i.AwayScore END,
  Status=i.Status,
  Completed=i.Completed,
  ProviderStatus=i.ProviderStatus,
  StatusText=i.StatusText,
  Period=i.Period,
  DisplayClock=i.DisplayClock,
  NeutralSite=i.NeutralSite,
  HomeShootout=i.HomeShootout,
  AwayShootout=i.AwayShootout,
  WinnerSide=i.WinnerSide,
  Venue=i.Venue,
  Broadcasts=i.Broadcasts,
  SourceUrl=i.SourceUrl,
  LiveDetailsJson=i.LiveDetailsJson,
  ProviderUpdatedAtUtc=i.ProviderUpdatedAtUtc
 FROM dbo.SportsScoreboardGames g JOIN #ScoreboardIncoming i ON g.Provider='espn'
 AND g.Sport=@P1 AND g.League=@P2 AND g.EventId=i.EventId;
 INSERT dbo.SportsScoreboardGames(Provider,Sport,League,EventId,ProviderDate,ScoreboardDate,StartsAtUtc,TimeTbd,HomeTeamId,HomeName,HomeLogoUrl,AwayTeamId,AwayName,AwayLogoUrl,HomeScore,AwayScore,Status,Completed,ProviderStatus,StatusText,Period,DisplayClock,NeutralSite,HomeShootout,AwayShootout,WinnerSide,Venue,Broadcasts,SourceUrl,LiveDetailsJson,ProviderUpdatedAtUtc,OriginalScoreboardDate,OriginalStartsAtUtc,LastObservedAtUtc,FirstSeenAtUtc,FinalFirstObservedAtUtc)
 SELECT 'espn',@P1,@P2,i.EventId,i.ProviderDate,i.ScoreboardDate,i.StartsAtUtc,i.TimeTbd,i.HomeTeamId,i.HomeName,i.HomeLogoUrl,i.AwayTeamId,i.AwayName,i.AwayLogoUrl,i.HomeScore,i.AwayScore,i.Status,i.Completed,i.ProviderStatus,i.StatusText,i.Period,i.DisplayClock,i.NeutralSite,i.HomeShootout,i.AwayShootout,i.WinnerSide,i.Venue,i.Broadcasts,i.SourceUrl,i.LiveDetailsJson,i.ProviderUpdatedAtUtc,i.ScoreboardDate,i.StartsAtUtc,@P4,@P4,CASE WHEN i.Status='final' THEN @P4 END
 FROM #ScoreboardIncoming i WHERE NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardGames g WITH(UPDLOCK,HOLDLOCK) WHERE g.Provider='espn' AND g.Sport=@P1 AND g.League=@P2 AND g.EventId=i.EventId);

 INSERT dbo.SportsScoreboardEventSources(GameId,SourceId)
 SELECT g.ID,CONVERT(INT,m.value) FROM #ScoreboardIncoming i
 JOIN dbo.SportsScoreboardGames g ON g.Provider='espn' AND g.Sport=@P1 AND g.League=@P2 AND g.EventId=i.EventId
 CROSS APPLY OPENJSON(i.Sources) m
 WHERE NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardEventSources es WHERE es.GameId=g.ID AND es.SourceId=CONVERT(INT,m.value));
 DROP TABLE #ScoreboardIncoming;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
