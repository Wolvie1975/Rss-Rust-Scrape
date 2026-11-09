-- @P1 sport @P2 league @P3 provider date @P4 now @P5 live minutes.
-- Status/near-start checks use persisted data. Missing days use hourly discovery.
SELECT CASE
 WHEN EXISTS(SELECT 1 FROM dbo.SportsScoreboardGames WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3
  AND (Status IN('live','delayed','suspended') OR (Status='upcoming' AND StartsAtUtc<=DATEADD(hour,1,@P4)
    AND StartsAtUtc>=DATEADD(hour,-12,@P4)))) THEN @P5
 WHEN EXISTS(SELECT 1 FROM dbo.SportsScoreboardGames WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3
  AND FinalFirstObservedAtUtc>DATEADD(hour,-1,@P4)) THEN 15
 WHEN @P3<CONVERT(date,@P4 AT TIME ZONE 'UTC' AT TIME ZONE 'Eastern Standard Time') AND NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardGames
  WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3 AND Status IN('upcoming','unknown')) THEN 1440
 ELSE 60 END Minutes;
