IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE ProviderKey='espn-mls' AND ProviderSchoolId=0)
 INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,ProviderKey,ProviderSchoolId,Enabled)
 VALUES('https://site.api.espn.com/apis/site/v2/sports/soccer/usa.1/scoreboard','MLS','espn-mls',0,0);
