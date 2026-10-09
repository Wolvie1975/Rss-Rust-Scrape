-- One validated show + complete episode-list transaction. Missing episodes are never deleted.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.TvTrackedSeries WITH(UPDLOCK,HOLDLOCK) WHERE TvmazeShowId=@P1 AND Enabled=1 AND LeaseToken=TRY_CONVERT(UNIQUEIDENTIFIER,@P3))
 BEGIN UPDATE dbo.TvTrackedSeries SET LeaseUntil=NULL,LeaseToken=NULL WHERE TvmazeShowId=@P1 AND LeaseToken=TRY_CONVERT(UNIQUEIDENTIFIER,@P3); COMMIT TRANSACTION; SELECT 'disabled_or_superseded' AS Outcome; RETURN; END;
 DECLARE @series INT;
 SELECT @series=ID FROM dbo.TvSeries WITH(UPDLOCK,HOLDLOCK) WHERE TvmazeShowId=@P1;
 -- A shorter or partial identity set is rejected rather than replacing any stored schedule.
 IF @series IS NOT NULL AND EXISTS(SELECT 1 FROM dbo.TvEpisodes e WHERE e.SeriesId=@series AND NOT EXISTS(SELECT 1 FROM OPENJSON(@P2,'$.episodes') WITH(Id INT '$.id') s WHERE s.Id=e.TvmazeEpisodeId))
  THROW 51000,'Incomplete episode list omits stored episode IDs; last-good snapshot preserved.',1;
 IF EXISTS(SELECT 1 FROM dbo.TvEpisodes e JOIN OPENJSON(@P2,'$.episodes') WITH(Id INT '$.id') s ON s.Id=e.TvmazeEpisodeId WHERE @series IS NULL OR e.SeriesId<>@series)
  THROW 51000,'Episode ID is already assigned to a different series.',1;
 IF @series IS NOT NULL AND EXISTS(SELECT 1 FROM dbo.TvEpisodes e JOIN OPENJSON(@P2,'$.episodes') WITH(Id INT '$.id',Airdate DATE '$.airdate') s ON s.Id=e.TvmazeEpisodeId WHERE e.SeriesId=@series AND e.Airdate IS NOT NULL AND s.Airdate IS NULL)
  THROW 51000,'Episode snapshot loses a previously known airdate; last-good schedule preserved.',1;
 MERGE dbo.TvSeries WITH(HOLDLOCK) t
 USING (SELECT * FROM OPENJSON(@P2) WITH(
  ShowId INT '$.show_id',Title NVARCHAR(500) '$.title',NetworkId INT '$.network_id',NetworkName NVARCHAR(200) '$.network_name',NetworkCountry CHAR(2) '$.network_country',NetworkZone NVARCHAR(100) '$.network_timezone',
  WebId INT '$.web_channel_id',WebName NVARCHAR(200) '$.web_channel_name',WebCountry CHAR(2) '$.web_channel_country',WebZone NVARCHAR(100) '$.web_channel_timezone',Status NVARCHAR(50) '$.status',Premiere DATE '$.premiere',Poster NVARCHAR(2048) '$.poster_url',Url NVARCHAR(2048) '$.source_url',Imdb VARCHAR(20) '$.imdb_id',Tvdb INT '$.thetvdb_id',Tvrage INT '$.tvrage_id',Raw NVARCHAR(MAX) '$.raw_show' AS JSON)) s
 ON t.TvmazeShowId=s.ShowId
 WHEN MATCHED THEN UPDATE SET Title=s.Title,NetworkId=s.NetworkId,NetworkName=s.NetworkName,NetworkCountryCode=s.NetworkCountry,NetworkTimeZone=s.NetworkZone,
 WebChannelId=s.WebId,WebChannelName=s.WebName,WebChannelCountryCode=s.WebCountry,WebChannelTimeZone=s.WebZone,Status=s.Status,
 PremiereDate=COALESCE(s.Premiere,t.PremiereDate),PosterUrl=COALESCE(s.Poster,t.PosterUrl),SourceUrl=s.Url,ImdbId=COALESCE(s.Imdb,t.ImdbId),TheTvdbId=COALESCE(s.Tvdb,t.TheTvdbId),TvRageId=COALESCE(s.Tvrage,t.TvRageId),RawMetadata=s.Raw,LastSeenAt=SYSUTCDATETIME()
 WHEN NOT MATCHED THEN INSERT(TvmazeShowId,Title,NetworkId,NetworkName,NetworkCountryCode,NetworkTimeZone,WebChannelId,WebChannelName,WebChannelCountryCode,WebChannelTimeZone,Status,PremiereDate,PosterUrl,SourceUrl,ImdbId,TheTvdbId,TvRageId,RawMetadata)
 VALUES(s.ShowId,s.Title,s.NetworkId,s.NetworkName,s.NetworkCountry,s.NetworkZone,s.WebId,s.WebName,s.WebCountry,s.WebZone,s.Status,s.Premiere,s.Poster,s.Url,s.Imdb,s.Tvdb,s.Tvrage,s.Raw);
 SELECT @series=ID FROM dbo.TvSeries WHERE TvmazeShowId=@P1;
 MERGE dbo.TvEpisodes WITH(HOLDLOCK) t
 USING (SELECT * FROM OPENJSON(@P2,'$.episodes') WITH(Id INT '$.id',Season INT '$.season',Number INT '$.number',Title NVARCHAR(500) '$.title',Type VARCHAR(50) '$.type',Special BIT '$.special',Airdate DATE '$.airdate',Airtime TIME(0) '$.airtime',Stamp DATETIMEOFFSET '$.starts_at_utc',DateOnly BIT '$.date_only',Runtime INT '$.runtime',Image NVARCHAR(2048) '$.image_url',Url NVARCHAR(2048) '$.source_url',RawDate NVARCHAR(50) '$.raw_airdate',RawTime NVARCHAR(50) '$.raw_airtime',RawStamp NVARCHAR(100) '$.raw_airstamp',Raw NVARCHAR(MAX) '$.raw' AS JSON)) s
 ON t.TvmazeEpisodeId=s.Id
 WHEN MATCHED THEN UPDATE SET SeasonNumber=COALESCE(s.Season,t.SeasonNumber),EpisodeNumber=COALESCE(s.Number,t.EpisodeNumber),
 Title=CASE WHEN s.Title IS NULL OR s.Title IN('','TBA') THEN COALESCE(t.Title,s.Title) ELSE s.Title END,EpisodeType=s.Type,IsSpecial=s.Special,
 Airdate=COALESCE(s.Airdate,t.Airdate),Airtime=CASE WHEN s.Airdate IS NOT NULL OR t.Airdate IS NULL THEN s.Airtime ELSE t.Airtime END,
 StartsAtUtc=CASE WHEN s.Airdate IS NOT NULL OR t.Airdate IS NULL THEN CAST(s.Stamp AS DATETIME2) ELSE t.StartsAtUtc END,
 IsDateOnly=CASE WHEN s.Airdate IS NOT NULL OR t.Airdate IS NULL THEN s.DateOnly ELSE t.IsDateOnly END,
 RuntimeMinutes=COALESCE(s.Runtime,t.RuntimeMinutes),ImageUrl=COALESCE(s.Image,t.ImageUrl),SourceUrl=s.Url,
 RawAirdate=s.RawDate,RawAirtime=s.RawTime,RawAirstamp=s.RawStamp,RawMetadata=s.Raw,LastSeenAt=SYSUTCDATETIME()
 WHEN NOT MATCHED THEN INSERT(TvmazeEpisodeId,SeriesId,SeasonNumber,EpisodeNumber,Title,EpisodeType,IsSpecial,Airdate,Airtime,StartsAtUtc,IsDateOnly,RuntimeMinutes,ImageUrl,SourceUrl,RawAirdate,RawAirtime,RawAirstamp,RawMetadata)
 VALUES(s.Id,@series,s.Season,s.Number,s.Title,s.Type,s.Special,s.Airdate,s.Airtime,CAST(s.Stamp AS DATETIME2),s.DateOnly,s.Runtime,s.Image,s.Url,s.RawDate,s.RawTime,s.RawStamp,s.Raw);
 UPDATE dbo.TvTrackedSeries SET SeriesId=@series,LastSuccessAt=SYSUTCDATETIME(),LastError=NULL,
 EpisodeCount=(SELECT COUNT(*) FROM OPENJSON(@P2,'$.episodes')),LastIssueCount=TRY_CONVERT(INT,JSON_VALUE(@P2,'$.issue_count')),
 CacheExpiresAt=DATEADD(hour,1,SYSUTCDATETIME()),RetryAfter=NULL,LeaseUntil=NULL,LeaseToken=NULL,
 CachedShow=JSON_QUERY(@P2,'$.raw_show'),CachedEpisodes=JSON_QUERY(@P2,'$.raw_episodes') WHERE TvmazeShowId=@P1;
 COMMIT TRANSACTION;
 SELECT 'saved' AS Outcome;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
