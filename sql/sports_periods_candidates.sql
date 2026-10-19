WITH Games AS (
 SELECT Sport,League,EventId,HomeTeamId,AwayTeamId,StartsAtUtc
 FROM dbo.ResultGames WHERE Completed=1 AND State='post'
 UNION ALL
 SELECT CASE WHEN r.ProviderKey='espn-nhl' THEN N'hockey' ELSE N'soccer' END,
  CASE r.ProviderKey WHEN 'espn-nhl' THEN N'nhl' WHEN 'espn-mls' THEN N'usa.1' ELSE N'eng.2' END,
  CONVERT(NVARCHAR(20),r.GameId),CONVERT(NVARCHAR(20),r.HomeTeamId),CONVERT(NVARCHAR(20),r.AwayTeamId),r.StartsAtUtc
 FROM dbo.SportsMatchResults r JOIN dbo.SportsEventsType s ON s.ID=r.SourceId AND s.Enabled=1
 WHERE r.ProviderKey IN('espn-nhl','espn-mls','espn-championship')
), Ranked AS (
 SELECT *,ROW_NUMBER() OVER(PARTITION BY Sport,League,EventId ORDER BY StartsAtUtc DESC) rn FROM Games
)
SELECT TOP (@P1) g.Sport,g.League,g.EventId,g.HomeTeamId,g.AwayTeamId,
 CONVERT(VARCHAR(8),CONVERT(date,g.StartsAtUtc AT TIME ZONE 'UTC' AT TIME ZONE 'Eastern Standard Time'),112) GameDate,b.Payload
FROM Ranked g LEFT JOIN dbo.SportsBoxScoreCache b
 ON b.Sport=g.Sport AND b.League=g.League AND b.EventId=g.EventId
WHERE g.rn=1 AND b.Availability='available'
 AND ISNULL(JSON_VALUE(b.Payload,'$.periodLabelsVersion'),'')<>'1'
 AND (b.LastError IS NULL OR b.NextAttemptAt<=SYSUTCDATETIME())
ORDER BY CASE WHEN g.StartsAtUtc>=DATEADD(day,-2,SYSUTCDATETIME()) THEN 0
 WHEN b.EventId IS NULL THEN 1 ELSE 2 END,g.StartsAtUtc DESC,g.Sport,g.League,g.EventId;
