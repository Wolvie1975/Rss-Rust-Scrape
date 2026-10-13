-- Register disabled; preserve operator configuration on repeated setup.
IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE ProviderKey='espn-epl' AND ProviderSchoolId=0)
 INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,ProviderKey,ProviderSchoolId,Enabled)
 VALUES('https://site.api.espn.com/apis/site/v2/sports/soccer/eng.1/scoreboard','Premier League','espn-epl',0,0);
