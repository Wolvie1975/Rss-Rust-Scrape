//! Explicit SQL setup/audit helper. SQL comes from a reviewed file; target is always WebScraper.
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = std::env::args().nth(1).ok_or("profile required")?;
    dotenvy::from_filename(profile)?;
    let sql = std::fs::read_to_string(std::env::args().nth(2).ok_or("SQL file required")?)?;
    let mut cfg = Config::from_ado_string(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
    cfg.database("WebScraper");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tcp = TcpStream::connect(cfg.get_addr()).await?;
            let mut db = Client::connect(cfg, tcp.compat_write()).await?;
            for rows in db.simple_query(sql).await?.into_results().await? {
                for row in rows {
                    let mut out = serde_json::Map::new();
                    for (i, c) in row.columns().iter().enumerate() {
                        let val = if let Ok(v) = row.try_get::<&str, _>(i) {
                            serde_json::json!(v)
                        } else if let Ok(v) = row.try_get::<i32, _>(i) {
                            serde_json::json!(v)
                        } else if let Ok(v) = row.try_get::<i64, _>(i) {
                            serde_json::json!(v)
                        } else if let Ok(v) = row.try_get::<bool, _>(i) {
                            serde_json::json!(v)
                        } else {
                            serde_json::Value::Null
                        };
                        out.insert(c.name().into(), val);
                    }
                    println!("{}", serde_json::Value::Object(out));
                }
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
}
