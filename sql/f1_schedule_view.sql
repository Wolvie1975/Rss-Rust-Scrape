CREATE OR ALTER VIEW dbo.F1Schedule AS
SELECT s.Season,r.Round,r.RaceName,r.CircuitId,r.CircuitName,r.Locality,r.Country,
 r.RaceDate,r.RaceTimeUtc,r.RaceUrl,
 JSON_QUERY(r.RaceJson,'$.FirstPractice') AS FirstPractice,
 JSON_QUERY(r.RaceJson,'$.SecondPractice') AS SecondPractice,
 JSON_QUERY(r.RaceJson,'$.ThirdPractice') AS ThirdPractice,
 JSON_QUERY(r.RaceJson,'$.Qualifying') AS Qualifying,
 JSON_QUERY(r.RaceJson,'$.Sprint') AS Sprint,
 JSON_QUERY(r.RaceJson,'$.SprintQualifying') AS SprintQualifying,
 JSON_QUERY(r.RaceJson,'$.SprintShootout') AS SprintShootout,
 s.FetchedAt
FROM dbo.F1Snapshot s
JOIN dbo.F1Source f ON f.ID=s.SourceId AND f.Enabled=1
CROSS APPLY OPENJSON(s.Payload,'$.races') WITH(
 Round INT '$.round', RaceName NVARCHAR(200) '$.raceName',
 CircuitId NVARCHAR(100) '$.Circuit.circuitId', CircuitName NVARCHAR(200) '$.Circuit.circuitName',
 Locality NVARCHAR(200) '$.Circuit.Location.locality', Country NVARCHAR(100) '$.Circuit.Location.country',
 RaceDate DATE '$.date', RaceTimeUtc NVARCHAR(30) '$.time', RaceUrl NVARCHAR(2048) '$.url',
 RaceJson NVARCHAR(MAX) '$' AS JSON
) r;
