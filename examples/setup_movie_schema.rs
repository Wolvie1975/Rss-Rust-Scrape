//! Apply only the movie schema without running scrapes or other schema changes.
//! Run from the project directory: cargo run --example setup_movie_schema
//! Append -- --verify to run SQL integration checks (fixtures are rolled back).
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.is_empty() && args != ["--verify"] {
        return Err("usage: setup_movie_schema [--verify]".into());
    }
    let verify = !args.is_empty();
    let _ = dotenvy::dotenv();
    let ado = std::env::var("MSSQL_CONNECTION_STRING")
        .map_err(|_| "MSSQL_CONNECTION_STRING must be set")?;
    let mut config = Config::from_ado_string(&ado)?;
    config.database("WebScraper");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tcp = TcpStream::connect(config.get_addr()).await?;
            tcp.set_nodelay(true)?;
            let mut client = Client::connect(config, tcp.compat_write()).await?;
            client
                .simple_query(include_str!("../sql/movie_releases.sql"))
                .await?
                .into_results()
                .await?;
            println!("Movie release schema ready in WebScraper (4 tables).");
            if verify {
                client
                    .simple_query(include_str!("../tests/sql/movie_releases.sql"))
                    .await?
                    .into_results()
                    .await?;
                println!("SQL integration checks passed; test rows rolled back.");
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
}
