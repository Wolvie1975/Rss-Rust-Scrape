-- Register disabled; repeated setup preserves operator settings.
IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE ProviderKey='espn-championship' AND ProviderSchoolId=0)
 INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,ProviderKey,ProviderSchoolId,Enabled)
 VALUES('https://site.api.espn.com/apis/site/v2/sports/soccer/eng.2/scoreboard','EFL Championship','espn-championship',0,0);
