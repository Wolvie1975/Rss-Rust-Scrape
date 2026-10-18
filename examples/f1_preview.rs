//! Read-only provider verification; optionally save the normalized snapshot JSON.
#![allow(dead_code)]
#[path = "../src/f1.rs"]
mod f1;
#[path = "../src/http.rs"]
mod http;
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
