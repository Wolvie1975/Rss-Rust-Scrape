DECLARE @Path nvarchar(4000)=N'/var/opt/mssql/backup/webscraper-managed/log/WebScraper_log_'+CONVERT(char(8),SYSUTCDATETIME(),112)+N'_'+REPLACE(CONVERT(char(8),SYSUTCDATETIME(),108),':','')+N'.trn';
BACKUP LOG WebScraper TO DISK=@Path WITH CHECKSUM,COMPRESSION;
RESTORE VERIFYONLY FROM DISK=@Path WITH CHECKSUM;
