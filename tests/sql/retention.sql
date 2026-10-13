DECLARE @P1 date=CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time');
BEGIN TRANSACTION;
SELECT ID INTO #KeepSports FROM dbo.SportsEvents WHERE EventDate>=@P1 AND (ScheduleStatus IS NULL OR ScheduleStatus NOT IN('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY'));
SELECT ID INTO #KeepMovies FROM dbo.MovieReleases WHERE ReleaseDate>=@P1 OR ReleaseDate IS NULL;
SELECT ID INTO #KeepTV FROM dbo.TvEpisodes WHERE CASE WHEN StartsAtUtc IS NOT NULL THEN CONVERT(date,StartsAtUtc AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time') ELSE Airdate END>=@P1 OR (StartsAtUtc IS NULL AND Airdate IS NULL);
SELECT ID INTO #KeepTracking FROM dbo.TvTrackedSeries;
-- @P1 is today's date in America/Chicago. Preserve undated/TBD records.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DELETE s FROM dbo.SportsEventSources s
 WHERE TRY_CONVERT(date,JSON_VALUE(s.Payload,'$.event.event_date')) < @P1
 OR EXISTS (SELECT 1 FROM dbo.SportsEvents e WHERE e.ID=s.SportsEventId
  AND (e.EventDate<@P1 OR e.ScheduleStatus IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY')));
 DELETE dbo.SportsEvents WHERE EventDate<@P1
  OR ScheduleStatus IN ('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY');
 DELETE dbo.MovieReleases WHERE ReleaseDate<@P1;
 DELETE c FROM dbo.MoviePosterCache c WHERE NOT EXISTS
  (SELECT 1 FROM dbo.MovieReleases r WHERE r.MovieSourceLinkId=c.SourceLinkId);
 DELETE l FROM dbo.MovieSourceLinks l WHERE NOT EXISTS
  (SELECT 1 FROM dbo.MovieReleases r WHERE r.MovieSourceLinkId=l.ID);
 DELETE m FROM dbo.Movies m WHERE NOT EXISTS
  (SELECT 1 FROM dbo.MovieSourceLinks l WHERE l.MovieId=m.ID);
 DELETE dbo.TvEpisodes WHERE
  CASE WHEN StartsAtUtc IS NOT NULL THEN CONVERT(date,StartsAtUtc AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time') ELSE Airdate END < @P1;
 -- Tracking configuration survives even when all known episodes have aired.
 DELETE s FROM dbo.TvSeries s WHERE NOT EXISTS
  (SELECT 1 FROM dbo.TvEpisodes e WHERE e.SeriesId=s.ID)
  AND NOT EXISTS (SELECT 1 FROM dbo.TvTrackedSeries t WHERE t.SeriesId=s.ID);
 DELETE t FROM dbo.SportsRankingTeams t JOIN dbo.SportsRankingPolls p ON p.ID=t.PollRowId
 WHERE p.IsCurrent=0 OR t.IsCurrent=0;
 DELETE dbo.SportsRankingPolls WHERE IsCurrent=0;
 DELETE dbo.ScraperRuns WHERE FinishedAt IS NOT NULL AND StartedAt<DATEADD(day,-7,SYSUTCDATETIME());
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;

IF EXISTS(SELECT ID FROM #KeepSports EXCEPT SELECT ID FROM dbo.SportsEvents) THROW 50000,'Current/future sports lost',1;
IF EXISTS(SELECT ID FROM #KeepMovies EXCEPT SELECT ID FROM dbo.MovieReleases) THROW 50000,'Current/future movies lost',1;
IF EXISTS(SELECT ID FROM #KeepTV EXCEPT SELECT ID FROM dbo.TvEpisodes) THROW 50000,'Current/future episodes lost',1;
IF EXISTS(SELECT ID FROM #KeepTracking EXCEPT SELECT ID FROM dbo.TvTrackedSeries) THROW 50000,'TV tracking lost',1;
IF EXISTS(SELECT 1 FROM dbo.SportsEvents WHERE EventDate<@P1 OR ScheduleStatus IN('STATUS_CANCELED','STATUS_CANCELLED','STATUS_UNNECESSARY')) THROW 50000,'Expired sports remain',1;
IF EXISTS(SELECT 1 FROM dbo.MovieReleases WHERE ReleaseDate<@P1) THROW 50000,'Expired movies remain',1;
IF EXISTS(SELECT 1 FROM dbo.SportsRankingPolls WHERE IsCurrent=0) THROW 50000,'Old polls remain',1;
SELECT 'Retention rollback validation passed';
ROLLBACK;
