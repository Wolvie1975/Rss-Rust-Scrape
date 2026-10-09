SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 DECLARE @game INT, @pair INT, @key VARCHAR(100), @url NVARCHAR(2048), @school INT,
 @schoolname NVARCHAR(100), @opp INT, @sport INT, @neutral BIT, @zone NVARCHAR(30),
 @radio NVARCHAR(200), @audio NVARCHAR(2048), @issue NVARCHAR(1000), @eid INT, @count INT;
 SELECT @game=GameId,@pair=PairId,@key=CanonicalKey,@url=SourceUrl,@school=SchoolId,
 @schoolname=SchoolName,@opp=OpponentId,@sport=SportId,@neutral=IsNeutral,@zone=TimeZone,
 @radio=Radio,@audio=AudioUrl,@issue=Issue
 FROM OPENJSON(@P2) WITH(GameId INT '$.provider_game_id',PairId INT '$.paired_game_id',
 CanonicalKey VARCHAR(100) '$.canonical_key',SourceUrl NVARCHAR(2048) '$.event.url',
 SchoolId INT '$.school_id',SchoolName NVARCHAR(100) '$.school_name',OpponentId INT '$.opponent_school_id',
 SportId INT '$.sport_id',IsNeutral BIT '$.is_neutral',TimeZone NVARCHAR(30) '$.calendar_timezone',
 Radio NVARCHAR(200) '$.radio',AudioUrl NVARCHAR(2048) '$.audio_url',Issue NVARCHAR(1000) '$.issue');
 IF @game IS NULL OR @url IS NULL THROW 51000,'Observation requires provider game ID and URL.',1;
 IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ID=@P1 AND Enabled=1 AND ProviderKey='big12')
 BEGIN
  COMMIT TRANSACTION; SELECT 'disabled' AS Outcome; RETURN;
 END;
 IF @issue IS NULL
 BEGIN
  SELECT @count=COUNT(*),@eid=MIN(e.ID) FROM dbo.SportsEvents e WITH(UPDLOCK,HOLDLOCK)
  WHERE (e.ProviderKey='big12' AND (e.CanonicalKey=@key OR e.GameId=@game OR e.GameId=@pair))
   OR e.UrlHash=HASHBYTES('SHA2_256',@url)
   OR EXISTS(SELECT 1 FROM dbo.SportsEventSources s WITH(UPDLOCK,HOLDLOCK)
     JOIN dbo.SportsEventsType f ON f.ID=s.SportsEventsTypeId
     WHERE s.SportsEventId=e.ID AND f.ProviderKey='big12' AND s.ProviderGameId IN(@game,@pair));
  IF @count>1 SET @issue='Multiple existing public events match the provider aliases; reconciliation required';
  IF @count=1 AND EXISTS(SELECT 1 FROM dbo.SportsEvents WHERE ID=@eid AND
    ((ProviderSportId IS NOT NULL AND ProviderSportId<>@sport)
      OR (SchoolId IS NOT NULL AND SchoolId<>@school AND SchoolId<>COALESCE(@opp,-1))))
   SET @issue='Existing canonical sport/participant conflicts with provider aliases';
 END;
 IF @issue IS NULL
 BEGIN
  DECLARE @title NVARCHAR(500),@sportname NVARCHAR(100),@opponent NVARCHAR(200),@away BIT,
   @venue NVARCHAR(300),@date DATE,@start DATETIME2,@end DATETIME2,@tbd BIT,@tv NVARCHAR(200),
   @video NVARCHAR(2048),@stats NVARCHAR(2048),@logo NVARCHAR(2048),@opplogo NVARCHAR(2048);
  SELECT @title=Title,@sportname=Sport,@opponent=Opponent,@away=IsAway,@venue=Location,@date=EventDate,
    @start=CAST(StartUtc AS DATETIME2),@end=CAST(EndUtc AS DATETIME2),@tbd=TimeTbd,@tv=Tv,
    @video=Video,@stats=Stats,@logo=Logo,@opplogo=OpponentLogo
  FROM OPENJSON(@P2,'$.event') WITH(Title NVARCHAR(500) '$.title',Sport NVARCHAR(100) '$.sport',
    Opponent NVARCHAR(200) '$.opponent',IsAway BIT '$.is_away',Location NVARCHAR(300) '$.location',
    EventDate DATE '$.event_date',StartUtc DATETIMEOFFSET '$.starts_at',EndUtc DATETIMEOFFSET '$.ends_at',
    TimeTbd BIT '$.time_tbd',Tv NVARCHAR(200) '$.tv',Video NVARCHAR(2048) '$.stream_url',
    Stats NVARCHAR(2048) '$.live_stats_url',Logo NVARCHAR(2048) '$.team_logo_url',OpponentLogo NVARCHAR(2048) '$.opponent_logo_url');
  IF @eid IS NULL
  BEGIN
   INSERT dbo.SportsEvents(Url,GameId,Title,Sport,Opponent,IsAway,Location,EventDate,StartsAtUtc,EndsAtUtc,
    TimeTbd,Tv,StreamUrl,LiveStatsUrl,TeamLogoUrl,OpponentLogoUrl,SportsEventsTypeId,CanonicalKey,ProviderKey,
    SchoolId,SchoolName,OpponentSchoolId,ProviderSportId,IsNeutral,CalendarTimeZone,Radio,AudioUrl)
   VALUES(@url,@game,@title,@sportname,@opponent,@away,@venue,@date,@start,@end,@tbd,@tv,@video,@stats,
    @logo,@opplogo,@P1,@key,'big12',@school,@schoolname,@opp,@sport,@neutral,@zone,@radio,@audio);
   SET @eid=SCOPE_IDENTITY();
  END
  ELSE
  BEGIN
   -- Primary perspective never flips when the other school is scraped.
   UPDATE dbo.SportsEvents SET
    CanonicalKey=CASE WHEN @pair IS NOT NULL OR CanonicalKey IS NULL THEN @key ELSE CanonicalKey END,
    ProviderKey='big12',
    SchoolId=CASE WHEN SportsEventsTypeId=@P1 THEN @school ELSE SchoolId END,
    SchoolName=CASE WHEN SportsEventsTypeId=@P1 THEN @schoolname ELSE SchoolName END,
    ProviderSportId=@sport,
    EventDate=CASE WHEN SportsEventsTypeId=@P1 THEN @date ELSE EventDate END,
    StartsAtUtc=CASE WHEN SportsEventsTypeId=@P1 OR (TimeTbd=1 AND @tbd=0 AND EventDate=@date) THEN @start ELSE StartsAtUtc END,
    EndsAtUtc=CASE WHEN SportsEventsTypeId=@P1 OR (TimeTbd=1 AND @tbd=0 AND EventDate=@date) THEN @end ELSE EndsAtUtc END,
    TimeTbd=CASE WHEN SportsEventsTypeId=@P1 OR (TimeTbd=1 AND @tbd=0 AND EventDate=@date) THEN @tbd ELSE TimeTbd END,
    Title=CASE WHEN SportsEventsTypeId=@P1 THEN @title ELSE Title END,
    Sport=COALESCE(@sportname,Sport),
    Opponent=CASE WHEN SportsEventsTypeId=@P1 THEN @opponent ELSE Opponent END,
    IsAway=CASE WHEN @neutral=1 THEN NULL WHEN SportsEventsTypeId=@P1 THEN @away ELSE IsAway END,
    IsNeutral=COALESCE(@neutral,IsNeutral),
    Location=CASE WHEN SportsEventsTypeId=@P1 THEN @venue ELSE COALESCE(Location,@venue) END,
    TeamLogoUrl=CASE WHEN SportsEventsTypeId=@P1 THEN @logo ELSE TeamLogoUrl END,
    OpponentLogoUrl=CASE WHEN SportsEventsTypeId=@P1 THEN @opplogo ELSE OpponentLogoUrl END,
    OpponentSchoolId=CASE WHEN SportsEventsTypeId=@P1 THEN @opp ELSE OpponentSchoolId END,
    CalendarTimeZone=CASE WHEN SportsEventsTypeId=@P1 THEN @zone ELSE CalendarTimeZone END,
    Tv=CASE WHEN SportsEventsTypeId=@P1 THEN COALESCE(@tv,Tv) ELSE COALESCE(Tv,@tv) END,
    StreamUrl=CASE WHEN SportsEventsTypeId=@P1 THEN COALESCE(@video,StreamUrl) ELSE COALESCE(StreamUrl,@video) END,
    LiveStatsUrl=CASE WHEN SportsEventsTypeId=@P1 THEN COALESCE(@stats,LiveStatsUrl) ELSE COALESCE(LiveStatsUrl,@stats) END,
    Radio=CASE WHEN SportsEventsTypeId=@P1 THEN COALESCE(@radio,Radio) ELSE COALESCE(Radio,@radio) END,
    AudioUrl=CASE WHEN SportsEventsTypeId=@P1 THEN COALESCE(@audio,AudioUrl) ELSE COALESCE(AudioUrl,@audio) END,LastSeenAt=SYSUTCDATETIME()
   WHERE ID=@eid;
  END;
 END;
 IF EXISTS(SELECT 1 FROM dbo.SportsEventSources WITH(UPDLOCK,HOLDLOCK) WHERE SportsEventsTypeId=@P1 AND ProviderGameId=@game)
  UPDATE dbo.SportsEventSources SET
   SportsEventId=CASE WHEN @issue IS NULL THEN @eid ELSE SportsEventId END,
   Payload=CASE WHEN @issue IS NULL OR SportsEventId IS NULL THEN @P2 ELSE Payload END,
   SourceUrl=CASE WHEN @issue IS NULL OR SportsEventId IS NULL THEN @url ELSE SourceUrl END,
   SchoolId=CASE WHEN @issue IS NULL OR SportsEventId IS NULL THEN @school ELSE SchoolId END,
   SchoolName=CASE WHEN @issue IS NULL OR SportsEventId IS NULL THEN @schoolname ELSE SchoolName END,
   OpponentSchoolId=CASE WHEN @issue IS NULL OR SportsEventId IS NULL THEN @opp ELSE OpponentSchoolId END,
   Issue=@issue,LastSeenAt=SYSUTCDATETIME()
  WHERE SportsEventsTypeId=@P1 AND ProviderGameId=@game;
 ELSE
  INSERT dbo.SportsEventSources(SportsEventId,SportsEventsTypeId,ProviderGameId,SourceUrl,SchoolId,SchoolName,OpponentSchoolId,Payload,Issue)
  VALUES(CASE WHEN @issue IS NULL THEN @eid ELSE NULL END,@P1,@game,@url,@school,@schoolname,@opp,@P2,@issue);
 COMMIT TRANSACTION;
 SELECT CASE WHEN @issue IS NULL THEN 'saved' ELSE 'quarantined' END AS Outcome;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
