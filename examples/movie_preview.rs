//! Preview live movie releases without opening SQL Server.
#[allow(dead_code)]
#[path="../src/movie_metadata.rs"]
mod movie_metadata;
#[allow(dead_code)] // This example uses only the read-only portion of the shared module.
#[path = "../src/movies.rs"]
mod movies;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    /// Any date in the desired Monday-Sunday week (YYYY-MM-DD).
    #[arg(long)]
    week_of: Option<NaiveDate>,
    /// Save a JSON report, including failures and exclusions.
    #[arg(long)]
    output: Option<PathBuf>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let day = args.week_of.unwrap_or_else(|| Utc::now().with_timezone(&chrono_tz::America::Chicago).date_naive());
    let start = day - Duration::days(day.weekday().num_days_from_monday().into());
    let report = movies::preview(start)?;
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = args.output { std::fs::write(path, &json)?; }
    println!("{json}");
    if report["issues"].as_array().is_some_and(|v| !v.is_empty()) {
        return Err("preview has unresolved issues; review the saved report".into());
    }
    Ok(())
}
