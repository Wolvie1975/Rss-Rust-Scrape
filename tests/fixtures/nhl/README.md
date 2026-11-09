# NHL fixture provenance

Captured from ESPN's public NHL scoreboard for October 2026 on 2026-10-14:
https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/scoreboard?dates=202610&limit=1000

Four games retain original schedule/score fields (regulation final, overtime final,
shootout final and scheduled); player leaders and editorial fields were removed.
The complete division standings fixture is `../standings/nhl.json`, captured from:
https://site.api.espn.com/apis/v2/sports/hockey/nhl/standings?level=3
