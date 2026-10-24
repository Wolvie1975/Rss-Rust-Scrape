DECLARE @P1 INT=1;
DECLARE @Floor DATE=DATEADD(day,-1,CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time'));
CREATE TABLE #RetentionGames(ID INT,ScoreboardDate DATE);
CREATE TABLE #RetentionSources(GameId INT);
CREATE TABLE #RetentionRequests(ProviderDate DATE);
INSERT #RetentionGames VALUES(1,DATEADD(day,-1,@Floor)),(2,@Floor),(3,DATEADD(day,1,@Floor)),(4,DATEADD(day,2,@Floor));
INSERT #RetentionSources VALUES(1),(2),(3),(4);
INSERT #RetentionRequests SELECT ScoreboardDate FROM #RetentionGames;
-- @P1 = number of previous America/Chicago calendar days to retain.
-- Today/future rows remain. This touches only the dedicated scoreboard tables.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @Cutoff DATE=DATEADD(day,-@P1,CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time'));
 DELETE es FROM #RetentionSources es
 JOIN #RetentionGames g ON g.ID=es.GameId WHERE g.ScoreboardDate<@Cutoff;
 DELETE #RetentionGames WHERE ScoreboardDate<@Cutoff;
 DELETE #RetentionRequests WHERE ProviderDate<@Cutoff;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;

IF (SELECT COUNT(*) FROM #RetentionGames)<>3 OR EXISTS(SELECT 1 FROM #RetentionGames WHERE ID=1)
 THROW 50001,'Retention boundary test failed.',1;
IF (SELECT COUNT(*) FROM #RetentionSources)<>3 OR (SELECT COUNT(*) FROM #RetentionRequests)<>3
 THROW 50001,'Associated retention test failed.',1;
SELECT 'Passed: removes older day, keeps yesterday/today/future and matching associations/requests' Result;
