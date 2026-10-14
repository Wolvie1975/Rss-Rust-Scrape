-- Apply only after the first successful chart import. Retire the two dedicated
-- news sources; preserve source configuration so operators can re-enable them.
SET XACT_ABORT ON;
BEGIN TRANSACTION;
IF (SELECT COUNT(*) FROM dbo.BoxOfficeWeekend) <> 10
 THROW 50000,'Import a complete box-office chart before retiring news feeds',1;
UPDATE dbo.Sources SET Enabled=0 WHERE Url IN
 (N'https://deadline.com/v/box-office/feed/',N'https://variety.com/v/film/box-office/feed/');
DELETE p FROM dbo.Pages p JOIN dbo.Sources s ON s.ID=p.SourceId
WHERE s.Url IN (N'https://deadline.com/v/box-office/feed/',N'https://variety.com/v/film/box-office/feed/');
COMMIT;
