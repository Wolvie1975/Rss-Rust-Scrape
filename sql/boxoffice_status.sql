-- Additive, safe to rerun; no legacy inference or permission changes.
IF COL_LENGTH('dbo.BoxOfficeWeekend','ResultStatus') IS NULL
 ALTER TABLE dbo.BoxOfficeWeekend ADD ResultStatus NVARCHAR(16) NULL;
IF COL_LENGTH('dbo.BoxOfficeWeekend','StatusSourceUrl') IS NULL
 ALTER TABLE dbo.BoxOfficeWeekend ADD StatusSourceUrl NVARCHAR(2048) NULL;
IF COL_LENGTH('dbo.BoxOfficeWeekend','StatusAsOfUtc') IS NULL
 ALTER TABLE dbo.BoxOfficeWeekend ADD StatusAsOfUtc DATETIME2 NULL;
