-- One league feed, registered disabled; preserve edited URLs and Enabled on rerun.
SET XACT_ABORT ON;
BEGIN TRANSACTION;
IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ProviderKey='espn-nfl' AND ProviderSchoolId=0)
BEGIN
 IF EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE RssUrl='https://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard')
  UPDATE dbo.SportsEventsType SET ProviderKey='espn-nfl',ProviderSchoolId=0
  WHERE RssUrl='https://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard';
 ELSE
  INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,ProviderKey,ProviderSchoolId,Enabled)
  VALUES('https://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard','NFL','espn-nfl',0,0);
END;
COMMIT TRANSACTION;
