IF COL_LENGTH('dbo.SportsMatchResults','HomeShootoutScore') IS NULL
 ALTER TABLE dbo.SportsMatchResults ADD HomeShootoutScore INT NULL;
IF COL_LENGTH('dbo.SportsMatchResults','AwayShootoutScore') IS NULL
 ALTER TABLE dbo.SportsMatchResults ADD AwayShootoutScore INT NULL;
