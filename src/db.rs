use chrono::{DateTime, Utc};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::PageMeta;

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
}
