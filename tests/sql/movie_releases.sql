-- Integration checks against the real SQL Server schema. All fixtures roll back.
-- Rolled-back IDENTITY inserts may leave harmless ID gaps.
SET XACT_ABORT OFF;
BEGIN TRY
    BEGIN TRANSACTION;
    DECLARE @source INT, @movie INT, @link INT, @release INT;
    INSERT dbo.MovieReleaseSources(SourceKey, Name, Url)
        VALUES(CONVERT(VARCHAR(36), NEWID()), N'Schema test', N'https://example.invalid');
    SET @source = SCOPE_IDENTITY();
    INSERT dbo.Movies(Title, OriginalYear) VALUES(N'Schema test', 2026);
    SET @movie = SCOPE_IDENTITY();
    INSERT dbo.MovieSourceLinks(MovieId, MovieReleaseSourceId, ExternalMovieId, Url)
        VALUES(@movie, @source, N'test-movie', N'https://example.invalid/movie');
    SET @link = SCOPE_IDENTITY();
    INSERT dbo.MovieReleases(MovieSourceLinkId, ReleaseKey, ReleaseType, ReleaseDate, DateStatus, Platform, Format, SourceUrl)
    VALUES
        (@link, N'theatrical-wide', 'theatrical', '20261009', 'announced', NULL, NULL, N'https://example.invalid'),
        (@link, N'digital', 'digital', '20261013', 'announced', NULL, NULL, N'https://example.invalid'),
        (@link, N'subscription-test', 'subscription', '20261016', 'announced', N'Test service', NULL, N'https://example.invalid'),
        (@link, N'disc-dvd', 'disc', '20261020', 'announced', NULL, N'DVD', N'https://example.invalid'),
        (@link, N'disc-4k', 'disc', NULL, 'tbd', NULL, N'4K UHD', N'https://example.invalid');
    SELECT @release = ID FROM dbo.MovieReleases WHERE MovieSourceLinkId = @link AND ReleaseKey = N'theatrical-wide';
    UPDATE dbo.MovieReleases SET ReleaseDate = '20261010' WHERE ID = @release;
    IF (SELECT COUNT(*) FROM dbo.MovieReleases WHERE MovieSourceLinkId = @link) <> 5
        THROW 51000, 'Rescheduling changed the release count.', 1;
    IF (SELECT COUNT(*) FROM dbo.MovieReleases WHERE MovieSourceLinkId = @link AND ReleaseDate >= '20261012' AND ReleaseDate < '20261019') <> 2
        THROW 51000, 'Weekly date range returned the wrong releases.', 1;

    BEGIN TRY
        INSERT dbo.MovieReleases(MovieSourceLinkId, ReleaseKey, ReleaseType, SourceUrl)
            VALUES(@link, N'digital', 'digital', N'https://example.invalid');
        THROW 51000, 'Duplicate release identity was accepted.', 1;
    END TRY
    BEGIN CATCH
        IF ERROR_NUMBER() NOT IN (2601, 2627) THROW;
    END CATCH;
    BEGIN TRY
        INSERT dbo.MovieReleases(MovieSourceLinkId, ReleaseKey, ReleaseType, DateStatus, SourceUrl)
            VALUES(@link, N'invalid-date', 'disc', 'announced', N'https://example.invalid');
        THROW 51000, 'Announced release without a date was accepted.', 1;
    END TRY
    BEGIN CATCH
        IF ERROR_NUMBER() <> 547 THROW;
    END CATCH;
    BEGIN TRY
        INSERT dbo.MovieReleases(MovieSourceLinkId, ReleaseKey, ReleaseType, SourceUrl)
            VALUES(@link, N'invalid-platform', 'subscription', N'https://example.invalid');
        THROW 51000, 'Subscription release without a platform was accepted.', 1;
    END TRY
    BEGIN CATCH
        IF ERROR_NUMBER() <> 547 THROW;
    END CATCH;
    BEGIN TRY
        INSERT dbo.MovieReleases(MovieSourceLinkId, ReleaseKey, ReleaseType, CountryCode, SourceUrl)
            VALUES(@link, N'invalid-country', 'disc', 'CA', N'https://example.invalid');
        THROW 51000, 'Non-US release was accepted.', 1;
    END TRY
    BEGIN CATCH
        IF ERROR_NUMBER() <> 547 THROW;
    END CATCH;
    BEGIN TRY
        DELETE dbo.Movies WHERE ID = @movie;
        THROW 51000, 'Deleting a movie with source links was accepted.', 1;
    END TRY
    BEGIN CATCH
        IF ERROR_NUMBER() <> 547 THROW;
    END CATCH;
    ROLLBACK TRANSACTION;
END TRY
BEGIN CATCH
    IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION;
    THROW;
END CATCH;
