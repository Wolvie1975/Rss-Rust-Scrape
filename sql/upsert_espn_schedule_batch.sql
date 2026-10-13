MERGE dbo.SportsEvents WITH (HOLDLOCK) AS t
                         USING (SELECT j.* FROM OPENJSON(@P2) WITH (Url NVARCHAR(2048) '$.Url', GameId INT '$.GameId', Title NVARCHAR(500) '$.Title', Sport NVARCHAR(100) '$.Sport', Opponent NVARCHAR(200) '$.Opponent', IsAway BIT '$.IsAway', Location NVARCHAR(300) '$.Location', EventDate DATE '$.EventDate', StartsAtUtc DATETIME2 '$.StartsAtUtc', EndsAtUtc DATETIME2 '$.EndsAtUtc', TimeTbd BIT '$.TimeTbd', Tv NVARCHAR(200) '$.Tv', StreamUrl NVARCHAR(2048) '$.StreamUrl', LiveStatsUrl NVARCHAR(2048) '$.LiveStatsUrl', TeamLogoUrl NVARCHAR(2048) '$.TeamLogoUrl', OpponentLogoUrl NVARCHAR(2048) '$.OpponentLogoUrl', Status NVARCHAR(100) '$.Status', CanonicalKey VARCHAR(100) '$.CanonicalKey', IsNeutral BIT '$.IsNeutral', ProviderKey VARCHAR(50) '$.ProviderKey', SchoolName NVARCHAR(100) '$.SchoolName', SchoolId INT '$.SchoolId', OpponentSchoolId INT '$.OpponentSchoolId', SkipInsert BIT '$.SkipInsert') j WHERE EXISTS (SELECT 1 FROM dbo.SportsEventsType WHERE ID=@P1 AND Enabled=1)) AS s
ON t.CanonicalKey = s.CanonicalKey OR t.UrlHash = HASHBYTES('SHA2_256', s.Url)
                         WHEN MATCHED THEN UPDATE SET
                             GameId = s.GameId, Title = s.Title, Sport = s.Sport, Opponent = s.Opponent, IsAway = s.IsAway,
                             Location = s.Location, EventDate = s.EventDate, StartsAtUtc = s.StartsAtUtc, EndsAtUtc = s.EndsAtUtc,
                             TimeTbd = s.TimeTbd, Tv = s.Tv, StreamUrl = s.StreamUrl, LiveStatsUrl = s.LiveStatsUrl,
                             TeamLogoUrl = s.TeamLogoUrl, OpponentLogoUrl = s.OpponentLogoUrl, SportsEventsTypeId = @P1,
                             ProviderKey = s.ProviderKey, SchoolName = s.SchoolName, SchoolId = s.SchoolId, OpponentSchoolId = s.OpponentSchoolId, CanonicalKey = s.CanonicalKey, CalendarTimeZone = 'America/Chicago', ScheduleStatus = s.Status, IsNeutral = s.IsNeutral, LastSeenAt = SYSUTCDATETIME()
                         WHEN NOT MATCHED AND s.SkipInsert=0 THEN INSERT
                             (Url, GameId, Title, Sport, Opponent, IsAway, Location, EventDate, StartsAtUtc,
                              EndsAtUtc, TimeTbd, Tv, StreamUrl, LiveStatsUrl, TeamLogoUrl, OpponentLogoUrl,
                              SportsEventsTypeId, ProviderKey, CanonicalKey, CalendarTimeZone, ScheduleStatus, IsNeutral, SchoolName, SchoolId, OpponentSchoolId)
                             VALUES (s.Url, s.GameId, s.Title, s.Sport, s.Opponent, s.IsAway, s.Location, s.EventDate, s.StartsAtUtc,
                                     s.EndsAtUtc, s.TimeTbd, s.Tv, s.StreamUrl, s.LiveStatsUrl, s.TeamLogoUrl, s.OpponentLogoUrl, @P1, s.ProviderKey, s.CanonicalKey, 'America/Chicago', s.Status, s.IsNeutral, s.SchoolName, s.SchoolId, s.OpponentSchoolId);
