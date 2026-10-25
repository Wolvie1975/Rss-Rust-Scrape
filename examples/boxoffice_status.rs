//! Explicit additive migration, isolated SQL checks, and read-only live status audit.
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
fn main() -> Result<(), web_scraper::Error> {
    dotenvy::dotenv().ok();
    let migrate = std::env::args().any(|a| a == "--migrate");
    let mut cfg = Config::from_ado_string(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
    cfg.database("WebScraper");
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let tcp = TcpStream::connect(cfg.get_addr()).await?;
        let mut db = Client::connect(cfg, tcp.compat_write()).await?;
        if migrate {
            for _ in 0..2 { db.simple_query(include_str!("../sql/boxoffice_status.sql")).await?.into_results().await?; }
            println!("Additive migration applied twice successfully.");
        }
        db.simple_query("SELECT TOP(0) * INTO #BoxOfficeWeekend FROM dbo.BoxOfficeWeekend").await?.into_results().await?;
        let sql = include_str!("../sql/boxoffice_replace.sql").replace("dbo.BoxOfficeWeekend", "#BoxOfficeWeekend");
        let rows = |status: &str, gross: i64| serde_json::to_string(&(1..=10).map(|n| serde_json::json!({"rank":n,"title":format!("Fixture {n}"),"weekendGross":gross,"totalDomesticGross":gross,"theaters":10,"movieUrl":format!("https://example.com/{n}"),"resultStatus":status})).collect::<Vec<_>>()).unwrap();
        for (date,status,gross,ok) in [
            ("2026-10-09","estimated",100,true),
            ("2026-10-09","final",110,true),
            ("2026-10-09","final",111,true),
            ("2026-10-09","estimated",100,false),
            ("2026-10-09","unknown",100,false),
            ("2026-10-02","final",100,false),
            ("2026-10-16","estimated",90,true),
        ] {
            let json = rows(status,gross);
            let result = db.execute(sql.as_str(), &[&date,&json,&"https://example.com/chart"]).await;
            assert_eq!(result.is_ok(),ok,"{date} {status}");
            let r = db.query("SELECT COUNT(*) AS N,MIN(WeekendGross) AS Gross FROM #BoxOfficeWeekend",&[]).await?.into_row().await?.unwrap();
            assert_eq!(r.get::<i32,_>("N"),Some(10));
            assert_eq!(r.get::<i64,_>("Gross"),Some(if ok {gross} else {111}));
        }
        // Verify complete preservation on malformed input and unknown/older publication times.
        db.simple_query("UPDATE #BoxOfficeWeekend SET ResultStatus='final', StatusAsOfUtc='2026-10-19T12:00:00', PosterUrl='https://example.com/poster', PosterLookupStatus='found'").await?.into_results().await?;
        let baseline = db.query("SELECT * FROM #BoxOfficeWeekend ORDER BY Rank FOR JSON PATH, INCLUDE_NULL_VALUES",&[]).await?.into_row().await?.unwrap().get::<&str,_>(0).unwrap().to_owned();
        for payload in ["[]".to_owned(),rows("final",95),rows("invalid",95), {
            let mut data: serde_json::Value=serde_json::from_str(&rows("final",95))?;
            for row in data.as_array_mut().unwrap() { row["statusAsOfUtc"]=serde_json::json!("2026-10-18T12:00:00"); }
            data.to_string()
        }] {
            assert!(db.execute(sql.as_str(),&[&"2026-10-16",&payload,&"https://example.com/chart"]).await.is_err());
            let after=db.query("SELECT * FROM #BoxOfficeWeekend ORDER BY Rank FOR JSON PATH, INCLUDE_NULL_VALUES",&[]).await?.into_row().await?.unwrap().get::<&str,_>(0).unwrap().to_owned();
            assert_eq!(baseline,after,"failed refresh changed the snapshot");
        }
        let mut revision:serde_json::Value=serde_json::from_str(&rows("final",96))?;
        for row in revision.as_array_mut().unwrap(){row["statusAsOfUtc"]=serde_json::json!("2026-10-19T13:00:00");}
        db.execute(sql.as_str(),&[&"2026-10-16",&revision.to_string(),&"https://example.com/chart"]).await?;
        let preserved=db.query("SELECT COUNT(*) FROM #BoxOfficeWeekend WHERE PosterUrl='https://example.com/poster' AND WeekendGross=96 AND StatusAsOfUtc='2026-10-19T13:00:00'",&[]).await?.into_row().await?.unwrap();
        assert_eq!(preserved.get::<i32,_>(0),Some(10));
        println!("Twelve isolated SQL checks passed, including exact failed-snapshot preservation, publication guards and poster preservation; no production rows changed by tests.");
        let rows=db.query("SELECT Rank,WeekendStart,MovieTitle,WeekendGross,ResultStatus,StatusSourceUrl,StatusAsOfUtc,FetchedAt FROM dbo.BoxOfficeWeekend ORDER BY Rank FOR JSON PATH, INCLUDE_NULL_VALUES",&[]).await?.into_first_result().await?;
        let json: String = rows.iter().filter_map(|r| r.get::<&str,_>(0)).collect();
        println!("{json}");
        Ok::<_,web_scraper::Error>(())
    })
}
