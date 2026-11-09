-- Targeted optimizer-statistics maintenance; no application rows are deleted.
-- Preview by default. Set @Execute=1 only for an explicitly authorized maintenance run.
-- The 1,000-modification/day thresholds are review heuristics, not performance guarantees.
SET NOCOUNT ON; SET LOCK_TIMEOUT 10000;
DECLARE @Execute BIT=0;
DECLARE @Targets TABLE(ObjectId INT,StatsId INT,TableName NVARCHAR(300),StatsName SYSNAME);
INSERT @Targets
SELECT st.object_id,st.stats_id,QUOTENAME(SCHEMA_NAME(t.schema_id))+N'.'+QUOTENAME(t.name),st.name
FROM sys.stats st JOIN sys.tables t ON t.object_id=st.object_id
CROSS APPLY sys.dm_db_stats_properties(st.object_id,st.stats_id) p
WHERE p.modification_counter>1000 AND (p.last_updated<DATEADD(day,-1,SYSUTCDATETIME()) OR p.modification_counter>p.rows);
SELECT COUNT(*) StatisticsToRefresh,COUNT(DISTINCT ObjectId) TablesAffected FROM @Targets;
IF @Execute=0
BEGIN
 SELECT TableName,StatsName FROM @Targets ORDER BY TableName,StatsName;
 RETURN;
END;
DECLARE @TableName NVARCHAR(300),@Stat SYSNAME,@Statement NVARCHAR(MAX),@Updated INT=0;
DECLARE StatsCursor CURSOR LOCAL FAST_FORWARD FOR SELECT TableName,StatsName FROM @Targets;
OPEN StatsCursor;
FETCH NEXT FROM StatsCursor INTO @TableName,@Stat;
WHILE @@FETCH_STATUS=0
BEGIN
 SET @Statement=N'UPDATE STATISTICS '+@TableName+N' '+QUOTENAME(@Stat)+N' WITH FULLSCAN,MAXDOP=1;';
 EXEC sys.sp_executesql @Statement;
 SET @Updated+=1;
 FETCH NEXT FROM StatsCursor INTO @TableName,@Stat;
END;
CLOSE StatsCursor;DEALLOCATE StatsCursor;
SELECT @Updated StatisticsRefreshed;
SELECT CAST(OBJECT_NAME(x.ObjectId) AS NVARCHAR(50)) TableName,COUNT(*) StatisticsRefreshed,
 MAX(p.last_updated) LatestRefresh,MAX(p.modification_counter) MaximumModificationsSinceRefresh
FROM @Targets x CROSS APPLY sys.dm_db_stats_properties(x.ObjectId,x.StatsId) p
GROUP BY x.ObjectId ORDER BY TableName;
