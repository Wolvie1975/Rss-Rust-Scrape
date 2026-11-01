CREATE OR ALTER VIEW dbo.SportsScoreboardCoverage AS
SELECT s.ID,s.SourceKind,s.OriginId,s.Sport,s.League,s.Label,
 s.LastAttemptAt,s.LastSuccessAt,s.Availability
FROM dbo.SportsScoreboardSources s
LEFT JOIN dbo.SportsEventsType e ON s.SourceKind='events' AND e.ID=s.OriginId
LEFT JOIN dbo.ResultFollows f ON s.SourceKind='follow' AND f.ID=s.OriginId
LEFT JOIN dbo.SportsRankingSources r ON s.SourceKind='rankings' AND r.ID=s.OriginId
WHERE (e.Enabled=1 OR f.Enabled=1 OR (r.Enabled=1 AND r.PollId='1'));
