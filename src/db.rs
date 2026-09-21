use chrono::{DateTime, Utc};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::PageMeta;
use crate::events::Event;
use crate::youtube::Video;

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

/// A single SQL Server session on the `WebScraper` database.
pub struct Db {
    rt: Runtime,
    client: Conn,
}

impl Db {
    /// Connects, creating the database and its tables first if they are missing.
    pub fn open(ado: &str) -> Result<Db> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = rt.block_on(async {
            let mut master = connect(ado, "master").await?;
            master
                .execute(
                    format!("IF DB_ID('{DB_NAME}') IS NULL CREATE DATABASE [{DB_NAME}]"),
                    &[],
                )
                .await?;
            drop(master);

            let mut client = connect(ado, DB_NAME).await?;
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
            Result::Ok(client)
        })?;
        Ok(Db { rt, client })
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

    /// Upserts each video keyed on its YouTube video ID, linked to its `YoutubeVideoFeed` row. Title, description and the view/rating
    /// counts change over time, so they are overwritten; `FirstSeenAt` is kept.
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
                             ChannelId = @P2, ChannelName = @P3, Title = @P4, Url = @P5,
                             PublishedAt = @P6, UpdatedAt = @P7, ThumbnailUrl = @P8, Description = @P9,
                             ViewCount = @P10, RatingCount = @P11, RatingAverage = @P12,
                             YoutubeVideoFeedId = @P13, LastSeenAt = SYSUTCDATETIME()
                         WHEN NOT MATCHED THEN INSERT
                             (VideoId, ChannelId, ChannelName, Title, Url, PublishedAt, UpdatedAt,
                              ThumbnailUrl, Description, ViewCount, RatingCount, RatingAverage,
                              YoutubeVideoFeedId)
                             VALUES (@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10, @P11, @P12, @P13);",
                        &[
                            &v.video_id, &v.channel_id, &v.channel_name, &title, &v.url,
                            &v.published_at, &v.updated_at, &v.thumbnail_url, &v.description,
                            &v.views, &v.rating_count, &v.rating_average, &feed_id,
                        ],
                    )
                    .await?;
            }
            Ok(videos.len())
        })
    }

    /// `(ID, RssUrl)` of every calendar feed in the `SportsEventsType` lookup table.
    pub fn sports_event_feeds(&mut self) -> Result<Vec<(i32, String)>> {
        self.rt.block_on(async {
            let rows = self
                .client
                .query("SELECT ID, RssUrl FROM dbo.SportsEventsType ORDER BY ID", &[])
                .await?
                .into_first_result()
                .await?;
            Ok(rows
                .iter()
                .filter_map(|r| Some((r.get::<i32, _>(0)?, r.get::<&str, _>(1)?.to_string())))
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
