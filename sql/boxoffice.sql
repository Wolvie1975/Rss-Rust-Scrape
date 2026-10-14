IF OBJECT_ID('dbo.BoxOfficeWeekend','U') IS NULL
CREATE TABLE dbo.BoxOfficeWeekend (
 Rank INT NOT NULL PRIMARY KEY CHECK(Rank BETWEEN 1 AND 10),
 WeekendStart DATE NOT NULL, MovieTitle NVARCHAR(300) NOT NULL,
 WeekendGross BIGINT NOT NULL CHECK(WeekendGross>=0), TotalDomesticGross BIGINT NOT NULL CHECK(TotalDomesticGross>=0),
 WeeklyChangePercent FLOAT NULL, Theaters INT NOT NULL,
 MovieUrl NVARCHAR(2048) NOT NULL, SourceUrl NVARCHAR(2048) NOT NULL,
 FetchedAt DATETIME2 NOT NULL
);
IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NOT NULL
 GRANT SELECT,INSERT,DELETE ON dbo.BoxOfficeWeekend TO WebScraperIngest;
