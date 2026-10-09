-- Parameterized atomic upsert. HOLDLOCK protects identities against concurrent runs.
SET XACT_ABORT ON;
BEGIN TRY
    BEGIN TRANSACTION;
    DECLARE @source INT, @movie INT, @link INT, @written INT = 0;
    SELECT @source=ID FROM dbo.MovieReleaseSources WITH (UPDLOCK,HOLDLOCK)
        WHERE SourceKey=@P1 AND Enabled=1;
    IF @source IS NOT NULL
    BEGIN
        SELECT @link=ID, @movie=MovieId FROM dbo.MovieSourceLinks WITH (UPDLOCK,HOLDLOCK)
            WHERE MovieReleaseSourceId=@source AND ExternalMovieId=@P2;
        IF @link IS NULL
        BEGIN
            IF @P4 IS NOT NULL
                SELECT @movie=ID FROM dbo.Movies WITH (UPDLOCK,HOLDLOCK) WHERE ImdbId=@P4;
            IF @movie IS NULL
            BEGIN
                INSERT dbo.Movies(Title,ImdbId) VALUES(@P3,@P4);
                SET @movie=SCOPE_IDENTITY();
            END;
            INSERT dbo.MovieSourceLinks(MovieId,MovieReleaseSourceId,ExternalMovieId,Url)
                VALUES(@movie,@source,@P2,@P8);
            SET @link=SCOPE_IDENTITY();
        END;
        UPDATE dbo.Movies SET LastSeenAt=SYSUTCDATETIME() WHERE ID=@movie;
        UPDATE dbo.MovieSourceLinks SET Url=@P8,LastSeenAt=SYSUTCDATETIME() WHERE ID=@link;
        IF EXISTS (SELECT 1 FROM dbo.MovieReleases WITH (UPDLOCK,HOLDLOCK)
            WHERE MovieSourceLinkId=@link AND ReleaseKey=@P5)
            UPDATE dbo.MovieReleases SET ReleaseDate=@P7,DateStatus='announced',
                ReleaseType=@P6,Platform=@P10,Format=@P9,SourceUrl=@P8,LastSeenAt=SYSUTCDATETIME()
                WHERE MovieSourceLinkId=@link AND ReleaseKey=@P5;
        ELSE
            INSERT dbo.MovieReleases(MovieSourceLinkId,ReleaseKey,ReleaseType,ReleaseDate,DateStatus,Platform,Format,SourceUrl)
                VALUES(@link,@P5,@P6,@P7,'announced',@P10,@P9,@P8);
        SET @written=1;
    END;
    COMMIT TRANSACTION;
    SELECT @written AS Written;
END TRY
BEGIN CATCH
    IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION;
    THROW;
END CATCH;
