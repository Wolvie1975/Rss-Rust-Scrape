CREATE OR ALTER VIEW dbo.SportsScoreboard AS
SELECT g.*,
 CAST(CASE WHEN g.Status IN('live','delayed','suspended')
  AND g.LastObservedAtUtc<DATEADD(minute,-25,SYSUTCDATETIME()) THEN 1 ELSE 0 END AS BIT) IsStale,
 CAST(CASE WHEN b.Availability='available' THEN 1 ELSE 0 END AS BIT) HasBoxScore,
 b.SummaryUrl AS BoxScoreSummaryUrl,b.BoxScoreUrl,
 r.LastAttemptAt AS RequestLastAttemptAt,r.LastSuccessAt AS RequestLastSuccessAt,
 r.Availability AS RequestAvailability,r.NextAttemptAt AS RequestNextAttemptAt
FROM dbo.SportsScoreboardGames g
LEFT JOIN dbo.SportsBoxScoreCache b ON b.Sport=g.Sport AND b.League=g.League AND b.EventId=g.EventId
LEFT JOIN dbo.SportsScoreboardRequests r ON r.Sport=g.Sport AND r.League=g.League AND r.ProviderDate=g.ProviderDate
WHERE EXISTS(
 SELECT 1 FROM dbo.SportsScoreboardEventSources es
 JOIN dbo.SportsScoreboardCoverage s ON s.ID=es.SourceId
 LEFT JOIN dbo.ResultFollows f ON s.SourceKind='follow' AND f.ID=s.OriginId
 WHERE es.GameId=g.ID AND (
 s.SourceKind='events' OR f.Kind='league'
 OR (f.Kind='team' AND f.TeamId IN(g.HomeTeamId,g.AwayTeamId))
 OR (f.Kind='conference' AND EXISTS(SELECT 1 FROM dbo.ResultFollowTeams ft
  WHERE ft.FollowId=f.ID AND ft.TeamId IN(g.HomeTeamId,g.AwayTeamId)))
 ));
