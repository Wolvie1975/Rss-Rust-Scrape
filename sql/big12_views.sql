-- One public event per SportsEvents row. School calendars join the source perspectives.
CREATE OR ALTER VIEW dbo.SportsEventSchoolCalendar AS
SELECT e.ID AS SportsEventId, s.SportsEventsTypeId, s.ProviderGameId, s.SchoolId, s.SchoolName,
 s.OpponentSchoolId, s.SourceUrl, e.CanonicalKey, e.Sport, e.EventDate,
 e.StartsAtUtc, e.EndsAtUtc, e.TimeTbd,
 JSON_VALUE(s.Payload,'$.event.title') AS Title,
 JSON_VALUE(s.Payload,'$.event.opponent') AS Opponent,
 CASE WHEN e.IsNeutral=1 THEN NULL ELSE
  CASE JSON_VALUE(s.Payload,'$.event.is_away') WHEN 'true' THEN CAST(1 AS BIT) WHEN 'false' THEN CAST(0 AS BIT) ELSE NULL END END AS IsAway,
 COALESCE(e.IsNeutral,CASE JSON_VALUE(s.Payload,'$.is_neutral') WHEN 'true' THEN CAST(1 AS BIT) WHEN 'false' THEN CAST(0 AS BIT) ELSE NULL END) AS IsNeutral,
 JSON_VALUE(s.Payload,'$.calendar_timezone') AS CalendarTimeZone,
 COALESCE(JSON_VALUE(s.Payload,'$.event.location'),e.Location) AS Location,
 JSON_VALUE(s.Payload,'$.event.team_logo_url') AS TeamLogoUrl,
 JSON_VALUE(s.Payload,'$.event.opponent_logo_url') AS OpponentLogoUrl,
 COALESCE(JSON_VALUE(s.Payload,'$.event.tv'),e.Tv) AS Tv,
 COALESCE(JSON_VALUE(s.Payload,'$.event.stream_url'),e.StreamUrl) AS StreamUrl,
 COALESCE(JSON_VALUE(s.Payload,'$.event.live_stats_url'),e.LiveStatsUrl) AS LiveStatsUrl,
 COALESCE(JSON_VALUE(s.Payload,'$.radio'),e.Radio) AS Radio,
 COALESCE(JSON_VALUE(s.Payload,'$.audio_url'),e.AudioUrl) AS AudioUrl,
 s.Issue, JSON_VALUE(s.Payload,'$.warning') AS Warning, s.LastSeenAt
FROM dbo.SportsEventSources s JOIN dbo.SportsEvents e ON e.ID=s.SportsEventId;
