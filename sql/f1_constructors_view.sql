CREATE OR ALTER VIEW dbo.F1ConstructorStandings AS
SELECT s.Season,s.StandingsRound,r.Position,r.PositionText,r.Points,r.Wins,
 r.ConstructorId,r.ConstructorName,r.Nationality,s.FetchedAt
FROM dbo.F1Snapshot s
JOIN dbo.F1Source f ON f.ID=s.SourceId AND f.Enabled=1
CROSS APPLY OPENJSON(s.Payload,'$.constructors') WITH(
 Position INT '$.position',PositionText NVARCHAR(20) '$.positionText',Points DECIMAL(10,3) '$.points',Wins INT '$.wins',
 ConstructorId NVARCHAR(100) '$.Constructor.constructorId',ConstructorName NVARCHAR(200) '$.Constructor.name',
 Nationality NVARCHAR(100) '$.Constructor.nationality'
) r;
