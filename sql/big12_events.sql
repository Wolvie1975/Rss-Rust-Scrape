-- Additive and repeatable. Source rows may be quarantined until identity is verified.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF COL_LENGTH('dbo.SportsEventsType','ProviderKey') IS NULL ALTER TABLE dbo.SportsEventsType ADD ProviderKey VARCHAR(50) NULL;
 IF COL_LENGTH('dbo.SportsEventsType','ProviderSchoolId') IS NULL ALTER TABLE dbo.SportsEventsType ADD ProviderSchoolId INT NULL;
 IF COL_LENGTH('dbo.SportsEventsType','Enabled') IS NULL ALTER TABLE dbo.SportsEventsType ADD Enabled BIT NOT NULL CONSTRAINT DF_SportsEventsType_Enabled DEFAULT 1;
 IF COL_LENGTH('dbo.SportsEventsType','LastAttemptAt') IS NULL ALTER TABLE dbo.SportsEventsType ADD LastAttemptAt DATETIME2 NULL;
 IF COL_LENGTH('dbo.SportsEventsType','LastSuccessAt') IS NULL ALTER TABLE dbo.SportsEventsType ADD LastSuccessAt DATETIME2 NULL;
 IF COL_LENGTH('dbo.SportsEventsType','LastError') IS NULL ALTER TABLE dbo.SportsEventsType ADD LastError NVARCHAR(1000) NULL;
 IF COL_LENGTH('dbo.SportsEventsType','LastEventCount') IS NULL ALTER TABLE dbo.SportsEventsType ADD LastEventCount INT NULL;
 IF COL_LENGTH('dbo.SportsEventsType','LastIssueCount') IS NULL ALTER TABLE dbo.SportsEventsType ADD LastIssueCount INT NULL;
 IF COL_LENGTH('dbo.SportsEvents','CanonicalKey') IS NULL ALTER TABLE dbo.SportsEvents ADD CanonicalKey VARCHAR(100) NULL;
 IF COL_LENGTH('dbo.SportsEvents','ProviderKey') IS NULL ALTER TABLE dbo.SportsEvents ADD ProviderKey VARCHAR(50) NULL;
 IF COL_LENGTH('dbo.SportsEvents','SchoolId') IS NULL ALTER TABLE dbo.SportsEvents ADD SchoolId INT NULL;
 IF COL_LENGTH('dbo.SportsEvents','SchoolName') IS NULL ALTER TABLE dbo.SportsEvents ADD SchoolName NVARCHAR(100) NULL;
 IF COL_LENGTH('dbo.SportsEvents','OpponentSchoolId') IS NULL ALTER TABLE dbo.SportsEvents ADD OpponentSchoolId INT NULL;
 IF COL_LENGTH('dbo.SportsEvents','ProviderSportId') IS NULL ALTER TABLE dbo.SportsEvents ADD ProviderSportId INT NULL;
 IF COL_LENGTH('dbo.SportsEvents','IsNeutral') IS NULL ALTER TABLE dbo.SportsEvents ADD IsNeutral BIT NULL;
 IF COL_LENGTH('dbo.SportsEvents','CalendarTimeZone') IS NULL ALTER TABLE dbo.SportsEvents ADD CalendarTimeZone NVARCHAR(30) NULL;
 IF COL_LENGTH('dbo.SportsEvents','Radio') IS NULL ALTER TABLE dbo.SportsEvents ADD Radio NVARCHAR(200) NULL;
 IF COL_LENGTH('dbo.SportsEvents','AudioUrl') IS NULL ALTER TABLE dbo.SportsEvents ADD AudioUrl NVARCHAR(2048) NULL;
 IF OBJECT_ID('dbo.SportsEventSources','U') IS NULL
 CREATE TABLE dbo.SportsEventSources (
  ID INT IDENTITY(1,1) CONSTRAINT PK_SportsEventSources PRIMARY KEY,
  SportsEventId INT NULL CONSTRAINT FK_SportsEventSources_Event REFERENCES dbo.SportsEvents(ID),
  SportsEventsTypeId INT NOT NULL CONSTRAINT FK_SportsEventSources_Type REFERENCES dbo.SportsEventsType(ID),
  ProviderGameId INT NOT NULL,
  SourceUrl NVARCHAR(2048) NOT NULL,
  SchoolId INT NULL, SchoolName NVARCHAR(100) NULL, OpponentSchoolId INT NULL,
  Payload NVARCHAR(MAX) NULL CONSTRAINT CK_SportsEventSources_JSON CHECK (Payload IS NULL OR ISJSON(Payload)=1),
  Issue NVARCHAR(1000) NULL,
  FirstSeenAt DATETIME2 NOT NULL CONSTRAINT DF_SportsEventSources_First DEFAULT SYSUTCDATETIME(),
  LastSeenAt DATETIME2 NOT NULL CONSTRAINT DF_SportsEventSources_Last DEFAULT SYSUTCDATETIME(),
  CONSTRAINT UQ_SportsEventSources_Game UNIQUE(SportsEventsTypeId,ProviderGameId)
 );
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsEvents') AND name='UX_SportsEvents_CanonicalKey')
  EXEC(N'CREATE UNIQUE INDEX UX_SportsEvents_CanonicalKey ON dbo.SportsEvents(CanonicalKey) WHERE CanonicalKey IS NOT NULL');
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsEventSources') AND name='IX_SportsEventSources_Event')
  CREATE INDEX IX_SportsEventSources_Event ON dbo.SportsEventSources(SportsEventId);
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsEventsType') AND name='UX_SportsEventsType_ProviderSchool')
  EXEC(N'CREATE UNIQUE INDEX UX_SportsEventsType_ProviderSchool ON dbo.SportsEventsType(ProviderKey,ProviderSchoolId) WHERE ProviderKey IS NOT NULL AND ProviderSchoolId IS NOT NULL');
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsEvents') AND name='IX_SportsEvents_ProviderGame')
  EXEC(N'CREATE INDEX IX_SportsEvents_ProviderGame ON dbo.SportsEvents(ProviderKey,GameId)');
 IF NOT EXISTS(SELECT 1 FROM sys.indexes WHERE object_id=OBJECT_ID('dbo.SportsEventSources') AND name='IX_SportsEventSources_Game')
  CREATE INDEX IX_SportsEventSources_Game ON dbo.SportsEventSources(ProviderGameId,SportsEventId);
 COMMIT TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
