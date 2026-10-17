-- Explicit league expansion. Preserve the legacy source ID and Enabled setting.
-- Retain the legacy ProviderSchoolId tag for deployed frontend compatibility;
-- the feed URL, not this compatibility tag, determines collector team scope.
-- New installations register disabled. Repeated setup preserves operator settings.
SET XACT_ABORT ON;
BEGIN TRANSACTION;
IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ProviderKey='espn-nwsl' AND (ProviderSchoolId=0 OR RssUrl='https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard'))
BEGIN
 IF EXISTS(SELECT 1 FROM dbo.SportsEventsType WHERE ProviderKey='espn-nwsl' AND ProviderSchoolId=20907 AND RssUrl='https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907')
  UPDATE dbo.SportsEventsType SET RssUrl='https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard',EventsTypeName='NWSL',SchoolName=NULL
  WHERE ProviderKey='espn-nwsl' AND ProviderSchoolId=20907 AND RssUrl='https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907';
 ELSE
  INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,SchoolName,ProviderKey,ProviderSchoolId,Enabled)
  VALUES('https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard','NWSL',NULL,'espn-nwsl',0,0);
END;
COMMIT TRANSACTION;
