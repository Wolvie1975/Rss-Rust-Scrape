-- Additive migration for Top 25 scoreboard and ranked games
SET XACT_ABORT ON;
BEGIN TRY
    BEGIN TRANSACTION;

    -- 1. Add HomeRank and AwayRank columns to dbo.SportsScoreboardGames if not present
    IF COL_LENGTH('dbo.SportsScoreboardGames', 'HomeRank') IS NULL
    BEGIN
        ALTER TABLE dbo.SportsScoreboardGames
        ADD HomeRank TINYINT NULL CONSTRAINT CK_SportsScoreboardGames_HomeRank CHECK (HomeRank IS NULL OR HomeRank BETWEEN 1 AND 25);
    END;

    IF COL_LENGTH('dbo.SportsScoreboardGames', 'AwayRank') IS NULL
    BEGIN
        ALTER TABLE dbo.SportsScoreboardGames
        ADD AwayRank TINYINT NULL CONSTRAINT CK_SportsScoreboardGames_AwayRank CHECK (AwayRank IS NULL OR AwayRank BETWEEN 1 AND 25);
    END;

    -- 2. Extend SourceKind check on dbo.SportsScoreboardSources to include 'rankings'
    IF EXISTS (
        SELECT 1 FROM sys.check_constraints
        WHERE parent_object_id = OBJECT_ID('dbo.SportsScoreboardSources')
          AND definition LIKE '%SourceKind%'
    )
    BEGIN
        DECLARE @chkName NVARCHAR(128);
        SELECT TOP(1) @chkName = name
        FROM sys.check_constraints
        WHERE parent_object_id = OBJECT_ID('dbo.SportsScoreboardSources')
          AND definition LIKE '%SourceKind%';

        EXEC('ALTER TABLE dbo.SportsScoreboardSources DROP CONSTRAINT [' + @chkName + '];');
        ALTER TABLE dbo.SportsScoreboardSources
        ADD CONSTRAINT CK_SportsScoreboardSources_SourceKind CHECK (SourceKind IN ('events', 'follow', 'rankings'));
    END;

    COMMIT;
END TRY
BEGIN CATCH
    IF @@TRANCOUNT > 0 ROLLBACK;
    THROW;
END CATCH;

