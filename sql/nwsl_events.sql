-- Additive schedule-only status; no ResultGames/ResultFollows changes.
IF COL_LENGTH('dbo.SportsEvents','ScheduleStatus') IS NULL
 ALTER TABLE dbo.SportsEvents ADD ScheduleStatus NVARCHAR(100) NULL;
