CREATE OR ALTER VIEW dbo.SportsBoxScores AS
SELECT Sport,League,EventId,
 CAST(CASE WHEN Availability='available' THEN 1 ELSE 0 END AS BIT) HasBoxScore,
 Availability,SummaryUrl,BoxScoreUrl,Payload,FetchedAt,LastAttemptAt,NextAttemptAt,LastError
FROM dbo.SportsBoxScoreCache;
