//! Read-only movie audit: audit_movies [DATABASE] [OUTPUT.json]
use tiberius::{Client,Config};use tokio::net::TcpStream;use tokio_util::compat::TokioAsyncWriteCompatExt;
fn main()->Result<(),Box<dyn std::error::Error>>{
 let _=dotenvy::dotenv();let args:Vec<_>=std::env::args().skip(1).collect();let mut cfg=Config::from_ado_string(&std::env::var("MSSQL_CONNECTION_STRING")?)?;cfg.database(args.first().map(String::as_str).unwrap_or("WebScraper"));
 tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
  let tcp=TcpStream::connect(cfg.get_addr()).await?;let mut client=Client::connect(cfg,tcp.compat_write()).await?;let mut out=serde_json::Map::new();
  for (key,sql) in [("movies","SELECT * FROM dbo.Movies ORDER BY ID FOR JSON PATH"),("links","SELECT l.*,s.SourceKey FROM dbo.MovieSourceLinks l JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId ORDER BY l.ID FOR JSON PATH"),("releases","SELECT * FROM dbo.MovieReleases ORDER BY ID FOR JSON PATH"),("schema","SELECT c.name,t.name AS Type,c.max_length,c.is_nullable FROM sys.columns c JOIN sys.types t ON t.user_type_id=c.user_type_id WHERE c.object_id=OBJECT_ID('dbo.Movies') ORDER BY c.column_id FOR JSON PATH")] {
   let rows=client.query(sql,&[]).await?.into_first_result().await?;let s:String=rows.iter().filter_map(|r|r.get::<&str,_>(0)).collect();out.insert(key.into(),if s.is_empty(){serde_json::json!([])}else{serde_json::from_str(&s)?});
  }
  let s=serde_json::to_string_pretty(&out)?;if let Some(path)=args.get(1){std::fs::write(path,s)?;}else{println!("{s}");}
  println!("{} movies, {} source links, {} releases, {} posters",out["movies"].as_array().unwrap().len(),out["links"].as_array().unwrap().len(),out["releases"].as_array().unwrap().len(),out["movies"].as_array().unwrap().iter().filter(|r|r["PosterUrl"].is_string()).count());Ok::<_,Box<dyn std::error::Error>>(())
 })
}
