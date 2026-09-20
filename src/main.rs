use std::path::{Path, PathBuf};
use std::time::Duration;

mod db;
mod events;
mod feed;
mod youtube;

use chrono::{DateTime, Utc};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use reqwest::blocking::Client;
use rss::{ChannelBuilder, ItemBuilder};
use scraper::{Html, Selector};

/// Metadata scraped from a single page.
struct PageMeta {
    /// The URL that was requested (the row in `Sources` this page came from).
    source: String,
    url: String,
    title: String,
    description: Option<String>,
    published: Option<DateTime<Utc>>,
    image_url: Option<String>,
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

/// Best-effort lookup of a preview image, resolved to an absolute http(s) URL.
fn image_url(doc: &Html, page_url: &str) -> Option<String> {
    let raw = meta_content(doc, r#"meta[property="og:image"]"#)
        .or_else(|| meta_content(doc, r#"meta[property="og:image:secure_url"]"#))
        .or_else(|| meta_content(doc, r#"meta[name="twitter:image"]"#))
        .or_else(|| {
            let sel = Selector::parse(r#"link[rel="image_src"]"#).ok()?;
            let href = doc.select(&sel).next()?.value().attr("href")?.trim();
            (!href.is_empty()).then(|| href.to_string())
        })?;
    let resolved = url::Url::parse(page_url).ok()?.join(&raw).ok()?;
    matches!(resolved.scheme(), "http" | "https").then(|| resolved.to_string())
}

fn scrape(client: &Client, url: &str) -> Result<PageMeta, Box<dyn std::error::Error>> {
    let body = client.get(url).send()?.error_for_status()?.text()?;
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

    let image_url = image_url(&doc, url);

    Ok(PageMeta {
        source: url.to_string(),
        url: canonical,
        title,
        description,
        published,
        image_url,
    })
}

/// For feed entries that came without an image, tries each article page's own `og:image`.
/// An image several articles share is a site-wide logo, not the article's, so it is dropped.
fn fill_missing_images(client: &Client, pages: &mut [PageMeta]) {
    let mut found: Vec<(usize, String)> = Vec::new();
    for (i, p) in pages.iter().enumerate().filter(|(_, p)| p.image_url.is_none()) {
        let Ok(body) = client
            .get(&p.url)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.text())
        else {
            continue;
        };
        if let Some(img) = image_url(&Html::parse_document(&body), &p.url) {
            found.push((i, img));
        }
    }
    for (i, img) in &found {
        if found.iter().filter(|(_, other)| other == img).count() == 1 {
            pages[*i].image_url = Some(img.clone());
        }
    }
}

/// Scrapes one source: each entry of its RSS/Atom feed if it has one, otherwise the
/// page's own metadata (a single entry). Returns the pages and which method produced them.
fn scrape_source(
    client: &Client,
    url: &str,
) -> Result<(Vec<PageMeta>, &'static str), Box<dyn std::error::Error>> {
    let feed_err = match feed::fetch_entries(client, url) {
        Ok(entries) => return Ok((entries, "feed")),
        Err(e) => e,
    };
    match scrape(client, url) {
        Ok(page) => Ok((vec![page], "page metadata")),
        Err(e) => Err(format!("{feed_err}; page scrape failed: {e}").into()),
    }
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

    /// Also save scraped pages to SQL Server (connection string from MSSQL_CONNECTION_STRING)
    #[arg(long)]
    db: bool,

    /// Also scrape every enabled row of the Sources table (implies --db)
    #[arg(long)]
    from_db: bool,

    /// Only keep pages published within the last N days, counting today (UTC); 1 = today and yesterday.
    /// Pages with no published date are kept unless --require-date is set.
    #[arg(long, value_name = "N")]
    last_days: Option<u32>,

    /// With --last-days, also drop pages that have no published date
    #[arg(long, requires = "last_days")]
    require_date: bool,

    /// Keep at most N pages per source: each run saves only a source's N newest entries, then
    /// deletes older rows for that source from the Pages table (undated rows go first)
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    per_source: Option<u32>,

    /// Keep only the newest N pages: each run saves at most the N newest scraped pages, then
    /// deletes the oldest rows from the Pages table until N remain (undated rows go first)
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    keep: Option<u32>,

    /// Run forever, scraping every N hours aligned to midnight US Central time (N must divide 24;
    /// 3 = 00:00, 03:00, 06:00 ... Central). Errors in a run are logged and the loop continues.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=24))]
    every_hours: Option<u32>,

    /// Also ingest a Sidearm-style calendar RSS feed of games into the SportsEvents table
    /// (repeatable; implies --db). Runs alongside, and independently of, the page scrape.
    #[arg(long, value_name = "URL")]
    events_feed: Vec<String>,

    /// Also ingest a YouTube channel's Atom feed (https://www.youtube.com/feeds/videos.xml?channel_id=...)
    /// into the YouTubeVideos table (repeatable; implies --db)
    #[arg(long, value_name = "URL")]
    youtube_feed: Vec<String>,

    /// How many of each YouTube channel's newest videos to keep
    #[arg(long, value_name = "N", default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..))]
    youtube_latest: u32,

    /// Scrape and filter but write nothing to the database; print what would be saved
    #[arg(long, conflicts_with = "save_sources")]
    dry_run: bool,

    /// Add the given URLs / --urls-file entries to the Sources table (implies --db)
    #[arg(long)]
    save_sources: bool,
}

fn read_urls_file(path: &Path) -> std::io::Result<Vec<String>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect())
}

/// One full scrape: collect sources, scrape, filter, save, prune, and write the RSS feed.
fn run_once(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let use_db = cli.db || cli.from_db || cli.save_sources || !cli.events_feed.is_empty() || !cli.youtube_feed.is_empty();
    let mut db = if use_db {
        let ado = std::env::var("MSSQL_CONNECTION_STRING")
            .map_err(|_| "database options require MSSQL_CONNECTION_STRING to be set")?;
        Some(db::Db::open(&ado)?)
    } else {
        None
    };

    let mut urls = cli.urls.clone();
    if let Some(path) = &cli.urls_file {
        urls.extend(read_urls_file(path)?);
    }
    if cli.save_sources {
        let added = db.as_mut().unwrap().add_sources(&urls)?;
        eprintln!("added {added} new sources ({} already present)", urls.len() - added);
    }
    if cli.from_db {
        for url in db.as_mut().unwrap().enabled_sources()? {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }
    if urls.is_empty() && cli.events_feed.is_empty() && cli.youtube_feed.is_empty() {
        Cli::command().error(ErrorKind::MissingRequiredArgument, "provide at least one URL, --urls-file, --from-db, --events-feed, or --youtube-feed").exit();
    }

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; web_scraper/0.1)")
        .timeout(Duration::from_secs(15))
        .build()?;

    let mut pages = Vec::new();
    for url in &urls {
        let result = scrape_source(&client, url);
        if let Some(db) = db.as_mut().filter(|_| !cli.dry_run) {
            let err = result.as_ref().err().map(|e| e.to_string());
            db.record_result(url, err.as_deref())?;
        }
        match result {
            Ok((mut found, how)) => {
                let total = found.len();
                if let Some(cap) = cli.per_source {
                    found.sort_by(|a, b| b.published.cmp(&a.published));
                    found.truncate(cap as usize);
                }
                if how == "feed" {
                    let missing = found.iter().filter(|p| p.image_url.is_none()).count();
                    fill_missing_images(&client, &mut found);
                    let still = found.iter().filter(|p| p.image_url.is_none()).count();
                    if missing > 0 {
                        eprintln!("{url}: found {} of {missing} missing images on article pages", missing - still);
                    }
                }
                eprintln!("{url}: {total} pages via {how}, using {}", found.len());
                pages.append(&mut found);
            }
            Err(e) => eprintln!("skipping {url}: {e}"),
        }
    }

    if let Some(days) = cli.last_days {
        let cutoff = (Utc::now() - chrono::Duration::days(days.into()))
            .date_naive()
            .and_time(chrono::NaiveTime::MIN)
            .and_utc();
        let before = pages.len();
        pages.retain(|p| match p.published {
            Some(d) => d >= cutoff,
            None => !cli.require_date,
        });
        eprintln!(
            "date filter (published on/after {cutoff}): kept {}, dropped {}",
            pages.len(),
            before - pages.len()
        );
    }

    // Newest first; undated pages sort last.
    pages.sort_by(|a, b| b.published.cmp(&a.published));
    if let Some(keep) = cli.keep {
        pages.truncate(keep as usize);
    }

    if cli.dry_run {
        eprintln!("dry run: nothing written to the database. {} pages would be saved:", pages.len());
        for p in &pages {
            eprintln!(
                "  - {}\n    published: {}\n    image: {}\n    title: {}",
                p.url,
                p.published.map_or("(none)".into(), |d| d.to_rfc3339()),
                p.image_url.as_deref().unwrap_or("(none)"),
                p.title
            );
        }
    } else if let Some(db) = db.as_mut().filter(|_| !urls.is_empty()) {
        let n = db.save_pages(&pages)?;
        eprintln!("saved {n} pages to SQL Server");
        if let Some(cap) = cli.per_source {
            let pruned = db.prune_per_source(cap)?;
            eprintln!("pruned {pruned} old pages (keeping the newest {cap} per source)");
        }
        if let Some(keep) = cli.keep {
            let pruned = db.prune_pages(keep)?;
            eprintln!("pruned {pruned} old pages (keeping the newest {keep})");
        }
    }

    for feed_url in &cli.events_feed {
        // A broken calendar feed shouldn't stop the rest of the run.
        match events::fetch_events(&client, feed_url) {
            Ok(found) if cli.dry_run => {
                eprintln!("{feed_url}: {} events (dry run, not saved)", found.len());
                for e in &found {
                    eprintln!(
                        "  - {} | {} {} {} | {} | {}",
                        e.event_date,
                        e.sport.as_deref().unwrap_or("?"),
                        if e.is_away == Some(true) { "at" } else { "vs" },
                        e.opponent.as_deref().unwrap_or("?"),
                        e.starts_at.map_or("time TBD".into(), |t| t.to_rfc3339()),
                        e.tv.as_deref().unwrap_or("no TV listed"),
                    );
                }
            }
            Ok(found) => {
                let n = db.as_mut().unwrap().save_events(&found)?;
                eprintln!("{feed_url}: saved {n} events to SQL Server");
            }
            Err(e) => eprintln!("events feed {feed_url} failed: {e}"),
        }
    }

    for feed_url in &cli.youtube_feed {
        match youtube::fetch_latest(&client, feed_url, cli.youtube_latest as usize) {
            Ok(found) if cli.dry_run => {
                eprintln!("{feed_url}: {} videos (dry run, not saved)", found.len());
                for v in &found {
                    eprintln!(
                        "  - {} | {} | {} views | {}",
                        v.published_at.to_rfc3339(),
                        v.channel_name.as_deref().unwrap_or("?"),
                        v.views.map_or("?".into(), |n| n.to_string()),
                        v.title
                    );
                }
            }
            Ok(found) => {
                let db = db.as_mut().unwrap();
                let n = db.save_videos(&found)?;
                let pruned = db.prune_videos(cli.youtube_latest)?;
                eprintln!("{feed_url}: saved {n} videos, pruned {pruned} older (keeping the newest {} per channel)", cli.youtube_latest);
            }
            Err(e) => eprintln!("youtube feed {feed_url} failed: {e}"),
        }
    }

    let items: Vec<_> = pages
        .into_iter()
        .map(|meta| {
            ItemBuilder::default()
                .title(meta.title)
                .link(meta.url.clone())
                .description(meta.description)
                .pub_date(meta.published.map(|d| d.to_rfc2822()))
                .guid(rss::GuidBuilder::default().value(meta.url).permalink(true).build())
                .build()
        })
        .collect();

    let channel = ChannelBuilder::default()
        .title(cli.title.clone())
        .link(cli.link.clone())
        .description(cli.description.clone())
        .last_build_date(Some(Utc::now().to_rfc2822()))
        .items(items)
        .build();

    match &cli.output {
        Some(path) => std::fs::write(path, channel.to_string())?,
        // In loop mode, stdout would fill the log with a feed every run.
        None if cli.every_hours.is_some() => {}
        None => println!("{channel}"),
    }
    Ok(())
}

/// Next moment strictly after `now` whose Central-time clock reads a multiple of `hours`:00.
fn next_slot(now: DateTime<Utc>, hours: u32) -> DateTime<Utc> {
    use chrono::TimeZone;
    let tz = chrono_tz::America::Chicago;
    let local_date = now.with_timezone(&tz).date_naive();
    (0..3)
        .flat_map(|day| {
            let date = local_date + chrono::Days::new(day);
            (0..24).step_by(hours as usize).map(move |h| date.and_hms_opt(h, 0, 0).unwrap())
        })
        .filter_map(|naive| tz.from_local_datetime(&naive).earliest())
        .map(|t| t.with_timezone(&Utc))
        .find(|t| *t > now)
        .expect("a slot exists within three days")
}

fn stamp(t: DateTime<Utc>) -> String {
    t.with_timezone(&chrono_tz::America::Chicago)
        .format("%Y-%m-%d %H:%M %Z")
        .to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    let Some(hours) = cli.every_hours else {
        return run_once(&cli);
    };
    if 24 % hours != 0 {
        Cli::command().error(ErrorKind::InvalidValue, "--every-hours must divide 24 (1, 2, 3, 4, 6, 8, 12, 24)").exit();
    }
    if cli.urls.is_empty() && cli.urls_file.is_none() && !cli.from_db && cli.events_feed.is_empty() && cli.youtube_feed.is_empty() {
        Cli::command().error(ErrorKind::MissingRequiredArgument, "provide at least one URL, --urls-file, or --from-db").exit();
    }

    loop {
        let slot = next_slot(Utc::now(), hours);
        eprintln!("[{}] next run at {}", stamp(Utc::now()), stamp(slot));
        while let Ok(wait) = (slot - Utc::now()).to_std() {
            std::thread::sleep(wait.min(Duration::from_secs(60)));
        }
        eprintln!("[{}] run starting", stamp(Utc::now()));
        match run_once(&cli) {
            Ok(()) => eprintln!("[{}] run finished", stamp(Utc::now())),
            Err(e) => eprintln!("[{}] run failed: {e}", stamp(Utc::now())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn slots_are_midnight_central_and_every_three_hours() {
        // Sat 21:00 CDT -> Sun 00:00 CDT (05:00 UTC)
        assert_eq!(next_slot(utc("2026-09-20T02:00:00Z"), 3), utc("2026-09-20T05:00:00Z"));
        // exactly on a slot -> the following slot
        assert_eq!(next_slot(utc("2026-09-20T05:00:00Z"), 3), utc("2026-09-20T08:00:00Z"));
        // just before midnight Central rolls to the next day's 00:00
        assert_eq!(next_slot(utc("2026-09-21T04:59:00Z"), 3), utc("2026-09-21T05:00:00Z"));
    }

    #[test]
    fn slots_follow_daylight_saving() {
        // Fall back (Nov 1 2026): 00:30 CDT -> next slot is 03:00 CST = 09:00 UTC
        assert_eq!(next_slot(utc("2026-11-01T05:30:00Z"), 3), utc("2026-11-01T09:00:00Z"));
        // Spring forward (Mar 8 2026): 00:30 CST -> 03:00 CDT = 08:00 UTC
        assert_eq!(next_slot(utc("2026-03-08T06:30:00Z"), 3), utc("2026-03-08T08:00:00Z"));
    }
}
