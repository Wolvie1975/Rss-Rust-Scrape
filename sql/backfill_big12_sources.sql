SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 UPDATE e SET ProviderKey='big12',SchoolId=f.ProviderSchoolId,SchoolName=f.SchoolName,
  ProviderSportId=CASE e.Sport WHEN 'Football' THEN 4 WHEN 'Men''s Basketball' THEN 7 WHEN 'Women''s Basketball' THEN 15 WHEN 'Soccer' THEN 31 WHEN 'Volleyball' THEN 37 ELSE NULL END,
  CanonicalKey=CONCAT('big12:game:',e.GameId)
 FROM dbo.SportsEvents e JOIN dbo.SportsEventsType f ON f.ID=e.SportsEventsTypeId
 WHERE f.ProviderKey='big12' AND e.CanonicalKey IS NULL AND e.GameId IS NOT NULL;
 INSERT dbo.SportsEventSources(SportsEventId,SportsEventsTypeId,ProviderGameId,SourceUrl,SchoolId,SchoolName,Payload,FirstSeenAt,LastSeenAt)
 SELECT e.ID,e.SportsEventsTypeId,e.GameId,e.Url,e.SchoolId,e.SchoolName,
  CONCAT('{"event":',(SELECT e.Url AS url,e.GameId AS game_id,e.Title AS title,e.Sport AS sport,
   e.Opponent AS opponent,e.IsAway AS is_away,e.Location AS location,
   CONVERT(VARCHAR(10),e.EventDate,23) AS event_date,
   CASE WHEN e.StartsAtUtc IS NOT NULL THEN CONCAT(CONVERT(VARCHAR(30),e.StartsAtUtc,126),'Z') END AS starts_at,
   CASE WHEN e.EndsAtUtc IS NOT NULL THEN CONCAT(CONVERT(VARCHAR(30),e.EndsAtUtc,126),'Z') END AS ends_at,
   e.TimeTbd AS time_tbd,e.Tv AS tv,e.StreamUrl AS stream_url,e.LiveStatsUrl AS live_stats_url,
   e.TeamLogoUrl AS team_logo_url,e.OpponentLogoUrl AS opponent_logo_url FOR JSON PATH,WITHOUT_ARRAY_WRAPPER,INCLUDE_NULL_VALUES),'}'),e.FirstSeenAt,e.LastSeenAt
 FROM dbo.SportsEvents e WHERE e.ProviderKey='big12' AND e.GameId IS NOT NULL
  AND NOT EXISTS(SELECT 1 FROM dbo.SportsEventSources s WITH(UPDLOCK,HOLDLOCK) WHERE s.SportsEventsTypeId=e.SportsEventsTypeId AND s.ProviderGameId=e.GameId);
 COMMIT TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
