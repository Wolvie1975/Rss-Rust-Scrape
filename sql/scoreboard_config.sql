SELECT 'follow' SourceKind,f.ID OriginId,f.Sport,f.League,f.Name Label,
 f.Kind Scope,f.TeamId,f.GroupId,CAST(NULL AS NVARCHAR(2048)) Url,
 (SELECT ft.TeamId FROM dbo.ResultFollowTeams ft WHERE ft.FollowId=f.ID FOR JSON PATH) Members
FROM dbo.ResultFollows f WHERE f.Enabled=1
UNION ALL
SELECT 'events',e.ID,
 CASE e.ProviderKey WHEN 'espn-mlb' THEN 'baseball' WHEN 'espn-wnba' THEN 'basketball'
 WHEN 'espn-nhl' THEN 'hockey' ELSE 'soccer' END,
 CASE e.ProviderKey WHEN 'espn-mlb' THEN 'mlb' WHEN 'espn-wnba' THEN 'wnba'
 WHEN 'espn-nhl' THEN 'nhl' WHEN 'espn-nwsl' THEN 'usa.nwsl'
 WHEN 'espn-mls' THEN 'usa.1' WHEN 'espn-epl' THEN 'eng.1' WHEN 'espn-championship' THEN 'eng.2' END,
 e.EventsTypeName,'schedule',NULL,NULL,e.RssUrl,N'[]'
FROM dbo.SportsEventsType e WHERE e.Enabled=1 AND e.ProviderKey IN(
 'espn-mlb','espn-wnba','espn-nhl','espn-nwsl','espn-mls','espn-epl','espn-championship');
