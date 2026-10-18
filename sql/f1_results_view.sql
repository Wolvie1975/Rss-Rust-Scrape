CREATE OR ALTER VIEW dbo.F1Results AS
SELECT s.Season,k.SessionType,r.Round,r.Position,r.PositionText,r.Points,
 r.DriverId,r.DriverCode,r.GivenName,r.FamilyName,r.ConstructorId,r.ConstructorName,
 r.Grid,r.Laps,r.Status,r.TimeDisplay,r.TimeMillis,r.FastestLapTime,r.FastestLapRank,s.FetchedAt
FROM dbo.F1Snapshot s
JOIN dbo.F1Source f ON f.ID=s.SourceId AND f.Enabled=1
CROSS APPLY (VALUES
 (CAST('Race' AS VARCHAR(10)),JSON_QUERY(s.Payload,'$.results')),
 (CAST('Sprint' AS VARCHAR(10)),JSON_QUERY(s.Payload,'$.sprint'))
) k(SessionType,RowsJson)
CROSS APPLY OPENJSON(k.RowsJson) WITH(
 Round INT '$.round',Position INT '$.position',PositionText NVARCHAR(20) '$.positionText',
 Points DECIMAL(10,3) '$.points',DriverId NVARCHAR(100) '$.Driver.driverId',
 DriverCode NVARCHAR(10) '$.Driver.code',GivenName NVARCHAR(100) '$.Driver.givenName',
 FamilyName NVARCHAR(100) '$.Driver.familyName',ConstructorId NVARCHAR(100) '$.Constructor.constructorId',
 ConstructorName NVARCHAR(200) '$.Constructor.name',Grid INT '$.grid',Laps INT '$.laps',
 Status NVARCHAR(100) '$.status',TimeDisplay NVARCHAR(100) '$.Time.time',TimeMillis BIGINT '$.Time.millis',
 FastestLapTime NVARCHAR(50) '$.FastestLap.Time.time',FastestLapRank INT '$.FastestLap.rank'
) r;
