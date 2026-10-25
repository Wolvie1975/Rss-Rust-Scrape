use chrono::Utc;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::events::Event;
use crate::pages::PageMeta;
use crate::youtube::Video;

const DB_NAME: &str = "WebScraper";

type Conn = Client<Compat<TcpStream>>;
use crate::Result;
const DB_TIMEOUT: Duration = Duration::from_secs(180);
async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T>>,
    poisoned: &AtomicBool,
) -> Result<T> {
    if poisoned.load(Ordering::Relaxed) {
        return Err("SQL connection unusable after timeout; reconnect required".into());
    }
    match tokio::time::timeout(DB_TIMEOUT, future).await {
        Ok(result) => result,
        Err(_) => {
            poisoned.store(true, Ordering::Relaxed);
            Err("SQL operation timed out; connection must be replaced".into())
        }
    }
}
fn checked_keep(keep: u32) -> Result<i32> {
    i32::try_from(keep)
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| "retention count must be between 1 and i32::MAX".into())
}
fn utf16_prefix(raw: &str, max: usize) -> String {
    let mut n = 0;
    raw.chars()
        .take_while(|c| {
            n += c.len_utf16();
            n <= max
        })
        .collect()
}

async fn connect(ado: &str, database: &str) -> Result<Conn> {
    let mut config = Config::from_ado_string(ado)?;
    config.database(database);
    let tcp = tokio::time::timeout(
        Duration::from_secs(10),
        TcpStream::connect(config.get_addr()),
    )
    .await??;
    tcp.set_nodelay(true)?;
    let mut client = tokio::time::timeout(
        Duration::from_secs(15),
        Client::connect(config, tcp.compat_write()),
    )
    .await??;
    tokio::time::timeout(
        Duration::from_secs(10),
        client.simple_query("SET LOCK_TIMEOUT 15000; SET XACT_ABORT ON;"),
    )
    .await??
    .into_results()
    .await?;
    Ok(client)
}

/// A row of the `SportsEventsType` lookup table: one calendar feed.
pub struct EventFeed {
    pub id: i32,
    pub url: String,
    /// The host school as written in the feed's titles, e.g. "Kansas".
    pub school: Option<String>,
}

/// A single SQL Server session on the `WebScraper` database.
pub struct Db {
    rt: Runtime,
    client: Conn,
    poisoned: AtomicBool,
}

impl Db {
    /// Connects, creating the database and its tables first if they are missing.
    pub fn open(ado: &str) -> Result<Db> {
        Self::open_named(ado, DB_NAME)
    }

    /// Ingestion only: no master connection, DDL or automatic migrations.
    pub fn connect_named(ado: &str, database: &str) -> Result<Db> {
        if database.is_empty()
            || !database
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("invalid database name".into());
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let poisoned = AtomicBool::new(false);
        let mut client = rt.block_on(bounded(connect(ado, database), &poisoned))?;
        rt.block_on(bounded(async {
            let r=client.query("SELECT CASE WHEN OBJECT_ID('dbo.ScraperRuns','U') IS NOT NULL AND COL_LENGTH('dbo.SportsEvents','ScheduleStatus') IS NOT NULL AND OBJECT_ID('dbo.UpsertArticleIdentity','P') IS NOT NULL THEN 1 ELSE 0 END",&[]).await?.into_first_result().await?;
            if r.first().and_then(|r|r.get::<i32,_>(0))!=Some(1){return Err("schema missing: apply scraper migrations with migration credentials".into())}
            Ok(())
        },&poisoned))?;
        Ok(Db {
            rt,
            client,
            poisoned,
        })
    }

    pub fn open_named(ado: &str, database: &str) -> Result<Db> {
        if database.is_empty()
            || !database
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(
                "database name must contain only ASCII letters, digits or underscores".into(),
            );
        }
        let poisoned = AtomicBool::new(false);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = rt.block_on(bounded(async {
            let mut master = connect(ado, "master").await?;
            master
                .execute(
                    format!("IF DB_ID('{database}') IS NULL CREATE DATABASE [{database}]"),
                    &[],
                )
                .await?;
            drop(master);

            let mut client = connect(ado, database).await?;
            for ddl in [
                "IF OBJECT_ID('dbo.SourceCategories', 'U') IS NULL
                 CREATE TABLE dbo.SourceCategories (
                     ID            INT IDENTITY(1,1) CONSTRAINT PK_SourceCategories PRIMARY KEY,
                     Category_Name NVARCHAR(100) NOT NULL,
                     DateAdded     DATETIME2     NOT NULL DEFAULT SYSUTCDATETIME(),
                     CONSTRAINT UQ_SourceCategories_Category_Name UNIQUE (Category_Name)
                 )",
                "IF OBJECT_ID('dbo.Sources', 'U') IS NULL
                 CREATE TABLE dbo.Sources (
                     ID               INT IDENTITY(1,1) PRIMARY KEY,
                     Url              NVARCHAR(2048) NOT NULL,
                     UrlHash          AS CAST(HASHBYTES('SHA2_256', Url) AS BINARY(32)) PERSISTED,
                     Label            NVARCHAR(200)  NULL,
                     Enabled          BIT            NOT NULL DEFAULT 1,
                     CreatedAt        DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     LastScrapedAt    DATETIME2      NULL,
                     LastError        NVARCHAR(1000) NULL,
                     SourceCategoryId INT NULL CONSTRAINT FK_Sources_SourceCategories
                                          REFERENCES dbo.SourceCategories (ID),
                     CONSTRAINT UQ_Sources_UrlHash UNIQUE (UrlHash)
                 )",
                "IF OBJECT_ID('dbo.Pages', 'U') IS NULL
                 CREATE TABLE dbo.Pages (
                     ID          INT IDENTITY(1,1) PRIMARY KEY,
                     Url         NVARCHAR(2048) NOT NULL,
                     UrlHash     AS CAST(HASHBYTES('SHA2_256', Url) AS BINARY(32)) PERSISTED,
                     Title       NVARCHAR(500)  NOT NULL,
                     Description NVARCHAR(MAX)  NULL,
                     Published   DATETIME2      NULL,
                     ScrapedAt   DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     SourceId    INT NOT NULL CONSTRAINT FK_Pages_Sources REFERENCES dbo.Sources (ID),
                     CONSTRAINT UQ_Pages_UrlHash UNIQUE (UrlHash)
                 )",
                "IF COL_LENGTH('dbo.Pages', 'ImageUrl') IS NULL
                 ALTER TABLE dbo.Pages ADD ImageUrl NVARCHAR(2048) NULL",
                "IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_Pages_SourceId' AND object_id = OBJECT_ID('dbo.Pages'))
                 CREATE INDEX IX_Pages_SourceId ON dbo.Pages (SourceId)",
                "IF OBJECT_ID('dbo.SportsEventsType', 'U') IS NULL
                 CREATE TABLE dbo.SportsEventsType (
                     ID             INT IDENTITY(1,1) CONSTRAINT PK_SportsEventsType PRIMARY KEY,
                     RssUrl         NVARCHAR(450) NOT NULL,
                     EventsTypeName NVARCHAR(200) NOT NULL,
                     DateAdded      DATETIME2     NOT NULL DEFAULT SYSUTCDATETIME(),
                     SchoolName     NVARCHAR(100) NULL,
                     CONSTRAINT UQ_SportsEventsType_RssUrl UNIQUE (RssUrl)
                 )",
                "IF OBJECT_ID('dbo.SportsEvents', 'U') IS NULL
                 CREATE TABLE dbo.SportsEvents (
                     ID              INT IDENTITY(1,1) PRIMARY KEY,
                     Url             NVARCHAR(2048) NOT NULL,
                     UrlHash         AS CAST(HASHBYTES('SHA2_256', Url) AS BINARY(32)) PERSISTED,
                     GameId          INT            NULL,
                     Title           NVARCHAR(500)  NOT NULL,
                     Sport           NVARCHAR(100)  NULL,
                     Opponent        NVARCHAR(200)  NULL,
                     IsAway          BIT            NULL,
                     Location        NVARCHAR(300)  NULL,
                     EventDate       DATE           NOT NULL,
                     StartsAtUtc     DATETIME2      NULL,
                     EndsAtUtc       DATETIME2      NULL,
                     TimeTbd         BIT            NOT NULL,
                     Tv              NVARCHAR(200)  NULL,
                     StreamUrl       NVARCHAR(2048) NULL,
                     LiveStatsUrl    NVARCHAR(2048) NULL,
                     TeamLogoUrl     NVARCHAR(2048) NULL,
                     OpponentLogoUrl NVARCHAR(2048) NULL,
                     SportsEventsTypeId INT         NULL CONSTRAINT FK_SportsEvents_SportsEventsType
                                                         REFERENCES dbo.SportsEventsType (ID),
                     FirstSeenAt     DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     LastSeenAt      DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     CONSTRAINT UQ_SportsEvents_UrlHash UNIQUE (UrlHash)
                 )",
                "IF COL_LENGTH('dbo.SportsEventsType', 'SchoolName') IS NULL
                 ALTER TABLE dbo.SportsEventsType ADD SchoolName NVARCHAR(100) NULL",
                "IF COL_LENGTH('dbo.SportsEvents', 'SportsEventsTypeId') IS NULL
                 ALTER TABLE dbo.SportsEvents ADD SportsEventsTypeId INT NULL
                     CONSTRAINT FK_SportsEvents_SportsEventsType REFERENCES dbo.SportsEventsType (ID)",
                "IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_SportsEvents_SportsEventsTypeId' AND object_id = OBJECT_ID('dbo.SportsEvents'))
                 CREATE INDEX IX_SportsEvents_SportsEventsTypeId ON dbo.SportsEvents (SportsEventsTypeId)",
                "IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_SportsEvents_EventDate' AND object_id = OBJECT_ID('dbo.SportsEvents'))
                 CREATE INDEX IX_SportsEvents_EventDate ON dbo.SportsEvents (EventDate)",
                "IF OBJECT_ID('dbo.YoutubeVideoFeed', 'U') IS NULL
                 CREATE TABLE dbo.YoutubeVideoFeed (
                     ID          INT IDENTITY(1,1) CONSTRAINT PK_YoutubeVideoFeed PRIMARY KEY,
                     ChannelId   NVARCHAR(40)   NOT NULL,
                     ChannelName NVARCHAR(200)  NULL,
                     Url         NVARCHAR(2048) NOT NULL,
                     DateAdded   DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     CONSTRAINT UQ_YoutubeVideoFeed_ChannelId UNIQUE (ChannelId)
                 )",
                "IF OBJECT_ID('dbo.YouTubeVideos', 'U') IS NULL
                 CREATE TABLE dbo.YouTubeVideos (
                     ID            INT IDENTITY(1,1) PRIMARY KEY,
                     VideoId       NVARCHAR(20)   NOT NULL,
                     ChannelId     NVARCHAR(40)   NOT NULL,
                     ChannelName   NVARCHAR(200)  NULL,
                     Title         NVARCHAR(500)  NOT NULL,
                     Url           NVARCHAR(2048) NOT NULL,
                     PublishedAt   DATETIME2      NOT NULL,
                     UpdatedAt     DATETIME2      NULL,
                     ThumbnailUrl  NVARCHAR(2048) NULL,
                     Description   NVARCHAR(MAX)  NULL,
                     ViewCount     BIGINT         NULL,
                     RatingCount   INT            NULL,
                     RatingAverage DECIMAL(3,2)   NULL,
                     YoutubeVideoFeedId INT       NULL CONSTRAINT FK_YouTubeVideos_YoutubeVideoFeed
                                                       REFERENCES dbo.YoutubeVideoFeed (ID),
                     FirstSeenAt   DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     LastSeenAt    DATETIME2      NOT NULL DEFAULT SYSUTCDATETIME(),
                     CONSTRAINT UQ_YouTubeVideos_VideoId UNIQUE (VideoId)
                 )",
                "IF COL_LENGTH('dbo.YouTubeVideos', 'YoutubeVideoFeedId') IS NULL
                 ALTER TABLE dbo.YouTubeVideos ADD YoutubeVideoFeedId INT NULL
                     CONSTRAINT FK_YouTubeVideos_YoutubeVideoFeed REFERENCES dbo.YoutubeVideoFeed (ID)",
                "IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_YouTubeVideos_YoutubeVideoFeedId' AND object_id = OBJECT_ID('dbo.YouTubeVideos'))
                 CREATE INDEX IX_YouTubeVideos_YoutubeVideoFeedId ON dbo.YouTubeVideos (YoutubeVideoFeedId)",
                "IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_YouTubeVideos_Channel_PublishedAt' AND object_id = OBJECT_ID('dbo.YouTubeVideos'))
                 CREATE INDEX IX_YouTubeVideos_Channel_PublishedAt ON dbo.YouTubeVideos (ChannelId, PublishedAt DESC)",
                "IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_Sources_SourceCategoryId' AND object_id = OBJECT_ID('dbo.Sources'))
                 CREATE INDEX IX_Sources_SourceCategoryId ON dbo.Sources (SourceCategoryId)",
            ] {
                client.execute(ddl, &[]).await?;
            }
            client
                .simple_query(include_str!("../sql/movie_releases.sql"))
                .await?
                .into_results()
                .await?;
            client.simple_query(include_str!("../sql/movie_posters.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/movie_commerce.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/movie_commerce_view.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/big12_events.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/nwsl_events.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/rankings.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/championship.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/soccer_shootouts.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/standings.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/scraper_runs.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/article_identity.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/upsert_article_identity.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/big12_views.sql")).await?.into_results().await?;
            Result::Ok(client)
        },&poisoned))?;
        Ok(Db {
            rt,
            client,
            poisoned,
        })
    }

    pub fn retry_missing_posters(&mut self) -> Result<()> {
        self.rt.block_on(bounded(async {
            self.client.execute("UPDATE dbo.Movies SET PosterRetryAfter=NULL WHERE PosterUrl IS NULL; UPDATE c SET RetryAfter=SYSUTCDATETIME() FROM dbo.MoviePosterCache c JOIN dbo.MovieSourceLinks l ON l.ID=c.SourceLinkId JOIN dbo.Movies m ON m.ID=l.MovieId WHERE m.PosterUrl IS NULL",&[]).await?;Ok(())
        },&self.poisoned))
    }

    pub fn poster_jobs(&mut self) -> Result<Vec<crate::posters::PosterJob>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("SELECT m.ID,m.Title,m.OriginalYear,m.ImdbId,m.TmdbId,m.PosterUrl,m.PosterCheckedAt,m.PosterRetryAfter,m.PosterLookupStatus,l.ID AS LinkId,l.Url,s.SourceKey,c.TargetIdentity,c.Metadata,c.PosterUrl AS CachePoster,c.RetryAfter,c.LastError FROM dbo.Movies m JOIN dbo.MovieSourceLinks l ON l.MovieId=m.ID JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId LEFT JOIN dbo.MoviePosterCache c ON c.SourceLinkId=l.ID WHERE s.Enabled=1 ORDER BY m.ID,CASE s.SourceKey WHEN 'dvdsreleasedates' THEN 0 ELSE 1 END,l.ID",&[]).await?.into_first_result().await?;
            Ok(rows.iter().map(|r|{
                let (title,suffix_year)=crate::movie_metadata::title_year(r.get::<&str,_>("Title").unwrap());
                crate::posters::PosterJob { movie_id:r.get::<i32,_>("ID").unwrap(),link_id:r.get::<i32,_>("LinkId").unwrap(),source:r.get::<&str,_>("SourceKey").unwrap().to_owned(),url:r.get::<&str,_>("Url").unwrap().to_owned(),
                    identity:crate::movie_metadata::MovieIdentity{title,year:r.get::<i16,_>("OriginalYear").or(suffix_year),imdb_id:r.get::<&str,_>("ImdbId").map(str::to_owned),tmdb_id:r.get::<i32,_>("TmdbId")},
                    existing:r.get::<&str,_>("PosterUrl").map(str::to_owned),poster_status:r.get::<&str,_>("PosterLookupStatus").map(str::to_owned),checked:r.get::<chrono::NaiveDateTime,_>("PosterCheckedAt"),retry:r.get::<chrono::NaiveDateTime,_>("PosterRetryAfter"),cache_identity:r.get::<&str,_>("TargetIdentity").map(str::to_owned),cache_metadata:r.get::<&str,_>("Metadata").map(str::to_owned),cache_poster:r.get::<&str,_>("CachePoster").map(str::to_owned),cache_retry:r.get::<chrono::NaiveDateTime,_>("RetryAfter"),cache_error:r.get::<&str,_>("LastError").map(str::to_owned)}
            }).collect())
        },&self.poisoned))
    }

    pub fn cache_poster_source(
        &mut self,
        link: i32,
        identity: &str,
        metadata: &serde_json::Value,
        poster: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let metadata = serde_json::to_string(metadata)?;
        let error = error.map(|e| e.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(async {
            self.client.execute("SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
             IF EXISTS(SELECT 1 FROM dbo.MoviePosterCache WITH(UPDLOCK,HOLDLOCK) WHERE SourceLinkId=@P1)
              UPDATE dbo.MoviePosterCache SET TargetIdentity=@P2,Metadata=@P3,PosterUrl=@P4,LastError=@P5,CheckedAt=SYSUTCDATETIME(),RetryAfter=CASE WHEN @P5 IS NOT NULL THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,CASE WHEN @P4 IS NULL THEN 7 ELSE 30 END,SYSUTCDATETIME()) END WHERE SourceLinkId=@P1;
             ELSE INSERT dbo.MoviePosterCache(SourceLinkId,TargetIdentity,Metadata,PosterUrl,LastError,CheckedAt,RetryAfter) VALUES(@P1,@P2,@P3,@P4,@P5,SYSUTCDATETIME(),CASE WHEN @P5 IS NOT NULL THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,CASE WHEN @P4 IS NULL THEN 7 ELSE 30 END,SYSUTCDATETIME()) END);
             COMMIT TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT>0 ROLLBACK TRANSACTION; THROW; END CATCH",&[&link,&identity,&metadata,&poster,&error]).await?;Ok(())
        },&self.poisoned))
    }

    #[allow(clippy::too_many_arguments)] // Parameter order mirrors the bound SQL columns.
    pub fn set_movie_poster(
        &mut self,
        id: i32,
        expected: Option<&str>,
        poster: Option<&str>,
        year: Option<i16>,
        status: &str,
        source: Option<&str>,
        error: Option<&str>,
        days: i32,
    ) -> Result<bool> {
        let error = error.map(|e| e.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(async {
            let updated=self.client.execute("UPDATE dbo.Movies SET PosterUrl=@P3,OriginalYear=COALESCE(OriginalYear,@P4),PosterCheckedAt=SYSUTCDATETIME(),PosterRetryAfter=CASE WHEN @P8=0 THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,@P8,SYSUTCDATETIME()) END,PosterLookupStatus=@P5,PosterSourceUrl=COALESCE(@P6,PosterSourceUrl),PosterLookupError=@P7 WHERE ID=@P1 AND (PosterUrl=@P2 OR (PosterUrl IS NULL AND @P2 IS NULL)) AND (OriginalYear IS NULL OR @P4 IS NULL OR OriginalYear=@P4)",&[&id,&expected,&poster,&year,&status,&source,&error,&days]).await?;Ok(updated.total()>0)
        },&self.poisoned))
    }

    pub fn setup_tv(&mut self, seed: bool) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/tvmaze.sql"))
                    .await?
                    .into_results()
                    .await?;
                if seed {
                    self.client
                        .simple_query(include_str!("../sql/tvmaze_seed.sql"))
                        .await?
                        .into_results()
                        .await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn tv_jobs(&mut self) -> Result<Vec<(i32, String)>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("SELECT TvmazeShowId,ExpectedTitle FROM dbo.TvTrackedSeries WHERE Enabled=1 ORDER BY ID",&[]).await?.into_first_result().await?;
            Ok(rows.iter().map(|r|(r.get::<i32,_>(0).unwrap(),r.get::<&str,_>(1).unwrap().to_owned())).collect())
        },&self.poisoned))
    }
    pub fn claim_tv_series(&mut self, id: i32, force: bool) -> Result<Option<String>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("UPDATE dbo.TvTrackedSeries WITH(UPDLOCK,ROWLOCK) SET LastAttemptAt=SYSUTCDATETIME(),LeaseUntil=DATEADD(minute,10,SYSUTCDATETIME()),LeaseToken=NEWID() OUTPUT CONVERT(VARCHAR(36),inserted.LeaseToken) WHERE TvmazeShowId=@P1 AND Enabled=1 AND (LeaseUntil IS NULL OR LeaseUntil<=SYSUTCDATETIME()) AND (@P2=1 OR ((CacheExpiresAt IS NULL OR CacheExpiresAt<=SYSUTCDATETIME()) AND (RetryAfter IS NULL OR RetryAfter<=SYSUTCDATETIME())))",&[&id,&force]).await?.into_first_result().await?;
            Ok(rows.first().and_then(|r|r.get::<&str,_>(0)).map(str::to_owned))
        },&self.poisoned))
    }
    pub fn save_tv_snapshot(
        &mut self,
        id: i32,
        lease: &str,
        payload: &serde_json::Value,
    ) -> Result<String> {
        let payload = serde_json::to_string(payload)?;
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        include_str!("../sql/upsert_tvmaze_snapshot.sql"),
                        &[&id, &payload, &lease],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .first()
                    .and_then(|r| r.get::<&str, _>("Outcome"))
                    .unwrap_or("unknown")
                    .to_owned())
            },
            &self.poisoned,
        ))
    }
    pub fn fail_tv_series(&mut self, id: i32, lease: &str, error: &str, retry: i32) -> Result<()> {
        let error = error.chars().take(1000).collect::<String>();
        self.rt.block_on(bounded(async {
            self.client.execute("UPDATE dbo.TvTrackedSeries SET LastError=@P3,LastIssueCount=1,RetryAfter=DATEADD(second,@P4,SYSUTCDATETIME()),LeaseUntil=NULL,LeaseToken=NULL WHERE TvmazeShowId=@P1 AND LeaseToken=TRY_CONVERT(UNIQUEIDENTIFIER,@P2)",&[&id,&lease,&error,&retry]).await?;Ok(())
        },&self.poisoned))
    }

    pub fn movie_commerce_jobs(&mut self) -> Result<Vec<crate::movie_commerce::Job>> {
        use crate::movie_commerce::{Job, Provider};
        self.rt.block_on(bounded(async {
            let available=self.client.query("SELECT OBJECT_ID('dbo.MovieCommerceLinks','U') AS Id",&[]).await?.into_first_result().await?;
            if available.first().and_then(|r|r.get::<i32,_>(0)).is_none() {
                eprintln!("movie retailer links: migration not applied; see sql/movie_commerce.sql");
                return Ok(vec![]);
            }
            let rows=self.client.query(r#"
DECLARE @Today DATE=CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time');
DECLARE @Monday DATE=DATEADD(day,-((DATEDIFF(day,CONVERT(date,'19000101'),@Today)%7+7)%7),@Today);
SELECT DISTINCT m.ID,m.Title,m.OriginalYear,m.ImdbId,l.Url,
 CASE WHEN r.ReleaseType='theatrical' THEN 'fandango' ELSE 'fandango-at-home' END AS Provider,
 CASE WHEN c.MovieId IS NULL OR c.RetryAfter IS NULL OR c.RetryAfter<=SYSUTCDATETIME()
 OR c.IdentityTitle COLLATE Latin1_General_100_BIN2<>m.Title COLLATE Latin1_General_100_BIN2
 OR ISNULL(c.IdentityYear,0)<>ISNULL(m.OriginalYear,0) OR ISNULL(c.IdentityImdbId,'')<>ISNULL(m.ImdbId,'') THEN 1 ELSE 0 END AS Due,
 c.CheckedAt,c.DirectUrl,c.VerifiedUntil
FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId
JOIN dbo.Movies m ON m.ID=l.MovieId JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId
LEFT JOIN dbo.MovieCommerceLinks c ON c.MovieId=m.ID AND c.Provider=CASE WHEN r.ReleaseType='theatrical' THEN 'fandango' ELSE 'fandango-at-home' END
WHERE s.Enabled=1 AND r.CountryCode='US' AND r.ReleaseStatus='scheduled' AND r.DateStatus='announced'
AND r.ReleaseDate>=@Monday AND r.ReleaseType IN ('theatrical','digital','digital_purchase','digital_rental')
ORDER BY c.CheckedAt,m.ID;
"#,&[]).await?.into_first_result().await?;
            let mut jobs:Vec<Job>=vec![];
            for r in rows {
                let movie_id=r.get::<i32,_>("ID").ok_or("missing movie ID")?;
                let provider=Provider::from_key(r.get::<&str,_>("Provider").unwrap_or("")).ok_or("invalid retailer provider")?;
                let source=r.get::<&str,_>("Url").ok_or("missing movie source URL")?.to_owned();
                if let Some(j)=jobs.iter_mut().find(|j|j.movie_id==movie_id && j.provider==provider) { j.source_urls.push(source); continue; }
                jobs.push(Job{movie_id,provider,source_urls:vec![source],due:r.get::<i32,_>("Due")==Some(1),cached_direct:r.get::<&str,_>("DirectUrl").map(str::to_owned),verified_until:r.get::<chrono::NaiveDateTime,_>("VerifiedUntil"),identity:crate::movie_metadata::MovieIdentity{
                    title:r.get::<&str,_>("Title").unwrap_or("").to_owned(),year:r.get::<i16,_>("OriginalYear"),imdb_id:r.get::<&str,_>("ImdbId").map(str::to_owned),tmdb_id:None,
                }});
            }
            Ok(jobs)
        },&self.poisoned))
    }
    pub fn seed_movie_commerce(&mut self, job: &crate::movie_commerce::Job) -> Result<()> {
        let provider = job.provider.key();
        let search = job.provider.search(&job.identity.title);
        self.rt.block_on(bounded(async {
            self.client.execute(r#"
MERGE dbo.MovieCommerceLinks WITH(HOLDLOCK) t USING(SELECT @P1 AS MovieId,@P2 AS Provider) s
ON t.MovieId=s.MovieId AND t.Provider=s.Provider
WHEN MATCHED AND (t.IdentityTitle COLLATE Latin1_General_100_BIN2<>@P3 COLLATE Latin1_General_100_BIN2 OR ISNULL(t.IdentityYear,0)<>ISNULL(@P4,0) OR ISNULL(t.IdentityImdbId,'')<>ISNULL(@P5,''))
THEN UPDATE SET IdentityTitle=@P3,IdentityYear=@P4,IdentityImdbId=@P5,SearchUrl=@P6,DirectUrl=NULL,PurchaseVerified=0,VerifiedUntil=NULL,CheckedAt=NULL,RetryAfter=NULL,LastError=NULL
WHEN NOT MATCHED THEN INSERT(MovieId,Provider,IdentityTitle,IdentityYear,IdentityImdbId,SearchUrl) VALUES(@P1,@P2,@P3,@P4,@P5,@P6);
"#,&[&job.movie_id,&provider,&job.identity.title,&job.identity.year,&job.identity.imdb_id,&search]).await?;
            Ok(())
        },&self.poisoned))
    }
    pub fn save_movie_commerce(
        &mut self,
        job: &crate::movie_commerce::Job,
        outcome: &crate::movie_commerce::Outcome,
    ) -> Result<()> {
        let provider = job.provider.key();
        self.rt.block_on(bounded(async {
            // Errors preserve a previously verified link only until its original expiry.
            self.client.execute(r#"
UPDATE c SET DirectUrl=CASE WHEN @P8 IS NOT NULL THEN c.DirectUrl ELSE @P6 END,
 PurchaseVerified=CASE WHEN @P8 IS NOT NULL THEN c.PurchaseVerified ELSE @P7 END,
 VerifiedUntil=CASE WHEN @P8 IS NOT NULL THEN c.VerifiedUntil WHEN @P6 IS NOT NULL THEN CASE WHEN @P9<DATEADD(day,7,SYSUTCDATETIME()) THEN @P9 ELSE DATEADD(day,7,SYSUTCDATETIME()) END ELSE NULL END,
 CheckedAt=SYSUTCDATETIME(),RetryAfter=CASE WHEN @P9<DATEADD(hour,CASE WHEN @P6 IS NOT NULL THEN 168 ELSE 6 END,SYSUTCDATETIME()) THEN @P9 ELSE DATEADD(hour,CASE WHEN @P6 IS NOT NULL THEN 168 ELSE 6 END,SYSUTCDATETIME()) END,LastError=@P8
FROM dbo.MovieCommerceLinks c JOIN dbo.Movies m ON m.ID=c.MovieId
WHERE c.MovieId=@P1 AND c.Provider=@P2 AND c.IdentityTitle COLLATE Latin1_General_100_BIN2=@P3 COLLATE Latin1_General_100_BIN2
AND ISNULL(c.IdentityYear,0)=ISNULL(@P4,0) AND ISNULL(c.IdentityImdbId,'')=ISNULL(@P5,'')
AND m.Title COLLATE Latin1_General_100_BIN2=c.IdentityTitle COLLATE Latin1_General_100_BIN2
AND ISNULL(m.OriginalYear,0)=ISNULL(@P4,0) AND ISNULL(m.ImdbId,'')=ISNULL(@P5,'');
"#,&[&job.movie_id,&provider,&job.identity.title,&job.identity.year,&job.identity.imdb_id,&outcome.direct_url,&outcome.purchase,&outcome.error,&outcome.reviewed_until]).await?;
            Ok(())
        },&self.poisoned))
    }

    pub fn enabled_movie_sources(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        "SELECT SourceKey FROM dbo.MovieReleaseSources WHERE Enabled=1",
                        &[],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .filter_map(|r| r.get::<&str, _>(0).map(str::to_owned))
                    .collect())
            },
            &self.poisoned,
        ))
    }

    pub fn disc_disabled_movie_sources(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        "SELECT SourceKey FROM dbo.MovieReleaseSources WHERE CollectDisc=0",
                        &[],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .filter_map(|r| r.get::<&str, _>(0).map(str::to_owned))
                    .collect())
            },
            &self.poisoned,
        ))
    }

    pub fn tracked_movie_urls(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(bounded(async {
            let rows = self.client.query("SELECT DISTINCT l.Url FROM dbo.MovieSourceLinks l JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.Enabled=1 AND EXISTS(SELECT 1 FROM dbo.MovieReleases r WHERE r.MovieSourceLinkId=l.ID AND r.ReleaseStatus='scheduled' AND (s.CollectDisc=1 OR r.ReleaseType<>'disc'))", &[]).await?.into_first_result().await?;
            Ok(rows.iter().filter_map(|r| r.get::<&str,_>(0).map(str::to_owned)).collect())
        },&self.poisoned))
    }

    /// Upsert stable source/release identities; never delete absent calendar entries.
    pub fn save_movie_releases(
        &mut self,
        rows: &[crate::movies::MovieRelease],
        report: &serde_json::Value,
    ) -> Result<usize> {
        self.rt.block_on(bounded(async {
            let mut written = 0;
            for (key, name, url) in [
                ("dvdsreleasedates", "DVDs Release Dates", "https://www.dvdsreleasedates.com/"),
                ("whentostream", "When To Stream", "https://whentostream.com/"),
            ] {
                self.client.execute(
                    "SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
                     IF NOT EXISTS (SELECT 1 FROM dbo.MovieReleaseSources WITH (UPDLOCK, HOLDLOCK) WHERE SourceKey=@P1)
                     INSERT dbo.MovieReleaseSources(SourceKey,Name,Url,Enabled) VALUES(@P1,@P2,@P3,1);
                     COMMIT TRANSACTION; END TRY BEGIN CATCH
                     IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION; THROW; END CATCH",
                    &[&key, &name, &url]).await?;
            }
            for r in rows {
                let result = self.client.query(include_str!("../sql/upsert_movie_release.sql"),
                    &[&r.source, &r.external_id, &r.title, &r.imdb_id, &r.key, &r.kind,
                      &r.date, &r.url, &r.format, &r.platform]).await?.into_first_result().await?;
                written += result.first().and_then(|r| r.get::<i32, _>("Written")).unwrap_or(0) as usize;
            }
            for (source, host) in [("dvdsreleasedates", "dvdsreleasedates.com"), ("whentostream", "whentostream.com"), ("movieinsider", "movieinsider.com")] {
                let errors: Vec<_> = report["issues"].as_array().into_iter().flatten()
                    .filter(|r| r["source"]==source || r["url"].as_str().is_some_and(|u| url::Url::parse(u).ok().and_then(|u|u.host_str().map(str::to_owned)).is_some_and(|h|h==host || h==format!("www.{host}"))))
                    .map(|r| r["reason"].as_str().unwrap_or("unknown issue")).collect();
                let error = if errors.is_empty() { None } else { Some(errors.join("; ").chars().take(1000).collect::<String>()) };
                self.client.execute("UPDATE dbo.MovieReleaseSources SET LastScrapedAt=SYSUTCDATETIME(), LastError=@P2 WHERE SourceKey=@P1 AND Enabled=1", &[&source, &error]).await?;
            }
            Ok(written)
        },&self.poisoned))
    }

    /// URLs of all enabled sources, oldest first.
    pub fn check_health(&mut self) -> Result<()> {
        self.rt.block_on(bounded(async {
            let r=self.client.query("SELECT TOP(1) Status,DATEDIFF(minute,HeartbeatAt,SYSUTCDATETIME()) AS Age FROM dbo.ScraperRuns ORDER BY ID DESC",&[]).await?.into_first_result().await?;
            let r=r.first().ok_or("collector has not completed its first run")?;
            if r.get::<i32,_>("Age").is_none_or(|age|age>90)||r.get::<&str,_>("Status")==Some("failed"){return Err("collector progress unhealthy or latest run failed".into())}
            Ok(())
        },&self.poisoned))
    }

    pub fn start_run(&mut self) -> Result<i32> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        "INSERT dbo.ScraperRuns(Status) OUTPUT inserted.ID VALUES('running')",
                        &[],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows[0].get::<i32, _>(0).unwrap())
            },
            &self.poisoned,
        ))
    }
    pub fn run_heartbeat(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        "UPDATE dbo.ScraperRuns SET HeartbeatAt=SYSUTCDATETIME() WHERE ID=@P1",
                        &[&id],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn apply_retention(&mut self) -> Result<()> {
        let today = Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive();
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(include_str!("../sql/retention.sql"), &[&today])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    pub fn finish_run(&mut self, id: i32, status: &str, error: Option<&str>) -> Result<()> {
        let error = error.map(|s| utf16_prefix(s, 1000));
        self.rt.block_on(bounded(async {self.client.execute("UPDATE dbo.ScraperRuns SET HeartbeatAt=SYSUTCDATETIME(),FinishedAt=SYSUTCDATETIME(),Status=@P2,ErrorSummary=@P3 WHERE ID=@P1",&[&id,&status,&error]).await?;
        self.client.execute("DELETE dbo.ScraperRuns WHERE FinishedAt IS NOT NULL AND StartedAt<DATEADD(day,-7,SYSUTCDATETIME())",&[]).await?;Ok(())},&self.poisoned))
    }

    pub fn enabled_sources(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        "SELECT Url FROM dbo.Sources WHERE Enabled = 1 ORDER BY ID",
                        &[],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .filter_map(|r| r.get::<&str, _>(0).map(str::to_string))
                    .collect())
            },
            &self.poisoned,
        ))
    }

    /// Inserts any URLs not already present. Returns how many were new.
    pub fn add_sources(&mut self, urls: &[String]) -> Result<usize> {
        self.rt.block_on(bounded(async {
            let mut added = 0;
            for url in urls {
                let res = self
                    .client
                    .execute(
                        "IF NOT EXISTS (SELECT 1 FROM dbo.Sources WHERE UrlHash = HASHBYTES('SHA2_256', @P1))
                         INSERT INTO dbo.Sources (Url) VALUES (@P1)",
                        &[url],
                    )
                    .await?;
                added += res.total() as usize;
            }
            Ok(added)
        },&self.poisoned))
    }

    /// Records the outcome of scraping a source (`None` error means success).
    pub fn record_result(&mut self, url: &str, error: Option<&str>) -> Result<()> {
        let error = error.map(|e| e.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        "UPDATE dbo.Sources SET LastScrapedAt = SYSUTCDATETIME(), LastError = @P2
                     WHERE UrlHash = HASHBYTES('SHA2_256', @P1)",
                        &[&url, &error],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    /// Upserts each page keyed on URL, linking it to its source (which is created
    /// in `Sources` if the requested URL isn't there yet). Returns the number of pages written.
    pub fn save_pages(&mut self, pages: &[PageMeta]) -> Result<usize> {
        self.save_page_refs(&pages.iter().collect::<Vec<_>>())
    }
    pub fn save_page_refs(&mut self, pages: &[&PageMeta]) -> Result<usize> {
        let mut count = 0;
        for page in pages {
            let (outcome, _, _) = self.save_article(page, false)?;
            if !outcome.starts_with("skipped-") {
                count += 1;
            }
        }
        Ok(count)
    }
    pub fn save_article(
        &mut self,
        p: &PageMeta,
        dry_run: bool,
    ) -> Result<(String, Option<i32>, String)> {
        for attempt in 0..3 {
            match self.save_article_once(p, dry_run) {
                Err(error)
                    if attempt < 2
                        && error
                            .downcast_ref::<tiberius::error::Error>()
                            .and_then(|e| e.code())
                            .is_some_and(|code| matches!(code, 1205 | 1222 | 2601 | 2627)) =>
                {
                    eprintln!(
                        "article identity: retrying SQL collision/deadlock, attempt={}",
                        attempt + 1
                    );
                    std::thread::sleep(Duration::from_millis(100 * (attempt + 1)));
                }
                result => return result,
            }
        }
        unreachable!()
    }
    fn save_article_once(
        &mut self,
        p: &PageMeta,
        dry_run: bool,
    ) -> Result<(String, Option<i32>, String)> {
        let title = utf16_prefix(&p.title, 500);
        let mut urls = p.identity_urls.clone();
        let verified = !urls.is_empty();
        urls.push(p.url.clone());
        urls.sort();
        urls.dedup();
        let aliases = serde_json::to_string(&urls)?;
        let published = p.published.map(|d| d.naive_utc());
        self.rt.block_on(bounded(async {
            let rows=self.client.query("EXEC dbo.UpsertArticleIdentity @P1,@P2,@P3,@P4,@P5,@P6,@P7,@P8,@P9",
                &[&p.source,&p.url,&title,&p.description,&published,&p.image_url,&aliases,&verified,&dry_run]).await?.into_first_result().await?;
            let row=rows.first().ok_or("article identity result missing")?;
            let outcome=row.get::<&str,_>("Outcome").ok_or("article outcome missing")?.to_owned();
            let id=row.get::<i32,_>("PageId");
            let candidates=row.get::<&str,_>("Candidates").unwrap_or("[]").to_owned();
            if outcome!="updated" { eprintln!("article identity: outcome={outcome} page_id={id:?} candidates={candidates} source={} target={} aliases={aliases}",p.source,p.url); }
            Ok((outcome,id,candidates))
        },&self.poisoned))
    }
    pub fn article_checked(&mut self, source: &str, url: &str) -> Result<()> {
        self.rt.block_on(bounded(async {
            self.client.execute("UPDATE a SET LastCheckedAt=SYSUTCDATETIME() FROM dbo.ArticleUrlAliases a JOIN dbo.Sources s ON s.ID=a.SourceId WHERE s.UrlHash=HASHBYTES('SHA2_256',@P1) AND a.UrlHash=HASHBYTES('SHA2_256',@P2)",&[&source,&url]).await?;
            Ok(())
        },&self.poisoned))
    }
    pub fn recent_articles(&mut self, source: &str) -> Result<Vec<PageMeta>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("WITH recent AS (SELECT TOP(10) p.* FROM dbo.Pages p JOIN dbo.Sources s ON s.ID=p.SourceId WHERE s.UrlHash=HASHBYTES('SHA2_256',@P1) ORDER BY CASE WHEN p.Published IS NULL THEN 1 ELSE 0 END,p.Published DESC,p.ScrapedAt DESC,p.ID DESC) SELECT p.Url,p.Title,p.Description,p.Published,p.ImageUrl FROM recent p LEFT JOIN dbo.ArticleUrlAliases a ON a.SourceId=p.SourceId AND a.UrlHash=p.UrlHash ORDER BY a.LastCheckedAt,p.ID",&[&source]).await?.into_first_result().await?;
            Ok(rows.into_iter().map(|row|PageMeta{source:source.to_owned(),url:row.get::<&str,_>("Url").unwrap().to_owned(),title:row.get::<&str,_>("Title").unwrap().to_owned(),description:row.get::<&str,_>("Description").map(str::to_owned),published:row.get::<chrono::NaiveDateTime,_>("Published").map(|d|d.and_utc()),image_url:row.get::<&str,_>("ImageUrl").map(str::to_owned),identity_urls:vec![]}).collect())
        },&self.poisoned))
    }

    /// Deletes each source's oldest pages so at most `keep` remain per source.
    /// Returns the number of rows deleted.
    pub fn prune_per_source(&mut self, keep: u32) -> Result<u64> {
        let keep = checked_keep(keep)?;
        self.rt.block_on(bounded(
            async {
                let res = self
                    .client
                    .execute(
                        ";WITH ranked AS (
                         SELECT ROW_NUMBER() OVER (
                             PARTITION BY SourceId
                             ORDER BY CASE WHEN Published IS NULL THEN 1 ELSE 0 END,
                                      Published DESC, ScrapedAt DESC, ID DESC) AS rn
                         FROM dbo.Pages
                     )
                     DELETE FROM ranked WHERE rn > @P1",
                        &[&keep],
                    )
                    .await?;
                Ok(res.total())
            },
            &self.poisoned,
        ))
    }

    /// Deletes the oldest pages so at most `keep` remain. Dated pages are ranked newest first
    /// and come before undated ones (which are ranked by when they were scraped).
    /// Returns the number of rows deleted.
    pub fn prune_pages(&mut self, keep: u32) -> Result<u64> {
        let keep = checked_keep(keep)?;
        self.rt.block_on(bounded(
            async {
                let res = self
                    .client
                    .execute(
                        ";WITH ranked AS (
                         SELECT ROW_NUMBER() OVER (
                             ORDER BY CASE WHEN Published IS NULL THEN 1 ELSE 0 END,
                                      Published DESC, ScrapedAt DESC, ID DESC) AS rn
                         FROM dbo.Pages
                     )
                     DELETE FROM ranked WHERE rn > @P1",
                        &[&keep],
                    )
                    .await?;
                Ok(res.total())
            },
            &self.poisoned,
        ))
    }

    /// Upserts each event keyed on its URL, linked to its `SportsEventsType` (calendar feed) row. The feed is the source of truth for schedule facts
    /// (time, TV, location), so those are overwritten; `FirstSeenAt` is kept.
    /// Returns the number of events written.
    pub fn save_events(&mut self, type_id: i32, events: &[Event]) -> Result<usize> {
        self.rt.block_on(bounded(async {
            for e in events {
                let title: String = utf16_prefix(&e.title,500);
                self.client
                    .execute(
                        "MERGE dbo.SportsEvents AS t
                         USING (SELECT @P1 AS Url) AS s
                         ON t.UrlHash = HASHBYTES('SHA2_256', s.Url)
                         WHEN MATCHED THEN UPDATE SET
                             GameId = @P2, Title = @P3, Sport = @P4, Opponent = @P5, IsAway = @P6,
                             Location = @P7, EventDate = @P8, StartsAtUtc = @P9, EndsAtUtc = @P10,
                             TimeTbd = @P11, Tv = @P12, StreamUrl = @P13, LiveStatsUrl = @P14,
                             TeamLogoUrl = @P15, OpponentLogoUrl = @P16, SportsEventsTypeId = @P17,
                             LastSeenAt = SYSUTCDATETIME()
                         WHEN NOT MATCHED THEN INSERT
                             (Url, GameId, Title, Sport, Opponent, IsAway, Location, EventDate, StartsAtUtc,
                              EndsAtUtc, TimeTbd, Tv, StreamUrl, LiveStatsUrl, TeamLogoUrl, OpponentLogoUrl,
                              SportsEventsTypeId)
                             VALUES (@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9,
                                     @P10, @P11, @P12, @P13, @P14, @P15, @P16, @P17);",
                        &[
                            &e.url, &e.game_id, &title, &e.sport, &e.opponent, &e.is_away,
                            &e.location, &e.event_date, &e.starts_at, &e.ends_at, &e.time_tbd,
                            &e.tv, &e.stream_url, &e.live_stats_url, &e.team_logo_url,
                            &e.opponent_logo_url, &type_id,
                        ],
                    )
                    .await?;
            }
            Ok(events.len())
        },&self.poisoned))
    }

    pub fn configure_nwsl(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/nwsl_seed.sql"))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    pub fn configure_wnba(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/wnba_seed.sql"))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    pub fn configure_rankings(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/rankings_seed.sql"))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn ranking_sources(&mut self) -> Result<Vec<crate::rankings::Source>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("SELECT ID,Sport,LeagueId,PollId,Url FROM dbo.SportsRankingSources WHERE Enabled=1 AND ProviderKey='espn-rankings' ORDER BY ID",&[]).await?.into_first_result().await?;
            rows.iter().map(|r|Ok(crate::rankings::Source {id:r.get(0).ok_or("missing ranking source ID")?,sport:r.get::<&str,_>(1).ok_or("missing ranking sport")?.into(),league_id:r.get::<&str,_>(2).ok_or("missing ranking league")?.into(),poll_id:r.get::<&str,_>(3).ok_or("missing ranking poll")?.into(),url:r.get::<&str,_>(4).ok_or("missing ranking URL")?.into()})).collect()
        },&self.poisoned))
    }
    pub fn ranking_attempt(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(bounded(async {self.client.execute("UPDATE dbo.SportsRankingSources SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=@P1",&[&id]).await?;Ok(())},&self.poisoned))
    }
    pub fn ranking_error(&mut self, id: i32, error: &str) -> Result<()> {
        let error = utf16_prefix(error, 2000);
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        "UPDATE dbo.SportsRankingSources SET LastError=@P2 WHERE ID=@P1",
                        &[&id, &error],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn save_ranking(
        &mut self,
        source: &crate::rankings::Source,
        poll: &crate::rankings::Poll,
    ) -> Result<()> {
        poll.validate()?;
        if source.poll_id != poll.poll_id {
            return Err("ranking poll differs from configured source".into());
        }
        let published = poll.published.naive_utc();
        let updated = poll.updated.map(|d| d.naive_utc());
        let rows = poll.rows_json();
        self.rt.block_on(bounded(async {
            self.client.execute(r#"
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SportsRankingSources WITH(UPDLOCK,HOLDLOCK) WHERE ID=@P1)
  THROW 50000,'Ranking source missing',1;
 IF EXISTS(SELECT 1 FROM dbo.SportsRankingPolls WHERE SourceId=@P1 AND IsCurrent=1 AND
  (PublishedAtUtc>@P6 OR (PublishedAtUtc=@P6 AND ProviderUpdatedAtUtc IS NOT NULL AND (@P7 IS NULL OR ProviderUpdatedAtUtc>@P7))))
  THROW 50000,'Older poll response rejected; last good rankings retained',1;
 DECLARE @Now DATETIME2=SYSUTCDATETIME(),@Poll BIGINT;
 UPDATE dbo.SportsRankingPolls SET IsCurrent=0 WHERE SourceId=@P1 AND IsCurrent=1;
 SELECT @Poll=ID FROM dbo.SportsRankingPolls WHERE SourceId=@P1 AND PollId=@P2 AND Season=@P4 AND PublishedAtUtc=@P6;
 IF @Poll IS NULL
 BEGIN
  INSERT dbo.SportsRankingPolls(SourceId,PollId,PollName,Season,SeasonLabel,PublishedAtUtc,ProviderUpdatedAtUtc,Period,SeasonPhase,SourceUrl,IsCurrent,FetchedAt)
  VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7,@P8,@P9,@P10,1,@Now);
  SET @Poll=SCOPE_IDENTITY();
 END
 ELSE UPDATE dbo.SportsRankingPolls SET PollName=@P3,SeasonLabel=@P5,ProviderUpdatedAtUtc=@P7,Period=@P8,SeasonPhase=@P9,SourceUrl=@P10,IsCurrent=1,FetchedAt=@Now WHERE ID=@Poll;
 UPDATE dbo.SportsRankingTeams SET IsCurrent=0 WHERE PollRowId=@Poll AND IsCurrent=1;
 MERGE dbo.SportsRankingTeams WITH(HOLDLOCK) AS t
 USING OPENJSON(@P11) WITH (
  TeamId NVARCHAR(80) '$.teamId',TeamName NVARCHAR(200) '$.teamName',Rank INT '$.rank',PreviousRank INT '$.previousRank',PreviouslyUnranked BIT '$.previouslyUnranked',Points FLOAT '$.points',FirstPlaceVotes INT '$.firstPlaceVotes',Record NVARCHAR(100) '$.record',LogoUrl NVARCHAR(2048) '$.logoUrl',TeamUrl NVARCHAR(2048) '$.teamUrl',Big12SchoolId INT '$.big12SchoolId'
 ) AS s ON t.PollRowId=@Poll AND t.TeamId=s.TeamId
 WHEN MATCHED THEN UPDATE SET TeamName=s.TeamName,Rank=s.Rank,PreviousRank=s.PreviousRank,PreviouslyUnranked=s.PreviouslyUnranked,Points=s.Points,FirstPlaceVotes=s.FirstPlaceVotes,Record=s.Record,LogoUrl=s.LogoUrl,TeamUrl=s.TeamUrl,Big12SchoolId=s.Big12SchoolId,IsCurrent=1
 WHEN NOT MATCHED THEN INSERT(PollRowId,TeamId,TeamName,Rank,PreviousRank,PreviouslyUnranked,Points,FirstPlaceVotes,Record,LogoUrl,TeamUrl,Big12SchoolId,IsCurrent)
 VALUES(@Poll,s.TeamId,s.TeamName,s.Rank,s.PreviousRank,s.PreviouslyUnranked,s.Points,s.FirstPlaceVotes,s.Record,s.LogoUrl,s.TeamUrl,s.Big12SchoolId,1);
 DELETE t FROM dbo.SportsRankingTeams t JOIN dbo.SportsRankingPolls p ON p.ID=t.PollRowId
 WHERE p.SourceId=@P1 AND (p.IsCurrent=0 OR t.IsCurrent=0);
 DELETE dbo.SportsRankingPolls WHERE SourceId=@P1 AND IsCurrent=0;
 UPDATE dbo.SportsRankingSources SET LastSuccessAt=@Now,LastError=NULL WHERE ID=@P1;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH
"#,&[&source.id,&poll.poll_id,&poll.name,&poll.season,&poll.season_label,&published,&updated,&poll.period,&poll.phase,&poll.source_url,&rows]).await?;Ok(())
        },&self.poisoned))
    }

    pub fn configure_movieinsider(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/movieinsider_seed.sql"))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    pub fn configure_standings(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/standings_seed.sql"))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn standings_sources(&mut self) -> Result<Vec<crate::standings::Source>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("SELECT ID,ProviderKey,Sport,Url,MinimumTeams FROM dbo.SportsStandingsSources WHERE Enabled=1 ORDER BY ID",&[]).await?.into_first_result().await?;
            rows.iter().map(|r| Ok(crate::standings::Source {
                id:r.get(0).ok_or("missing standings source ID")?,
                provider:r.get::<&str,_>(1).ok_or("missing standings provider")?.to_owned(),
                sport:r.get::<&str,_>(2).ok_or("missing standings sport")?.to_owned(),
                url:r.get::<&str,_>(3).ok_or("missing standings URL")?.to_owned(),
                minimum_teams:usize::try_from(r.get::<i32,_>(4).ok_or("missing standings minimum")?)?,
            })).collect()
        }, &self.poisoned))
    }
    pub fn standings_attempt(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(bounded(async {
            self.client.execute("UPDATE dbo.SportsStandingsSources SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=@P1",&[&id]).await?;
            Ok(())
        }, &self.poisoned))
    }
    pub fn standings_error(&mut self, id: i32, error: &str) -> Result<()> {
        let error = utf16_prefix(error, 2000);
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        "UPDATE dbo.SportsStandingsSources SET LastError=@P2 WHERE ID=@P1",
                        &[&id, &error],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn save_standings(
        &mut self,
        source: &crate::standings::Source,
        snapshot: &crate::standings::Snapshot,
    ) -> Result<()> {
        snapshot.validate(source.minimum_teams)?;
        let rows = snapshot.rows_json();
        let count = i32::try_from(snapshot.entries.len())?;
        self.rt.block_on(bounded(async {
            self.client.execute(r#"
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 -- Serialize refreshes of this source, including first import.
 IF NOT EXISTS(SELECT 1 FROM dbo.SportsStandingsSources WITH(UPDLOCK,HOLDLOCK) WHERE ID=@P1)
  THROW 50000,'Unknown standings source',1;
 DECLARE @Now DATETIME2=SYSUTCDATETIME();
 UPDATE dbo.SportsStandings SET IsCurrent=0 WHERE SourceId=@P1 AND IsCurrent=1;
 MERGE dbo.SportsStandings WITH(HOLDLOCK) AS t
 USING OPENJSON(@P6) WITH (
  GroupId NVARCHAR(80) '$.groupId',GroupName NVARCHAR(200) '$.groupName',
  TeamId NVARCHAR(80) '$.teamId',TeamName NVARCHAR(200) '$.teamName',
  LogoUrl NVARCHAR(2048) '$.logoUrl',TeamUrl NVARCHAR(2048) '$.teamUrl',
  Position INT '$.position',Rank INT '$.rank',StatsJson NVARCHAR(MAX) '$.stats' AS JSON,Note NVARCHAR(1000) '$.note'
 ) AS s
 ON t.SourceId=@P1 AND t.Season=@P2 AND t.SeasonType=@P3 AND t.GroupId=s.GroupId AND t.TeamId=s.TeamId
 WHEN MATCHED THEN UPDATE SET Title=@P4,SourceUrl=@P5,GroupName=s.GroupName,
  TeamName=s.TeamName,LogoUrl=s.LogoUrl,TeamUrl=s.TeamUrl,Position=s.Position,Rank=s.Rank,
  StatsJson=s.StatsJson,Note=s.Note,IsCurrent=1,FetchedAt=@Now
 WHEN NOT MATCHED THEN INSERT(SourceId,Season,SeasonType,Title,SourceUrl,GroupId,GroupName,TeamId,TeamName,LogoUrl,TeamUrl,Position,Rank,StatsJson,Note,IsCurrent,FetchedAt)
 VALUES(@P1,@P2,@P3,@P4,@P5,s.GroupId,s.GroupName,s.TeamId,s.TeamName,s.LogoUrl,s.TeamUrl,s.Position,s.Rank,s.StatsJson,s.Note,1,@Now);
 IF EXISTS(SELECT 1 FROM dbo.SportsStandingsSources WHERE ID=@P1 AND ProviderKey='espn-epl')
  DELETE dbo.SportsStandings WHERE SourceId=@P1 AND IsCurrent=0;
 UPDATE dbo.SportsStandingsSources SET LastSuccessAt=@Now,LastError=NULL,LatestSeason=@P2,LastTeamCount=@P7 WHERE ID=@P1;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH
"#, &[&source.id,&snapshot.season,&snapshot.season_type,&snapshot.title,&snapshot.source_url,&rows,&count]).await?;
            Ok(())
        }, &self.poisoned))
    }

    pub fn cinema_poster_jobs(&mut self) -> Result<Vec<crate::cinema_posters::Job>> {
        self.rt.block_on(bounded(async {
            let exists=self.client.query("SELECT CASE WHEN OBJECT_ID('dbo.CinemaPosterCache','U') IS NULL THEN 0 ELSE 1 END",&[]).await?.into_first_result().await?;
            if exists[0].get::<i32,_>(0)==Some(0){return Ok(Vec::new());}
            let rows=self.client.query("SELECT 'results' AS Section,MovieTitle,CAST(NULL AS SMALLINT) AS OriginalYear,MovieUrl FROM dbo.BoxOfficeWeekend UNION SELECT 'forecasts',MovieTitle,CAST(YEAR(TargetDate) AS SMALLINT),NULL FROM dbo.BoxOfficeForecasts",&[]).await?.into_first_result().await?;
            Ok(rows.into_iter().map(|r| {
                let url=r.get::<&str,_>(3).map(str::to_string);
                let year=r.get::<i16,_>(2).or_else(||url.as_deref().and_then(crate::cinema_posters::url_year));
                crate::cinema_posters::Job{section:r.get::<&str,_>(0).unwrap().to_string(),title:r.get::<&str,_>(1).unwrap().to_string(),year,url}
            }).collect())
        }, &self.poisoned))
    }
    pub fn cinema_release_posters(
        &mut self,
    ) -> Result<Vec<(crate::movie_metadata::MovieIdentity, String)>> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("SELECT Title,OriginalYear,PosterUrl FROM dbo.Movies WHERE PosterUrl IS NOT NULL AND PosterLookupStatus IN ('matched','valid')",&[]).await?.into_first_result().await?;
            Ok(rows.into_iter().map(|r| {
                let (title,suffix_year)=crate::movie_metadata::title_year(r.get::<&str,_>(0).unwrap());
                (crate::movie_metadata::MovieIdentity{title,year:r.get::<i16,_>(1).or(suffix_year),..Default::default()},r.get::<&str,_>(2).unwrap().to_string())
            }).collect())
        }, &self.poisoned))
    }
    pub fn cinema_poster_caches(
        &mut self,
    ) -> Result<std::collections::BTreeMap<String, crate::cinema_posters::Cache>> {
        self.rt.block_on(bounded(async {
            let exists=self.client.query("SELECT CASE WHEN OBJECT_ID('dbo.CinemaPosterCache','U') IS NULL THEN 0 ELSE 1 END",&[]).await?.into_first_result().await?;
            if exists[0].get::<i32,_>(0)==Some(0){return Ok(std::collections::BTreeMap::new());}
            let rows=self.client.query("SELECT IdentityKey,PosterUrl,LookupStatus,RetryAfterUtc FROM dbo.CinemaPosterCache",&[]).await?.into_first_result().await?;
            Ok(rows.into_iter().map(|r|(r.get::<&str,_>(0).unwrap().to_string(),crate::cinema_posters::Cache{poster:r.get::<&str,_>(1).map(str::to_string),status:r.get::<&str,_>(2).unwrap().to_string(),retry:r.get::<chrono::NaiveDateTime,_>(3).unwrap()})).collect())
        }, &self.poisoned))
    }
    #[allow(clippy::too_many_arguments)] // Parameter order mirrors the bound SQL columns.
    pub fn save_cinema_poster(
        &mut self,
        key: &str,
        identity: &crate::movie_metadata::MovieIdentity,
        poster: Option<&str>,
        source: Option<&str>,
        status: &str,
        error: Option<&str>,
        retry: chrono::NaiveDateTime,
    ) -> Result<()> {
        let error = error.map(|s| s.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(async {
            self.client.execute("MERGE dbo.CinemaPosterCache WITH(HOLDLOCK) t USING(SELECT @P1 AS IdentityKey) s ON t.IdentityKey=s.IdentityKey WHEN MATCHED THEN UPDATE SET PosterUrl=@P4,SourceUrl=COALESCE(@P5,t.SourceUrl),LookupStatus=@P6,LastError=@P7,CheckedAtUtc=SYSUTCDATETIME(),RetryAfterUtc=@P8,LastUsedAtUtc=SYSUTCDATETIME() WHEN NOT MATCHED THEN INSERT(IdentityKey,MovieTitle,OriginalYear,PosterUrl,SourceUrl,LookupStatus,LastError,CheckedAtUtc,RetryAfterUtc,LastUsedAtUtc) VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7,SYSUTCDATETIME(),@P8,SYSUTCDATETIME());",&[&key,&identity.title,&identity.year,&poster,&source,&status,&error,&retry]).await?;
            Ok(())
        }, &self.poisoned))
    }
    pub fn attach_cinema_poster(
        &mut self,
        job: &crate::cinema_posters::Job,
        key: &str,
        poster: Option<&str>,
        status: &str,
    ) -> Result<()> {
        self.rt.block_on(bounded(async {
            if job.section=="results" {
                self.client.execute("UPDATE dbo.BoxOfficeWeekend SET PosterUrl=@P1,PosterLookupStatus=@P2 WHERE MovieTitle=@P3 AND MovieUrl=@P4",&[&poster,&status,&job.title,&job.url]).await?;
            } else {
                self.client.execute("UPDATE dbo.BoxOfficeForecasts SET PosterUrl=@P1,PosterLookupStatus=@P2 WHERE MovieTitle=@P3 AND YEAR(TargetDate)=@P4",&[&poster,&status,&job.title,&job.year]).await?;
            }
            self.client.execute("UPDATE dbo.CinemaPosterCache SET LastUsedAtUtc=SYSUTCDATETIME() WHERE IdentityKey=@P1",&[&key]).await?;
            Ok(())
        }, &self.poisoned))
    }
    pub fn configure_boxoffice(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(concat!(
                        include_str!("../sql/boxoffice.sql"),
                        "\n",
                        include_str!("../sql/boxoffice_status.sql"),
                        "\n",
                        include_str!("../sql/boxoffice_forecasts.sql"),
                        "\n",
                        include_str!("../sql/cinema_posters.sql")
                    ))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn save_boxoffice(&mut self, chart: &crate::boxoffice::Chart) -> Result<()> {
        if chart.rows.len() != 10 {
            return Err("Expected ten box office rows".into());
        }
        let date = chart.weekend.to_string();
        let rows = serde_json::to_string(&chart.rows)?;
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        include_str!("../sql/boxoffice_replace.sql"),
                        &[&date, &rows, &crate::boxoffice::URL],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn boxoffice_forecast_status(&mut self, error: Option<&str>) -> Result<()> {
        let message = error.map(|s| s.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(async {
            if error.is_some() {
                self.client.execute("UPDATE dbo.BoxOfficeForecastSources SET LastError=@P1", &[&message]).await?;
            } else {
                self.client.execute("UPDATE dbo.BoxOfficeForecastSources SET LastAttemptAt=SYSUTCDATETIME()", &[]).await?;
            }
            Ok(())
        }, &self.poisoned))
    }
    pub fn save_boxoffice_forecasts(
        &mut self,
        snapshots: &[crate::boxoffice_forecasts::Snapshot],
    ) -> Result<()> {
        if snapshots.len() != 2
            || !snapshots.iter().any(|s| s.kind == "long-range")
            || !snapshots.iter().any(|s| s.kind == "weekend-preview")
        {
            return Err("Expected both domestic forecast snapshots".into());
        }
        let rows = serde_json::to_string(
            &snapshots
                .iter()
                .map(|s| serde_json::json!({"kind":s.kind,"published":s.published,"rows":s.rows}))
                .collect::<Vec<_>>(),
        )?;
        let today = chrono::Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive()
            .to_string();
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        include_str!("../sql/boxoffice_forecasts_replace.sql"),
                        &[&rows, &today],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn configure_epl(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                for sql in [
                    include_str!("../sql/epl_seed.sql"),
                    include_str!("../sql/standings_seed.sql"),
                ] {
                    self.client.simple_query(sql).await?.into_results().await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn configure_championship(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                for sql in [
                    include_str!("../sql/championship.sql"),
                    include_str!("../sql/championship_seed.sql"),
                    include_str!("../sql/soccer_shootouts.sql"),
                    include_str!("../sql/standings_seed.sql"),
                ] {
                    self.client.simple_query(sql).await?.into_results().await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn save_championship_results(
        &mut self,
        source: i32,
        batch: &crate::championship::Batch,
    ) -> Result<()> {
        self.save_match_results(
            source,
            crate::espn_schedule::League::Championship,
            batch.season,
            &batch.results,
        )
    }
    /// Explicit migration only; runtime does no DDL.
    pub fn configure_f1(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query("BEGIN TRANSACTION")
                    .await?
                    .into_results()
                    .await?;
                let result = async {
                    for sql in [
                        include_str!("../sql/f1.sql"),
                        include_str!("../sql/f1_schedule_view.sql"),
                        include_str!("../sql/f1_results_view.sql"),
                        include_str!("../sql/f1_drivers_view.sql"),
                        include_str!("../sql/f1_constructors_view.sql"),
                    ] {
                        self.client.simple_query(sql).await?.into_results().await?;
                    }
                    Ok::<_, crate::Error>(())
                }
                .await;
                if let Err(e) = result {
                    self.client
                        .simple_query("IF @@TRANCOUNT>0 ROLLBACK")
                        .await?
                        .into_results()
                        .await?;
                    return Err(e);
                }
                self.client
                    .simple_query("COMMIT")
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn f1_enabled(&mut self) -> Result<bool> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("IF OBJECT_ID('dbo.F1Source','U') IS NOT NULL SELECT Enabled FROM dbo.F1Source WHERE ID=1", &[]).await?.into_first_result().await?;
            Ok(rows.first().and_then(|r|r.get::<bool,_>(0)).unwrap_or(false))
        }, &self.poisoned))
    }
    pub fn f1_status(&mut self, error: Option<&str>) -> Result<()> {
        let message = error.map(|s| utf16_prefix(s, 1000));
        self.rt.block_on(bounded(async {
            if let Some(message)=message {
                self.client.execute("UPDATE dbo.F1Source SET LastError=@P1 WHERE ID=1", &[&message]).await?;
            } else {
                self.client.execute("UPDATE dbo.F1Source SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=1 AND Enabled=1", &[]).await?;
            }
            Ok(())
        }, &self.poisoned))
    }
    pub fn save_f1(&mut self, snapshot: &crate::f1::Snapshot) -> Result<()> {
        crate::f1::validate(snapshot)?;
        let payload = snapshot.payload();
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        include_str!("../sql/f1_replace.sql"),
                        &[&snapshot.season, &snapshot.standings_round, &payload],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn configure_boxscores(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                for sql in [
                    include_str!("../sql/sports_boxscores.sql"),
                    include_str!("../sql/sports_boxscores_view.sql"),
                    include_str!("../sql/sports_boxscores_permissions.sql"),
                ] {
                    self.client.simple_query(sql).await?.into_results().await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn boxscore_candidates(
        &mut self,
        limit: i32,
    ) -> Result<Vec<crate::sports_boxscores::Game>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        include_str!("../sql/sports_boxscores_candidates.sql"),
                        &[&limit],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                rows.iter()
                    .map(|r| {
                        Ok(crate::sports_boxscores::Game {
                            sport: r.get::<&str, _>(0).ok_or("missing sport")?.to_string(),
                            league: r.get::<&str, _>(1).ok_or("missing league")?.to_string(),
                            event_id: r.get::<&str, _>(2).ok_or("missing event ID")?.to_string(),
                            home_id: r.get::<&str, _>(3).ok_or("missing home ID")?.to_string(),
                            away_id: r.get::<&str, _>(4).ok_or("missing away ID")?.to_string(),
                            date: r.get::<&str, _>(5).ok_or("missing game date")?.to_string(),
                        })
                    })
                    .collect()
            },
            &self.poisoned,
        ))
    }
    pub fn period_candidates(
        &mut self,
        limit: i32,
    ) -> Result<Vec<(crate::sports_boxscores::Game, String)>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        include_str!("../sql/sports_periods_candidates.sql"),
                        &[&limit],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                rows.iter()
                    .map(|r| {
                        Ok((
                            crate::sports_boxscores::Game {
                                sport: r.get::<&str, _>(0).ok_or("missing sport")?.into(),
                                league: r.get::<&str, _>(1).ok_or("missing league")?.into(),
                                event_id: r.get::<&str, _>(2).ok_or("missing event")?.into(),
                                home_id: r.get::<&str, _>(3).ok_or("missing home")?.into(),
                                away_id: r.get::<&str, _>(4).ok_or("missing away")?.into(),
                                date: r.get::<&str, _>(5).ok_or("missing date")?.into(),
                            },
                            r.get::<&str, _>(6).ok_or("missing cached payload")?.into(),
                        ))
                    })
                    .collect()
            },
            &self.poisoned,
        ))
    }
    pub fn update_periods(
        &mut self,
        g: &crate::sports_boxscores::Game,
        old: &str,
        new: &str,
    ) -> Result<bool> {
        self.rt.block_on(bounded(
            async {
                let result = self
                    .client
                    .execute(
                        include_str!("../sql/sports_periods_update.sql"),
                        &[&g.sport, &g.league, &g.event_id, &old, &new],
                    )
                    .await?;
                Ok(result.total() == 1)
            },
            &self.poisoned,
        ))
    }
    pub fn boxscore_available(&mut self, g: &crate::sports_boxscores::Game) -> Result<bool> {
        self.rt.block_on(bounded(async {
            let rows=self.client.query("SELECT CAST(CASE WHEN EXISTS(SELECT 1 FROM dbo.SportsBoxScoreCache WHERE Sport=@P1 AND League=@P2 AND EventId=@P3 AND Payload IS NOT NULL) THEN 1 ELSE 0 END AS BIT)", &[&g.sport,&g.league,&g.event_id]).await?.into_first_result().await?;
            Ok(rows.first().and_then(|r|r.get::<bool,_>(0)).unwrap_or(false))
        },&self.poisoned))
    }
    pub fn save_boxscore(
        &mut self,
        game: &crate::sports_boxscores::Game,
        detail: Option<&crate::sports_boxscores::Detail>,
        error: Option<&str>,
    ) -> Result<()> {
        let payload = detail.and_then(|d| d.payload.as_deref());
        let summary = detail.and_then(|d| d.summary_url.as_deref());
        let box_url = detail.and_then(|d| d.boxscore_url.as_deref());
        let error = error.map(|e| utf16_prefix(e, 1000));
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        include_str!("../sql/sports_boxscores_upsert.sql"),
                        &[
                            &game.sport,
                            &game.league,
                            &game.event_id,
                            &payload,
                            &summary,
                            &box_url,
                            &error,
                        ],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn configure_nhl(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                for sql in [
                    include_str!("../sql/championship.sql"),
                    include_str!("../sql/soccer_shootouts.sql"),
                    include_str!("../sql/standings.sql"),
                    include_str!("../sql/nhl_seed.sql"),
                ] {
                    self.client.simple_query(sql).await?.into_results().await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn configure_mls(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                for sql in [
                    include_str!("../sql/championship.sql"),
                    include_str!("../sql/soccer_shootouts.sql"),
                    include_str!("../sql/mls_seed.sql"),
                ] {
                    self.client.simple_query(sql).await?.into_results().await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
    }
    pub fn save_match_results(
        &mut self,
        source: i32,
        league: crate::espn_schedule::League,
        season: i32,
        results: &[serde_json::Value],
    ) -> Result<()> {
        if !matches!(
            league,
            crate::espn_schedule::League::Mls
                | crate::espn_schedule::League::Championship
                | crate::espn_schedule::League::Nhl
        ) {
            return Err("unsupported match results league".into());
        }
        let provider = league.provider_key();
        let rows = serde_json::to_string(results)?;

        self.rt.block_on(bounded(async {
            self.client.execute(r#"
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ID=@P1 AND Enabled=1 AND ProviderKey=@P4)
  THROW 50000,'Match results source disabled or unavailable',1;
 MERGE dbo.SportsMatchResults WITH(HOLDLOCK) AS t
 USING OPENJSON(@P2) WITH (
 GameId INT '$.gameId',Season INT '$.season',MatchDate DATE '$.matchDate',StartsAt DATETIMEOFFSET '$.startsAt',
 HomeId INT '$.homeId',HomeName NVARCHAR(200) '$.homeName',AwayId INT '$.awayId',AwayName NVARCHAR(200) '$.awayName',
 HomeScore INT '$.homeScore',AwayScore INT '$.awayScore',HomeShootout INT '$.homeShootout',AwayShootout INT '$.awayShootout',HomeLogo NVARCHAR(2048) '$.homeLogo',AwayLogo NVARCHAR(2048) '$.awayLogo',
 Status NVARCHAR(100) '$.status',Url NVARCHAR(2048) '$.url') AS s
 ON t.ProviderKey=@P4 AND t.GameId=s.GameId
 WHEN MATCHED THEN UPDATE SET SourceId=@P1,Season=s.Season,MatchDate=s.MatchDate,
 StartsAtUtc=CONVERT(datetime2,SWITCHOFFSET(s.StartsAt,'+00:00')),HomeTeamId=s.HomeId,HomeTeamName=s.HomeName,
 AwayTeamId=s.AwayId,AwayTeamName=s.AwayName,HomeScore=s.HomeScore,AwayScore=s.AwayScore,HomeShootoutScore=s.HomeShootout,AwayShootoutScore=s.AwayShootout,
 HomeLogoUrl=s.HomeLogo,AwayLogoUrl=s.AwayLogo,ScheduleStatus=s.Status,SourceUrl=s.Url,LastSeenAt=SYSUTCDATETIME()
 WHEN NOT MATCHED THEN INSERT(SourceId,ProviderKey,GameId,Season,MatchDate,StartsAtUtc,HomeTeamId,HomeTeamName,AwayTeamId,AwayTeamName,HomeScore,AwayScore,HomeShootoutScore,AwayShootoutScore,HomeLogoUrl,AwayLogoUrl,ScheduleStatus,SourceUrl)
 VALUES(@P1,@P4,s.GameId,s.Season,s.MatchDate,CONVERT(datetime2,SWITCHOFFSET(s.StartsAt,'+00:00')),s.HomeId,s.HomeName,s.AwayId,s.AwayName,s.HomeScore,s.AwayScore,s.HomeShootout,s.AwayShootout,s.HomeLogo,s.AwayLogo,s.Status,s.Url);
 IF EXISTS(SELECT 1 FROM OPENJSON(@P2))
  DELETE dbo.SportsMatchResults WHERE SourceId=@P1 AND Season<@P3;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
"#, &[&source,&rows,&season,&provider]).await?;
            Ok(())
        }, &self.poisoned))
    }

    pub fn configure_mlb(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/mlb_seed.sql"))
                    .await?
                    .into_results()
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    pub fn save_espn_schedule(
        &mut self,
        type_id: i32,
        fixtures: &[crate::espn_schedule::Fixture],
    ) -> Result<usize> {
        let _timer = crate::metrics::Stage::new("espn_schedule_sql");
        self.rt.block_on(bounded(async {
            for fixture in fixtures {
                let e=&fixture.event;
                for (value,limit) in [(e.opponent.as_deref(),200),(e.location.as_deref(),300),(Some(fixture.team_name.as_str()),100),(Some(fixture.status.as_str()),100),(e.tv.as_deref(),200),(Some(e.url.as_str()),2048),(e.stream_url.as_deref(),2048),(e.live_stats_url.as_deref(),2048),(e.team_logo_url.as_deref(),2048),(e.opponent_logo_url.as_deref(),2048)] {
                    if value.is_some_and(|s|s.encode_utf16().count()>limit){return Err("ESPN field exceeds SQL schema limit; snapshot not saved".into())}
                }
            }
            self.client.simple_query("BEGIN TRANSACTION").await?.into_results().await?;
            let result:Result<usize>=(async {
            let mut saved=0;
            let today=Utc::now().with_timezone(&chrono_tz::America::Chicago).date_naive();
            let mut rows=std::collections::BTreeMap::new();
            for fixture in fixtures {
                let e=&fixture.event;
                let key=fixture.league.canonical_key(e.game_id.ok_or("schedule game ID missing")?);
                rows.insert(key.clone(),serde_json::json!({
                    "Url":e.url,"GameId":e.game_id,"Title":utf16_prefix(&e.title,500),"Sport":e.sport,
                    "Opponent":e.opponent,"IsAway":e.is_away,"Location":e.location,"EventDate":e.event_date.to_string(),
                    "StartsAtUtc":e.starts_at.map(|d|d.format("%Y-%m-%dT%H:%M:%S%.f").to_string()),
                    "EndsAtUtc":e.ends_at.map(|d|d.format("%Y-%m-%dT%H:%M:%S%.f").to_string()),
                    "TimeTbd":e.time_tbd,"Tv":e.tv,"StreamUrl":e.stream_url,"LiveStatsUrl":e.live_stats_url,
                    "TeamLogoUrl":e.team_logo_url,"OpponentLogoUrl":e.opponent_logo_url,
                    "Status":fixture.status,"CanonicalKey":key,"IsNeutral":fixture.neutral,
                    "ProviderKey":fixture.league.provider_key(),"SchoolName":fixture.team_name,
                    "SchoolId":fixture.team_id,"OpponentSchoolId":fixture.opponent_id,
                    "SkipInsert":fixture.completed || e.event_date<today
                }));
            }
            let rows=rows.into_values().collect::<Vec<_>>();
            for chunk in rows.chunks(100) {
                let payload=serde_json::to_string(chunk)?;
                saved+=self.client.execute(include_str!("../sql/upsert_espn_schedule_batch.sql"),&[&type_id,&payload]).await?.total() as usize;
            }
            eprintln!("metrics: espn_schedule rows={} sql_batches={}",rows.len(),rows.len().div_ceil(100));
            Ok(saved)
            }).await;
            match result {
                Ok(n)=>{self.client.simple_query("COMMIT TRANSACTION").await?.into_results().await?;Ok(n)},
                Err(e)=>{self.client.simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION").await?.into_results().await?;Err(e)},
            }
        },&self.poisoned))
    }

    /// A single-school calendar's logo also applies to retained events no longer in its RSS.
    /// Touch only TeamLogoUrl; keep event identities, opponents and schedule facts intact.
    pub fn refresh_school_team_logo(&mut self, type_id: i32, logo: Option<&str>) -> Result<u64> {
        self.rt.block_on(bounded(
            async {
                let result = self
                    .client
                    .execute(
                        "UPDATE dbo.SportsEvents SET TeamLogoUrl=@P2 WHERE SportsEventsTypeId=@P1
                 AND (TeamLogoUrl <> @P2 OR (TeamLogoUrl IS NULL AND @P2 IS NOT NULL)
                      OR (TeamLogoUrl IS NOT NULL AND @P2 IS NULL))",
                        &[&type_id, &logo],
                    )
                    .await?;
                Ok(result.total())
            },
            &self.poisoned,
        ))
    }

    /// Upserts each video keyed on its YouTube video ID, linked to its `YoutubeVideoFeed` row. Title, description and the view/rating
    /// counts change over time, so they are overwritten; `FirstSeenAt` is kept.
    /// A value the source didn't supply (`None`) never blanks a stored one, and an estimated
    /// publish date is only used for a video that isn't stored yet.
    /// Returns the number of videos written.
    pub fn save_videos(&mut self, feed_id: i32, videos: &[Video]) -> Result<usize> {
        self.rt.block_on(bounded(async {
            for v in videos {
                let title: String = utf16_prefix(&v.title,500);
                self.client
                    .execute(
                        "MERGE dbo.YouTubeVideos AS t
                         USING (SELECT @P1 AS VideoId) AS s
                         ON t.VideoId = s.VideoId
                         WHEN MATCHED THEN UPDATE SET
                             ChannelId = @P2, ChannelName = COALESCE(@P3, t.ChannelName), Title = @P4, Url = @P5,
                             PublishedAt = CASE WHEN @P14 = 1 THEN t.PublishedAt ELSE @P6 END,
                             UpdatedAt = COALESCE(@P7, t.UpdatedAt), ThumbnailUrl = COALESCE(@P8, t.ThumbnailUrl),
                             Description = COALESCE(@P9, t.Description), ViewCount = COALESCE(@P10, t.ViewCount),
                             RatingCount = COALESCE(@P11, t.RatingCount),
                             RatingAverage = COALESCE(@P12, t.RatingAverage),
                             YoutubeVideoFeedId = @P13, LastSeenAt = SYSUTCDATETIME()
                         WHEN NOT MATCHED THEN INSERT
                             (VideoId, ChannelId, ChannelName, Title, Url, PublishedAt, UpdatedAt,
                              ThumbnailUrl, Description, ViewCount, RatingCount, RatingAverage,
                              YoutubeVideoFeedId)
                             VALUES (@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10, @P11, @P12, @P13);",
                        &[
                            &v.video_id, &v.channel_id, &v.channel_name, &title, &v.url,
                            &v.published_at, &v.updated_at, &v.thumbnail_url, &v.description,
                            &v.views, &v.rating_count, &v.rating_average, &feed_id, &v.published_is_estimate,
                        ],
                    )
                    .await?;
            }
            Ok(videos.len())
        },&self.poisoned))
    }

    pub fn configure_big12(
        &mut self,
        members: &[crate::big12::Member],
        enable: bool,
    ) -> Result<()> {
        let known = self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query("SELECT ID,RssUrl FROM dbo.SportsEventsType", &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok::<_, crate::Error>(
                    rows.iter()
                        .filter_map(|r| {
                            Some((r.get::<i32, _>(0)?, r.get::<&str, _>(1)?.to_owned()))
                        })
                        .collect::<Vec<_>>(),
                )
            },
            &self.poisoned,
        ))?;
        for m in members {
            let matches: Vec<_> = known
                .iter()
                .filter(|(_, url)| {
                    crate::big12::school_id(url) == Some(m.id)
                        && url::Url::parse(url).ok().is_some_and(|u| {
                            u.query_pairs().any(|(k, v)| k == "sport_id" && v == "0")
                        })
                })
                .collect();
            if matches.len() > 1 {
                return Err(format!("multiple all-sport feeds configured for {}", m.name).into());
            }
            let existing = matches.first().map(|r| r.0);
            let url = crate::big12::feed_url(m.id);
            let label = format!("{} – Big 12 calendar", m.name);
            self.rt.block_on(bounded(async {
                self.client.execute("SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
                 IF @P1 IS NOT NULL UPDATE dbo.SportsEventsType SET ProviderKey='big12',ProviderSchoolId=@P2,SchoolName=@P3 WHERE ID=@P1;
                 ELSE IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ProviderKey='big12' AND ProviderSchoolId=@P2)
                  INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,SchoolName,ProviderKey,ProviderSchoolId,Enabled) VALUES(@P4,@P5,@P3,'big12',@P2,0);
                 COMMIT TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT>0 ROLLBACK TRANSACTION; THROW; END CATCH",&[&existing,&m.id,&m.name,&url,&label]).await?;
                Ok::<_,crate::Error>(())
            },&self.poisoned))?;
        }
        self.rt.block_on(bounded(
            async {
                self.client
                    .simple_query(include_str!("../sql/backfill_big12_sources.sql"))
                    .await?
                    .into_results()
                    .await?;
                if enable {
                    self.client
                        .execute(
                            "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='big12'",
                            &[],
                        )
                        .await?;
                }
                Ok::<_, crate::Error>(())
            },
            &self.poisoned,
        ))
    }

    pub fn event_attempt(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=@P1",
                    &[&id],
                )
                .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    pub fn save_big12_observation(
        &mut self,
        feed: i32,
        observation: &serde_json::Value,
    ) -> Result<String> {
        let payload = serde_json::to_string(observation)?;
        for attempt in 0..3 {
            let result: Result<String> = self.rt.block_on(bounded(
                async {
                    let rows = self
                        .client
                        .query(
                            include_str!("../sql/upsert_big12_event.sql"),
                            &[&feed, &payload],
                        )
                        .await?
                        .into_first_result()
                        .await?;
                    Ok(rows
                        .first()
                        .and_then(|r| r.get::<&str, _>("Outcome"))
                        .unwrap_or("unknown")
                        .to_owned())
                },
                &self.poisoned,
            ));
            match result {
                Err(e) if attempt < 2 && e.to_string().to_lowercase().contains("deadlock") => {
                    std::thread::sleep(std::time::Duration::from_millis(100 * (attempt + 1)))
                }
                other => return other,
            }
        }
        unreachable!("retry loop always returns")
    }

    pub fn event_result(
        &mut self,
        id: i32,
        success: bool,
        count: i32,
        issues: i32,
        error: Option<&str>,
    ) -> Result<()> {
        let error = error.map(|s| s.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(async {
            self.client.execute("UPDATE dbo.SportsEventsType SET LastSuccessAt=CASE WHEN @P2=1 THEN SYSUTCDATETIME() ELSE LastSuccessAt END,LastEventCount=@P3,LastIssueCount=@P4,LastError=@P5 WHERE ID=@P1",&[&id,&success,&count,&issues,&error]).await?;Ok(())
        },&self.poisoned))
    }
    /// Every calendar feed in the `SportsEventsType` lookup table.
    pub fn sports_event_feeds(&mut self) -> Result<Vec<EventFeed>> {
        self.rt.block_on(bounded(async {
            let rows = self
                .client
                .query("SELECT ID, RssUrl, SchoolName FROM dbo.SportsEventsType WHERE Enabled=1 ORDER BY ID", &[])
                .await?
                .into_first_result()
                .await?;
            Ok(rows
                .iter()
                .filter_map(|r| {
                    Some(EventFeed {
                        id: r.get::<i32, _>(0)?,
                        url: r.get::<&str, _>(1)?.to_string(),
                        school: r.get::<&str, _>(2).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string),
                    })
                })
                .collect())
        },&self.poisoned))
    }

    /// ID of the `SportsEventsType` row for this feed URL, adding the row (named `name`) if it
    /// isn't there yet. An existing row keeps its name.
    pub fn ensure_sports_events_type(&mut self, rss_url: &str, name: &str) -> Result<i32> {
        if rss_url.chars().count() > 450 {
            return Err("calendar feed URL is longer than the 450 characters SportsEventsType.RssUrl allows".into());
        }
        self.rt.block_on(bounded(async {
            let rows = self
                .client
                .query(
                    "DECLARE @id INT = (SELECT ID FROM dbo.SportsEventsType WHERE RssUrl = @P1);
                     IF @id IS NULL
                     BEGIN
                         INSERT INTO dbo.SportsEventsType (RssUrl, EventsTypeName) VALUES (@P1, @P2);
                         SET @id = SCOPE_IDENTITY();
                     END
                     SELECT @id;",
                    &[&rss_url, &name],
                )
                .await?
                .into_first_result()
                .await?;
            rows.first()
                .and_then(|r| r.get::<i32, _>(0))
                .ok_or_else(|| "could not read the SportsEventsType ID".into())
        },&self.poisoned))
    }

    /// `(ID, Url)` of every channel in the `YoutubeVideoFeed` lookup table.
    pub fn youtube_feeds(&mut self) -> Result<Vec<(i32, String)>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query("SELECT ID, Url FROM dbo.YoutubeVideoFeed ORDER BY ID", &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .filter_map(|r| Some((r.get::<i32, _>(0)?, r.get::<&str, _>(1)?.to_string())))
                    .collect())
            },
            &self.poisoned,
        ))
    }

    /// ID of the `YoutubeVideoFeed` row for this channel, adding the row if it isn't there yet.
    /// An existing row keeps its URL; a missing channel name is filled in.
    pub fn ensure_youtube_feed(
        &mut self,
        channel_id: &str,
        name: Option<&str>,
        url: &str,
    ) -> Result<i32> {
        self.rt.block_on(bounded(async {
            let rows = self
                .client
                .query(
                    "DECLARE @id INT = (SELECT ID FROM dbo.YoutubeVideoFeed WHERE ChannelId = @P1);
                     IF @id IS NULL
                     BEGIN
                         INSERT INTO dbo.YoutubeVideoFeed (ChannelId, ChannelName, Url) VALUES (@P1, @P2, @P3);
                         SET @id = SCOPE_IDENTITY();
                     END
                     ELSE
                         UPDATE dbo.YoutubeVideoFeed SET ChannelName = COALESCE(ChannelName, @P2) WHERE ID = @id;
                     SELECT @id;",
                    &[&channel_id, &name, &url],
                )
                .await?
                .into_first_result()
                .await?;
            rows.first()
                .and_then(|r| r.get::<i32, _>(0))
                .ok_or_else(|| "could not read the YoutubeVideoFeed ID".into())
        },&self.poisoned))
    }

    /// Deletes each channel's oldest videos so at most `keep` remain per channel.
    /// Returns the number of rows deleted.
    pub fn prune_videos(&mut self, keep: u32) -> Result<u64> {
        let keep = checked_keep(keep)?;
        self.rt.block_on(bounded(
            async {
                let res = self
                    .client
                    .execute(
                        ";WITH ranked AS (
                         SELECT ROW_NUMBER() OVER (
                             PARTITION BY ChannelId ORDER BY PublishedAt DESC, ID DESC) AS rn
                         FROM dbo.YouTubeVideos
                     )
                     DELETE FROM ranked WHERE rn > @P1",
                        &[&keep],
                    )
                    .await?;
                Ok(res.total())
            },
            &self.poisoned,
        ))
    }
}

#[cfg(test)]
mod movie_tests {
    use super::*;

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn movie_disc_policy_blocks_writes_and_preserves_other_channels() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, DB_NAME)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            let source = "disc-policy-fixture";
            db.rt.block_on(db.client.execute("INSERT dbo.MovieReleaseSources(SourceKey,Name,Url,Enabled,CollectDisc) VALUES(@P1,N'Disc policy fixture',N'https://example.invalid/',1,0)", &[&source]))?;
            let report = serde_json::json!({"issues":[]});
            let mut row = crate::movies::MovieRelease {
                source: source.into(),
                external_id: "fixture".into(),
                title: "Policy fixture".into(),
                imdb_id: None,
                key: "disc:format:dvd".into(),
                kind: "disc".into(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
                url: "https://example.invalid/film".into(),
                format: Some("DVD".into()),
                platform: None,
            };
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET CollectDisc=1 WHERE SourceKey=@P1",
                &[&source],
            ))?;
            assert_eq!(
                db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                1
            );
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET CollectDisc=0 WHERE SourceKey=@P1",
                &[&source],
            ))?;
            assert!(!db.tracked_movie_urls()?.contains(&row.url));
            db.rt.block_on(db.client.execute("UPDATE r SET ReleaseStatus='withdrawn' FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.SourceKey=@P1 AND r.ReleaseType='disc'", &[&source]))?;
            assert_eq!(
                db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                0
            );
            for kind in [
                "theatrical",
                "digital",
                "digital_purchase",
                "digital_rental",
                "subscription",
            ] {
                row.kind = kind.into();
                row.key = format!("{kind}:first");
                row.platform = (kind == "subscription").then(|| "Fixture service".into());
                assert_eq!(
                    db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                    1
                );
                assert_eq!(
                    db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                    1
                );
            }
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey=@P1",
                &[&source],
            ))?;
            assert_eq!(
                db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                0
            );
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn movie_upserts_are_repeatable_and_reschedule_in_place() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, DB_NAME)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let report = serde_json::json!({"issues": []});
            db.save_movie_releases(&[], &report)?;
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET Enabled=1,CollectDisc=1 WHERE SourceKey='dvdsreleasedates'",
                &[],
            ))?;
            let mut row = crate::movies::MovieRelease {
                source: "dvdsreleasedates".into(),
                external_id: "integration-fixture-movie".into(),
                title: "Integration fixture".into(),
                imdb_id: None,
                key: "disc:upc:fixture".into(),
                kind: "disc".into(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(),
                url: "https://example.invalid/integration-fixture".into(),
                format: Some("DVD".into()),
                platform: None,
            };
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            row.date = chrono::NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            db.rt.block_on(async {
                let rows = db.client.query("SELECT r.ReleaseDate FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId WHERE l.ExternalMovieId='integration-fixture-movie'", &[]).await?.into_first_result().await?;
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].get::<chrono::NaiveDate,_>(0), Some(row.date));
                db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='dvdsreleasedates'", &[]).await?;
                Ok::<_, crate::Error>(())
            })?;
            assert_eq!(db.save_movie_releases(&[row], &report)?, 0);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

#[cfg(test)]
mod school_logo_tests {
    use super::*;

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn school_logo_refresh_preserves_identity_opponents_and_other_feeds() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let kansas = db.ensure_sports_events_type(
                "https://example.invalid/logo-test-kansas",
                "Logo test Kansas",
            )?;
            let other = db.ensure_sports_events_type(
                "https://example.invalid/logo-test-other",
                "Logo test other",
            )?;
            let fixture = |id| Event {
                url: format!("https://example.invalid/logo-test-event-{id}"),
                game_id: Some(id),
                title: "Logo test event".into(),
                sport: None,
                opponent: Some("Opponent".into()),
                is_away: Some(false),
                location: None,
                event_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
                starts_at: None,
                ends_at: None,
                time_tbd: true,
                tv: None,
                stream_url: None,
                live_stats_url: None,
                team_logo_url: Some("http://big12sports.com/images/logos/site/site.png".into()),
                opponent_logo_url: Some("https://example.invalid/opponent.png".into()),
            };
            let mut current = fixture(1);
            db.save_events(kansas, &[fixture(1), fixture(2)])?;
            db.save_events(other, &[fixture(3)])?;
            let before = db.rt.block_on(async {
                db.client
                    .query(
                        "SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE Url=@P1",
                        &[&current.url],
                    )
                    .await?
                    .into_first_result()
                    .await
            })?;
            let old_id = before[0].get::<i32, _>(0);
            let old_seen = before[0].get::<chrono::NaiveDateTime, _>(1);
            current.team_logo_url = Some("https://example.invalid/jayhawk.png".into());
            db.save_events(kansas, &[current])?;
            assert_eq!(
                db.refresh_school_team_logo(kansas, Some("https://example.invalid/jayhawk.png"))?,
                1
            );
            assert_eq!(
                db.refresh_school_team_logo(kansas, Some("https://example.invalid/jayhawk.png"))?,
                0
            );
            db.rt.block_on(async {
                let rows = db.client.query("SELECT ID,Url,FirstSeenAt,TeamLogoUrl,OpponentLogoUrl FROM dbo.SportsEvents WHERE SportsEventsTypeId IN (@P1,@P2) ORDER BY ID", &[&kansas,&other]).await?.into_first_result().await?;
                assert_eq!(rows.len(),3);
                assert_eq!(rows[0].get::<i32,_>("ID"),old_id);
                assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>("FirstSeenAt"),old_seen);
                for row in &rows {
                    assert_eq!(row.get::<&str,_>("OpponentLogoUrl"),Some("https://example.invalid/opponent.png"));
                }
                assert_eq!(rows[0].get::<&str,_>("TeamLogoUrl"),Some("https://example.invalid/jayhawk.png"));
                assert_eq!(rows[1].get::<&str,_>("TeamLogoUrl"),Some("https://example.invalid/jayhawk.png"));
                assert_eq!(rows[2].get::<&str,_>("TeamLogoUrl"),Some("http://big12sports.com/images/logos/site/site.png"));
                Ok::<_,crate::Error>(())
            })?;
            assert_eq!(db.refresh_school_team_logo(kansas, None)?, 2);
            assert_eq!(db.refresh_school_team_logo(kansas, None)?, 0);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

#[cfg(test)]
mod big12_tests {
    use super::*;
    #[test]
    #[ignore = "requires the named Big 12 staging database; fixtures roll back"]
    fn mirrored_imports_reschedule_without_duplicates_or_perspective_changes() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open_named(
            &std::env::var("MSSQL_CONNECTION_STRING")?,
            "WebScraper_Big12_Validation_20261009",
        )?;
        let feeds = db.sports_event_feeds()?;
        let kansas = feeds
            .iter()
            .find(|f| crate::big12::school_id(&f.url) == Some(3))
            .ok_or("Kansas staging feed missing")?
            .id;
        let utah = feeds
            .iter()
            .find(|f| crate::big12::school_id(&f.url) == Some(36))
            .ok_or("Utah staging feed missing")?
            .id;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let before=db.rt.block_on(async{db.client.query("SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE ProviderKey='big12' AND GameId=179871",&[]).await?.into_first_result().await})?;
            let original_id = before[0].get::<i32, _>(0).unwrap();
            let original_seen = before[0].get::<chrono::NaiveDateTime, _>(1);
            let mut a = crate::big12::tests::fixture(3);
            let b = crate::big12::tests::fixture(36);
            for _ in 0..2 {
                assert_eq!(db.save_big12_observation(kansas, &a)?, "saved");
                assert_eq!(db.save_big12_observation(utah, &b)?, "saved");
            }
            a["event"]["event_date"] = serde_json::json!("2026-10-17");
            assert_eq!(db.save_big12_observation(kansas, &a)?, "saved");
            assert_eq!(db.save_big12_observation(utah, &b)?, "saved"); // Secondary stale date cannot revert the primary schedule.
            db.rt.block_on(async {
    let rows=db.client.query("SELECT ID,FirstSeenAt,SchoolId,Opponent,CanonicalKey,EventDate FROM dbo.SportsEvents WHERE CanonicalKey='big12:pair:179871:179872'",&[]).await?.into_first_result().await?;
    assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<i32,_>("ID"),Some(original_id));assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>("FirstSeenAt"),original_seen);
    assert_eq!(rows[0].get::<i32,_>("SchoolId"),Some(3));assert_eq!(rows[0].get::<&str,_>("Opponent"),Some("Utah"));assert_eq!(rows[0].get::<chrono::NaiveDate,_>("EventDate"),chrono::NaiveDate::from_ymd_opt(2026,10,17));
    let sources=db.client.query("SELECT ProviderGameId FROM dbo.SportsEventSources WHERE SportsEventId=@P1 AND ProviderGameId IN(179871,179872)",&[&original_id]).await?.into_first_result().await?;assert_eq!(sources.len(),2);
    Ok::<_,crate::Error>(())
   })?;
            db.event_result(utah, true, 1, 0, None)?;
            db.event_attempt(utah)?;
            db.event_result(utah, false, 0, 1, Some("fixture feed failure"))?;
            db.rt.block_on(async {
    let rows=db.client.query("SELECT LastAttemptAt,LastSuccessAt,LastError FROM dbo.SportsEventsType WHERE ID=@P1",&[&utah]).await?.into_first_result().await?;
    assert!(rows[0].get::<chrono::NaiveDateTime,_>(0).is_some());assert!(rows[0].get::<chrono::NaiveDateTime,_>(1).is_some());assert_eq!(rows[0].get::<&str,_>(2),Some("fixture feed failure"));
    Ok::<_,crate::Error>(())
   })?;
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                        &[&utah],
                    )
                    .await?;
                Ok::<_, crate::Error>(())
            })?;
            assert_eq!(db.save_big12_observation(utah, &b)?, "disabled");
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ID=@P1",
                        &[&utah],
                    )
                    .await?;
                Ok::<_, crate::Error>(())
            })?;
            let mut bad = b;
            bad["issue"] = serde_json::json!("identity unavailable");
            bad["event"]["url"] = serde_json::json!("https://example.invalid/broken");
            assert_eq!(db.save_big12_observation(utah, &bad)?, "quarantined");
            db.rt.block_on(async {
    let rows=db.client.query("SELECT Payload,SourceUrl FROM dbo.SportsEventSources WHERE SportsEventsTypeId=@P1 AND ProviderGameId=179872",&[&utah]).await?.into_first_result().await?;
    let payload:serde_json::Value=serde_json::from_str(rows[0].get::<&str,_>(0).unwrap())?;assert!(payload["issue"].is_null());assert_eq!(rows[0].get::<&str,_>(1),Some("http://big12sports.com/calendar.aspx?id=179872"));
    Ok::<_,crate::Error>(())
   })?;
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

#[cfg(test)]
mod poster_tests {
    use super::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn poster_backfill_preserves_rows_and_concurrent_valid_posters() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let report = serde_json::json!({"issues":[]});
            db.save_movie_releases(&[], &report)?;
            db.rt.block_on(async{db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='dvdsreleasedates'",&[]).await?;Ok::<_,crate::Error>(())})?;
            let row = crate::movies::MovieRelease {
                source: "dvdsreleasedates".into(),
                external_id: "poster-regression-fixture".into(),
                title: "Poster regression fixture".into(),
                imdb_id: None,
                key: "theatrical:first".into(),
                kind: "theatrical".into(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
                url: "https://example.invalid/poster-fixture".into(),
                format: None,
                platform: None,
            };
            db.save_movie_releases(std::slice::from_ref(&row), &report)?;
            let job = db
                .poster_jobs()?
                .into_iter()
                .find(|j| j.identity.title == row.title)
                .unwrap();
            let poster = "https://image.tmdb.org/t/p/w500/fixture.jpg";
            assert!(db.set_movie_poster(
                job.movie_id,
                None,
                Some(poster),
                Some(1947),
                "matched",
                Some(&job.url),
                None,
                30
            )?);
            assert!(!db.set_movie_poster(
                job.movie_id,
                None,
                Some("https://image.tmdb.org/t/p/w500/wrong-remake.jpg"),
                Some(2026),
                "matched",
                Some(&job.url),
                None,
                30
            )?);
            let metadata = serde_json::json!({"title":row.title,"year":1947,"imdb_id":null,"tmdb_id":null,"posters":[poster]});
            db.cache_poster_source(job.link_id, "fixture", &metadata, Some(poster), None)?;
            db.cache_poster_source(job.link_id, "fixture", &metadata, Some(poster), None)?;
            db.save_movie_releases(&[row], &report)?;
            db.rt.block_on(async {
                let rows=db.client.query("SELECT PosterUrl,OriginalYear FROM dbo.Movies WHERE ID=@P1",&[&job.movie_id]).await?.into_first_result().await?;
                assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<&str,_>(0),Some(poster));assert_eq!(rows[0].get::<i16,_>(1),Some(1947));
                let rows=db.client.query("SELECT COUNT(*) AS Total FROM dbo.MovieReleases WHERE MovieSourceLinkId=@P1",&[&job.link_id]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(1));
                let rows=db.client.query("SELECT COUNT(*) FROM dbo.MoviePosterCache WHERE SourceLinkId=@P1",&[&job.link_id]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(1));
                Ok::<_,crate::Error>(())
            })?;
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

#[cfg(test)]
mod tvmaze_tests {
    use super::*;
    #[test]
    #[ignore = "requires named TVmaze staging database; test failures/status changes isolated to staging"]
    fn tv_snapshots_reschedule_preserve_history_and_disabled_settings() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open_named(
            &std::env::var("MSSQL_CONNECTION_STRING")?,
            "WebScraper_Tvmaze_Validation_20261009",
        )?;
        db.setup_tv(true)?;
        let before = db.rt.block_on(async {
            db.client
                .query(
                    "SELECT ID,FirstSeenAt FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?;
        let original_id = before[0].get::<i32, _>(0);
        let original_seen = before[0].get::<chrono::NaiveDateTime, _>(1);
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let phase = (|| -> Result<()> {
            let (s, mut e) = crate::tvmaze::tests::fixtures(83073);
            e[0]["airdate"] = serde_json::json!("2026-10-11");
            let (p, _) = crate::tvmaze::normalize(83073, &s, &e)?;
            for _ in 0..2 {
                let lease = db.claim_tv_series(83073, true)?.unwrap();
                assert_eq!(db.save_tv_snapshot(83073, &lease, &p)?, "saved");
            }
            db.rt.block_on(async {
    let rows=db.client.query("SELECT ID,FirstSeenAt,Airdate,StartsAtUtc,IsDateOnly FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await?;
    assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<i32,_>(0),original_id);assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>(1),original_seen);assert_eq!(rows[0].get::<chrono::NaiveDate,_>(2),chrono::NaiveDate::from_ymd_opt(2026,10,11));assert!(rows[0].get::<chrono::NaiveDateTime,_>(3).is_none());assert_eq!(rows[0].get::<bool,_>(4),Some(true));
    db.client.execute("UPDATE dbo.TvTrackedSeries SET Enabled=0 WHERE TvmazeShowId=83073",&[]).await?;Ok::<_,crate::Error>(())
   })?;
            db.setup_tv(true)?;
            assert!(!db.tv_jobs()?.iter().any(|(id, _)| *id == 83073));
            assert!(db.claim_tv_series(83073, true)?.is_none());
            assert_eq!(
                db.save_tv_snapshot(83073, "00000000-0000-0000-0000-000000000000", &p)?,
                "disabled_or_superseded"
            );
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        phase?;
        let (s, mut e) = crate::tvmaze::tests::fixtures(83073);
        e.as_array_mut().unwrap().pop();
        let (p, _) = crate::tvmaze::normalize(83073, &s, &e)?;
        let lease = db.claim_tv_series(83073, true)?.unwrap();
        assert!(db.save_tv_snapshot(83073, &lease, &p).is_err());
        db.fail_tv_series(83073, &lease, "fixture incomplete response", 900)?;
        let lease = db.claim_tv_series(64950, true)?.unwrap();
        db.fail_tv_series(64950, &lease, "fixture HTTP 503", 900)?;
        let (s, e) = crate::tvmaze::tests::fixtures(45039);
        let (p, _) = crate::tvmaze::normalize(45039, &s, &e)?;
        let lease = db.claim_tv_series(45039, true)?.unwrap();
        assert_eq!(db.save_tv_snapshot(45039, &lease, &p)?, "saved");
        db.rt.block_on(async {
   let rows=db.client.query("SELECT COUNT(*) FROM dbo.TvEpisodes",&[]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(90));
   let rows=db.client.query("SELECT LastSuccessAt,EpisodeCount,LastError FROM dbo.TvTrackedSeries WHERE TvmazeShowId=83073",&[]).await?.into_first_result().await?;assert!(rows[0].get::<chrono::NaiveDateTime,_>(0).is_some());assert_eq!(rows[0].get::<i32,_>(1),Some(13));assert_eq!(rows[0].get::<&str,_>(2),Some("fixture incomplete response"));
   let rows=db.client.query("SELECT ID,FirstSeenAt,Airdate FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),original_id);assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>(1),original_seen);assert_eq!(rows[0].get::<chrono::NaiveDate,_>(2),chrono::NaiveDate::from_ymd_opt(2026,10,9));
   Ok::<_,crate::Error>(())
  })?;
        Ok(())
    }
}

#[cfg(test)]
mod nwsl_tests {
    use super::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING and NWSL_TEST_DATABASE (isolated staging); fixture writes roll back"]
    fn schedule_reimports_reschedules_and_cancellation_preserve_rows() -> Result<()> {
        if let Ok(profile) = std::env::var("NWSL_TEST_PROFILE") {
            dotenvy::from_filename(profile)?;
        }
        let _ = dotenvy::dotenv();
        let name = std::env::var("NWSL_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Nwsl_Validation") {
            return Err("use an isolated WebScraper_Nwsl_Validation database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            let legacy=db.ensure_sports_events_type("https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907","Kansas City Current")?;
            db.rt.block_on(async {db.client.execute("UPDATE dbo.SportsEventsType SET ProviderKey='espn-nwsl',ProviderSchoolId=20907,Enabled=0 WHERE ID=@P1",&[&legacy]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_nwsl()?;
            let id = db.ensure_sports_events_type(
                "https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard",
                "Kansas City Current",
            )?;
            assert_eq!(id, legacy);
            let mut fixtures = crate::nwsl::parse(&crate::nwsl::tests::sample(), "20907")?;
            for f in &mut fixtures {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 0); // Disabled configuration stays disabled.
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ID=@P1",
                        &[&id],
                    )
                    .await?;
                Ok::<_, crate::Error>(())
            })?;
            db.configure_nwsl()?; // Reconfiguration must preserve Enabled.
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 3);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,GameId,ScheduleStatus,EventDate,StartsAtUtc FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&id]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].event.event_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
            fixtures[0].event.starts_at = Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-08T01:00:00Z")?.with_timezone(&Utc),
            );
            db.save_espn_schedule(id, &fixtures)?;
            let rescheduled = read(&mut db)?;
            let game = rescheduled
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(
                game.get::<chrono::NaiveDateTime, _>("StartsAtUtc"),
                fixtures[0].event.starts_at.map(|t| t.naive_utc())
            );
            fixtures[0].event.starts_at = None;
            fixtures[0].event.time_tbd = true;
            fixtures[0].status = "STATUS_CANCELED".into();
            db.save_espn_schedule(id, &fixtures)?;
            db.save_espn_schedule(id, &[])?; // Missing observations never delete games.
            let after = read(&mut db)?;
            assert_eq!(before.len(), 3);
            assert_eq!(after.len(), 3);
            for (a, b) in before.iter().zip(&after) {
                assert_eq!(a.get::<i32, _>("ID"), b.get::<i32, _>("ID"));
                assert_eq!(
                    a.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    b.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
            }
            let canceled = after
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(
                canceled.get::<&str, _>("ScheduleStatus"),
                Some("STATUS_CANCELED")
            );
            assert!(
                canceled
                    .get::<chrono::NaiveDateTime, _>("StartsAtUtc")
                    .is_none()
            );
            assert_eq!(
                canceled.get::<chrono::NaiveDate, _>("EventDate"),
                Some(fixtures[0].event.event_date)
            );
            let root: serde_json::Value = serde_json::from_str(include_str!(
                "../tests/fixtures/nwsl/league-october-2026.json"
            ))?;
            let mut all =
                crate::espn_schedule::parse(&root, crate::espn_schedule::League::Nwsl, None)?;
            for f in &mut all {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            db.save_espn_schedule(id, &all)?;
            let expanded = read(&mut db)?;
            assert_eq!(expanded.len(), 24);
            for old in &before {
                let now = expanded
                    .iter()
                    .find(|r| r.get::<i32, _>("GameId") == old.get::<i32, _>("GameId"))
                    .unwrap();
                assert_eq!(now.get::<i32, _>("ID"), old.get::<i32, _>("ID"));
                assert_eq!(
                    now.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    old.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
            }
            db.save_espn_schedule(id, &all)?;
            assert_eq!(read(&mut db)?.len(), 24);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

#[cfg(test)]
mod wnba_tests {
    use super::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING and WNBA_TEST_DATABASE (isolated staging); fixture writes roll back"]
    fn wnba_configuration_and_schedule_updates_keep_identity_and_other_leagues() -> Result<()> {
        let _ = dotenvy::dotenv();
        let name = std::env::var("WNBA_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Wnba_Validation") {
            return Err("use an isolated WebScraper_Wnba_Validation database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            db.configure_wnba()?;
            let id = db.ensure_sports_events_type(
                "https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard",
                "WNBA",
            )?;
            let mut fixtures = crate::espn_schedule::parse(
                &crate::wnba::tests::sample(),
                crate::espn_schedule::League::Wnba,
                None,
            )?;
            for f in &mut fixtures {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 0);
            db.rt.block_on(async {db.client.execute("UPDATE dbo.SportsEventsType SET Enabled=1,RssUrl=RssUrl+'?season=2026' WHERE ID=@P1",&[&id]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_wnba()?;
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .filter(|f| f.id == id)
                    .count(),
                1
            );
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .find(|f| f.id == id)
                    .unwrap()
                    .url,
                "https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard?season=2026"
            );
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 2);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,GameId,ProviderKey,CanonicalKey,ScheduleStatus,EventDate,StartsAtUtc,SchoolName,Opponent FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&id]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].event.event_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
            fixtures[0].event.starts_at = Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-05T01:00:00Z")?.with_timezone(&Utc),
            );
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].status = "STATUS_POSTPONED".into();
            fixtures[0].event.starts_at = None;
            fixtures[0].event.time_tbd = true;
            db.save_espn_schedule(id, &fixtures)?;
            db.save_espn_schedule(id, &[])?;
            let after = read(&mut db)?;
            assert_eq!(after.len(), 2);
            for (a, b) in before.iter().zip(&after) {
                assert_eq!(a.get::<i32, _>("ID"), b.get::<i32, _>("ID"));
                assert_eq!(
                    a.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    b.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
                assert_eq!(b.get::<&str, _>("ProviderKey"), Some("espn-wnba"));
            }
            let f = after
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(f.get::<&str, _>("SchoolName"), Some("Atlanta Dream"));
            assert_eq!(f.get::<&str, _>("Opponent"), Some("Connecticut Sun"));
            assert_eq!(f.get::<&str, _>("ScheduleStatus"), Some("STATUS_POSTPONED"));
            assert!(f.get::<chrono::NaiveDateTime, _>("StartsAtUtc").is_none());
            assert_eq!(
                f.get::<chrono::NaiveDate, _>("EventDate"),
                Some(fixtures[0].event.event_date)
            );
            // Equal numeric IDs in another league remain independent public rows.
            let soccer_id = db.ensure_sports_events_type(
                "https://example.invalid/wnba-test-nwsl",
                "Other league",
            )?;
            let mut soccer = crate::nwsl::parse(&crate::nwsl::tests::sample(), "20907")?.remove(0);
            soccer.completed = false;
            soccer.status = "STATUS_SCHEDULED".into();
            soccer.event.event_date = Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive()
                + chrono::Duration::days(1);
            soccer.event.game_id = fixtures[0].event.game_id;
            soccer.event.url = format!(
                "https://www.espn.com/soccer/match/_/gameId/{}",
                soccer.event.game_id.unwrap()
            );
            assert_eq!(db.save_espn_schedule(soccer_id, &[soccer])?, 1);
            assert_eq!(read(&mut db)?.len(), 2);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

#[cfg(test)]
mod mlb_tests {
    use super::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING and MLB_TEST_DATABASE (isolated staging); fixture writes roll back"]
    fn mlb_configuration_and_schedule_updates_keep_identity_and_other_leagues() -> Result<()> {
        let _ = dotenvy::dotenv();
        let name = std::env::var("MLB_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Mlb_Validation") {
            return Err("use an isolated WebScraper_Mlb_Validation database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            db.configure_mlb()?;
            let id = db.ensure_sports_events_type(
                "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard",
                "MLB",
            )?;
            let mut fixtures = crate::espn_schedule::parse(
                &crate::mlb::tests::sample(),
                crate::espn_schedule::League::Mlb,
                None,
            )?;
            let mut doubleheader = crate::espn_schedule::parse(
                &crate::mlb::tests::sample(),
                crate::espn_schedule::League::Mlb,
                None,
            )?
            .remove(0);
            doubleheader.event.game_id = Some(401907995);
            doubleheader.event.url = "https://www.espn.com/mlb/game/_/gameId/401907995".into();
            fixtures.push(doubleheader);
            for f in &mut fixtures {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 0);
            db.rt.block_on(async {db.client.execute("UPDATE dbo.SportsEventsType SET Enabled=1,RssUrl=RssUrl+'?season=2026' WHERE ID=@P1",&[&id]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_mlb()?;
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .filter(|f| f.id == id)
                    .count(),
                1
            );
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .find(|f| f.id == id)
                    .unwrap()
                    .url,
                "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard?season=2026"
            );
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 4);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,GameId,ProviderKey,CanonicalKey,ScheduleStatus,EventDate,StartsAtUtc,SchoolName,Opponent FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&id]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].event.event_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
            fixtures[0].event.starts_at = Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-05T01:00:00Z")?.with_timezone(&Utc),
            );
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].status = "STATUS_POSTPONED".into();
            fixtures[0].event.starts_at = None;
            fixtures[0].event.time_tbd = true;
            db.save_espn_schedule(id, &fixtures)?;
            db.save_espn_schedule(id, &[])?;
            let after = read(&mut db)?;
            assert_eq!(after.len(), 4);
            for (a, b) in before.iter().zip(&after) {
                assert_eq!(a.get::<i32, _>("ID"), b.get::<i32, _>("ID"));
                assert_eq!(
                    a.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    b.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
                assert_eq!(b.get::<&str, _>("ProviderKey"), Some("espn-mlb"));
            }
            let f = after
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(f.get::<&str, _>("SchoolName"), Some("Cleveland Guardians"));
            assert_eq!(f.get::<&str, _>("Opponent"), Some("Chicago White Sox"));
            assert_eq!(f.get::<&str, _>("ScheduleStatus"), Some("STATUS_POSTPONED"));
            assert!(f.get::<chrono::NaiveDateTime, _>("StartsAtUtc").is_none());
            assert_eq!(
                f.get::<chrono::NaiveDate, _>("EventDate"),
                Some(fixtures[0].event.event_date)
            );
            // Equal numeric IDs in another league remain independent public rows.
            let soccer_id = db.ensure_sports_events_type(
                "https://example.invalid/mlb-test-nwsl",
                "Other league",
            )?;
            let mut soccer = crate::nwsl::parse(&crate::nwsl::tests::sample(), "20907")?.remove(0);
            soccer.completed = false;
            soccer.status = "STATUS_SCHEDULED".into();
            soccer.event.event_date = Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive()
                + chrono::Duration::days(1);
            soccer.event.game_id = fixtures[0].event.game_id;
            soccer.event.url = format!(
                "https://www.espn.com/soccer/match/_/gameId/{}",
                soccer.event.game_id.unwrap()
            );
            assert_eq!(db.save_espn_schedule(soccer_id, &[soccer])?, 1);
            assert_eq!(read(&mut db)?.len(), 4);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn checked_retention_and_unicode_limits() {
        assert!(checked_keep(2147483648).is_err());
        assert!(checked_keep(0).is_err());
        assert_eq!(utf16_prefix("😀😀x", 4), "😀😀");
        assert_eq!(utf16_prefix("😀x", 1), "");
    }
}

#[cfg(test)]
mod hardening_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires isolated HARDENING_TEST_DATABASE; never runs against production"]
    fn final_status_reconciliation_and_failed_snapshot_are_safe() -> Result<()> {
        let _ = dotenvy::dotenv();
        let name = std::env::var("HARDENING_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Hardening_Validation_") {
            return Err("isolated hardening database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.setup_tv(false)?;
        let feed = db.ensure_sports_events_type(
            "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard?season=2026",
            "Hardening fixture",
        )?;
        db.rt.block_on(async {db.client.execute("DELETE dbo.SportsEvents WHERE ProviderKey='espn-mlb' AND GameId IN(90000001,90000002,90000003)",&[]).await?;Ok::<_,crate::Error>(())})?;
        let today = Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive();
        let mut f = crate::espn_schedule::parse(
            &crate::mlb::tests::sample(),
            crate::espn_schedule::League::Mlb,
            None,
        )?
        .remove(0);
        f.event.game_id = Some(90000001);
        f.event.url = "https://www.espn.com/mlb/game/_/gameId/90000001".into();
        f.event.event_date = today;
        f.completed = false;
        f.status = "STATUS_SCHEDULED".into();
        assert_eq!(db.save_espn_schedule(feed, std::slice::from_ref(&f))?, 1);
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,Title,ScheduleStatus FROM dbo.SportsEvents WHERE ProviderKey='espn-mlb' AND GameId=90000001",&[]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        f.status = "STATUS_IN_PROGRESS".into();
        db.save_espn_schedule(feed, std::slice::from_ref(&f))?;
        f.status = "STATUS_FINAL".into();
        f.completed = true;
        db.save_espn_schedule(feed, std::slice::from_ref(&f))?;
        let after = read(&mut db)?;
        assert_eq!(
            after[0].get::<&str, _>("ScheduleStatus"),
            Some("STATUS_FINAL")
        );
        assert_eq!(before[0].get::<i32, _>("ID"), after[0].get::<i32, _>("ID"));
        assert_eq!(
            before[0].get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
            after[0].get::<chrono::NaiveDateTime, _>("FirstSeenAt")
        );
        f.event.game_id = Some(90000003);
        f.event.url = "https://www.espn.com/mlb/game/_/gameId/90000003".into();
        assert_eq!(db.save_espn_schedule(feed, std::slice::from_ref(&f))?, 0); // New completed games are never imported.
        f.event.game_id = Some(90000002);
        f.event.url = "https://www.espn.com/mlb/game/_/gameId/90000002".into();
        f.completed = false;
        f.status = "STATUS_SCHEDULED".into();
        db.save_espn_schedule(feed, std::slice::from_ref(&f))?;
        let mut first = crate::espn_schedule::parse(
            &crate::mlb::tests::sample(),
            crate::espn_schedule::League::Mlb,
            None,
        )?
        .remove(0);
        first.event.game_id = Some(90000001);
        first.event.url = "https://www.espn.com/mlb/game/_/gameId/90000001".into();
        first.event.event_date = today;
        first.event.title = "must roll back".into();
        f.event.url = first.event.url.clone(); // This second source observation conflicts with two distinct canonical rows.
        assert!(db.save_espn_schedule(feed, &[first, f]).is_err());
        let after_error = read(&mut db)?;
        assert_eq!(
            after_error[0].get::<&str, _>("Title"),
            after[0].get::<&str, _>("Title")
        );
        assert_eq!(
            after_error[0].get::<&str, _>("ScheduleStatus"),
            Some("STATUS_FINAL")
        );
        db.rt.block_on(async {db.client.execute("DELETE dbo.SportsEvents WHERE ProviderKey='espn-mlb' AND GameId IN(90000001,90000002,90000003)",&[]).await?;Ok::<_,crate::Error>(())})?;
        let id = db.start_run()?;
        db.run_heartbeat(id)?;
        db.finish_run(id, "succeeded", None)?;
        db.check_health()?;
        Ok(())
    }
}

#[cfg(test)]
mod standings_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires migration credentials and isolated STANDINGS_TEST_DATABASE"]
    fn standings_upserts_rollover_and_failed_refresh_preserve_last_good() -> Result<()> {
        let name = std::env::var("STANDINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Standings_Validation_") {
            return Err("isolated standings database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.setup_tv(false)?;
        db.configure_standings()?;
        db.rt.block_on(async {
            db.client.simple_query("DELETE dbo.SportsStandings; UPDATE dbo.SportsStandingsSources SET Enabled=1; DROP TRIGGER IF EXISTS dbo.StandingsValidationFailure;").await?.into_results().await?;
            Ok::<_,crate::Error>(())
        })?;
        db.configure_standings()?;
        let sources = db.standings_sources()?;
        assert_eq!(sources.len(), 12);
        let source = sources.iter().find(|s| s.provider == "espn-nwsl").unwrap();
        let mut snapshot = crate::standings::tests::professional("nwsl");
        db.standings_attempt(source.id)?;
        db.save_standings(source, &snapshot)?;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,TeamId,Season,IsCurrent,StatsJson FROM dbo.SportsStandings WHERE SourceId=@P1 ORDER BY ID",&[&source.id]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        snapshot.title.push_str(" (updated display label)");
        snapshot.entries[0].position = 16;
        snapshot.entries[0].stats[0]["displayValue"] = serde_json::json!("28");
        db.save_standings(source, &snapshot)?;
        let after = read(&mut db)?;
        assert_eq!(after.len(), 16);
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.get::<i64, _>(0), b.get::<i64, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
        }
        let updated = after
            .iter()
            .find(|r| r.get::<&str, _>("TeamId") == Some(snapshot.entries[0].team_id.as_str()))
            .unwrap();
        let stats: serde_json::Value =
            serde_json::from_str(updated.get::<&str, _>("StatsJson").unwrap())?;
        assert_eq!(stats[0]["displayValue"], serde_json::json!("28"));
        let mut incomplete = snapshot.clone();
        incomplete.entries.pop();
        assert!(db.save_standings(source, &incomplete).is_err());
        db.standings_error(source.id, "fixture HTTP failure")?;
        assert!(
            read(&mut db)?
                .iter()
                .all(|r| r.get::<bool, _>("IsCurrent") == Some(true))
        );
        db.rt.block_on(async {db.client.simple_query("CREATE TRIGGER dbo.StandingsValidationFailure ON dbo.SportsStandings AFTER INSERT AS THROW 50000,'fixture write failure',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        snapshot.season = "2027".into();
        let failed = db.save_standings(source, &snapshot);
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.StandingsValidationFailure")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert!(failed.is_err());
        let rows = read(&mut db)?;
        assert_eq!(rows.len(), 16);
        assert!(
            rows.iter()
                .all(|r| r.get::<bool, _>("IsCurrent") == Some(true))
        );
        db.save_standings(source, &snapshot)?;
        let rows = read(&mut db)?;
        assert_eq!(rows.len(), 32);
        assert_eq!(
            rows.iter()
                .filter(|r| r.get::<bool, _>("IsCurrent") == Some(true))
                .count(),
            16
        );
        let epl_source = sources.iter().find(|s| s.provider == "espn-epl").unwrap();
        let mut epl_snapshot = crate::standings::tests::professional("epl");
        db.save_standings(epl_source, &epl_snapshot)?;
        epl_snapshot.season = "2027".into();
        db.save_standings(epl_source, &epl_snapshot)?;
        let epl_rows=db.rt.block_on(async {db.client.query("SELECT COUNT(*),SUM(CASE WHEN IsCurrent=0 THEN 1 ELSE 0 END) FROM dbo.SportsStandings WHERE SourceId=@P1",&[&epl_source.id]).await?.into_first_result().await})?;
        assert_eq!(epl_rows[0].get::<i32, _>(0), Some(20));
        assert_eq!(epl_rows[0].get::<i32, _>(1), Some(0));
        db.rt.block_on(async {
            let r=db.client.query("SELECT LatestSeason,LastTeamCount,LastError FROM dbo.SportsStandingsSources WHERE ID=@P1",&[&source.id]).await?.into_first_result().await?;
            assert_eq!(r[0].get::<&str,_>(0),Some("2027"));assert_eq!(r[0].get::<i32,_>(1),Some(16));assert!(r[0].get::<&str,_>(2).is_none());
            db.client.simple_query(include_str!("../sql/runtime_permissions.sql")).await?.into_results().await?;
            let p=db.client.query("EXECUTE AS USER='webscraper_ingest'; SELECT HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','SELECT'),HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','INSERT'),HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','UPDATE'),HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','DELETE'),HAS_PERMS_BY_NAME('dbo.SportsStandingsSources','OBJECT','UPDATE','Url','COLUMN'),HAS_PERMS_BY_NAME(DB_NAME(),'DATABASE','CREATE TABLE'); REVERT;",&[]).await?.into_first_result().await?;
            for i in 0..4 {assert_eq!(p[0].get::<i32,_>(i),Some(1));}
            for i in 4..6 {assert_eq!(p[0].get::<i32,_>(i),Some(0));}
            Ok::<_,crate::Error>(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod standings_live_tests {
    use super::*;
    #[test]
    #[ignore = "live network; isolated STANDINGS_TEST_DATABASE and runtime credentials required"]
    fn live_standings_all_sources_with_restricted_account() -> Result<()> {
        let name = std::env::var("STANDINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Standings_Validation_") {
            return Err("isolated standings database required".into());
        }
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        let sources = db.standings_sources()?;
        assert_eq!(sources.len(), 12);
        let client = crate::http::client_builder()
            .user_agent("My-Newsfeed/1.0")
            .timeout(Duration::from_secs(30))
            .build()?;
        let mut snapshots = vec![];
        for source in &sources {
            db.standings_attempt(source.id)?;
            let snapshot = crate::standings::fetch(&client, source)?;
            db.save_standings(source, &snapshot)?;
            eprintln!(
                "{} {}: {} teams, {}",
                source.provider,
                source.sport,
                snapshot.entries.len(),
                snapshot.season
            );
            snapshots.push(snapshot);
        }
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {
            db.client.query("SELECT ID,FirstSeenAt,SourceId,TeamId FROM dbo.SportsStandings WHERE IsCurrent=1 ORDER BY ID",&[]).await?.into_first_result().await
        })?)
        };
        let before = read(&mut db)?;
        assert_eq!(
            before.len(),
            snapshots.iter().map(|s| s.entries.len()).sum::<usize>()
        );
        for (source, snapshot) in sources.iter().zip(&snapshots) {
            db.save_standings(source, snapshot)?;
        }
        let after = read(&mut db)?;
        assert_eq!(before.len(), after.len());
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.get::<i64, _>(0), b.get::<i64, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod digital_movie_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires migration credentials and isolated DIGITAL_TEST_DATABASE"]
    fn digital_sources_preserve_identity_and_disable_settings() -> Result<()> {
        let name = std::env::var("DIGITAL_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Digital_Validation_") {
            return Err("isolated digital validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_movieinsider()?;
        db.rt.block_on(async {db.client.simple_query("BEGIN TRANSACTION; UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='movieinsider';").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        let result = (|| -> Result<()> {
            db.configure_movieinsider()?;
            assert!(
                !db.enabled_movie_sources()?
                    .iter()
                    .any(|s| s == "movieinsider")
            );
            let report = serde_json::json!({"releases":[{"title":"Digital Fixture","source":"movieinsider","release_type":"digital","release_date":"2026-10-09","source_url":"https://www.movieinsider.com/m900001/digital-fixture","format":"VOD / Digital"}],"issues":[]});
            let mut rows = crate::movies::release_rows(&report)?;
            assert_eq!(db.save_movie_releases(&rows, &report)?, 0);
            db.rt.block_on(async {db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='movieinsider'",&[]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_movieinsider()?;
            assert!(
                db.enabled_movie_sources()?
                    .iter()
                    .any(|s| s == "movieinsider")
            );
            assert_eq!(db.save_movie_releases(&rows, &report)?, 1);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT r.ID,r.FirstSeenAt,r.ReleaseDate,l.ExternalMovieId,r.ReleaseKey FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.SourceKey='movieinsider' AND l.ExternalMovieId='900001'",&[]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            rows[0].date = chrono::NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
            rows[0].url = "https://www.movieinsider.com/m900001/changed-slug".into();
            assert_eq!(db.save_movie_releases(&rows, &report)?, 1);
            assert_eq!(db.save_movie_releases(&rows, &report)?, 1);
            let after = read(&mut db)?;
            assert_eq!(after.len(), 1);
            assert_eq!(before[0].get::<i32, _>(0), after[0].get::<i32, _>(0));
            assert_eq!(
                before[0].get::<chrono::NaiveDateTime, _>(1),
                after[0].get::<chrono::NaiveDateTime, _>(1)
            );
            assert_eq!(after[0].get::<chrono::NaiveDate, _>(2), Some(rows[0].date));
            assert_eq!(after[0].get::<&str, _>(3), Some("900001"));
            assert_eq!(after[0].get::<&str, _>(4), Some("digital:first"));
            db.rt.block_on(async {db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='movieinsider'",&[]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_movieinsider()?;
            assert_eq!(db.save_movie_releases(&rows, &report)?, 0);
            assert_eq!(read(&mut db)?.len(), 1);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

#[cfg(test)]
mod ranking_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires migration credentials and isolated RANKINGS_TEST_DATABASE"]
    fn national_rankings_repeat_correct_advance_and_rollback_safely() -> Result<()> {
        let name = std::env::var("RANKINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Rankings_Validation_") {
            return Err("isolated rankings database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.setup_tv(false)?;
        db.configure_rankings()?;
        db.rt.block_on(async {db.client.simple_query("DELETE dbo.SportsRankingTeams; DELETE dbo.SportsRankingPolls; UPDATE dbo.SportsRankingSources SET Enabled=1; DROP TRIGGER IF EXISTS dbo.RankingValidationFailure;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        db.configure_rankings()?;
        let sources = db.ranking_sources()?;
        assert_eq!(sources.len(), 5);
        let source = sources.iter().find(|s| s.sport == "Football").unwrap();
        let (data, _) = crate::rankings::tests::fixture("football");
        let mut p = crate::rankings::parse(data, source, &crate::rankings::tests::members())?;
        db.ranking_attempt(source.id)?;
        db.save_ranking(source, &p)?;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT t.ID,t.FirstSeenAt,t.TeamId,t.Rank,t.IsBig12,p.ID AS PollId,p.PublishedAtUtc FROM dbo.SportsRankingTeams t JOIN dbo.SportsRankingPolls p ON p.ID=t.PollRowId WHERE p.SourceId=@P1 AND p.IsCurrent=1 AND t.IsCurrent=1 ORDER BY t.ID",&[&source.id]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        assert_eq!(before.len(), 25);
        p.teams.swap(0, 1);
        p.teams[0].rank = 1;
        p.teams[1].rank = 2;
        p.updated = p.updated.map(|t| t + chrono::Duration::minutes(1));
        db.save_ranking(source, &p)?;
        db.save_ranking(source, &p)?;
        let after = read(&mut db)?;
        let corrected = after
            .iter()
            .find(|r| r.get::<&str, _>("TeamId") == Some(p.teams[0].id.as_str()))
            .unwrap();
        assert_eq!(corrected.get::<i32, _>("Rank"), Some(1));
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.get::<i64, _>(0), b.get::<i64, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
            assert_eq!(a.get::<i64, _>("PollId"), b.get::<i64, _>("PollId"));
        }
        assert!(
            after
                .iter()
                .any(|r| r.get::<bool, _>("IsBig12") == Some(true))
        );
        assert!(
            after
                .iter()
                .any(|r| r.get::<bool, _>("IsBig12") == Some(false))
        );
        let mut incomplete = p.clone();
        incomplete.teams.pop();
        assert!(db.save_ranking(source, &incomplete).is_err());
        let mut older = p.clone();
        older.updated = older.updated.map(|t| t - chrono::Duration::minutes(2));
        assert!(db.save_ranking(source, &older).is_err());
        assert_eq!(read(&mut db)?.len(), 25);
        db.rt.block_on(async {db.client.simple_query("CREATE TRIGGER dbo.RankingValidationFailure ON dbo.SportsRankingTeams AFTER INSERT AS THROW 50000,'fixture insert failure',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        p.published += chrono::Duration::days(7);
        p.updated = p.updated.map(|t| t + chrono::Duration::days(7));
        let failure = db.save_ranking(source, &p);
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.RankingValidationFailure")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert!(failure.is_err());
        let unchanged = read(&mut db)?;
        assert_eq!(unchanged.len(), 25);
        assert_eq!(unchanged[0].get::<i64, _>(0), before[0].get::<i64, _>(0));
        db.save_ranking(source, &p)?;
        assert_ne!(
            read(&mut db)?[0].get::<i64, _>("PollId"),
            before[0].get::<i64, _>("PollId")
        );
        let obsolete = db.rt.block_on(async {
            db.client.query("SELECT COUNT(*) FROM dbo.SportsRankingPolls WHERE SourceId=@P1 AND IsCurrent=0", &[&source.id]).await?.into_first_result().await
        })?;
        assert_eq!(obsolete[0].get::<i32, _>(0), Some(0));
        let mut old = p.clone();
        old.published -= chrono::Duration::days(7);
        assert!(db.save_ranking(source, &old).is_err());
        db.rt.block_on(async {
            db.client
                .simple_query(include_str!("../sql/runtime_permissions.sql"))
                .await?
                .into_results()
                .await?;
            db.client
                .simple_query("DELETE dbo.SportsRankingTeams; DELETE dbo.SportsRankingPolls;")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }
    #[test]
    #[ignore = "live network; isolated RANKINGS_TEST_DATABASE with runtime credentials"]
    fn all_five_live_polls_import_with_restricted_account() -> Result<()> {
        let name = std::env::var("RANKINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Rankings_Validation_") {
            return Err("isolated rankings database required".into());
        }
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        let client = crate::http::client_builder()
            .user_agent("My-Newsfeed/1.0")
            .timeout(Duration::from_secs(30))
            .build()?;
        let members = crate::rankings::current_members(&client)?;
        // Run the isolated SQL setup test first; it clears its artificial polls.
        for source in db.ranking_sources()? {
            let p = crate::rankings::fetch(&client, &source, &members)?;
            db.ranking_attempt(source.id)?;
            db.save_ranking(&source, &p)?;
            eprintln!(
                "{}: {}, poll date {}, {} teams / {} Big 12",
                source.sport,
                p.season_label,
                p.published.date_naive(),
                p.teams.len(),
                p.teams
                    .iter()
                    .filter(|t| t.big12_school_id.is_some())
                    .count()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod championship_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires isolated CHAMPIONSHIP_TEST_DATABASE and migration credentials"]
    fn championship_results_repeat_correct_and_disable_safely() -> Result<()> {
        let name = std::env::var("CHAMPIONSHIP_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Championship_Validation_") {
            return Err("isolated Championship validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_championship()?;
        let source=db.rt.block_on(async {
            db.client.execute("UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-championship'",&[]).await?;
            db.client.query("SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-championship'",&[]).await?.into_first_result().await
        })?[0].get::<i32,_>(0).unwrap();
        let root = serde_json::from_str(include_str!(
            "../tests/fixtures/championship/october-2026.json"
        ))?;
        let (fixtures, results) = crate::championship::parse(&root, 2026)?;
        let mut batch = crate::championship::Batch {
            season: 2026,
            fixtures,
            results,
        };
        let id = batch.results[0]["gameId"].as_i64().unwrap() as i32;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,HomeScore FROM dbo.SportsMatchResults WHERE GameId=@P1",&[&id]).await?.into_first_result().await})?)
        };
        db.save_championship_results(source, &batch)?;
        let before = read(&mut db)?;
        db.save_championship_results(source, &batch)?;
        batch.results[0]["homeScore"] = serde_json::json!(7);
        db.save_championship_results(source, &batch)?;
        let after = read(&mut db)?;
        assert_eq!(before[0].get::<i64, _>(0), after[0].get::<i64, _>(0));
        assert_eq!(
            before[0].get::<chrono::NaiveDateTime, _>(1),
            after[0].get::<chrono::NaiveDateTime, _>(1)
        );
        assert_eq!(after[0].get::<i32, _>(2), Some(7));
        db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                    &[&source],
                )
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert!(db.save_championship_results(source, &batch).is_err());
        assert_eq!(read(&mut db)?[0].get::<i32, _>(2), Some(7));
        // The shared writer must preserve MLS shootouts without affecting Championship results.
        db.configure_mls()?;
        let mls_source = db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-mls'",
                    &[],
                )
                .await?;
            db.client
                .query(
                    "SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-mls'",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?[0]
            .get::<i32, _>(0)
            .unwrap();
        let playoff =
            serde_json::from_str(include_str!("../tests/fixtures/mls/playoffs-2025.json"))?;
        let (_, scores) = crate::mls::parse(&playoff, 2025)?;
        let penalty = scores
            .into_iter()
            .find(|r| r["status"] == "STATUS_FINAL_PEN")
            .unwrap();
        let penalty_id = penalty["gameId"].as_i64().unwrap() as i32;
        let rows = vec![penalty.clone()];
        db.save_match_results(mls_source, crate::espn_schedule::League::Mls, 2025, &rows)?;
        let read_penalty = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {
                db.client.query("SELECT ID,FirstSeenAt,HomeScore,AwayScore,HomeShootoutScore,AwayShootoutScore FROM dbo.SportsMatchResults WHERE ProviderKey='espn-mls' AND GameId=@P1",&[&penalty_id]).await?.into_first_result().await
            })?)
        };
        let original = read_penalty(&mut db)?;
        db.save_match_results(mls_source, crate::espn_schedule::League::Mls, 2025, &rows)?;
        let repeated = read_penalty(&mut db)?;
        assert_eq!(original.len(), 1);
        assert_eq!(original[0].get::<i64, _>(0), repeated[0].get::<i64, _>(0));
        assert_eq!(
            original[0].get::<chrono::NaiveDateTime, _>(1),
            repeated[0].get::<chrono::NaiveDateTime, _>(1)
        );
        for (column, key) in [
            (2, "homeScore"),
            (3, "awayScore"),
            (4, "homeShootout"),
            (5, "awayShootout"),
        ] {
            assert_eq!(
                repeated[0].get::<i32, _>(column),
                Some(penalty[key].as_i64().unwrap() as i32)
            );
        }
        db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                    &[&mls_source],
                )
                .await
        })?;
        assert!(
            db.save_match_results(mls_source, crate::espn_schedule::League::Mls, 2025, &rows)
                .is_err()
        );
        assert_eq!(
            read_penalty(&mut db)?[0].get::<i64, _>(0),
            original[0].get::<i64, _>(0)
        );
        assert_eq!(read(&mut db)?[0].get::<i32, _>(2), Some(7));
        Ok(())
    }
}

#[cfg(test)]
mod schedule_batch_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires isolated WNBA_TEST_DATABASE and migration credentials"]
    fn schedule_batches_rollback_across_chunks_and_preserve_ids() -> Result<()> {
        let name = std::env::var("WNBA_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Wnba_Validation") {
            return Err("isolated validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_wnba()?;
        let feed = db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-wnba'",
                    &[],
                )
                .await?;
            db.client
                .query(
                    "SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-wnba'",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?[0]
            .get::<i32, _>(0)
            .unwrap();
        let template = crate::espn_schedule::parse(
            &crate::wnba::tests::sample(),
            crate::espn_schedule::League::Wnba,
            None,
        )?
        .remove(0);
        let mut games = Vec::new();
        for n in 0..101 {
            let mut f = template.clone();
            let id = 90000000 + n;
            f.event.game_id = Some(id);
            f.event.url = format!("https://www.espn.com/wnba/game/_/gameId/{id}");
            f.event.event_date = Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive()
                + chrono::Days::new(1);
            f.completed = false;
            games.push(f);
        }
        db.rt.block_on(async {db.client.simple_query("CREATE TRIGGER dbo.BatchFailure ON dbo.SportsEvents AFTER INSERT AS IF EXISTS(SELECT 1 FROM inserted WHERE GameId=90000100) THROW 50000,'second chunk failure',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        assert!(db.save_espn_schedule(feed, &games).is_err());
        let count = db.rt.block_on(async {
            db.client
                .query(
                    "SELECT COUNT(*) FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1",
                    &[&feed],
                )
                .await?
                .into_first_result()
                .await
        })?;
        assert_eq!(count[0].get::<i32, _>(0), Some(0));
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.BatchFailure")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert_eq!(db.save_espn_schedule(feed, &games)?, 101);
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&feed]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        assert_eq!(db.save_espn_schedule(feed, &games)?, 101);
        let after = read(&mut db)?;
        for (a, b) in before.iter().zip(after.iter()) {
            assert_eq!(a.get::<i32, _>(0), b.get::<i32, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod nhl_sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires isolated NHL_TEST_DATABASE and migration credentials"]
    fn nhl_setup_results_and_disabled_source_are_safe() -> Result<()> {
        let name = std::env::var("NHL_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_NHL_Validation_") {
            return Err("isolated NHL validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_nhl()?;
        let source = db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-nhl'",
                    &[],
                )
                .await?;
            db.client
                .query(
                    "SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-nhl'",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?[0]
            .get::<i32, _>(0)
            .unwrap();
        db.configure_nhl()?;
        let enabled = db.rt.block_on(async {
            db.client
                .query(
                    "SELECT Enabled FROM dbo.SportsEventsType WHERE ID=@P1",
                    &[&source],
                )
                .await?
                .into_first_result()
                .await
        })?;
        assert_eq!(enabled[0].get::<bool, _>(0), Some(true));
        let root = serde_json::from_str(include_str!("../tests/fixtures/nhl/october-2026.json"))?;
        let (fixtures, mut results) = crate::nhl::parse(&root, 2027)?;
        let game_id = results[0]["gameId"].as_i64().unwrap() as i32;
        db.save_espn_schedule(source, &fixtures)?;
        db.save_match_results(source, crate::espn_schedule::League::Nhl, 2027, &results)?;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {
                db.client.query("SELECT ID,FirstSeenAt,HomeScore,ScheduleStatus FROM dbo.SportsMatchResults WHERE ProviderKey='espn-nhl' AND GameId=@P1", &[&game_id]).await?.into_first_result().await
            })?)
        };
        let original = read(&mut db)?;
        results[0]["homeScore"] = serde_json::json!(7);
        db.save_match_results(source, crate::espn_schedule::League::Nhl, 2027, &results)?;
        let corrected = read(&mut db)?;
        assert_eq!(original.len(), 1);
        assert_eq!(corrected.len(), 1);
        assert_eq!(original[0].get::<i64, _>(0), corrected[0].get::<i64, _>(0));
        assert_eq!(
            original[0].get::<chrono::NaiveDateTime, _>(1),
            corrected[0].get::<chrono::NaiveDateTime, _>(1)
        );
        assert_eq!(corrected[0].get::<i32, _>(2), Some(7));
        db.rt.block_on(async {
            db.client
                .execute(
                    "DELETE dbo.SportsEvents WHERE SportsEventsTypeId=@P1",
                    &[&source],
                )
                .await?;
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                    &[&source],
                )
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert_eq!(read(&mut db)?.len(), 1); // Schedule removal never removes results.
        assert!(
            db.save_match_results(source, crate::espn_schedule::League::Nhl, 2027, &results)
                .is_err()
        );
        assert_eq!(read(&mut db)?[0].get::<i32, _>(2), Some(7));
        Ok(())
    }
}

#[cfg(test)]
mod article_identity_sql_tests {
    use super::*;
    fn page(source: &str, url: &str) -> PageMeta {
        PageMeta {
            source: source.into(),
            url: url.into(),
            title: "same title".into(),
            description: Some("last-good description".into()),
            published: Some("2026-10-16T10:00:00Z".parse().unwrap()),
            image_url: Some("https://publisher.test/img/good.jpg".into()),
            identity_urls: vec![],
        }
    }
    fn scalar(db: &mut Db, sql: &str) -> Result<i32> {
        db.rt.block_on(async {
            Ok(
                db.client.query(sql, &[]).await?.into_first_result().await?[0]
                    .get::<i32, _>(0)
                    .unwrap(),
            )
        })
    }
    #[test]
    #[ignore = "requires isolated ARTICLE_IDENTITY_TEST_DATABASE and migration credentials"]
    fn identity_merge_idempotence_collision_references_and_concurrency() -> Result<()> {
        let name = std::env::var("ARTICLE_IDENTITY_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_ArticleIdentity_Validation_") {
            return Err("isolated article identity database required".into());
        }
        let ado = std::env::var("MSSQL_CONNECTION_STRING")?;
        let mut db = Db::open_named(&ado, &name)?;
        db.rt.block_on(async {db.client.simple_query("DROP TRIGGER IF EXISTS dbo.IdentityRollbackTest; DROP TABLE IF EXISTS dbo.IdentityTestSaved; DELETE dbo.ArticleIdRedirects; DELETE dbo.ArticleUrlAliases; DELETE dbo.Pages; DELETE dbo.Sources;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        let source = "https://publisher.test/";
        let old = "https://publisher.test/news/old";
        let new = "https://publisher.test/news/new";
        let old_page = page(source, old);
        db.save_pages(std::slice::from_ref(&old_page))?;
        let first = scalar(&mut db, "SELECT MIN(ID) FROM dbo.Pages")?;
        db.save_pages(std::slice::from_ref(&old_page))?;
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 1);
        // Content equality alone is never identity: same title/time/image, different URL.
        let mut incoming = page(source, new);
        db.save_pages(&[incoming.clone()])?;
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        incoming.identity_urls = vec![old.into(), new.into()];
        incoming.title = "new title".into();
        incoming.description = None;
        incoming.image_url = None;
        let preview = db.save_article(&incoming, true)?;
        assert_eq!(preview.1, Some(first));
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        let result = db.save_article(&incoming, false)?;
        assert_eq!(result.0, "merged");
        assert_eq!(result.1, Some(first));
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 1);
        assert_eq!(
            scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?,
            1
        );
        assert_eq!(
            scalar(
                &mut db,
                "SELECT COUNT(*) FROM dbo.Pages WHERE ImageUrl IS NOT NULL AND Description IS NOT NULL AND Published='2026-10-16T10:00:00'"
            )?,
            1
        );
        // A later unverified old URL must use its alias without reverting the canonical URL.
        db.save_pages(&[page(source, old)])?;
        assert_eq!(
            scalar(
                &mut db,
                "SELECT COUNT(*) FROM dbo.Pages WHERE Url='https://publisher.test/news/new'"
            )?,
            1
        );
        // Stale old-URL feed entries resolve to the survivor without changing its identity.
        let mut stale = old_page;
        stale.url = new.into();
        stale.identity_urls = vec![old.into(), new.into()];
        db.save_article(&stale, false)?;
        assert_eq!(scalar(&mut db, "SELECT MIN(ID) FROM dbo.Pages")?, first);
        db.save_article(&incoming, false)?;
        assert_eq!(
            scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?,
            1
        );
        let other = page("https://another.test/", new);
        assert!(
            db.save_article(&other, false)?
                .0
                .starts_with("skipped-cross-source")
        );
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 1);
        // Unknown foreign references block retirement rather than guessing a rewrite policy.
        let third = "https://publisher.test/news/third";
        let third_page = page(source, third);
        db.save_pages(&[third_page])?;
        db.rt.block_on(async {
            db.client
                .simple_query(
                    "CREATE TABLE dbo.IdentityTestSaved(PageId INT REFERENCES dbo.Pages(ID))",
                )
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        incoming.identity_urls.push(third.into());
        assert!(
            db.save_article(&incoming, false)?
                .0
                .starts_with("skipped-unhandled")
        );
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TABLE dbo.IdentityTestSaved")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.save_article(&incoming, false)?;
        let fourth = "https://publisher.test/news/fourth";
        db.save_pages(&[page(source, fourth)])?;
        incoming.identity_urls.push(fourth.into());
        let before = scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?;
        db.rt.block_on(async {db.client.simple_query("CREATE OR ALTER TRIGGER dbo.IdentityRollbackTest ON dbo.Pages AFTER UPDATE AS THROW 50000,'injected rollback test',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        assert!(db.save_article(&incoming, false).is_err());
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        assert_eq!(
            scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?,
            before
        );
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.IdentityRollbackTest")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.save_article(&incoming, false)?;
        // Separate sessions racing on a single URL converge under the global hash lock.
        let mut workers = vec![];
        for _ in 0..4 {
            let ado = ado.clone();
            let name = name.clone();
            workers.push(std::thread::spawn(move || -> Result<(), String> {
                let mut db = Db::connect_named(&ado, &name).map_err(|e| e.to_string())?;
                db.save_pages(&[page(
                    "https://publisher.test/",
                    "https://publisher.test/news/race",
                )])
                .map_err(|e| e.to_string())?;
                Ok(())
            }));
        }
        for worker in workers {
            worker
                .join()
                .unwrap()
                .map_err(|e| format!("concurrent ingestion: {e}"))?;
        }
        assert_eq!(
            scalar(
                &mut db,
                "SELECT COUNT(*) FROM dbo.Pages WHERE Url='https://publisher.test/news/race'"
            )?,
            1
        );
        // Deterministic HTTP + database integration: the new row is discovered first,
        // then revisiting the old URL confirms the redirect and preserves the oldest ID.
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let base = format!("http://publisher.test:{}", address.port());
        let server = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let n = socket.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..n]);
                let response = if request.starts_with("GET /news/previous ") {
                    "HTTP/1.1 301 Moved\r\nLocation: /news/current\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
                } else {
                    let body =
                        "<title>Current article</title><meta property='og:type' content='article'>";
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        let test_source = format!("{base}/");
        let previous = format!("{base}/news/previous");
        let current = format!("{base}/news/current");
        let old = page(&test_source, &previous);
        let new = page(&test_source, &current);
        let oldest = db.save_article(&old, false)?.1;
        db.save_pages(std::slice::from_ref(&new))?;
        let mut found = vec![new];
        let recent = db.recent_articles(&test_source)?;
        // Keep just the old URL as the reconciliation job; the incoming new URL
        // supplies the other request, and its redirect adds the third request.
        let recent = recent.into_iter().filter(|p| p.url == previous).collect();
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .resolve("publisher.test", address)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()?;
        crate::pages::enrich_article_identities(
            &client,
            &mut found,
            recent,
            std::time::Instant::now(),
            Some(&mut db),
            false,
        )?;
        db.save_pages(&found)?;
        assert_eq!(db.save_article(&found[0], true)?.1, oldest);
        server.join().unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod movie_commerce_sql_tests {
    use super::*;
    use crate::movie_commerce::{Outcome, Provider};
    #[test]
    #[ignore = "requires isolated MOVIE_COMMERCE_TEST_DATABASE and migration credentials"]
    fn movie_commerce_cache_preserves_identity_and_expires() -> Result<()> {
        let name = std::env::var("MOVIE_COMMERCE_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_MovieCommerce_Validation_") {
            return Err("isolated movie commerce database required".into());
        }
        let profile = std::env::var("MOVIE_COMMERCE_TEST_PROFILE")?;
        dotenvy::from_filename(profile)?;
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NULL CREATE ROLE WebScraperIngest; IF USER_ID('webscraper_ingest') IS NULL CREATE USER webscraper_ingest FOR LOGIN webscraper_ingest; IF NOT EXISTS(SELECT 1 FROM sys.database_role_members WHERE role_principal_id=DATABASE_PRINCIPAL_ID('WebScraperIngest') AND member_principal_id=USER_ID('webscraper_ingest')) ALTER ROLE WebScraperIngest ADD MEMBER webscraper_ingest; GRANT SELECT,INSERT,UPDATE ON dbo.MovieReleaseSources TO WebScraperIngest; GRANT SELECT,INSERT,UPDATE ON dbo.Movies TO WebScraperIngest; GRANT SELECT,INSERT,UPDATE ON dbo.MovieSourceLinks TO WebScraperIngest; GRANT SELECT,INSERT,UPDATE ON dbo.MovieReleases TO WebScraperIngest;")
                .await?
                .into_results()
                .await?;
            db.client.simple_query(include_str!("../sql/movie_commerce_permissions.sql")).await?.into_results().await?;
            db.client
                .simple_query("EXECUTE AS USER='webscraper_ingest';")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let report = serde_json::json!({"releases":[],"issues":[]});
        let row = crate::movies::MovieRelease {
            source: "dvdsreleasedates".into(),
            external_id: "commerce-validation".into(),
            title: "A & B: Commerce Fixture".into(),
            imdb_id: None,
            key: "theatrical:first".into(),
            kind: "theatrical".into(),
            date: chrono::Utc::now().date_naive() + chrono::Duration::days(1),
            url: "https://www.dvdsreleasedates.com/movies/999999/commerce-validation".into(),
            format: None,
            platform: None,
        };
        db.save_movie_releases(&[row], &report)?;
        let job = db
            .movie_commerce_jobs()?
            .into_iter()
            .find(|j| j.identity.title == "A & B: Commerce Fixture")
            .ok_or("missing commerce job")?;
        assert_eq!(job.provider, Provider::FandangoTickets);
        db.seed_movie_commerce(&job)?;
        db.seed_movie_commerce(&job)?;
        let read = |db: &mut Db| -> Result<(String, String)> {
            db.rt.block_on(async {
            let rows=db.client.query("SELECT LinkKind,ActionUrl FROM dbo.MovieCommerce WHERE MovieId=@P1 AND Provider='fandango'",&[&job.movie_id]).await?.into_first_result().await?;
            Ok((rows[0].get::<&str,_>(0).unwrap().into(),rows[0].get::<&str,_>(1).unwrap().into()))
        })
        };
        assert_eq!(read(&mut db)?.0, "search");
        assert_eq!(url::Url::parse(&read(&mut db)?.1)?.query_pairs().count(), 1);
        db.save_movie_commerce(
            &job,
            &Outcome {
                direct_url: Some("https://www.fandango.com/fixture-123/movie-overview".into()),
                purchase: false,
                error: None,
                ..Default::default()
            },
        )?;
        assert_eq!(read(&mut db)?.0, "direct");
        db.save_movie_commerce(
            &job,
            &Outcome {
                error: Some("temporary HTTP error".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(read(&mut db)?.0, "direct");
        let cap = chrono::Utc::now().naive_utc() + chrono::Duration::hours(2);
        db.save_movie_commerce(
            &job,
            &Outcome {
                direct_url: Some("https://www.fandango.com/fixture-123/movie-overview".into()),
                reviewed_until: Some(cap),
                ..Default::default()
            },
        )?;
        let capped=db.rt.block_on(async {db.client.query("SELECT VerifiedUntil,RetryAfter FROM dbo.MovieCommerceLinks WHERE MovieId=@P1 AND Provider='fandango'",&[&job.movie_id]).await?.into_first_result().await})?;
        assert!(capped[0].get::<chrono::NaiveDateTime, _>(0).unwrap() <= cap);
        assert!(capped[0].get::<chrono::NaiveDateTime, _>(1).unwrap() <= cap);
        db.rt.block_on(async {db.client.execute("UPDATE dbo.MovieCommerceLinks SET VerifiedUntil=DATEADD(day,-1,SYSUTCDATETIME()) WHERE MovieId=@P1",&[&job.movie_id]).await?;Ok::<_,crate::Error>(())})?;
        assert_eq!(read(&mut db)?.0, "search");
        db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.Movies SET OriginalYear=2016 WHERE ID=@P1",
                    &[&job.movie_id],
                )
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.save_movie_commerce(
            &job,
            &Outcome {
                direct_url: Some("https://www.fandango.com/fixture-123/movie-overview".into()),
                ..Default::default()
            },
        )?;
        let changed = db
            .movie_commerce_jobs()?
            .into_iter()
            .find(|j| j.movie_id == job.movie_id)
            .unwrap();
        assert!(changed.due);
        db.seed_movie_commerce(&changed)?;
        assert_eq!(read(&mut db)?.0, "search");
        db.rt.block_on(async {
            db.client
                .simple_query("REVERT;")
                .await?
                .into_results()
                .await?;
            db.client
                .simple_query(include_str!("../tests/sql/movie_commerce.sql"))
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod f1_sql_tests {
    use super::*;
    use serde_json::Value;
    #[test]
    #[ignore = "requires migration credentials, isolated F1_TEST_DATABASE and F1_TEST_PAYLOAD"]
    fn f1_views_atomic_refresh_disable_and_regression() -> Result<()> {
        let name = std::env::var("F1_TEST_DATABASE")?;
        if !name.starts_with("F1_Test_") {
            return Err("F1 test requires isolated F1_Test_ database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_f1()?;
        let payload: Value =
            serde_json::from_str(&std::fs::read_to_string(std::env::var("F1_TEST_PAYLOAD")?)?)?;
        let rows = |key: &str| payload[key].as_array().unwrap().clone();
        let mut snapshot = crate::f1::Snapshot {
            season: 2026,
            standings_round: 17,
            races: rows("races"),
            results: rows("results"),
            sprint: rows("sprint"),
            drivers: rows("drivers"),
            constructors: rows("constructors"),
        };
        assert!(!db.f1_enabled()?);
        assert!(db.save_f1(&snapshot).is_err());
        db.rt.block_on(async {
            db.client
                .simple_query("UPDATE dbo.F1Source SET Enabled=1 WHERE ID=1")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.configure_f1()?; // Repeat migration preserves Enabled and existing snapshots.
        assert!(db.f1_enabled()?);
        db.save_f1(&snapshot)?;
        db.save_f1(&snapshot)?;
        fn count(db: &mut Db, table: &str) -> Result<i32> {
            Ok(db.rt.block_on(async {
                db.client
                    .query(format!("SELECT COUNT(*) FROM dbo.{table}"), &[])
                    .await?
                    .into_first_result()
                    .await
            })?[0]
                .get::<i32, _>(0)
                .unwrap())
        }
        assert_eq!(count(&mut db, "F1Schedule")?, snapshot.races.len() as i32);
        assert_eq!(
            count(&mut db, "F1Results")?,
            (snapshot.results.len() + snapshot.sprint.len()) as i32
        );
        assert_eq!(
            count(&mut db, "F1DriverStandings")?,
            snapshot.drivers.len() as i32
        );
        assert_eq!(
            count(&mut db, "F1ConstructorStandings")?,
            snapshot.constructors.len() as i32
        );
        snapshot.standings_round = 16;
        assert!(db.save_f1(&snapshot).is_err());
        snapshot.standings_round = 17;
        snapshot.races[0]["date"] = serde_json::json!("bad");
        assert!(db.save_f1(&snapshot).is_err());
        assert_eq!(count(&mut db, "F1Schedule")?, snapshot.races.len() as i32);
        db.rt.block_on(async {
            db.client
                .simple_query("UPDATE dbo.F1Source SET Enabled=0 WHERE ID=1")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert_eq!(count(&mut db, "F1Schedule")?, 0);
        assert_eq!(count(&mut db, "F1DriverStandings")?, 0);
        assert!(db.save_f1(&snapshot).is_err());
        assert_eq!(count(&mut db, "F1Snapshot")?, 1);
        Ok(())
    }
}
