CREATE OR ALTER VIEW dbo.F1DriverStandings AS
SELECT s.Season,s.StandingsRound,r.Position,r.PositionText,r.Points,r.Wins,
 r.DriverId,r.DriverCode,r.GivenName,r.FamilyName,r.Nationality,r.ConstructorsJson,s.FetchedAt
FROM dbo.F1Snapshot s
JOIN dbo.F1Source f ON f.ID=s.SourceId AND f.Enabled=1
CROSS APPLY OPENJSON(s.Payload,'$.drivers') WITH(
 Position INT '$.position',PositionText NVARCHAR(20) '$.positionText',Points DECIMAL(10,3) '$.points',Wins INT '$.wins',
 DriverId NVARCHAR(100) '$.Driver.driverId',DriverCode NVARCHAR(10) '$.Driver.code',
 GivenName NVARCHAR(100) '$.Driver.givenName',FamilyName NVARCHAR(100) '$.Driver.familyName',
 Nationality NVARCHAR(100) '$.Driver.nationality',ConstructorsJson NVARCHAR(MAX) '$.Constructors' AS JSON
) r;
