-- Register disabled feeds. Re-running preserves operator URLs/enabling/limits.
INSERT dbo.SportsStandingsSources(ProviderKey,Sport,DisplayName,Url,MinimumTeams)
SELECT s.ProviderKey,s.Sport,s.DisplayName,s.Url,s.MinimumTeams
FROM (VALUES
 (N'espn-epl',N'Soccer',N'Premier League',N'https://site.api.espn.com/apis/v2/sports/soccer/eng.1/standings',20),
 (N'espn-championship',N'Soccer',N'EFL Championship',N'https://site.api.espn.com/apis/v2/sports/soccer/eng.2/standings',24),
 (N'espn-nfl',N'Football',N'NFL',N'https://site.api.espn.com/apis/v2/sports/football/nfl/standings?level=3',32),
 (N'espn-mlb',N'Baseball',N'MLB',N'https://site.api.espn.com/apis/v2/sports/baseball/mlb/standings?season=2026',30),
 (N'espn-wnba',N'Women''s Basketball',N'WNBA',N'https://site.api.espn.com/apis/v2/sports/basketball/wnba/standings',15),
 (N'espn-nwsl',N'Soccer',N'NWSL',N'https://site.api.espn.com/apis/v2/sports/soccer/usa.nwsl/standings',16),
 (N'espn-mls',N'Soccer',N'MLS',N'https://site.api.espn.com/apis/v2/sports/soccer/usa.1/standings',30),
 (N'big12',N'Football',N'Big 12 Football',N'https://big12sports.com/standings.aspx?path=football',16),
 (N'big12',N'Men''s Basketball',N'Big 12 Men''s Basketball',N'https://big12sports.com/standings.aspx?path=mbball',16),
 (N'big12',N'Women''s Basketball',N'Big 12 Women''s Basketball',N'https://big12sports.com/standings.aspx?path=wbball',16),
 (N'big12',N'Soccer',N'Big 12 Soccer',N'https://big12sports.com/standings.aspx?path=soc',16),
 (N'big12',N'Volleyball',N'Big 12 Volleyball',N'https://big12sports.com/standings.aspx?path=vb',15)
) s(ProviderKey,Sport,DisplayName,Url,MinimumTeams)
WHERE NOT EXISTS(SELECT 1 FROM dbo.SportsStandingsSources x WHERE x.ProviderKey=s.ProviderKey AND x.Sport=s.Sport);
