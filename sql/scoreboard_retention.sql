-- @P1 = number of previous America/Chicago calendar days to retain.
-- Today/future rows remain. This touches only the dedicated scoreboard tables.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Cutoff DATE=DATEADD(day,-@P1,CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time'));
 DELETE es FROM dbo.SportsScoreboardEventSources es
 JOIN dbo.SportsScoreboardGames g ON g.ID=es.GameId WHERE g.ScoreboardDate<@Cutoff;
 DELETE dbo.SportsScoreboardGames WHERE ScoreboardDate<@Cutoff;
 DELETE dbo.SportsScoreboardRequests WHERE ProviderDate<@Cutoff;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
