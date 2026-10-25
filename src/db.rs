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

    const CONNECT_NAMED_SQL: &str = "SELECT CASE WHEN OBJECT_ID('dbo.ScraperRuns','U') IS NOT NULL AND COL_LENGTH('dbo.SportsEvents','ScheduleStatus') IS NOT NULL AND OBJECT_ID('dbo.UpsertArticleIdentity','P') IS NOT NULL THEN 1 ELSE 0 END";
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
        rt.block_on(bounded(
            async {
                let r = client
                    .query(Self::CONNECT_NAMED_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                if r.first().and_then(|r| r.get::<i32, _>(0)) != Some(1) {
                    return Err(
                        "schema missing: apply scraper migrations with migration credentials"
                            .into(),
                    );
                }
                Ok(())
            },
            &poisoned,
        ))?;
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
            for migration in [
                include_str!("../sql/movie_releases.sql"),
                include_str!("../sql/movie_posters.sql"),
                include_str!("../sql/movie_commerce.sql"),
                include_str!("../sql/movie_commerce_view.sql"),
                include_str!("../sql/big12_events.sql"),
                include_str!("../sql/nwsl_events.sql"),
                include_str!("../sql/rankings.sql"),
                include_str!("../sql/championship.sql"),
                include_str!("../sql/soccer_shootouts.sql"),
                include_str!("../sql/standings.sql"),
                include_str!("../sql/scraper_runs.sql"),
                include_str!("../sql/article_identity.sql"),
                include_str!("../sql/upsert_article_identity.sql"),
                include_str!("../sql/big12_views.sql"),
            ] {
                client.simple_query(migration).await?.into_results().await?;
            }
            Result::Ok(client)
        },&poisoned))?;
        Ok(Db {
            rt,
            client,
            poisoned,
        })
    }

    const RETRY_MISSING_POSTERS_SQL: &str = "UPDATE dbo.Movies SET PosterRetryAfter=NULL WHERE PosterUrl IS NULL; UPDATE c SET RetryAfter=SYSUTCDATETIME() FROM dbo.MoviePosterCache c JOIN dbo.MovieSourceLinks l ON l.ID=c.SourceLinkId JOIN dbo.Movies m ON m.ID=l.MovieId WHERE m.PosterUrl IS NULL";
    pub fn retry_missing_posters(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::RETRY_MISSING_POSTERS_SQL, &[])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    const POSTER_JOBS_SQL: &str = "SELECT m.ID,m.Title,m.OriginalYear,m.ImdbId,m.TmdbId,m.PosterUrl,m.PosterCheckedAt,m.PosterRetryAfter,m.PosterLookupStatus,l.ID AS LinkId,l.Url,s.SourceKey,c.TargetIdentity,c.Metadata,c.PosterUrl AS CachePoster,c.RetryAfter,c.LastError FROM dbo.Movies m JOIN dbo.MovieSourceLinks l ON l.MovieId=m.ID JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId LEFT JOIN dbo.MoviePosterCache c ON c.SourceLinkId=l.ID WHERE s.Enabled=1 ORDER BY m.ID,CASE s.SourceKey WHEN 'dvdsreleasedates' THEN 0 ELSE 1 END,l.ID";
    pub fn poster_jobs(&mut self) -> Result<Vec<crate::posters::PosterJob>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::POSTER_JOBS_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .map(|r| {
                        let (title, suffix_year) =
                            crate::movie_metadata::title_year(r.get::<&str, _>("Title").unwrap());
                        crate::posters::PosterJob {
                            movie_id: r.get::<i32, _>("ID").unwrap(),
                            link_id: r.get::<i32, _>("LinkId").unwrap(),
                            source: r.get::<&str, _>("SourceKey").unwrap().to_owned(),
                            url: r.get::<&str, _>("Url").unwrap().to_owned(),
                            identity: crate::movie_metadata::MovieIdentity {
                                title,
                                year: r.get::<i16, _>("OriginalYear").or(suffix_year),
                                imdb_id: r.get::<&str, _>("ImdbId").map(str::to_owned),
                                tmdb_id: r.get::<i32, _>("TmdbId"),
                            },
                            existing: r.get::<&str, _>("PosterUrl").map(str::to_owned),
                            poster_status: r
                                .get::<&str, _>("PosterLookupStatus")
                                .map(str::to_owned),
                            checked: r.get::<chrono::NaiveDateTime, _>("PosterCheckedAt"),
                            retry: r.get::<chrono::NaiveDateTime, _>("PosterRetryAfter"),
                            cache_identity: r.get::<&str, _>("TargetIdentity").map(str::to_owned),
                            cache_metadata: r.get::<&str, _>("Metadata").map(str::to_owned),
                            cache_poster: r.get::<&str, _>("CachePoster").map(str::to_owned),
                            cache_retry: r.get::<chrono::NaiveDateTime, _>("RetryAfter"),
                            cache_error: r.get::<&str, _>("LastError").map(str::to_owned),
                        }
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }

    const CACHE_POSTER_SOURCE_SQL: &str = "SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
             IF EXISTS(SELECT 1 FROM dbo.MoviePosterCache WITH(UPDLOCK,HOLDLOCK) WHERE SourceLinkId=@P1)
              UPDATE dbo.MoviePosterCache SET TargetIdentity=@P2,Metadata=@P3,PosterUrl=@P4,LastError=@P5,CheckedAt=SYSUTCDATETIME(),RetryAfter=CASE WHEN @P5 IS NOT NULL THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,CASE WHEN @P4 IS NULL THEN 7 ELSE 30 END,SYSUTCDATETIME()) END WHERE SourceLinkId=@P1;
             ELSE INSERT dbo.MoviePosterCache(SourceLinkId,TargetIdentity,Metadata,PosterUrl,LastError,CheckedAt,RetryAfter) VALUES(@P1,@P2,@P3,@P4,@P5,SYSUTCDATETIME(),CASE WHEN @P5 IS NOT NULL THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,CASE WHEN @P4 IS NULL THEN 7 ELSE 30 END,SYSUTCDATETIME()) END);
             COMMIT TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT>0 ROLLBACK TRANSACTION; THROW; END CATCH";
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
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::CACHE_POSTER_SOURCE_SQL,
                        &[&link, &identity, &metadata, &poster, &error],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    const SET_MOVIE_POSTER_SQL: &str = "UPDATE dbo.Movies SET PosterUrl=@P3,OriginalYear=COALESCE(OriginalYear,@P4),PosterCheckedAt=SYSUTCDATETIME(),PosterRetryAfter=CASE WHEN @P8=0 THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,@P8,SYSUTCDATETIME()) END,PosterLookupStatus=@P5,PosterSourceUrl=COALESCE(@P6,PosterSourceUrl),PosterLookupError=@P7 WHERE ID=@P1 AND (PosterUrl=@P2 OR (PosterUrl IS NULL AND @P2 IS NULL)) AND (OriginalYear IS NULL OR @P4 IS NULL OR OriginalYear=@P4)";
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
        self.rt.block_on(bounded(
            async {
                let updated = self
                    .client
                    .execute(
                        Self::SET_MOVIE_POSTER_SQL,
                        &[
                            &id, &expected, &poster, &year, &status, &source, &error, &days,
                        ],
                    )
                    .await?;
                Ok(updated.total() > 0)
            },
            &self.poisoned,
        ))
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
    const TV_JOBS_SQL: &str =
        "SELECT TvmazeShowId,ExpectedTitle FROM dbo.TvTrackedSeries WHERE Enabled=1 ORDER BY ID";
    pub fn tv_jobs(&mut self) -> Result<Vec<(i32, String)>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::TV_JOBS_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .map(|r| {
                        (
                            r.get::<i32, _>(0).unwrap(),
                            r.get::<&str, _>(1).unwrap().to_owned(),
                        )
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }
    const CLAIM_TV_SERIES_SQL: &str = "UPDATE dbo.TvTrackedSeries WITH(UPDLOCK,ROWLOCK) SET LastAttemptAt=SYSUTCDATETIME(),LeaseUntil=DATEADD(minute,10,SYSUTCDATETIME()),LeaseToken=NEWID() OUTPUT CONVERT(VARCHAR(36),inserted.LeaseToken) WHERE TvmazeShowId=@P1 AND Enabled=1 AND (LeaseUntil IS NULL OR LeaseUntil<=SYSUTCDATETIME()) AND (@P2=1 OR ((CacheExpiresAt IS NULL OR CacheExpiresAt<=SYSUTCDATETIME()) AND (RetryAfter IS NULL OR RetryAfter<=SYSUTCDATETIME())))";
    pub fn claim_tv_series(&mut self, id: i32, force: bool) -> Result<Option<String>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::CLAIM_TV_SERIES_SQL, &[&id, &force])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .first()
                    .and_then(|r| r.get::<&str, _>(0))
                    .map(str::to_owned))
            },
            &self.poisoned,
        ))
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
    const FAIL_TV_SERIES_SQL: &str = "UPDATE dbo.TvTrackedSeries SET LastError=@P3,LastIssueCount=1,RetryAfter=DATEADD(second,@P4,SYSUTCDATETIME()),LeaseUntil=NULL,LeaseToken=NULL WHERE TvmazeShowId=@P1 AND LeaseToken=TRY_CONVERT(UNIQUEIDENTIFIER,@P2)";
    pub fn fail_tv_series(&mut self, id: i32, lease: &str, error: &str, retry: i32) -> Result<()> {
        let error = error.chars().take(1000).collect::<String>();
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::FAIL_TV_SERIES_SQL, &[&id, &lease, &error, &retry])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    const MOVIE_COMMERCE_JOBS_SQL: &str = r#"
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
"#;
    pub fn movie_commerce_jobs(&mut self) -> Result<Vec<crate::movie_commerce::Job>> {
        use crate::movie_commerce::{Job, Provider};
        self.rt.block_on(bounded(
            async {
                let available = self
                    .client
                    .query("SELECT OBJECT_ID('dbo.MovieCommerceLinks','U') AS Id", &[])
                    .await?
                    .into_first_result()
                    .await?;
                if available.first().and_then(|r| r.get::<i32, _>(0)).is_none() {
                    eprintln!(
                        "movie retailer links: migration not applied; see sql/movie_commerce.sql"
                    );
                    return Ok(vec![]);
                }
                let rows = self
                    .client
                    .query(Self::MOVIE_COMMERCE_JOBS_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                let mut jobs: Vec<Job> = vec![];
                for r in rows {
                    let movie_id = r.get::<i32, _>("ID").ok_or("missing movie ID")?;
                    let provider = Provider::from_key(r.get::<&str, _>("Provider").unwrap_or(""))
                        .ok_or("invalid retailer provider")?;
                    let source = r
                        .get::<&str, _>("Url")
                        .ok_or("missing movie source URL")?
                        .to_owned();
                    if let Some(j) = jobs
                        .iter_mut()
                        .find(|j| j.movie_id == movie_id && j.provider == provider)
                    {
                        j.source_urls.push(source);
                        continue;
                    }
                    jobs.push(Job {
                        movie_id,
                        provider,
                        source_urls: vec![source],
                        due: r.get::<i32, _>("Due") == Some(1),
                        cached_direct: r.get::<&str, _>("DirectUrl").map(str::to_owned),
                        verified_until: r.get::<chrono::NaiveDateTime, _>("VerifiedUntil"),
                        identity: crate::movie_metadata::MovieIdentity {
                            title: r.get::<&str, _>("Title").unwrap_or("").to_owned(),
                            year: r.get::<i16, _>("OriginalYear"),
                            imdb_id: r.get::<&str, _>("ImdbId").map(str::to_owned),
                            tmdb_id: None,
                        },
                    });
                }
                Ok(jobs)
            },
            &self.poisoned,
        ))
    }
    const SEED_MOVIE_COMMERCE_SQL: &str = r#"
MERGE dbo.MovieCommerceLinks WITH(HOLDLOCK) t USING(SELECT @P1 AS MovieId,@P2 AS Provider) s
ON t.MovieId=s.MovieId AND t.Provider=s.Provider
WHEN MATCHED AND (t.IdentityTitle COLLATE Latin1_General_100_BIN2<>@P3 COLLATE Latin1_General_100_BIN2 OR ISNULL(t.IdentityYear,0)<>ISNULL(@P4,0) OR ISNULL(t.IdentityImdbId,'')<>ISNULL(@P5,''))
THEN UPDATE SET IdentityTitle=@P3,IdentityYear=@P4,IdentityImdbId=@P5,SearchUrl=@P6,DirectUrl=NULL,PurchaseVerified=0,VerifiedUntil=NULL,CheckedAt=NULL,RetryAfter=NULL,LastError=NULL
WHEN NOT MATCHED THEN INSERT(MovieId,Provider,IdentityTitle,IdentityYear,IdentityImdbId,SearchUrl) VALUES(@P1,@P2,@P3,@P4,@P5,@P6);
"#;
    pub fn seed_movie_commerce(&mut self, job: &crate::movie_commerce::Job) -> Result<()> {
        let provider = job.provider.key();
        let search = job.provider.search(&job.identity.title);
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::SEED_MOVIE_COMMERCE_SQL,
                        &[
                            &job.movie_id,
                            &provider,
                            &job.identity.title,
                            &job.identity.year,
                            &job.identity.imdb_id,
                            &search,
                        ],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    const SAVE_MOVIE_COMMERCE_SQL: &str = r#"
UPDATE c SET DirectUrl=CASE WHEN @P8 IS NOT NULL THEN c.DirectUrl ELSE @P6 END,
 PurchaseVerified=CASE WHEN @P8 IS NOT NULL THEN c.PurchaseVerified ELSE @P7 END,
 VerifiedUntil=CASE WHEN @P8 IS NOT NULL THEN c.VerifiedUntil WHEN @P6 IS NOT NULL THEN CASE WHEN @P9<DATEADD(day,7,SYSUTCDATETIME()) THEN @P9 ELSE DATEADD(day,7,SYSUTCDATETIME()) END ELSE NULL END,
 CheckedAt=SYSUTCDATETIME(),RetryAfter=CASE WHEN @P9<DATEADD(hour,CASE WHEN @P6 IS NOT NULL THEN 168 ELSE 6 END,SYSUTCDATETIME()) THEN @P9 ELSE DATEADD(hour,CASE WHEN @P6 IS NOT NULL THEN 168 ELSE 6 END,SYSUTCDATETIME()) END,LastError=@P8
FROM dbo.MovieCommerceLinks c JOIN dbo.Movies m ON m.ID=c.MovieId
WHERE c.MovieId=@P1 AND c.Provider=@P2 AND c.IdentityTitle COLLATE Latin1_General_100_BIN2=@P3 COLLATE Latin1_General_100_BIN2
AND ISNULL(c.IdentityYear,0)=ISNULL(@P4,0) AND ISNULL(c.IdentityImdbId,'')=ISNULL(@P5,'')
AND m.Title COLLATE Latin1_General_100_BIN2=c.IdentityTitle COLLATE Latin1_General_100_BIN2
AND ISNULL(m.OriginalYear,0)=ISNULL(@P4,0) AND ISNULL(m.ImdbId,'')=ISNULL(@P5,'');
"#;
    pub fn save_movie_commerce(
        &mut self,
        job: &crate::movie_commerce::Job,
        outcome: &crate::movie_commerce::Outcome,
    ) -> Result<()> {
        let provider = job.provider.key();
        self.rt.block_on(bounded(
            async {
                // Errors preserve a previously verified link only until its original expiry.
                self.client
                    .execute(
                        Self::SAVE_MOVIE_COMMERCE_SQL,
                        &[
                            &job.movie_id,
                            &provider,
                            &job.identity.title,
                            &job.identity.year,
                            &job.identity.imdb_id,
                            &outcome.direct_url,
                            &outcome.purchase,
                            &outcome.error,
                            &outcome.reviewed_until,
                        ],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
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

    const TRACKED_MOVIE_URLS_SQL: &str = "SELECT DISTINCT l.Url FROM dbo.MovieSourceLinks l JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.Enabled=1 AND EXISTS(SELECT 1 FROM dbo.MovieReleases r WHERE r.MovieSourceLinkId=l.ID AND r.ReleaseStatus='scheduled' AND (s.CollectDisc=1 OR r.ReleaseType<>'disc'))";
    pub fn tracked_movie_urls(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::TRACKED_MOVIE_URLS_SQL, &[])
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

    const SAVE_MOVIE_RELEASES_SQL_1: &str = "SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
                     IF NOT EXISTS (SELECT 1 FROM dbo.MovieReleaseSources WITH (UPDLOCK, HOLDLOCK) WHERE SourceKey=@P1)
                     INSERT dbo.MovieReleaseSources(SourceKey,Name,Url,Enabled) VALUES(@P1,@P2,@P3,1);
                     COMMIT TRANSACTION; END TRY BEGIN CATCH
                     IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION; THROW; END CATCH";
    const SAVE_MOVIE_RELEASES_SQL_2: &str = "UPDATE dbo.MovieReleaseSources SET LastScrapedAt=SYSUTCDATETIME(), LastError=@P2 WHERE SourceKey=@P1 AND Enabled=1";
    /// Upsert stable source/release identities; never delete absent calendar entries.
    pub fn save_movie_releases(
        &mut self,
        rows: &[crate::movies::MovieRelease],
        report: &serde_json::Value,
    ) -> Result<usize> {
        self.rt.block_on(bounded(
            async {
                let mut written = 0;
                for (key, name, url) in [
                    (
                        "dvdsreleasedates",
                        "DVDs Release Dates",
                        "https://www.dvdsreleasedates.com/",
                    ),
                    (
                        "whentostream",
                        "When To Stream",
                        "https://whentostream.com/",
                    ),
                ] {
                    self.client
                        .execute(Self::SAVE_MOVIE_RELEASES_SQL_1, &[&key, &name, &url])
                        .await?;
                }
                for r in rows {
                    let result = self
                        .client
                        .query(
                            include_str!("../sql/upsert_movie_release.sql"),
                            &[
                                &r.source,
                                &r.external_id,
                                &r.title,
                                &r.imdb_id,
                                &r.key,
                                &r.kind,
                                &r.date,
                                &r.url,
                                &r.format,
                                &r.platform,
                            ],
                        )
                        .await?
                        .into_first_result()
                        .await?;
                    written += result
                        .first()
                        .and_then(|r| r.get::<i32, _>("Written"))
                        .unwrap_or(0) as usize;
                }
                for (source, host) in [
                    ("dvdsreleasedates", "dvdsreleasedates.com"),
                    ("whentostream", "whentostream.com"),
                    ("movieinsider", "movieinsider.com"),
                ] {
                    let errors: Vec<_> = report["issues"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|r| {
                            r["source"] == source
                                || r["url"].as_str().is_some_and(|u| {
                                    url::Url::parse(u)
                                        .ok()
                                        .and_then(|u| u.host_str().map(str::to_owned))
                                        .is_some_and(|h| h == host || h == format!("www.{host}"))
                                })
                        })
                        .map(|r| r["reason"].as_str().unwrap_or("unknown issue"))
                        .collect();
                    let error = if errors.is_empty() {
                        None
                    } else {
                        Some(errors.join("; ").chars().take(1000).collect::<String>())
                    };
                    self.client
                        .execute(Self::SAVE_MOVIE_RELEASES_SQL_2, &[&source, &error])
                        .await?;
                }
                Ok(written)
            },
            &self.poisoned,
        ))
    }

    const CHECK_HEALTH_SQL: &str = "SELECT TOP(1) Status,DATEDIFF(minute,HeartbeatAt,SYSUTCDATETIME()) AS Age FROM dbo.ScraperRuns ORDER BY ID DESC";
    /// URLs of all enabled sources, oldest first.
    pub fn check_health(&mut self) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                let r = self
                    .client
                    .query(Self::CHECK_HEALTH_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                let r = r
                    .first()
                    .ok_or("collector has not completed its first run")?;
                if r.get::<i32, _>("Age").is_none_or(|age| age > 90)
                    || r.get::<&str, _>("Status") == Some("failed")
                {
                    return Err("collector progress unhealthy or latest run failed".into());
                }
                Ok(())
            },
            &self.poisoned,
        ))
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

    const FINISH_RUN_SQL_1: &str = "UPDATE dbo.ScraperRuns SET HeartbeatAt=SYSUTCDATETIME(),FinishedAt=SYSUTCDATETIME(),Status=@P2,ErrorSummary=@P3 WHERE ID=@P1";
    const FINISH_RUN_SQL_2: &str = "DELETE dbo.ScraperRuns WHERE FinishedAt IS NOT NULL AND StartedAt<DATEADD(day,-7,SYSUTCDATETIME())";
    pub fn finish_run(&mut self, id: i32, status: &str, error: Option<&str>) -> Result<()> {
        let error = error.map(|s| utf16_prefix(s, 1000));
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::FINISH_RUN_SQL_1, &[&id, &status, &error])
                    .await?;
                self.client.execute(Self::FINISH_RUN_SQL_2, &[]).await?;
                Ok(())
            },
            &self.poisoned,
        ))
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

    const ADD_SOURCES_SQL: &str =
        "IF NOT EXISTS (SELECT 1 FROM dbo.Sources WHERE UrlHash = HASHBYTES('SHA2_256', @P1))
                         INSERT INTO dbo.Sources (Url) VALUES (@P1)";
    /// Inserts any URLs not already present. Returns how many were new.
    pub fn add_sources(&mut self, urls: &[String]) -> Result<usize> {
        self.rt.block_on(bounded(
            async {
                let mut added = 0;
                for url in urls {
                    let res = self.client.execute(Self::ADD_SOURCES_SQL, &[url]).await?;
                    added += res.total() as usize;
                }
                Ok(added)
            },
            &self.poisoned,
        ))
    }

    const RECORD_RESULT_SQL: &str =
        "UPDATE dbo.Sources SET LastScrapedAt = SYSUTCDATETIME(), LastError = @P2
                     WHERE UrlHash = HASHBYTES('SHA2_256', @P1)";
    /// Records the outcome of scraping a source (`None` error means success).
    pub fn record_result(&mut self, url: &str, error: Option<&str>) -> Result<()> {
        let error = error.map(|e| e.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::RECORD_RESULT_SQL, &[&url, &error])
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
    const ARTICLE_CHECKED_SQL: &str = "UPDATE a SET LastCheckedAt=SYSUTCDATETIME() FROM dbo.ArticleUrlAliases a JOIN dbo.Sources s ON s.ID=a.SourceId WHERE s.UrlHash=HASHBYTES('SHA2_256',@P1) AND a.UrlHash=HASHBYTES('SHA2_256',@P2)";
    pub fn article_checked(&mut self, source: &str, url: &str) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::ARTICLE_CHECKED_SQL, &[&source, &url])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    const RECENT_ARTICLES_SQL: &str = "WITH recent AS (SELECT TOP(10) p.* FROM dbo.Pages p JOIN dbo.Sources s ON s.ID=p.SourceId WHERE s.UrlHash=HASHBYTES('SHA2_256',@P1) ORDER BY CASE WHEN p.Published IS NULL THEN 1 ELSE 0 END,p.Published DESC,p.ScrapedAt DESC,p.ID DESC) SELECT p.Url,p.Title,p.Description,p.Published,p.ImageUrl FROM recent p LEFT JOIN dbo.ArticleUrlAliases a ON a.SourceId=p.SourceId AND a.UrlHash=p.UrlHash ORDER BY a.LastCheckedAt,p.ID";
    pub fn recent_articles(&mut self, source: &str) -> Result<Vec<PageMeta>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::RECENT_ARTICLES_SQL, &[&source])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| PageMeta {
                        source: source.to_owned(),
                        url: row.get::<&str, _>("Url").unwrap().to_owned(),
                        title: row.get::<&str, _>("Title").unwrap().to_owned(),
                        description: row.get::<&str, _>("Description").map(str::to_owned),
                        published: row
                            .get::<chrono::NaiveDateTime, _>("Published")
                            .map(|d| d.and_utc()),
                        image_url: row.get::<&str, _>("ImageUrl").map(str::to_owned),
                        identity_urls: vec![],
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }

    const PRUNE_PER_SOURCE_SQL: &str = ";WITH ranked AS (
                         SELECT ROW_NUMBER() OVER (
                             PARTITION BY SourceId
                             ORDER BY CASE WHEN Published IS NULL THEN 1 ELSE 0 END,
                                      Published DESC, ScrapedAt DESC, ID DESC) AS rn
                         FROM dbo.Pages
                     )
                     DELETE FROM ranked WHERE rn > @P1";
    /// Deletes each source's oldest pages so at most `keep` remain per source.
    /// Returns the number of rows deleted.
    pub fn prune_per_source(&mut self, keep: u32) -> Result<u64> {
        let keep = checked_keep(keep)?;
        self.rt.block_on(bounded(
            async {
                let res = self
                    .client
                    .execute(Self::PRUNE_PER_SOURCE_SQL, &[&keep])
                    .await?;
                Ok(res.total())
            },
            &self.poisoned,
        ))
    }

    const PRUNE_PAGES_SQL: &str = ";WITH ranked AS (
                         SELECT ROW_NUMBER() OVER (
                             ORDER BY CASE WHEN Published IS NULL THEN 1 ELSE 0 END,
                                      Published DESC, ScrapedAt DESC, ID DESC) AS rn
                         FROM dbo.Pages
                     )
                     DELETE FROM ranked WHERE rn > @P1";
    /// Deletes the oldest pages so at most `keep` remain. Dated pages are ranked newest first
    /// and come before undated ones (which are ranked by when they were scraped).
    /// Returns the number of rows deleted.
    pub fn prune_pages(&mut self, keep: u32) -> Result<u64> {
        let keep = checked_keep(keep)?;
        self.rt.block_on(bounded(
            async {
                let res = self.client.execute(Self::PRUNE_PAGES_SQL, &[&keep]).await?;
                Ok(res.total())
            },
            &self.poisoned,
        ))
    }

    const SAVE_EVENTS_SQL: &str = "MERGE dbo.SportsEvents AS t
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
                                     @P10, @P11, @P12, @P13, @P14, @P15, @P16, @P17);";
    /// Upserts each event keyed on its URL, linked to its `SportsEventsType` (calendar feed) row. The feed is the source of truth for schedule facts
    /// (time, TV, location), so those are overwritten; `FirstSeenAt` is kept.
    /// Returns the number of events written.
    pub fn save_events(&mut self, type_id: i32, events: &[Event]) -> Result<usize> {
        self.rt.block_on(bounded(
            async {
                for e in events {
                    let title: String = utf16_prefix(&e.title, 500);
                    self.client
                        .execute(
                            Self::SAVE_EVENTS_SQL,
                            &[
                                &e.url,
                                &e.game_id,
                                &title,
                                &e.sport,
                                &e.opponent,
                                &e.is_away,
                                &e.location,
                                &e.event_date,
                                &e.starts_at,
                                &e.ends_at,
                                &e.time_tbd,
                                &e.tv,
                                &e.stream_url,
                                &e.live_stats_url,
                                &e.team_logo_url,
                                &e.opponent_logo_url,
                                &type_id,
                            ],
                        )
                        .await?;
                }
                Ok(events.len())
            },
            &self.poisoned,
        ))
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
    const RANKING_SOURCES_SQL: &str = "SELECT ID,Sport,LeagueId,PollId,Url FROM dbo.SportsRankingSources WHERE Enabled=1 AND ProviderKey='espn-rankings' ORDER BY ID";
    pub fn ranking_sources(&mut self) -> Result<Vec<crate::rankings::Source>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::RANKING_SOURCES_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                rows.iter()
                    .map(|r| {
                        Ok(crate::rankings::Source {
                            id: r.get(0).ok_or("missing ranking source ID")?,
                            sport: r.get::<&str, _>(1).ok_or("missing ranking sport")?.into(),
                            league_id: r.get::<&str, _>(2).ok_or("missing ranking league")?.into(),
                            poll_id: r.get::<&str, _>(3).ok_or("missing ranking poll")?.into(),
                            url: r.get::<&str, _>(4).ok_or("missing ranking URL")?.into(),
                        })
                    })
                    .collect()
            },
            &self.poisoned,
        ))
    }
    const RANKING_ATTEMPT_SQL: &str =
        "UPDATE dbo.SportsRankingSources SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=@P1";
    pub fn ranking_attempt(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::RANKING_ATTEMPT_SQL, &[&id])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
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
    const SAVE_RANKING_SQL: &str = r#"
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
"#;
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
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::SAVE_RANKING_SQL,
                        &[
                            &source.id,
                            &poll.poll_id,
                            &poll.name,
                            &poll.season,
                            &poll.season_label,
                            &published,
                            &updated,
                            &poll.period,
                            &poll.phase,
                            &poll.source_url,
                            &rows,
                        ],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
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
    const STANDINGS_SOURCES_SQL: &str = "SELECT ID,ProviderKey,Sport,Url,MinimumTeams FROM dbo.SportsStandingsSources WHERE Enabled=1 ORDER BY ID";
    pub fn standings_sources(&mut self) -> Result<Vec<crate::standings::Source>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::STANDINGS_SOURCES_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                rows.iter()
                    .map(|r| {
                        Ok(crate::standings::Source {
                            id: r.get(0).ok_or("missing standings source ID")?,
                            provider: r
                                .get::<&str, _>(1)
                                .ok_or("missing standings provider")?
                                .to_owned(),
                            sport: r
                                .get::<&str, _>(2)
                                .ok_or("missing standings sport")?
                                .to_owned(),
                            url: r
                                .get::<&str, _>(3)
                                .ok_or("missing standings URL")?
                                .to_owned(),
                            minimum_teams: usize::try_from(
                                r.get::<i32, _>(4).ok_or("missing standings minimum")?,
                            )?,
                        })
                    })
                    .collect()
            },
            &self.poisoned,
        ))
    }
    const STANDINGS_ATTEMPT_SQL: &str =
        "UPDATE dbo.SportsStandingsSources SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=@P1";
    pub fn standings_attempt(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(Self::STANDINGS_ATTEMPT_SQL, &[&id])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
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
    const SAVE_STANDINGS_SQL: &str = r#"
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
"#;
    pub fn save_standings(
        &mut self,
        source: &crate::standings::Source,
        snapshot: &crate::standings::Snapshot,
    ) -> Result<()> {
        snapshot.validate(source.minimum_teams)?;
        let rows = snapshot.rows_json();
        let count = i32::try_from(snapshot.entries.len())?;
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::SAVE_STANDINGS_SQL,
                        &[
                            &source.id,
                            &snapshot.season,
                            &snapshot.season_type,
                            &snapshot.title,
                            &snapshot.source_url,
                            &rows,
                            &count,
                        ],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }

    const CINEMA_POSTER_JOBS_SQL_1: &str =
        "SELECT CASE WHEN OBJECT_ID('dbo.CinemaPosterCache','U') IS NULL THEN 0 ELSE 1 END";
    const CINEMA_POSTER_JOBS_SQL_2: &str = "SELECT 'results' AS Section,MovieTitle,CAST(NULL AS SMALLINT) AS OriginalYear,MovieUrl FROM dbo.BoxOfficeWeekend UNION SELECT 'forecasts',MovieTitle,CAST(YEAR(TargetDate) AS SMALLINT),NULL FROM dbo.BoxOfficeForecasts";
    pub fn cinema_poster_jobs(&mut self) -> Result<Vec<crate::cinema_posters::Job>> {
        self.rt.block_on(bounded(
            async {
                let exists = self
                    .client
                    .query(Self::CINEMA_POSTER_JOBS_SQL_1, &[])
                    .await?
                    .into_first_result()
                    .await?;
                if exists[0].get::<i32, _>(0) == Some(0) {
                    return Ok(Vec::new());
                }
                let rows = self
                    .client
                    .query(Self::CINEMA_POSTER_JOBS_SQL_2, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|r| {
                        let url = r.get::<&str, _>(3).map(str::to_string);
                        let year = r
                            .get::<i16, _>(2)
                            .or_else(|| url.as_deref().and_then(crate::cinema_posters::url_year));
                        crate::cinema_posters::Job {
                            section: r.get::<&str, _>(0).unwrap().to_string(),
                            title: r.get::<&str, _>(1).unwrap().to_string(),
                            year,
                            url,
                        }
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }
    const CINEMA_RELEASE_POSTERS_SQL: &str = "SELECT Title,OriginalYear,PosterUrl FROM dbo.Movies WHERE PosterUrl IS NOT NULL AND PosterLookupStatus IN ('matched','valid')";
    pub fn cinema_release_posters(
        &mut self,
    ) -> Result<Vec<(crate::movie_metadata::MovieIdentity, String)>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::CINEMA_RELEASE_POSTERS_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|r| {
                        let (title, suffix_year) =
                            crate::movie_metadata::title_year(r.get::<&str, _>(0).unwrap());
                        (
                            crate::movie_metadata::MovieIdentity {
                                title,
                                year: r.get::<i16, _>(1).or(suffix_year),
                                ..Default::default()
                            },
                            r.get::<&str, _>(2).unwrap().to_string(),
                        )
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }
    const CINEMA_POSTER_CACHES_SQL_1: &str =
        "SELECT CASE WHEN OBJECT_ID('dbo.CinemaPosterCache','U') IS NULL THEN 0 ELSE 1 END";
    const CINEMA_POSTER_CACHES_SQL_2: &str =
        "SELECT IdentityKey,PosterUrl,LookupStatus,RetryAfterUtc FROM dbo.CinemaPosterCache";
    pub fn cinema_poster_caches(
        &mut self,
    ) -> Result<std::collections::BTreeMap<String, crate::cinema_posters::Cache>> {
        self.rt.block_on(bounded(
            async {
                let exists = self
                    .client
                    .query(Self::CINEMA_POSTER_CACHES_SQL_1, &[])
                    .await?
                    .into_first_result()
                    .await?;
                if exists[0].get::<i32, _>(0) == Some(0) {
                    return Ok(std::collections::BTreeMap::new());
                }
                let rows = self
                    .client
                    .query(Self::CINEMA_POSTER_CACHES_SQL_2, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|r| {
                        (
                            r.get::<&str, _>(0).unwrap().to_string(),
                            crate::cinema_posters::Cache {
                                poster: r.get::<&str, _>(1).map(str::to_string),
                                status: r.get::<&str, _>(2).unwrap().to_string(),
                                retry: r.get::<chrono::NaiveDateTime, _>(3).unwrap(),
                            },
                        )
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }
    const SAVE_CINEMA_POSTER_SQL: &str = "MERGE dbo.CinemaPosterCache WITH(HOLDLOCK) t USING(SELECT @P1 AS IdentityKey) s ON t.IdentityKey=s.IdentityKey WHEN MATCHED THEN UPDATE SET PosterUrl=@P4,SourceUrl=COALESCE(@P5,t.SourceUrl),LookupStatus=@P6,LastError=@P7,CheckedAtUtc=SYSUTCDATETIME(),RetryAfterUtc=@P8,LastUsedAtUtc=SYSUTCDATETIME() WHEN NOT MATCHED THEN INSERT(IdentityKey,MovieTitle,OriginalYear,PosterUrl,SourceUrl,LookupStatus,LastError,CheckedAtUtc,RetryAfterUtc,LastUsedAtUtc) VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7,SYSUTCDATETIME(),@P8,SYSUTCDATETIME());";
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
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::SAVE_CINEMA_POSTER_SQL,
                        &[
                            &key,
                            &identity.title,
                            &identity.year,
                            &poster,
                            &source,
                            &status,
                            &error,
                            &retry,
                        ],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    const ATTACH_CINEMA_POSTER_SQL_1: &str = "UPDATE dbo.BoxOfficeWeekend SET PosterUrl=@P1,PosterLookupStatus=@P2 WHERE MovieTitle=@P3 AND MovieUrl=@P4";
    const ATTACH_CINEMA_POSTER_SQL_2: &str = "UPDATE dbo.BoxOfficeForecasts SET PosterUrl=@P1,PosterLookupStatus=@P2 WHERE MovieTitle=@P3 AND YEAR(TargetDate)=@P4";
    const ATTACH_CINEMA_POSTER_SQL_3: &str =
        "UPDATE dbo.CinemaPosterCache SET LastUsedAtUtc=SYSUTCDATETIME() WHERE IdentityKey=@P1";
    pub fn attach_cinema_poster(
        &mut self,
        job: &crate::cinema_posters::Job,
        key: &str,
        poster: Option<&str>,
        status: &str,
    ) -> Result<()> {
        self.rt.block_on(bounded(
            async {
                if job.section == "results" {
                    self.client
                        .execute(
                            Self::ATTACH_CINEMA_POSTER_SQL_1,
                            &[&poster, &status, &job.title, &job.url],
                        )
                        .await?;
                } else {
                    self.client
                        .execute(
                            Self::ATTACH_CINEMA_POSTER_SQL_2,
                            &[&poster, &status, &job.title, &job.year],
                        )
                        .await?;
                }
                self.client
                    .execute(Self::ATTACH_CINEMA_POSTER_SQL_3, &[&key])
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
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
    const F1_ENABLED_SQL: &str =
        "IF OBJECT_ID('dbo.F1Source','U') IS NOT NULL SELECT Enabled FROM dbo.F1Source WHERE ID=1";
    pub fn f1_enabled(&mut self) -> Result<bool> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::F1_ENABLED_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .first()
                    .and_then(|r| r.get::<bool, _>(0))
                    .unwrap_or(false))
            },
            &self.poisoned,
        ))
    }
    const F1_STATUS_SQL: &str =
        "UPDATE dbo.F1Source SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=1 AND Enabled=1";
    pub fn f1_status(&mut self, error: Option<&str>) -> Result<()> {
        let message = error.map(|s| utf16_prefix(s, 1000));
        self.rt.block_on(bounded(
            async {
                if let Some(message) = message {
                    self.client
                        .execute(
                            "UPDATE dbo.F1Source SET LastError=@P1 WHERE ID=1",
                            &[&message],
                        )
                        .await?;
                } else {
                    self.client.execute(Self::F1_STATUS_SQL, &[]).await?;
                }
                Ok(())
            },
            &self.poisoned,
        ))
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
    const BOXSCORE_AVAILABLE_SQL: &str = "SELECT CAST(CASE WHEN EXISTS(SELECT 1 FROM dbo.SportsBoxScoreCache WHERE Sport=@P1 AND League=@P2 AND EventId=@P3 AND Payload IS NOT NULL) THEN 1 ELSE 0 END AS BIT)";
    pub fn boxscore_available(&mut self, g: &crate::sports_boxscores::Game) -> Result<bool> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(
                        Self::BOXSCORE_AVAILABLE_SQL,
                        &[&g.sport, &g.league, &g.event_id],
                    )
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .first()
                    .and_then(|r| r.get::<bool, _>(0))
                    .unwrap_or(false))
            },
            &self.poisoned,
        ))
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
    const SAVE_MATCH_RESULTS_SQL: &str = r#"
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
"#;
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

        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::SAVE_MATCH_RESULTS_SQL,
                        &[&source, &rows, &season, &provider],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
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

    const REFRESH_SCHOOL_TEAM_LOGO_SQL: &str =
        "UPDATE dbo.SportsEvents SET TeamLogoUrl=@P2 WHERE SportsEventsTypeId=@P1
                 AND (TeamLogoUrl <> @P2 OR (TeamLogoUrl IS NULL AND @P2 IS NOT NULL)
                      OR (TeamLogoUrl IS NOT NULL AND @P2 IS NULL))";
    /// A single-school calendar's logo also applies to retained events no longer in its RSS.
    /// Touch only TeamLogoUrl; keep event identities, opponents and schedule facts intact.
    pub fn refresh_school_team_logo(&mut self, type_id: i32, logo: Option<&str>) -> Result<u64> {
        self.rt.block_on(bounded(
            async {
                let result = self
                    .client
                    .execute(Self::REFRESH_SCHOOL_TEAM_LOGO_SQL, &[&type_id, &logo])
                    .await?;
                Ok(result.total())
            },
            &self.poisoned,
        ))
    }

    const SAVE_VIDEOS_SQL: &str = "MERGE dbo.YouTubeVideos AS t
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
                             VALUES (@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10, @P11, @P12, @P13);";
    /// Upserts each video keyed on its YouTube video ID, linked to its `YoutubeVideoFeed` row. Title, description and the view/rating
    /// counts change over time, so they are overwritten; `FirstSeenAt` is kept.
    /// A value the source didn't supply (`None`) never blanks a stored one, and an estimated
    /// publish date is only used for a video that isn't stored yet.
    /// Returns the number of videos written.
    pub fn save_videos(&mut self, feed_id: i32, videos: &[Video]) -> Result<usize> {
        self.rt.block_on(bounded(
            async {
                for v in videos {
                    let title: String = utf16_prefix(&v.title, 500);
                    self.client
                        .execute(
                            Self::SAVE_VIDEOS_SQL,
                            &[
                                &v.video_id,
                                &v.channel_id,
                                &v.channel_name,
                                &title,
                                &v.url,
                                &v.published_at,
                                &v.updated_at,
                                &v.thumbnail_url,
                                &v.description,
                                &v.views,
                                &v.rating_count,
                                &v.rating_average,
                                &feed_id,
                                &v.published_is_estimate,
                            ],
                        )
                        .await?;
                }
                Ok(videos.len())
            },
            &self.poisoned,
        ))
    }

    const CONFIGURE_BIG12_SQL: &str = "SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
                 IF @P1 IS NOT NULL UPDATE dbo.SportsEventsType SET ProviderKey='big12',ProviderSchoolId=@P2,SchoolName=@P3 WHERE ID=@P1;
                 ELSE IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ProviderKey='big12' AND ProviderSchoolId=@P2)
                  INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,SchoolName,ProviderKey,ProviderSchoolId,Enabled) VALUES(@P4,@P5,@P3,'big12',@P2,0);
                 COMMIT TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT>0 ROLLBACK TRANSACTION; THROW; END CATCH";
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
            self.rt.block_on(bounded(
                async {
                    self.client
                        .execute(
                            Self::CONFIGURE_BIG12_SQL,
                            &[&existing, &m.id, &m.name, &url, &label],
                        )
                        .await?;
                    Ok::<_, crate::Error>(())
                },
                &self.poisoned,
            ))?;
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

    const EVENT_RESULT_SQL: &str = "UPDATE dbo.SportsEventsType SET LastSuccessAt=CASE WHEN @P2=1 THEN SYSUTCDATETIME() ELSE LastSuccessAt END,LastEventCount=@P3,LastIssueCount=@P4,LastError=@P5 WHERE ID=@P1";
    pub fn event_result(
        &mut self,
        id: i32,
        success: bool,
        count: i32,
        issues: i32,
        error: Option<&str>,
    ) -> Result<()> {
        let error = error.map(|s| s.chars().take(1000).collect::<String>());
        self.rt.block_on(bounded(
            async {
                self.client
                    .execute(
                        Self::EVENT_RESULT_SQL,
                        &[&id, &success, &count, &issues, &error],
                    )
                    .await?;
                Ok(())
            },
            &self.poisoned,
        ))
    }
    const SPORTS_EVENT_FEEDS_SQL: &str =
        "SELECT ID, RssUrl, SchoolName FROM dbo.SportsEventsType WHERE Enabled=1 ORDER BY ID";
    /// Every calendar feed in the `SportsEventsType` lookup table.
    pub fn sports_event_feeds(&mut self) -> Result<Vec<EventFeed>> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::SPORTS_EVENT_FEEDS_SQL, &[])
                    .await?
                    .into_first_result()
                    .await?;
                Ok(rows
                    .iter()
                    .filter_map(|r| {
                        Some(EventFeed {
                            id: r.get::<i32, _>(0)?,
                            url: r.get::<&str, _>(1)?.to_string(),
                            school: r
                                .get::<&str, _>(2)
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(str::to_string),
                        })
                    })
                    .collect())
            },
            &self.poisoned,
        ))
    }

    const ENSURE_SPORTS_EVENTS_TYPE_SQL: &str = "DECLARE @id INT = (SELECT ID FROM dbo.SportsEventsType WHERE RssUrl = @P1);
                     IF @id IS NULL
                     BEGIN
                         INSERT INTO dbo.SportsEventsType (RssUrl, EventsTypeName) VALUES (@P1, @P2);
                         SET @id = SCOPE_IDENTITY();
                     END
                     SELECT @id;";
    /// ID of the `SportsEventsType` row for this feed URL, adding the row (named `name`) if it
    /// isn't there yet. An existing row keeps its name.
    pub fn ensure_sports_events_type(&mut self, rss_url: &str, name: &str) -> Result<i32> {
        if rss_url.chars().count() > 450 {
            return Err("calendar feed URL is longer than the 450 characters SportsEventsType.RssUrl allows".into());
        }
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::ENSURE_SPORTS_EVENTS_TYPE_SQL, &[&rss_url, &name])
                    .await?
                    .into_first_result()
                    .await?;
                rows.first()
                    .and_then(|r| r.get::<i32, _>(0))
                    .ok_or_else(|| "could not read the SportsEventsType ID".into())
            },
            &self.poisoned,
        ))
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

    const ENSURE_YOUTUBE_FEED_SQL: &str = "DECLARE @id INT = (SELECT ID FROM dbo.YoutubeVideoFeed WHERE ChannelId = @P1);
                     IF @id IS NULL
                     BEGIN
                         INSERT INTO dbo.YoutubeVideoFeed (ChannelId, ChannelName, Url) VALUES (@P1, @P2, @P3);
                         SET @id = SCOPE_IDENTITY();
                     END
                     ELSE
                         UPDATE dbo.YoutubeVideoFeed SET ChannelName = COALESCE(ChannelName, @P2) WHERE ID = @id;
                     SELECT @id;";
    /// ID of the `YoutubeVideoFeed` row for this channel, adding the row if it isn't there yet.
    /// An existing row keeps its URL; a missing channel name is filled in.
    pub fn ensure_youtube_feed(
        &mut self,
        channel_id: &str,
        name: Option<&str>,
        url: &str,
    ) -> Result<i32> {
        self.rt.block_on(bounded(
            async {
                let rows = self
                    .client
                    .query(Self::ENSURE_YOUTUBE_FEED_SQL, &[&channel_id, &name, &url])
                    .await?
                    .into_first_result()
                    .await?;
                rows.first()
                    .and_then(|r| r.get::<i32, _>(0))
                    .ok_or_else(|| "could not read the YoutubeVideoFeed ID".into())
            },
            &self.poisoned,
        ))
    }

    const PRUNE_VIDEOS_SQL: &str = ";WITH ranked AS (
                         SELECT ROW_NUMBER() OVER (
                             PARTITION BY ChannelId ORDER BY PublishedAt DESC, ID DESC) AS rn
                         FROM dbo.YouTubeVideos
                     )
                     DELETE FROM ranked WHERE rn > @P1";
    /// Deletes each channel's oldest videos so at most `keep` remain per channel.
    /// Returns the number of rows deleted.
    pub fn prune_videos(&mut self, keep: u32) -> Result<u64> {
        let keep = checked_keep(keep)?;
        self.rt.block_on(bounded(
            async {
                let res = self
                    .client
                    .execute(Self::PRUNE_VIDEOS_SQL, &[&keep])
                    .await?;
                Ok(res.total())
            },
            &self.poisoned,
        ))
    }
}

#[cfg(test)]
mod tests;
