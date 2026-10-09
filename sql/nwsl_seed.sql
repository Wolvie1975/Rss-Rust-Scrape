-- Run after scraper schema initialization. New feed is disabled; existing settings survive.
SET XACT_ABORT ON;
BEGIN TRANSACTION;
IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ProviderKey='espn-nwsl' AND ProviderSchoolId=20907)
BEGIN
 IF EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE RssUrl='https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907')
  UPDATE dbo.SportsEventsType SET ProviderKey='espn-nwsl',ProviderSchoolId=20907,SchoolName='Kansas City Current'
  WHERE RssUrl='https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907';
 ELSE
  INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,SchoolName,ProviderKey,ProviderSchoolId,Enabled)
  VALUES('https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907','Kansas City Current (NWSL)','Kansas City Current','espn-nwsl',20907,0);
END;
COMMIT TRANSACTION;
