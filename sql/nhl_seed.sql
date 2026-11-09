-- Register disabled; repeated setup preserves existing operator settings.
IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE ProviderKey='espn-nhl' AND ProviderSchoolId=0)
 INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,ProviderKey,ProviderSchoolId,Enabled)
 VALUES('https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/scoreboard','NHL','espn-nhl',0,0);
IF NOT EXISTS(SELECT 1 FROM dbo.SportsStandingsSources WHERE ProviderKey='espn-nhl' AND Sport='Hockey')
 INSERT dbo.SportsStandingsSources(ProviderKey,Sport,DisplayName,Url,MinimumTeams)
 VALUES('espn-nhl','Hockey','NHL','https://site.api.espn.com/apis/v2/sports/hockey/nhl/standings?level=3',32);
