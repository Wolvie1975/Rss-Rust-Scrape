use tiberius::{Client,Config};use tokio::net::TcpStream;use tokio_util::compat::TokioAsyncWriteCompatExt;
fn main()->Result<(),Box<dyn std::error::Error>>{
 let _=dotenvy::dotenv();let args:Vec<_>=std::env::args().skip(1).collect();let mut cfg=Config::from_ado_string(&std::env::var("MSSQL_CONNECTION_STRING")?)?;cfg.database(args.first().map(String::as_str).unwrap_or("WebScraper"));
 tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async{
 let tcp=TcpStream::connect(cfg.get_addr()).await?;let mut c=Client::connect(cfg,tcp.compat_write()).await?;let mut out=serde_json::Map::new();
 for (key,sql) in [
 ("tables","SELECT name FROM sys.tables ORDER BY name FOR JSON PATH"),
 ("columns","SELECT t.name AS TableName,c.name,ty.name AS Type,c.max_length,c.is_nullable,dc.definition AS DefaultValue FROM sys.tables t JOIN sys.columns c ON c.object_id=t.object_id JOIN sys.types ty ON ty.user_type_id=c.user_type_id LEFT JOIN sys.default_constraints dc ON dc.object_id=c.default_object_id WHERE t.name LIKE '%Series%' OR t.name LIKE '%Episode%' OR t.name IN('MovieReleaseSources','SportsEventsType') ORDER BY t.name,c.column_id FOR JSON PATH"),
 ("series","IF OBJECT_ID('dbo.TvSeries','U') IS NOT NULL SELECT * FROM dbo.TvSeries ORDER BY ID FOR JSON PATH"),
 ("episodes","IF OBJECT_ID('dbo.TvEpisodes','U') IS NOT NULL SELECT * FROM dbo.TvEpisodes ORDER BY ID FOR JSON PATH"),
 ("tracking","IF OBJECT_ID('dbo.TvTrackedSeries','U') IS NOT NULL SELECT ID,TvmazeShowId,ExpectedTitle,Enabled,LastAttemptAt,LastSuccessAt,LastError,EpisodeCount,LastIssueCount,CacheExpiresAt,RetryAfter FROM dbo.TvTrackedSeries ORDER BY ID FOR JSON PATH")
 ] {let rows=c.query(sql,&[]).await?.into_first_result().await?;let s:String=rows.iter().filter_map(|r|r.get::<&str,_>(0)).collect();out.insert(key.into(),if s.is_empty(){serde_json::json!([])}else{serde_json::from_str(&s)?});}
 let s=serde_json::to_string_pretty(&out)?;if let Some(path)=args.get(1){std::fs::write(path,s)?;}else{println!("{s}");}println!("{} series; {} episodes; {} tracking rows",out["series"].as_array().unwrap().len(),out["episodes"].as_array().unwrap().len(),out["tracking"].as_array().unwrap().len());Ok::<_,Box<dyn std::error::Error>>(())
 })}
