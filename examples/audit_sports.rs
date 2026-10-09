//! Read-only sports schema/import audit. Usage: audit_sports [DATABASE] [OUTPUT.json]
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let mut cfg = Config::from_ado_string(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    cfg.database(args.first().map(String::as_str).unwrap_or("WebScraper"));
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
  let tcp=TcpStream::connect(cfg.get_addr()).await?;let mut c=Client::connect(cfg,tcp.compat_write()).await?;
  let mut audit=serde_json::Map::new();
  for (name,sql) in [
   ("feeds","SELECT * FROM dbo.SportsEventsType ORDER BY ID FOR JSON PATH"),
   ("events","SELECT * FROM dbo.SportsEvents ORDER BY ID FOR JSON PATH"),
   ("sources","IF OBJECT_ID('dbo.SportsEventSources','U') IS NOT NULL SELECT ID,SportsEventId,SportsEventsTypeId,ProviderGameId,SchoolId,SchoolName,OpponentSchoolId,Issue,JSON_VALUE(Payload,'$.warning') AS Warning,FirstSeenAt,LastSeenAt FROM dbo.SportsEventSources ORDER BY ID FOR JSON PATH"),
   ("canonical_duplicates","IF COL_LENGTH('dbo.SportsEvents','CanonicalKey') IS NOT NULL EXEC(N'SELECT CanonicalKey,COUNT(*) AS Rows FROM dbo.SportsEvents WHERE CanonicalKey IS NOT NULL GROUP BY CanonicalKey HAVING COUNT(*)>1 FOR JSON PATH')"),
   ("school_calendar_counts","IF OBJECT_ID('dbo.SportsEventSchoolCalendar','V') IS NOT NULL SELECT SchoolId,SchoolName,COUNT(*) AS Rows FROM dbo.SportsEventSchoolCalendar GROUP BY SchoolId,SchoolName ORDER BY SchoolName FOR JSON PATH"),
   ("mirror_example","IF OBJECT_ID('dbo.SportsEventSchoolCalendar','V') IS NOT NULL SELECT SportsEventId,ProviderGameId,SchoolId,SchoolName,Opponent,IsAway,IsNeutral,EventDate,StartsAtUtc,TeamLogoUrl,OpponentLogoUrl,Tv FROM dbo.SportsEventSchoolCalendar WHERE ProviderGameId IN(179871,179872) ORDER BY SchoolId FOR JSON PATH"),
   ("neutral_consistency","IF OBJECT_ID('dbo.SportsEventSchoolCalendar','V') IS NOT NULL SELECT COUNT(*) AS InvalidRows FROM dbo.SportsEventSchoolCalendar WHERE IsNeutral=1 AND IsAway IS NOT NULL FOR JSON PATH"),
  ] {
   let rows=c.query(sql,&[]).await?.into_first_result().await?;
   let text:String=rows.iter().filter_map(|r|r.get::<&str,_>(0)).collect();
   audit.insert(name.into(),if text.is_empty(){serde_json::json!([])}else{serde_json::from_str(&text)?});
  }
  let text=serde_json::to_string_pretty(&audit)?;
  if let Some(path)=args.get(1) {std::fs::write(path,text)?;}else{println!("{text}");}
  println!("{} event rows; {} feed rows; {} source rows; {} duplicate canonical keys",audit["events"].as_array().unwrap().len(),audit["feeds"].as_array().unwrap().len(),audit["sources"].as_array().unwrap().len(),audit["canonical_duplicates"].as_array().unwrap().len());
  Ok::<_,Box<dyn std::error::Error>>(())
 })
}
