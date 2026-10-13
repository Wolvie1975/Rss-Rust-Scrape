-- Repeated setup preserves disabled sources and operator URLs/settings.
INSERT dbo.SportsRankingSources(Sport,LeagueId,PollId,DisplayName,Url)
SELECT Sport,LeagueId,PollId,DisplayName,Url FROM (VALUES
(N'Football',N'23',N'1',N'Football — AP Top 25',N'https://site.api.espn.com/apis/site/v2/sports/football/college-football/rankings'),
(N'Men''s Basketball',N'41',N'1',N'Men''s Basketball — AP Top 25',N'https://site.api.espn.com/apis/site/v2/sports/basketball/mens-college-basketball/rankings'),
(N'Women''s Basketball',N'54',N'1',N'Women''s Basketball — AP Top 25',N'https://site.api.espn.com/apis/site/v2/sports/basketball/womens-college-basketball/rankings'),
(N'Soccer',N'5499',N'86',N'Women''s Soccer — United Soccer Coaches',N'https://site.api.espn.com/apis/site/v2/sports/soccer/usa.ncaa.w.1/rankings'),
(N'Volleyball',N'402',N'89',N'Women''s Volleyball — AVCA Top 25',N'https://site.api.espn.com/apis/site/v2/sports/volleyball/womens-college-volleyball/rankings')
) s(Sport,LeagueId,PollId,DisplayName,Url)
WHERE NOT EXISTS(SELECT 1 FROM dbo.SportsRankingSources r WHERE r.ProviderKey=N'espn-rankings' AND r.Sport=s.Sport AND r.PollId=s.PollId);
