use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use rss::{ChannelBuilder, ItemBuilder};
use scraper::{Html, Selector};

/// Metadata scraped from a single page.
struct PageMeta {
    url: String,
    title: String,
    description: Option<String>,
    published: Option<DateTime<Utc>>,
}

/// Returns the `content` of the first `<meta>` tag matching `selector`.
fn meta_content(doc: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn scrape(url: &str) -> Result<PageMeta, Box<dyn std::error::Error>> {
    let body = reqwest::blocking::get(url)?.error_for_status()?.text()?;
    let doc = Html::parse_document(&body);

    let title = meta_content(&doc, r#"meta[property="og:title"]"#)
        .or_else(|| {
            let sel = Selector::parse("title").ok()?;
            let t: String = doc.select(&sel).next()?.text().collect();
            Some(t.trim().to_string())
        })
        .unwrap_or_else(|| url.to_string());

    let description = meta_content(&doc, r#"meta[property="og:description"]"#)
        .or_else(|| meta_content(&doc, r#"meta[name="description"]"#));

    let canonical = Selector::parse(r#"link[rel="canonical"]"#)
        .ok()
        .and_then(|sel| {
            doc.select(&sel)
                .next()
                .and_then(|el| el.value().attr("href").map(str::to_string))
        })
        .and_then(|href| url::Url::parse(url).ok()?.join(&href).ok())
        .map(|u| u.to_string())
        .unwrap_or_else(|| url.to_string());

    let published = meta_content(&doc, r#"meta[property="article:published_time"]"#)
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));

    Ok(PageMeta {
        url: canonical,
        title,
        description,
        published,
    })
}

/// Build an RSS feed from the metadata of scraped web pages.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Page URLs to include in the feed
    urls: Vec<String>,

    /// File with one URL per line (blank lines and lines starting with # are ignored)
    #[arg(short = 'f', long, value_name = "FILE")]
    urls_file: Option<PathBuf>,

    /// Feed title
    #[arg(short, long, default_value = "Scraped Feed")]
    title: String,

    /// Feed link (the website the feed represents)
    #[arg(short, long, default_value = "https://example.com")]
    link: String,

    /// Feed description
    #[arg(short, long, default_value = "Feed generated from scraped page metadata")]
    description: String,

    /// Write the feed to FILE instead of stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

fn read_urls_file(path: &Path) -> std::io::Result<Vec<String>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut urls = cli.urls;
    if let Some(path) = &cli.urls_file {
        urls.extend(read_urls_file(path)?);
    }
    if urls.is_empty() {
        Cli::command().error(ErrorKind::MissingRequiredArgument, "provide at least one URL or --urls-file").exit();
    }

    let mut items = Vec::new();
    for url in &urls {
        match scrape(url) {
            Ok(meta) => items.push(
                ItemBuilder::default()
                    .title(meta.title)
                    .link(meta.url.clone())
                    .description(meta.description)
                    .pub_date(meta.published.map(|d| d.to_rfc2822()))
                    .guid(rss::GuidBuilder::default().value(meta.url).permalink(true).build())
                    .build(),
            ),
            Err(e) => eprintln!("skipping {url}: {e}"),
        }
    }

    let channel = ChannelBuilder::default()
        .title(cli.title)
        .link(cli.link)
        .description(cli.description)
        .last_build_date(Some(Utc::now().to_rfc2822()))
        .items(items)
        .build();

    match cli.output {
        Some(path) => std::fs::write(path, channel.to_string())?,
        None => println!("{channel}"),
    }
    Ok(())
}
