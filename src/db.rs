use chrono::{DateTime, Utc};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::PageMeta;
use crate::events::Event;
use crate::youtube::Video;

#[allow(dead_code)] // Default used by database integration tests.
const DB_NAME: &str = "WebScraper";

type Conn = Client<Compat<TcpStream>>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

async fn connect(ado: &str, database: &str) -> Result<Conn> {
    let mut config = Config::from_ado_string(ado)?;
    config.database(database);
    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;
    Ok(Client::connect(config, tcp.compat_write()).await?)
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
}

impl Db {
    /// Connects, creating the database and its tables first if they are missing.
    #[allow(dead_code)] // Convenience entry point for tests and callers using the default database.
    pub fn open(ado: &str) -> Result<Db> {
        Self::open_named(ado, DB_NAME)
    }

    pub fn open_named(ado: &str, database: &str) -> Result<Db> {
        if database.is_empty() || !database.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("database name must contain only ASCII letters, digits or underscores".into());
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = rt.block_on(async {
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
            client.simple_query(include_str!("../sql/big12_events.sql")).await?.into_results().await?;
            client.simple_query(include_str!("../sql/big12_views.sql")).await?.into_results().await?;
            Result::Ok(client)
        })?;
        Ok(Db { rt, client })
    }

    pub fn retry_missing_posters(&mut self)->Result<()> {
        self.rt.block_on(async {
            self.client.execute("UPDATE dbo.Movies SET PosterRetryAfter=NULL WHERE PosterUrl IS NULL; UPDATE c SET RetryAfter=SYSUTCDATETIME() FROM dbo.MoviePosterCache c JOIN dbo.MovieSourceLinks l ON l.ID=c.SourceLinkId JOIN dbo.Movies m ON m.ID=l.MovieId WHERE m.PosterUrl IS NULL",&[]).await?;Ok(())
        })
    }

    pub fn poster_jobs(&mut self) -> Result<Vec<crate::posters::PosterJob>> {
        self.rt.block_on(async {
            let rows=self.client.query("SELECT m.ID,m.Title,m.OriginalYear,m.ImdbId,m.TmdbId,m.PosterUrl,m.PosterCheckedAt,m.PosterRetryAfter,m.PosterLookupStatus,l.ID AS LinkId,l.Url,s.SourceKey,c.TargetIdentity,c.Metadata,c.PosterUrl AS CachePoster,c.RetryAfter,c.LastError FROM dbo.Movies m JOIN dbo.MovieSourceLinks l ON l.MovieId=m.ID JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId LEFT JOIN dbo.MoviePosterCache c ON c.SourceLinkId=l.ID ORDER BY m.ID,CASE s.SourceKey WHEN 'dvdsreleasedates' THEN 0 ELSE 1 END,l.ID",&[]).await?.into_first_result().await?;
            Ok(rows.iter().map(|r|{
                let (title,suffix_year)=crate::movie_metadata::title_year(r.get::<&str,_>("Title").unwrap());
                crate::posters::PosterJob { movie_id:r.get::<i32,_>("ID").unwrap(),link_id:r.get::<i32,_>("LinkId").unwrap(),source:r.get::<&str,_>("SourceKey").unwrap().to_owned(),url:r.get::<&str,_>("Url").unwrap().to_owned(),
                    identity:crate::movie_metadata::MovieIdentity{title,year:r.get::<i16,_>("OriginalYear").or(suffix_year),imdb_id:r.get::<&str,_>("ImdbId").map(str::to_owned),tmdb_id:r.get::<i32,_>("TmdbId")},
                    existing:r.get::<&str,_>("PosterUrl").map(str::to_owned),poster_status:r.get::<&str,_>("PosterLookupStatus").map(str::to_owned),checked:r.get::<chrono::NaiveDateTime,_>("PosterCheckedAt"),retry:r.get::<chrono::NaiveDateTime,_>("PosterRetryAfter"),cache_identity:r.get::<&str,_>("TargetIdentity").map(str::to_owned),cache_metadata:r.get::<&str,_>("Metadata").map(str::to_owned),cache_poster:r.get::<&str,_>("CachePoster").map(str::to_owned),cache_retry:r.get::<chrono::NaiveDateTime,_>("RetryAfter"),cache_error:r.get::<&str,_>("LastError").map(str::to_owned)}
            }).collect())
        })
    }

    pub fn cache_poster_source(&mut self,link:i32,identity:&str,metadata:&serde_json::Value,poster:Option<&str>,error:Option<&str>)->Result<()> {
        let metadata=serde_json::to_string(metadata)?;let error=error.map(|e|e.chars().take(1000).collect::<String>());
        self.rt.block_on(async {
            self.client.execute("SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
             IF EXISTS(SELECT 1 FROM dbo.MoviePosterCache WITH(UPDLOCK,HOLDLOCK) WHERE SourceLinkId=@P1)
              UPDATE dbo.MoviePosterCache SET TargetIdentity=@P2,Metadata=@P3,PosterUrl=@P4,LastError=@P5,CheckedAt=SYSUTCDATETIME(),RetryAfter=CASE WHEN @P5 IS NOT NULL THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,CASE WHEN @P4 IS NULL THEN 7 ELSE 30 END,SYSUTCDATETIME()) END WHERE SourceLinkId=@P1;
             ELSE INSERT dbo.MoviePosterCache(SourceLinkId,TargetIdentity,Metadata,PosterUrl,LastError,CheckedAt,RetryAfter) VALUES(@P1,@P2,@P3,@P4,@P5,SYSUTCDATETIME(),CASE WHEN @P5 IS NOT NULL THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,CASE WHEN @P4 IS NULL THEN 7 ELSE 30 END,SYSUTCDATETIME()) END);
             COMMIT TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT>0 ROLLBACK TRANSACTION; THROW; END CATCH",&[&link,&identity,&metadata,&poster,&error]).await?;Ok(())
        })
    }

    pub fn set_movie_poster(&mut self,id:i32,expected:Option<&str>,poster:Option<&str>,year:Option<i16>,status:&str,source:Option<&str>,error:Option<&str>,days:i32)->Result<bool> {
        let error=error.map(|e|e.chars().take(1000).collect::<String>());
        self.rt.block_on(async {
            let updated=self.client.execute("UPDATE dbo.Movies SET PosterUrl=@P3,OriginalYear=COALESCE(OriginalYear,@P4),PosterCheckedAt=SYSUTCDATETIME(),PosterRetryAfter=CASE WHEN @P8=0 THEN DATEADD(hour,6,SYSUTCDATETIME()) ELSE DATEADD(day,@P8,SYSUTCDATETIME()) END,PosterLookupStatus=@P5,PosterSourceUrl=COALESCE(@P6,PosterSourceUrl),PosterLookupError=@P7 WHERE ID=@P1 AND (PosterUrl=@P2 OR (PosterUrl IS NULL AND @P2 IS NULL)) AND (OriginalYear IS NULL OR @P4 IS NULL OR OriginalYear=@P4)",&[&id,&expected,&poster,&year,&status,&source,&error,&days]).await?;Ok(updated.total()>0)
        })
    }

    pub fn setup_tv(&mut self,seed:bool)->Result<()> {
        self.rt.block_on(async {
            self.client.simple_query(include_str!("../sql/tvmaze.sql")).await?.into_results().await?;
            if seed {self.client.simple_query(include_str!("../sql/tvmaze_seed.sql")).await?.into_results().await?;}
            Ok(())
        })
    }
    pub fn tv_jobs(&mut self)->Result<Vec<(i32,String)>> {
        self.rt.block_on(async {
            let rows=self.client.query("SELECT TvmazeShowId,ExpectedTitle FROM dbo.TvTrackedSeries WHERE Enabled=1 ORDER BY ID",&[]).await?.into_first_result().await?;
            Ok(rows.iter().map(|r|(r.get::<i32,_>(0).unwrap(),r.get::<&str,_>(1).unwrap().to_owned())).collect())
        })
    }
    pub fn claim_tv_series(&mut self,id:i32,force:bool)->Result<Option<String>> {
        self.rt.block_on(async {
            let rows=self.client.query("UPDATE dbo.TvTrackedSeries WITH(UPDLOCK,ROWLOCK) SET LastAttemptAt=SYSUTCDATETIME(),LeaseUntil=DATEADD(minute,10,SYSUTCDATETIME()),LeaseToken=NEWID() OUTPUT CONVERT(VARCHAR(36),inserted.LeaseToken) WHERE TvmazeShowId=@P1 AND Enabled=1 AND (LeaseUntil IS NULL OR LeaseUntil<=SYSUTCDATETIME()) AND (@P2=1 OR ((CacheExpiresAt IS NULL OR CacheExpiresAt<=SYSUTCDATETIME()) AND (RetryAfter IS NULL OR RetryAfter<=SYSUTCDATETIME())))",&[&id,&force]).await?.into_first_result().await?;
            Ok(rows.first().and_then(|r|r.get::<&str,_>(0)).map(str::to_owned))
        })
    }
    pub fn save_tv_snapshot(&mut self,id:i32,lease:&str,payload:&serde_json::Value)->Result<String> {
        let payload=serde_json::to_string(payload)?;
        self.rt.block_on(async {
            let rows=self.client.query(include_str!("../sql/upsert_tvmaze_snapshot.sql"),&[&id,&payload,&lease]).await?.into_first_result().await?;
            Ok(rows.first().and_then(|r|r.get::<&str,_>("Outcome")).unwrap_or("unknown").to_owned())
        })
    }
    pub fn fail_tv_series(&mut self,id:i32,lease:&str,error:&str,retry:i32)->Result<()> {
        let error=error.chars().take(1000).collect::<String>();
        self.rt.block_on(async {
            self.client.execute("UPDATE dbo.TvTrackedSeries SET LastError=@P3,LastIssueCount=1,RetryAfter=DATEADD(second,@P4,SYSUTCDATETIME()),LeaseUntil=NULL,LeaseToken=NULL WHERE TvmazeShowId=@P1 AND LeaseToken=TRY_CONVERT(UNIQUEIDENTIFIER,@P2)",&[&id,&lease,&error,&retry]).await?;Ok(())
        })
    }

    pub fn tracked_movie_urls(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(async {
            let rows = self.client.query("SELECT DISTINCT l.Url FROM dbo.MovieSourceLinks l JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.Enabled=1", &[]).await?.into_first_result().await?;
            Ok(rows.iter().filter_map(|r| r.get::<&str,_>(0).map(str::to_owned)).collect())
        })
    }

    /// Upsert stable source/release identities; never delete absent calendar entries.
    pub fn save_movie_releases(&mut self, rows: &[crate::movies::MovieRelease], report: &serde_json::Value) -> Result<usize> {
        self.rt.block_on(async {
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
            for (source, host) in [("dvdsreleasedates", "dvdsreleasedates.com"), ("whentostream", "whentostream.com")] {
                let errors: Vec<_> = report["issues"].as_array().into_iter().flatten()
                    .filter(|r| r["url"].as_str().is_some_and(|u| u.contains(host)))
                    .map(|r| r["reason"].as_str().unwrap_or("unknown issue")).collect();
                let error = if errors.is_empty() { None } else { Some(errors.join("; ").chars().take(1000).collect::<String>()) };
                self.client.execute("UPDATE dbo.MovieReleaseSources SET LastScrapedAt=SYSUTCDATETIME(), LastError=@P2 WHERE SourceKey=@P1 AND Enabled=1", &[&source, &error]).await?;
            }
            Ok(written)
        })
    }

    /// URLs of all enabled sources, oldest first.
    pub fn enabled_sources(&mut self) -> Result<Vec<String>> {
        self.rt.block_on(async {
            let rows = self
                .client
                .query("SELECT Url FROM dbo.Sources WHERE Enabled = 1 ORDER BY ID", &[])
                .await?
                .into_first_result()
                .await?;
            Ok(rows
                .iter()
                .filter_map(|r| r.get::<&str, _>(0).map(str::to_string))
                .collect())
        })
    }

    /// Inserts any URLs not already present. Returns how many were new.
    pub fn add_sources(&mut self, urls: &[String]) -> Result<usize> {
        self.rt.block_on(async {
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
        })
    }

    /// Records the outcome of scraping a source (`None` error means success).
    pub fn record_result(&mut self, url: &str, error: Option<&str>) -> Result<()> {
        let error = error.map(|e| e.chars().take(1000).collect::<String>());
        self.rt.block_on(async {
            self.client
                .execute(
                    "UPDATE dbo.Sources SET LastScrapedAt = SYSUTCDATETIME(), LastError = @P2
                     WHERE UrlHash = HASHBYTES('SHA2_256', @P1)",
                    &[&url, &error],
                )
                .await?;
            Ok(())
        })
    }

    /// Upserts each page keyed on URL, linking it to its source (which is created
    /// in `Sources` if the requested URL isn't there yet). Returns the number of pages written.
    pub fn save_pages(&mut self, pages: &[PageMeta]) -> Result<usize> {
        self.rt.block_on(async {
            for p in pages {
                let published: Option<DateTime<Utc>> = p.published;
                let title: String = p.title.chars().take(500).collect();
                self.client
                    .execute(
                        "DECLARE @sid INT = (SELECT ID FROM dbo.Sources WHERE UrlHash = HASHBYTES('SHA2_256', @P1));
                         IF @sid IS NULL
                         BEGIN
                             INSERT INTO dbo.Sources (Url) VALUES (@P1);
                             SET @sid = SCOPE_IDENTITY();
                         END
                         MERGE dbo.Pages AS t
                         USING (SELECT @P2 AS Url) AS s
                         ON t.UrlHash = HASHBYTES('SHA2_256', s.Url)
                         WHEN MATCHED THEN UPDATE SET
                             Title = @P3,
                             Description = COALESCE(@P4, t.Description),
                             Published = COALESCE(@P5, t.Published),
                             ImageUrl = COALESCE(@P6, t.ImageUrl),
                             SourceId = @sid
                         WHEN NOT MATCHED THEN INSERT (Url, Title, Description, Published, ImageUrl, SourceId)
                             VALUES (@P2, @P3, @P4, @P5, @P6, @sid);",
                        &[&p.source, &p.url, &title, &p.description, &published, &p.image_url],
                    )
                    .await?;
            }
            Ok(pages.len())
        })
    }

    /// Deletes each source's oldest pages so at most `keep` remain per source.
    /// Returns the number of rows deleted.
    pub fn prune_per_source(&mut self, keep: u32) -> Result<u64> {
        let keep = keep as i32;
        self.rt.block_on(async {
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
        })
    }

    /// Deletes the oldest pages so at most `keep` remain. Dated pages are ranked newest first
    /// and come before undated ones (which are ranked by when they were scraped).
    /// Returns the number of rows deleted.
    pub fn prune_pages(&mut self, keep: u32) -> Result<u64> {
        let keep = keep as i32;
        self.rt.block_on(async {
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
        })
    }

    /// Upserts each event keyed on its URL, linked to its `SportsEventsType` (calendar feed) row. The feed is the source of truth for schedule facts
    /// (time, TV, location), so those are overwritten; `FirstSeenAt` is kept.
    /// Returns the number of events written.
    pub fn save_events(&mut self, type_id: i32, events: &[Event]) -> Result<usize> {
        self.rt.block_on(async {
            for e in events {
                let title: String = e.title.chars().take(500).collect();
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
        })
    }

    /// A single-school calendar's logo also applies to retained events no longer in its RSS.
    /// Touch only TeamLogoUrl; keep event identities, opponents and schedule facts intact.
    pub fn refresh_school_team_logo(&mut self, type_id: i32, logo: Option<&str>) -> Result<u64> {
        self.rt.block_on(async {
            let result = self.client.execute(
                "UPDATE dbo.SportsEvents SET TeamLogoUrl=@P2 WHERE SportsEventsTypeId=@P1
                 AND (TeamLogoUrl <> @P2 OR (TeamLogoUrl IS NULL AND @P2 IS NOT NULL)
                      OR (TeamLogoUrl IS NOT NULL AND @P2 IS NULL))", &[&type_id, &logo]).await?;
            Ok(result.total())
        })
    }

    /// Upserts each video keyed on its YouTube video ID, linked to its `YoutubeVideoFeed` row. Title, description and the view/rating
    /// counts change over time, so they are overwritten; `FirstSeenAt` is kept.
    /// A value the source didn't supply (`None`) never blanks a stored one, and an estimated
    /// publish date is only used for a video that isn't stored yet.
    /// Returns the number of videos written.
    pub fn save_videos(&mut self, feed_id: i32, videos: &[Video]) -> Result<usize> {
        self.rt.block_on(async {
            for v in videos {
                let title: String = v.title.chars().take(500).collect();
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
        })
    }

    pub fn configure_big12(
        &mut self,
        members: &[crate::big12::Member],
        enable: bool,
    ) -> Result<()> {
        let known = self.rt.block_on(async {
            let rows = self
                .client
                .query("SELECT ID,RssUrl FROM dbo.SportsEventsType", &[])
                .await?
                .into_first_result()
                .await?;
            Ok::<_, Box<dyn std::error::Error>>(
                rows.iter()
                    .filter_map(|r| Some((r.get::<i32, _>(0)?, r.get::<&str, _>(1)?.to_owned())))
                    .collect::<Vec<_>>(),
            )
        })?;
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
            self.rt.block_on(async {
                self.client.execute("SET XACT_ABORT ON; BEGIN TRY BEGIN TRANSACTION;
                 IF @P1 IS NOT NULL UPDATE dbo.SportsEventsType SET ProviderKey='big12',ProviderSchoolId=@P2,SchoolName=@P3 WHERE ID=@P1;
                 ELSE IF NOT EXISTS(SELECT 1 FROM dbo.SportsEventsType WITH(UPDLOCK,HOLDLOCK) WHERE ProviderKey='big12' AND ProviderSchoolId=@P2)
                  INSERT dbo.SportsEventsType(RssUrl,EventsTypeName,SchoolName,ProviderKey,ProviderSchoolId,Enabled) VALUES(@P4,@P5,@P3,'big12',@P2,0);
                 COMMIT TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT>0 ROLLBACK TRANSACTION; THROW; END CATCH",&[&existing,&m.id,&m.name,&url,&label]).await?;
                Ok::<_,Box<dyn std::error::Error>>(())
            })?;
        }
        self.rt.block_on(async {
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
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    }

    pub fn event_attempt(&mut self, id: i32) -> Result<()> {
        self.rt.block_on(async {
            self.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET LastAttemptAt=SYSUTCDATETIME() WHERE ID=@P1",
                    &[&id],
                )
                .await?;
            Ok(())
        })
    }

    pub fn save_big12_observation(
        &mut self,
        feed: i32,
        observation: &serde_json::Value,
    ) -> Result<String> {
        let payload = serde_json::to_string(observation)?;
        for attempt in 0..3 {
            let result: Result<String> = self.rt.block_on(async {
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
            });
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
        self.rt.block_on(async {
            self.client.execute("UPDATE dbo.SportsEventsType SET LastSuccessAt=CASE WHEN @P2=1 THEN SYSUTCDATETIME() ELSE LastSuccessAt END,LastEventCount=@P3,LastIssueCount=@P4,LastError=@P5 WHERE ID=@P1",&[&id,&success,&count,&issues,&error]).await?;Ok(())
        })
    }
    /// Every calendar feed in the `SportsEventsType` lookup table.
    pub fn sports_event_feeds(&mut self) -> Result<Vec<EventFeed>> {
        self.rt.block_on(async {
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
        })
    }

    /// ID of the `SportsEventsType` row for this feed URL, adding the row (named `name`) if it
    /// isn't there yet. An existing row keeps its name.
    pub fn ensure_sports_events_type(&mut self, rss_url: &str, name: &str) -> Result<i32> {
        if rss_url.chars().count() > 450 {
            return Err("calendar feed URL is longer than the 450 characters SportsEventsType.RssUrl allows".into());
        }
        self.rt.block_on(async {
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
        })
    }

    /// `(ID, Url)` of every channel in the `YoutubeVideoFeed` lookup table.
    pub fn youtube_feeds(&mut self) -> Result<Vec<(i32, String)>> {
        self.rt.block_on(async {
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
        })
    }

    /// ID of the `YoutubeVideoFeed` row for this channel, adding the row if it isn't there yet.
    /// An existing row keeps its URL; a missing channel name is filled in.
    pub fn ensure_youtube_feed(&mut self, channel_id: &str, name: Option<&str>, url: &str) -> Result<i32> {
        self.rt.block_on(async {
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
        })
    }

    /// Deletes each channel's oldest videos so at most `keep` remain per channel.
    /// Returns the number of rows deleted.
    pub fn prune_videos(&mut self, keep: u32) -> Result<u64> {
        let keep = keep as i32;
        self.rt.block_on(async {
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
        })
    }
}

#[cfg(test)]
mod movie_tests {
    use super::*;

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn movie_upserts_are_repeatable_and_reschedule_in_place() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        db.rt.block_on(async { db.client.simple_query("BEGIN TRANSACTION").await?.into_results().await })?;
        let result = (|| -> Result<()> {
            let report = serde_json::json!({"issues": []});
            db.save_movie_releases(&[], &report)?;
            db.rt.block_on(db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='dvdsreleasedates'", &[]))?;
            let mut row = crate::movies::MovieRelease {
                source: "dvdsreleasedates".into(), external_id: "integration-fixture-movie".into(),
                title: "Integration fixture".into(), imdb_id: None, key: "disc:upc:fixture".into(),
                kind: "disc".into(), date: chrono::NaiveDate::from_ymd_opt(2026,10,6).unwrap(),
                url: "https://example.invalid/integration-fixture".into(), format: Some("DVD".into()), platform: None,
            };
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            row.date = chrono::NaiveDate::from_ymd_opt(2026,10,13).unwrap();
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            db.rt.block_on(async {
                let rows = db.client.query("SELECT r.ReleaseDate FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId WHERE l.ExternalMovieId='integration-fixture-movie'", &[]).await?.into_first_result().await?;
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].get::<chrono::NaiveDate,_>(0), Some(row.date));
                db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='dvdsreleasedates'", &[]).await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            })?;
            assert_eq!(db.save_movie_releases(&[row], &report)?, 0);
            Ok(())
        })();
        db.rt.block_on(async { db.client.simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION").await?.into_results().await })?;
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
        db.rt.block_on(async { db.client.simple_query("BEGIN TRANSACTION").await?.into_results().await })?;
        let result = (|| -> Result<()> {
            let kansas = db.ensure_sports_events_type("https://example.invalid/logo-test-kansas", "Logo test Kansas")?;
            let other = db.ensure_sports_events_type("https://example.invalid/logo-test-other", "Logo test other")?;
            let fixture = |id| Event {
                url: format!("https://example.invalid/logo-test-event-{id}"), game_id: Some(id),
                title: "Logo test event".into(), sport: None, opponent: Some("Opponent".into()),
                is_away: Some(false), location: None,
                event_date: chrono::NaiveDate::from_ymd_opt(2026,10,10).unwrap(),
                starts_at: None, ends_at: None, time_tbd: true, tv: None,
                stream_url: None, live_stats_url: None,
                team_logo_url: Some("http://big12sports.com/images/logos/site/site.png".into()),
                opponent_logo_url: Some("https://example.invalid/opponent.png".into()),
            };
            let mut current = fixture(1);
            db.save_events(kansas, &[fixture(1), fixture(2)])?;
            db.save_events(other, &[fixture(3)])?;
            let before = db.rt.block_on(async { db.client.query("SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE Url=@P1", &[&current.url]).await?.into_first_result().await })?;
            let old_id = before[0].get::<i32,_>(0);
            let old_seen = before[0].get::<chrono::NaiveDateTime,_>(1);
            current.team_logo_url = Some("https://example.invalid/jayhawk.png".into());
            db.save_events(kansas, &[current])?;
            assert_eq!(db.refresh_school_team_logo(kansas, Some("https://example.invalid/jayhawk.png"))?, 1);
            assert_eq!(db.refresh_school_team_logo(kansas, Some("https://example.invalid/jayhawk.png"))?, 0);
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
                Ok::<_,Box<dyn std::error::Error>>(())
            })?;
            assert_eq!(db.refresh_school_team_logo(kansas, None)?,2);
            assert_eq!(db.refresh_school_team_logo(kansas, None)?,0);
            Ok(())
        })();
        db.rt.block_on(async { db.client.simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION").await?.into_results().await })?;
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
    Ok::<_,Box<dyn std::error::Error>>(())
   })?;
            db.event_result(utah, true, 1, 0, None)?;
            db.event_attempt(utah)?;
            db.event_result(utah, false, 0, 1, Some("fixture feed failure"))?;
            db.rt.block_on(async {
    let rows=db.client.query("SELECT LastAttemptAt,LastSuccessAt,LastError FROM dbo.SportsEventsType WHERE ID=@P1",&[&utah]).await?.into_first_result().await?;
    assert!(rows[0].get::<chrono::NaiveDateTime,_>(0).is_some());assert!(rows[0].get::<chrono::NaiveDateTime,_>(1).is_some());assert_eq!(rows[0].get::<&str,_>(2),Some("fixture feed failure"));
    Ok::<_,Box<dyn std::error::Error>>(())
   })?;
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                        &[&utah],
                    )
                    .await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            })?;
            assert_eq!(db.save_big12_observation(utah, &b)?, "disabled");
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ID=@P1",
                        &[&utah],
                    )
                    .await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            })?;
            let mut bad = b;
            bad["issue"] = serde_json::json!("identity unavailable");
            bad["event"]["url"] = serde_json::json!("https://example.invalid/broken");
            assert_eq!(db.save_big12_observation(utah, &bad)?, "quarantined");
            db.rt.block_on(async {
    let rows=db.client.query("SELECT Payload,SourceUrl FROM dbo.SportsEventSources WHERE SportsEventsTypeId=@P1 AND ProviderGameId=179872",&[&utah]).await?.into_first_result().await?;
    let payload:serde_json::Value=serde_json::from_str(rows[0].get::<&str,_>(0).unwrap())?;assert!(payload["issue"].is_null());assert_eq!(rows[0].get::<&str,_>(1),Some("http://big12sports.com/calendar.aspx?id=179872"));
    Ok::<_,Box<dyn std::error::Error>>(())
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
    #[ignore="requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn poster_backfill_preserves_rows_and_concurrent_valid_posters()->Result<()> {
        let _=dotenvy::dotenv();let mut db=Db::open(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        db.rt.block_on(async{db.client.simple_query("BEGIN TRANSACTION").await?.into_results().await})?;
        let result=(||->Result<()> {
            let report=serde_json::json!({"issues":[]});
            db.save_movie_releases(&[],&report)?;
            db.rt.block_on(async{db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='dvdsreleasedates'",&[]).await?;Ok::<_,Box<dyn std::error::Error>>(())})?;
            let row=crate::movies::MovieRelease{source:"dvdsreleasedates".into(),external_id:"poster-regression-fixture".into(),title:"Poster regression fixture".into(),imdb_id:None,key:"theatrical:first".into(),kind:"theatrical".into(),date:chrono::NaiveDate::from_ymd_opt(2026,10,9).unwrap(),url:"https://example.invalid/poster-fixture".into(),format:None,platform:None};
            db.save_movie_releases(&[row.clone()],&report)?;
            let job=db.poster_jobs()?.into_iter().find(|j|j.identity.title==row.title).unwrap();
            let poster="https://image.tmdb.org/t/p/w500/fixture.jpg";
            assert!(db.set_movie_poster(job.movie_id,None,Some(poster),Some(1947),"matched",Some(&job.url),None,30)?);
            assert!(!db.set_movie_poster(job.movie_id,None,Some("https://image.tmdb.org/t/p/w500/wrong-remake.jpg"),Some(2026),"matched",Some(&job.url),None,30)?);
            let metadata=serde_json::json!({"title":row.title,"year":1947,"imdb_id":null,"tmdb_id":null,"posters":[poster]});
            db.cache_poster_source(job.link_id,"fixture",&metadata,Some(poster),None)?;
            db.cache_poster_source(job.link_id,"fixture",&metadata,Some(poster),None)?;
            db.save_movie_releases(&[row],&report)?;
            db.rt.block_on(async {
                let rows=db.client.query("SELECT PosterUrl,OriginalYear FROM dbo.Movies WHERE ID=@P1",&[&job.movie_id]).await?.into_first_result().await?;
                assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<&str,_>(0),Some(poster));assert_eq!(rows[0].get::<i16,_>(1),Some(1947));
                let rows=db.client.query("SELECT COUNT(*) AS Total FROM dbo.MovieReleases WHERE MovieSourceLinkId=@P1",&[&job.link_id]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(1));
                let rows=db.client.query("SELECT COUNT(*) FROM dbo.MoviePosterCache WHERE SourceLinkId=@P1",&[&job.link_id]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(1));
                Ok::<_,Box<dyn std::error::Error>>(())
            })?;Ok(())
        })();
        db.rt.block_on(async{db.client.simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION").await?.into_results().await})?;result
    }
}

#[cfg(test)]
mod tvmaze_tests {
 use super::*;
 #[test]
 #[ignore="requires named TVmaze staging database; test failures/status changes isolated to staging"]
 fn tv_snapshots_reschedule_preserve_history_and_disabled_settings()->Result<()> {
  let _=dotenvy::dotenv();let mut db=Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?,"WebScraper_Tvmaze_Validation_20261009")?;db.setup_tv(true)?;
  let before=db.rt.block_on(async{db.client.query("SELECT ID,FirstSeenAt FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await})?;
  let original_id=before[0].get::<i32,_>(0);let original_seen=before[0].get::<chrono::NaiveDateTime,_>(1);
  db.rt.block_on(async{db.client.simple_query("BEGIN TRANSACTION").await?.into_results().await})?;
  let phase=(||->Result<()> {
   let(s,mut e)=crate::tvmaze::tests::fixtures(83073);e[0]["airdate"]=serde_json::json!("2026-10-11");let(p,_)=crate::tvmaze::normalize(83073,&s,&e)?;
   for _ in 0..2 {let lease=db.claim_tv_series(83073,true)?.unwrap();assert_eq!(db.save_tv_snapshot(83073,&lease,&p)?,"saved");}
   db.rt.block_on(async {
    let rows=db.client.query("SELECT ID,FirstSeenAt,Airdate,StartsAtUtc,IsDateOnly FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await?;
    assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<i32,_>(0),original_id);assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>(1),original_seen);assert_eq!(rows[0].get::<chrono::NaiveDate,_>(2),chrono::NaiveDate::from_ymd_opt(2026,10,11));assert!(rows[0].get::<chrono::NaiveDateTime,_>(3).is_none());assert_eq!(rows[0].get::<bool,_>(4),Some(true));
    db.client.execute("UPDATE dbo.TvTrackedSeries SET Enabled=0 WHERE TvmazeShowId=83073",&[]).await?;Ok::<_,Box<dyn std::error::Error>>(())
   })?;
   db.setup_tv(true)?;assert!(!db.tv_jobs()?.iter().any(|(id,_)|*id==83073));assert!(db.claim_tv_series(83073,true)?.is_none());assert_eq!(db.save_tv_snapshot(83073,"00000000-0000-0000-0000-000000000000",&p)?,"disabled_or_superseded");Ok(())
  })();
  db.rt.block_on(async{db.client.simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION").await?.into_results().await})?;phase?;
  let(s,mut e)=crate::tvmaze::tests::fixtures(83073);e.as_array_mut().unwrap().pop();let(p,_)=crate::tvmaze::normalize(83073,&s,&e)?;let lease=db.claim_tv_series(83073,true)?.unwrap();assert!(db.save_tv_snapshot(83073,&lease,&p).is_err());db.fail_tv_series(83073,&lease,"fixture incomplete response",900)?;
  let lease=db.claim_tv_series(64950,true)?.unwrap();db.fail_tv_series(64950,&lease,"fixture HTTP 503",900)?;
  let(s,e)=crate::tvmaze::tests::fixtures(45039);let(p,_)=crate::tvmaze::normalize(45039,&s,&e)?;let lease=db.claim_tv_series(45039,true)?.unwrap();assert_eq!(db.save_tv_snapshot(45039,&lease,&p)?,"saved");
  db.rt.block_on(async {
   let rows=db.client.query("SELECT COUNT(*) FROM dbo.TvEpisodes",&[]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(90));
   let rows=db.client.query("SELECT LastSuccessAt,EpisodeCount,LastError FROM dbo.TvTrackedSeries WHERE TvmazeShowId=83073",&[]).await?.into_first_result().await?;assert!(rows[0].get::<chrono::NaiveDateTime,_>(0).is_some());assert_eq!(rows[0].get::<i32,_>(1),Some(13));assert_eq!(rows[0].get::<&str,_>(2),Some("fixture incomplete response"));
   let rows=db.client.query("SELECT ID,FirstSeenAt,Airdate FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),original_id);assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>(1),original_seen);assert_eq!(rows[0].get::<chrono::NaiveDate,_>(2),chrono::NaiveDate::from_ymd_opt(2026,10,9));
   Ok::<_,Box<dyn std::error::Error>>(())
  })?;Ok(())
 }
}
