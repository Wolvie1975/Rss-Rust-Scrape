//! Read-only provider verification; optionally save the normalized snapshot JSON.
use web_scraper::{f1, http};
fn main() -> Result<(), web_scraper::Error> {
    let client = http::client_builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let snapshot = f1::fetch(&client)?;
    println!("{}", snapshot.summary());
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, snapshot.payload())?;
    }
    Ok(())
}
