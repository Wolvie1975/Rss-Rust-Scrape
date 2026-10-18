use std::path::{Path, PathBuf};
use std::time::Duration;

mod article_identity;
mod big12;
mod boxoffice;
mod boxoffice_forecasts;
mod championship;
mod cinema_posters;
mod db;
mod epl;
mod espn_schedule;
mod events;
mod f1;
mod feed;
mod http;
mod match_results;
mod metrics;
mod mlb;
mod mls;
mod movie_commerce;
mod movie_metadata;
mod movies;
mod nhl;
mod nwsl;
mod posters;
mod rankings;
mod standings;
mod tvmaze;
mod wnba;
mod youtube;

use chrono::{DateTime, Utc};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use reqwest::blocking::Client;
use rss::{ChannelBuilder, ItemBuilder};
use scraper::{Html, Selector};

/// Metadata scraped from a single page.
#[derive(Clone)]
struct PageMeta {
    /// The URL that was requested (the row in `Sources` this page came from).
    source: String,
    url: String,
    title: String,
    description: Option<String>,
    published: Option<DateTime<Utc>>,
    image_url: Option<String>,
    /// Verified source-scoped redirect/canonical aliases, never inferred from content.
    identity_urls: Vec<String>,
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
    crate::http::public_url(resolved.as_str())
        .is_ok()
        .then(|| resolved.to_string())
}

fn scrape(client: &Client, url: &str) -> Result<PageMeta, Box<dyn std::error::Error>> {
    let body = http::get_text(client, url)?;
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

    let published = meta_content(&doc, r#"meta[property="article:published_time"]"#)
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));

    let image_url = image_url(&doc, url);

    Ok(PageMeta {
        source: url.to_string(),
        identity_urls: vec![],
        url: url.to_owned(),
        title,
        description,
        published,
        image_url,
    })
}

/// Resolve at most 20 recent/incoming URLs per source, within a run-wide budget.
/// Old URLs are revisited even if the feed already switched to a newer URL.
fn enrich_article_identities(
    client: &Client,
    found: &mut [PageMeta],
    recent: Vec<PageMeta>,
    started: std::time::Instant,
    mut db: Option<&mut db::Db>,
    dry: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeSet;
    let source = found
        .first()
        .map(|p| p.source.clone())
        .or_else(|| recent.first().map(|p| p.source.clone()));
    let Some(source) = source else { return Ok(()) };
    let unknown = found
        .iter()
        .any(|p| !recent.iter().any(|old| old.url == p.url));
    let mut jobs = recent
        .iter()
        .take(if unknown { 10 } else { 2 })
        .cloned()
        .collect::<Vec<_>>();
    for page in found.iter() {
        if !recent.iter().any(|old| old.url == page.url) && !jobs.iter().any(|p| p.url == page.url)
        {
            jobs.push(page.clone());
        }
    }
    jobs.truncate(20);
    eprintln!(
        "article identity: plan source={source} jobs={} recent={} run_elapsed_ms={}",
        jobs.len(),
        recent.len(),
        started.elapsed().as_millis()
    );
    let source_started = std::time::Instant::now();
    let mut evidence = Vec::new();
    for chunk in jobs.chunks(4) {
        if started.elapsed() > Duration::from_secs(90)
            || source_started.elapsed() > Duration::from_secs(12)
        {
            break;
        }
        let results = std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|p| {
                    scope.spawn(move || {
                        article_identity::fetch(client, &p.url).map_err(|e| e.to_string())
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("article identity worker panicked".into()))
                })
                .collect::<Vec<_>>()
        });
        for (page, result) in chunk.iter().zip(results) {
            if !dry && let Some(d) = db.as_deref_mut() {
                d.article_checked(&source, &page.url)?;
            }
            match result {
                Ok(mut e) => {
                    if e.target != e.requested {
                        eprintln!(
                            "article identity: verified {} -> {} aliases={:?}",
                            e.requested, e.target, e.aliases
                        );
                    }
                    if !e.aliases.contains(&page.url) {
                        e.aliases.push(page.url.clone());
                    }
                    evidence.push((page.clone(), e));
                }
                Err(error) => eprintln!(
                    "article identity: skipped source={source} url={} reason={error}",
                    page.url
                ),
            }
        }
    }
    // Reconcile retained URLs first. They never become extra feed/RSS entries.
    for (old, e) in &evidence {
        if recent.iter().any(|p| p.url == old.url) && (e.target != old.url || e.aliases.len() > 1) {
            let mut update = found
                .iter()
                .find(|p| e.aliases.contains(&p.url))
                .cloned()
                .unwrap_or_else(|| old.clone());
            update.url = e.target.clone();
            update.identity_urls = e.aliases.clone();
            if let Some(d) = db.as_deref_mut() {
                d.save_article(&update, dry)?;
            } else {
                eprintln!(
                    "article identity preview: {} -> {} aliases={:?}",
                    old.url, e.target, e.aliases
                );
            }
        }
    }
    for page in found {
        let mut aliases = BTreeSet::from([page.url.clone()]);
        let mut target = page.url.clone();
        for _ in 0..evidence.len() + 1 {
            let before = aliases.len();
            for (_, e) in &evidence {
                if aliases.iter().any(|u| e.aliases.contains(u)) {
                    aliases.extend(e.aliases.iter().cloned());
                    target = e.target.clone();
                    if page.image_url.is_none() {
                        page.image_url = image_url(&Html::parse_document(&e.body), &e.target);
                    }
                }
            }
            if before == aliases.len() {
                break;
            }
        }
        if evidence
            .iter()
            .any(|(_, e)| aliases.iter().any(|u| e.aliases.contains(u)))
        {
            page.url = target;
            page.identity_urls = aliases.into_iter().collect();
        }
    }
    Ok(())
}

/// For feed entries that came without an image, tries each article page's own `og:image`.
/// An image several articles share is a site-wide logo, not the article's, so it is dropped.
fn fill_missing_images(client: &Client, pages: &mut [PageMeta]) {
    let started = std::time::Instant::now();
    let mut found: Vec<(usize, String)> = Vec::new();
    for (i, p) in pages
        .iter()
        .enumerate()
        .filter(|(_, p)| p.image_url.is_none())
    {
        if started.elapsed() > Duration::from_secs(30) {
            break;
        }
        let Ok(body) = http::get_text(client, &p.url) else {
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
    /// Read-only collector health/progress check for the service watchdog.
    #[arg(long, conflicts_with_all=["initialize_db","every_hours"])]
    health_check: bool,
    /// Explicitly initialize/migrate schema using deployment credentials.
    #[arg(long,conflicts_with_all=["dry_run","every_hours"])]
    initialize_db: bool,
    /// Apply the TV schema and seed the five explicit provider IDs without scraping.
    #[arg(long,conflicts_with_all=["dry_run","every_hours"])]
    configure_tv: bool,
    /// Collect enabled tracked TVmaze series and full episode lists (implies --db).
    #[arg(long)]
    tv_from_db: bool,
    /// Ignore the TV request cache for an explicit validation/manual refresh.
    #[arg(long, requires = "tv_from_db", conflicts_with = "every_hours")]
    tv_force_refresh: bool,
    /// Target SQL database; use a separate database for staging validation.
    #[arg(long, default_value = "WebScraper")]
    database: String,
    /// Register the five national Top 25 polls disabled.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_rankings: bool,
    /// Import full national rankings and identify current Big 12 members.
    #[arg(long)]
    rankings_from_db: bool,
    /// Register the Movie Insider digital release source disabled.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_movieinsider: bool,
    /// Register professional and Big 12 standings sources disabled.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_standings: bool,
    /// Refresh enabled standings sources (implies --db).
    #[arg(long)]
    standings_from_db: bool,
    /// Register the MLB league schedule disabled; preserve existing settings.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_mlb: bool,
    /// Register the WNBA league schedule disabled; preserve existing settings.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_wnba: bool,

    /// Register EFL Championship fixtures/results and standings disabled.
    #[arg(long)]
    configure_championship: bool,

    /// Register Premier League fixtures and standings disabled.
    #[arg(long)]
    configure_epl: bool,
    /// Register league-wide MLS fixtures/results disabled.
    #[arg(long)]
    configure_mls: bool,
    /// Register NHL fixtures/results and division standings disabled.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_nhl: bool,
    /// Configure F1 calendar, race/sprint results and championships disabled.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_f1: bool,
    /// Refresh enabled F1 data only (also included by --events-from-db).
    #[arg(long)]
    f1_from_db: bool,
    /// Create the latest domestic weekend box-office chart table.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_boxoffice: bool,
    /// Refresh the latest top-ten weekend box-office chart.
    #[arg(long)]
    boxoffice: bool,
    /// Refresh domestic weekend previews and future opening-weekend forecasts.
    #[arg(long)]
    boxoffice_forecasts: bool,
    /// Register league-wide NWSL schedule; promote the legacy Current source in place.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_nwsl: bool,
    /// Register verified Big 12 member feeds disabled, preserving existing feed settings.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_big12: bool,
    /// Explicitly enable registered Big 12 feeds (also configures current members).
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    enable_big12: bool,
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
    #[arg(
        short,
        long,
        default_value = "Feed generated from scraped page metadata"
    )]
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
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=i32::MAX as i64))]
    per_source: Option<u32>,

    /// Keep only the newest N pages: each run saves at most the N newest scraped pages, then
    /// deletes the oldest rows from the Pages table until N remain (undated rows go first)
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=i32::MAX as i64))]
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

    /// Also ingest every channel listed in the YoutubeVideoFeed table (implies --db)
    #[arg(long)]
    youtube_from_db: bool,

    /// Backfill/validate movie posters without scraping releases or other feeds.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    movie_posters: bool,
    /// Retry cached missing posters now; valid existing posters are preserved.
    #[arg(long, requires = "movie_posters")]
    retry_missing_posters: bool,

    /// Ingest this week's U.S. movie releases from public calendars (implies --db).
    #[arg(long)]
    movies: bool,

    /// How many of each YouTube channel's newest videos to keep
    #[arg(long, value_name = "N", default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=i32::MAX as i64))]
    youtube_latest: u32,

    /// Also ingest every calendar feed listed in the SportsEventsType table (implies --db)
    #[arg(long)]
    events_from_db: bool,

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
    let use_db = cli.health_check
        || cli.initialize_db
        || cli.configure_rankings
        || cli.rankings_from_db
        || cli.configure_movieinsider
        || cli.configure_standings
        || cli.standings_from_db
        || cli.configure_mlb
        || cli.configure_wnba
        || cli.configure_championship
        || cli.boxoffice
        || cli.boxoffice_forecasts
        || cli.configure_boxoffice
        || cli.configure_mls
        || cli.configure_nhl
        || cli.configure_f1
        || cli.f1_from_db
        || cli.configure_epl
        || cli.configure_nwsl
        || cli.configure_tv
        || cli.tv_from_db
        || cli.movie_posters
        || cli.configure_big12
        || cli.enable_big12
        || cli.movies
        || cli.db
        || cli.from_db
        || cli.save_sources
        || !cli.events_feed.is_empty()
        || cli.events_from_db
        || !cli.youtube_feed.is_empty()
        || cli.youtube_from_db;
    let mut db = if use_db {
        let setup = cli.configure_rankings
            || cli.configure_movieinsider
            || cli.configure_standings
            || cli.initialize_db
            || cli.configure_mlb
            || cli.configure_wnba
            || cli.configure_championship
            || cli.configure_boxoffice
            || cli.configure_mls
            || cli.configure_nhl
            || cli.configure_f1
            || cli.configure_epl
            || cli.configure_nwsl
            || cli.configure_tv
            || cli.configure_big12
            || cli.enable_big12;
        let ado = if setup {
            std::env::var("MSSQL_MIGRATION_CONNECTION_STRING")
                .or_else(|_| std::env::var("MSSQL_CONNECTION_STRING"))
        } else {
            std::env::var("MSSQL_CONNECTION_STRING")
        }
        .map_err(|_| "database options require MSSQL_CONNECTION_STRING to be set")?;
        Some(if cli.configure_f1 {
            db::Db::connect_named(&ado, &cli.database)?
        } else if setup {
            db::Db::open_named(&ado, &cli.database)?
        } else {
            db::Db::connect_named(&ado, &cli.database)?
        })
    } else {
        None
    };

    if cli.health_check {
        db.as_mut().unwrap().check_health()?;
        return Ok(());
    }
    if cli.initialize_db {
        db.as_mut().unwrap().setup_tv(false)?;
        eprintln!("database schema initialized");
        return Ok(());
    }
    if cli.configure_rankings {
        db.as_mut().unwrap().configure_rankings()?;
        eprintln!(
            "National Top 25 sources registered disabled; enable SportsRankingSources to import"
        );
        return Ok(());
    }
    if cli.configure_movieinsider {
        db.as_mut().unwrap().configure_movieinsider()?;
        eprintln!(
            "Movie Insider digital source registered disabled; enable MovieReleaseSources to import"
        );
        return Ok(());
    }
    if cli.configure_standings {
        db.as_mut().unwrap().configure_standings()?;
        eprintln!("Standings sources registered disabled; enable SportsStandingsSources to import");
        return Ok(());
    }
    if cli.configure_mlb {
        db.as_mut().unwrap().configure_mlb()?;
        eprintln!("MLB schedule registered; enable it in SportsEventsType to import");
        return Ok(());
    }
    if cli.configure_boxoffice {
        db.as_mut().unwrap().configure_boxoffice()?;
        eprintln!("Box-office chart table configured");
        return Ok(());
    }
    if cli.configure_f1 {
        db.as_mut().unwrap().configure_f1()?;
        eprintln!("F1 schedule/results and driver/constructor championships configured disabled");
        return Ok(());
    }
    if cli.configure_nhl {
        db.as_mut().unwrap().configure_nhl()?;
        eprintln!("NHL schedule/results and standings registered disabled");
        return Ok(());
    }
    if cli.configure_mls {
        db.as_mut().unwrap().configure_mls()?;
        eprintln!("MLS schedule/results registered disabled");
        return Ok(());
    }
    if cli.configure_epl {
        db.as_mut().unwrap().configure_epl()?;
        eprintln!("Premier League sources registered disabled");
        return Ok(());
    }
    if cli.configure_championship {
        db.as_mut().unwrap().configure_championship()?;
        eprintln!("EFL Championship sources registered disabled");
        return Ok(());
    }
    if cli.configure_wnba {
        db.as_mut().unwrap().configure_wnba()?;
        eprintln!("WNBA schedule registered; enable it in SportsEventsType to import");
        return Ok(());
    }
    if cli.configure_nwsl {
        db.as_mut().unwrap().configure_nwsl()?;
        eprintln!("League-wide NWSL schedule registered; existing Enabled setting preserved");
        return Ok(());
    }
    if cli.configure_tv {
        db.as_mut().unwrap().setup_tv(true)?;
    }
    if cli.configure_tv {
        eprintln!(
            "TVmaze schema ready; five tracking rows seeded without changing existing Enabled settings"
        );
        return Ok(());
    }
    if cli.configure_big12 || cli.enable_big12 {
        let client = http::client_builder()
            .timeout(Duration::from_secs(20))
            .user_agent("Mozilla/5.0 (compatible; web_scraper/0.1)")
            .build()?;
        let members = big12::current_members(&client)?;
        db.as_mut()
            .unwrap()
            .configure_big12(&members, cli.enable_big12)?;
        eprintln!(
            "configured {} current Big 12 members; new feeds {}",
            members.len(),
            if cli.enable_big12 {
                "enabled"
            } else {
                "disabled"
            }
        );
        return Ok(());
    }

    if cli.movie_posters {
        if cli.retry_missing_posters {
            db.as_mut().unwrap().retry_missing_posters()?;
        }
        let report = posters::enrich(db.as_mut().unwrap(), &std::collections::BTreeMap::new())?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        eprintln!(
            "box-office posters: {}",
            cinema_posters::enrich(db.as_mut().unwrap())?
        );
        return Ok(());
    }

    let run_id = if !cli.dry_run {
        db.as_mut().map(|d| d.start_run()).transpose()?
    } else {
        None
    };
    let mut failures: Vec<String> = Vec::new();
    let mut succeeded_sources = 0usize;
    let _run_timer = metrics::Stage::new("run");
    let mut fetched_news = Vec::new();
    let outcome: Result<(), Box<dyn std::error::Error>> = (|| {
        let mut urls = cli.urls.clone();
        if let Some(path) = &cli.urls_file {
            urls.extend(read_urls_file(path)?);
        }
        if cli.save_sources {
            let added = db.as_mut().unwrap().add_sources(&urls)?;
            eprintln!(
                "added {added} new sources ({} already present)",
                urls.len() - added
            );
        }
        if cli.from_db {
            for url in db.as_mut().unwrap().enabled_sources()? {
                if !urls.contains(&url) {
                    urls.push(url);
                }
            }
        }
        if !cli.tv_from_db
            && !cli.movies
            && urls.is_empty()
            && cli.events_feed.is_empty()
            && !cli.events_from_db
            && !cli.f1_from_db
            && !cli.boxoffice
            && !cli.boxoffice_forecasts
            && !cli.standings_from_db
            && !cli.rankings_from_db
            && cli.youtube_feed.is_empty()
            && !cli.youtube_from_db
        {
            Cli::command().error(ErrorKind::MissingRequiredArgument, "provide at least one URL, --urls-file, --from-db, --events-feed, --events-from-db, --standings-from-db, --rankings-from-db, --youtube-feed, or --youtube-from-db").exit();
        }

        let client = http::client_builder()
            .user_agent("Mozilla/5.0 (compatible; web_scraper/0.1)")
            .timeout(Duration::from_secs(15))
            .build()?;

        if cli.f1_from_db || cli.events_from_db {
            let _timer = metrics::Stage::new("f1");
            let enabled = db.as_mut().unwrap().f1_enabled()?;
            if enabled {
                if !cli.dry_run {
                    db.as_mut().unwrap().f1_status(None)?;
                }
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let f1_client = http::client_builder()
                        .timeout(Duration::from_secs(30))
                        .build()?;
                    let snapshot = f1::fetch(&f1_client)?;
                    eprintln!(
                        "{}{}",
                        snapshot.summary(),
                        if cli.dry_run { " (dry run)" } else { "" }
                    );
                    if !cli.dry_run {
                        db.as_mut().unwrap().save_f1(&snapshot)?;
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => succeeded_sources += 1,
                    Err(e) => {
                        if !cli.dry_run {
                            db.as_mut().unwrap().f1_status(Some(&e.to_string()))?;
                        }
                        failures.push(format!("F1: {e}"));
                    }
                }
            } else if cli.f1_from_db {
                failures.push("F1 source is disabled or not configured".to_string());
            }
        }

        if cli.rankings_from_db {
            let _stage_timer = metrics::Stage::new("rankings");
            let client = http::client_builder()
                .user_agent("My-Newsfeed/1.0")
                .timeout(Duration::from_secs(30))
                .build()?;
            let sources = db.as_mut().unwrap().ranking_sources()?;
            if !sources.is_empty() {
                let members = rankings::current_members(&client);
                for source in sources {
                    if !cli.dry_run {
                        db.as_mut().unwrap().ranking_attempt(source.id)?;
                    }
                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                        let members = members
                            .as_ref()
                            .map_err(|e| format!("Big 12 membership: {e}"))?;
                        let poll = rankings::fetch(&client, &source, members)?;
                        if !cli.dry_run {
                            db.as_mut().unwrap().save_ranking(&source, &poll)?;
                        }
                        eprintln!(
                            "{} {}: {} ranked teams, {} Big 12, published {}{}",
                            source.sport,
                            poll.name,
                            poll.teams.len(),
                            poll.teams
                                .iter()
                                .filter(|t| t.big12_school_id.is_some())
                                .count(),
                            poll.published,
                            if cli.dry_run { " (dry run)" } else { "" }
                        );
                        Ok(())
                    })();
                    if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
                        d.run_heartbeat(id)?;
                    }
                    match result {
                        Ok(()) => succeeded_sources += 1,
                        Err(e) => {
                            let message = format!("{} rankings: {e}", source.sport);
                            eprintln!("{message}");
                            if !cli.dry_run
                                && let Err(e) =
                                    db.as_mut().unwrap().ranking_error(source.id, &message)
                            {
                                failures.push(format!("ranking error tracking: {e}"));
                            }
                            failures.push(message);
                        }
                    }
                }
            }
        }

        if cli.boxoffice {
            let _timer = metrics::Stage::new("boxoffice");
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                let client = http::client_builder()
                    .user_agent("My-Newsfeed/1.0")
                    .timeout(Duration::from_secs(30))
                    .build()?;
                let chart = boxoffice::fetch(&client)?;
                if !cli.dry_run {
                    db.as_mut().unwrap().save_boxoffice(&chart)?;
                }
                eprintln!(
                    "Box office: 10 movies, weekend {}{}",
                    chart.weekend,
                    if cli.dry_run { " (dry run)" } else { "" }
                );
                Ok(())
            })();
            match result {
                Ok(()) => succeeded_sources += 1,
                Err(e) => failures.push(format!("Box office: {e}")),
            }
        }

        if cli.boxoffice_forecasts {
            let _timer = metrics::Stage::new("boxoffice_forecasts");
            if !cli.dry_run {
                db.as_mut().unwrap().boxoffice_forecast_status(None)?;
            }
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                let client = http::client_builder()
                    .user_agent("My-Newsfeed/1.0")
                    .timeout(Duration::from_secs(30))
                    .build()?;
                let snapshots = boxoffice_forecasts::fetch(&client)?;
                if !cli.dry_run {
                    db.as_mut().unwrap().save_boxoffice_forecasts(&snapshots)?;
                }
                for snapshot in &snapshots {
                    eprintln!(
                        "Boxoffice Pro {}: {} current/future movies{}",
                        snapshot.kind,
                        snapshot.rows.len(),
                        if cli.dry_run { " (dry run)" } else { "" }
                    );
                }
                Ok(())
            })();
            match result {
                Ok(()) => succeeded_sources += 1,
                Err(e) => {
                    let message = format!("Boxoffice Pro forecasts: {e}");
                    if !cli.dry_run {
                        db.as_mut()
                            .unwrap()
                            .boxoffice_forecast_status(Some(&message))?;
                    }
                    failures.push(message);
                }
            }
        }

        if cli.standings_from_db {
            let _stage_timer = metrics::Stage::new("standings");
            let client = http::client_builder()
                .user_agent("My-Newsfeed/1.0")
                .timeout(Duration::from_secs(30))
                .build()?;
            for source in db.as_mut().unwrap().standings_sources()? {
                if !cli.dry_run {
                    db.as_mut().unwrap().standings_attempt(source.id)?;
                }
                let result = standings::fetch(&client, &source).and_then(|snapshot| {
                    if !cli.dry_run {
                        db.as_mut().unwrap().save_standings(&source, &snapshot)?;
                    }
                    eprintln!(
                        "{} {} standings: {} teams, season {}{}",
                        source.provider,
                        source.sport,
                        snapshot.entries.len(),
                        snapshot.season,
                        if cli.dry_run { " (dry run)" } else { "" }
                    );
                    Ok(())
                });
                if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
                    d.run_heartbeat(id)?;
                }
                match result {
                    Ok(()) => succeeded_sources += 1,
                    Err(e) => {
                        let message =
                            format!("{} {} standings: {e}", source.provider, source.sport);
                        eprintln!("{message}");
                        if !cli.dry_run
                            && let Err(e) =
                                db.as_mut().unwrap().standings_error(source.id, &message)
                        {
                            failures.push(format!("standings error tracking: {e}"));
                        }
                        failures.push(message);
                    }
                }
            }
        }

        let mut pages = Vec::new();
        let identity_started = std::time::Instant::now();
        let identity_client = article_identity::client()?;
        // Rotate the identity work order so the bounded HTTP budget cannot starve later sources.
        if !urls.is_empty() {
            let offset = (Utc::now().timestamp().div_euclid(3600) as usize) % urls.len();
            urls.rotate_left(offset);
        }
        let news_timer = metrics::Stage::new("news");
        for url in &urls {
            let result = scrape_source(&client, url);
            if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
                d.run_heartbeat(id)?;
            }
            match result {
                Ok((mut found, how)) => {
                    fetched_news.push(url.clone());
                    if cli.dry_run || db.is_none() {
                        succeeded_sources += 1;
                    }
                    let total = found.len();
                    if let Some(cap) = cli.per_source {
                        found.sort_by_key(|p| std::cmp::Reverse(p.published));
                        found.truncate(cap as usize);
                    }
                    let recent = if let Some(d) = db.as_mut() {
                        d.recent_articles(url)?
                    } else {
                        vec![]
                    };
                    enrich_article_identities(
                        &identity_client,
                        &mut found,
                        recent,
                        identity_started,
                        db.as_mut(),
                        cli.dry_run,
                    )?;
                    if how == "feed" {
                        let missing = found.iter().filter(|p| p.image_url.is_none()).count();
                        fill_missing_images(&client, &mut found);
                        let still = found.iter().filter(|p| p.image_url.is_none()).count();
                        if missing > 0 {
                            eprintln!(
                                "{url}: found {} of {missing} missing images on article pages",
                                missing - still
                            );
                        }
                    }
                    eprintln!("{url}: {total} pages via {how}, using {}", found.len());
                    pages.append(&mut found);
                }
                Err(e) => {
                    eprintln!("skipping {url}: {e}");
                    failures.push(format!("news {url}: {e}"));
                    if let Some(d) = db.as_mut().filter(|_| !cli.dry_run) {
                        d.record_result(url, Some(&e.to_string()))?;
                    }
                }
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
        pages.sort_by_key(|p| std::cmp::Reverse(p.published));
        if let Some(keep) = cli.keep {
            pages.truncate(keep as usize);
        }

        if cli.dry_run {
            eprintln!(
                "dry run: nothing written to the database. {} pages would be saved:",
                pages.len()
            );
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
            let mut n = 0;
            for url in &urls {
                let source_pages: Vec<_> = pages.iter().filter(|p| p.source == *url).collect();
                if !fetched_news.contains(url) {
                    continue;
                }
                match db.save_page_refs(&source_pages) {
                    Ok(count) => {
                        n += count;
                        succeeded_sources += 1;
                        db.record_result(url, None)?;
                    }
                    Err(e) => {
                        failures.push(format!("save news {url}: {e}"));
                        let _ = db.record_result(url, Some(&e.to_string()));
                    }
                }
                if let Some(id) = run_id {
                    db.run_heartbeat(id)?;
                }
            }
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

        // Calendar feeds from --events-feed and from the lookup table, each with its known type row
        // (if any) and school name. A flagged URL that is already in the table reuses that row.
        drop(news_timer);
        let mut event_feeds: Vec<(Option<i32>, String, Option<String>)> = Vec::new();
        if !cli.events_feed.is_empty() || cli.events_from_db {
            let known = db.as_mut().unwrap().sports_event_feeds()?;
            for url in &cli.events_feed {
                match known.iter().find(|f| f.url == *url) {
                    Some(f) => event_feeds.push((Some(f.id), url.clone(), f.school.clone())),
                    None => event_feeds.push((None, url.clone(), None)),
                }
            }
            if cli.events_from_db {
                for f in known {
                    if !event_feeds.iter().any(|(_, u, _)| *u == f.url) {
                        event_feeds.push((Some(f.id), f.url, f.school));
                    }
                }
            }
        }
        if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
            d.run_heartbeat(id)?;
        }
        let big12_feeds: Vec<_> = event_feeds
            .iter()
            .filter(|(_, url, _)| big12::school_id(url).is_some())
            .filter_map(|(id, url, school)| {
                id.map(|id| db::EventFeed {
                    id,
                    url: url.clone(),
                    school: school.clone(),
                })
            })
            .collect();
        if !big12_feeds.is_empty() {
            let _stage_timer = metrics::Stage::new("big12_schedules");
            if !cli.dry_run {
                for f in &big12_feeds {
                    db.as_mut().unwrap().event_attempt(f.id)?;
                }
            }
            let collection = {
                let _timer = metrics::Stage::new("big12_http");
                big12::collect(&client, &big12_feeds)
            };
            match collection {
                Ok(results) => {
                    let _sql_timer = metrics::Stage::new("big12_sql");
                    use std::collections::BTreeMap;
                    let mut counts: BTreeMap<i32, (i32, i32, Vec<String>, bool)> = results
                        .iter()
                        .map(|r| (r.id, (0, 0, r.warnings.clone(), r.error.is_none())))
                        .collect();
                    for r in &results {
                        if let Some(e) = &r.error {
                            counts.get_mut(&r.id).unwrap().2.push(e.clone());
                        }
                    }
                    // New events choose a home perspective first; existing primary IDs never flip.
                    let mut observations: Vec<_> = results
                        .iter()
                        .flat_map(|r| r.observations.iter().map(move |o| (r.id, o)))
                        .collect();
                    observations.sort_by_key(|(feed, o)| {
                        (
                            o["event"]["is_away"] == true,
                            *feed,
                            o["provider_game_id"].as_i64().unwrap_or(0),
                        )
                    });
                    for (index, (feed, observation)) in observations.into_iter().enumerate() {
                        if index % 100 == 0 {
                            eprintln!("Big 12 source observation {}", index + 1);
                        }
                        let (saved, quarantined, errors, success) = counts.get_mut(&feed).unwrap();
                        if let Some(issue) = observation["issue"].as_str() {
                            errors.push(format!("{}: {issue}", observation["provider_game_id"]));
                        }
                        if cli.dry_run {
                            if observation["issue"].is_null() {
                                *saved += 1;
                            } else {
                                *quarantined += 1;
                            }
                        } else {
                            match db
                                .as_mut()
                                .unwrap()
                                .save_big12_observation(feed, observation)
                            {
                                Ok(outcome) if outcome == "saved" => *saved += 1,
                                Ok(outcome) => {
                                    *quarantined += 1;
                                    if observation["issue"].is_null() {
                                        errors.push(format!(
                                            "{}: {outcome}",
                                            observation["provider_game_id"]
                                        ));
                                    }
                                }
                                Err(e) => {
                                    *quarantined += 1;
                                    *success = false;
                                    errors.push(e.to_string());
                                }
                            }
                        }
                    }
                    for result in results {
                        let (saved, quarantined, errors, success) =
                            counts.remove(&result.id).unwrap();
                        if !success {
                            failures.push(format!("Big12 {} failed", result.school));
                        } else {
                            succeeded_sources += 1;
                        }
                        eprintln!(
                            "Big 12 {}: {saved} source observations saved, {quarantined} quarantined, {} issues{}",
                            result.school,
                            errors.len(),
                            if cli.dry_run { " (dry run)" } else { "" }
                        );
                        if !cli.dry_run {
                            let error = (!errors.is_empty()).then(|| errors.join("; "));
                            db.as_mut().unwrap().event_result(
                                result.id,
                                success,
                                result.observations.len() as i32,
                                errors.len() as i32,
                                error.as_deref(),
                            )?;
                        }
                    }
                }
                Err(e) => {
                    failures.push(format!("Big12: {e}"));
                    eprintln!("Big 12 collection failed: {e}");
                    if !cli.dry_run {
                        for f in &big12_feeds {
                            db.as_mut().unwrap().event_result(
                                f.id,
                                false,
                                0,
                                1,
                                Some(&e.to_string()),
                            )?;
                        }
                    }
                }
            }
        }
        if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
            d.run_heartbeat(id)?;
        }
        for (known_id, feed_url, school) in &event_feeds {
            if big12::is_provider_feed(feed_url) {
                if known_id.is_none() {
                    eprintln!(
                        "{feed_url}: register and enable this school using --configure-big12 / --enable-big12"
                    );
                } else if big12::school_id(feed_url).is_none() {
                    eprintln!(
                        "{feed_url}: conference-wide RSS is not supported for ingestion; use registered school feeds"
                    );
                }
                continue;
            }
            if !cli.dry_run
                && let Some(id) = known_id
            {
                db.as_mut().unwrap().event_attempt(*id)?;
            }

            if epl::is_feed(feed_url) {
                let _stage_timer = metrics::Stage::new("epl_schedule");
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let fixtures = epl::fetch(&client, feed_url)?;
                    eprintln!(
                        "Premier League: {} current/future schedule observations",
                        fixtures.len()
                    );
                    if !cli.dry_run {
                        let d = db.as_mut().unwrap();
                        let id = match known_id {
                            Some(id) => *id,
                            None => d.ensure_sports_events_type(feed_url, "Premier League")?,
                        };
                        let n = d.save_espn_schedule(id, &fixtures)?;
                        d.event_result(id, true, n as i32, 0, None)?;
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => succeeded_sources += 1,
                    Err(e) => {
                        failures.push(format!("Premier League: {e}"));
                        if !cli.dry_run
                            && let Some(id) = known_id
                        {
                            db.as_mut().unwrap().event_result(
                                *id,
                                false,
                                0,
                                1,
                                Some(&e.to_string()),
                            )?;
                        }
                    }
                }
                continue;
            }
            if nhl::is_feed(feed_url) {
                let _stage_timer = metrics::Stage::new("nhl_schedule");
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let batch = nhl::fetch(&client, feed_url)?;
                    eprintln!(
                        "NHL season {}: {} schedule observations, {} final results",
                        batch.season,
                        batch.fixtures.len(),
                        batch.results.len()
                    );
                    if !cli.dry_run {
                        let d = db.as_mut().unwrap();
                        let id = match known_id {
                            Some(id) => *id,
                            None => d.ensure_sports_events_type(feed_url, "NHL")?,
                        };
                        d.save_espn_schedule(id, &batch.fixtures)?;
                        d.save_match_results(
                            id,
                            espn_schedule::League::Nhl,
                            batch.season,
                            &batch.results,
                        )?;
                        d.event_result(
                            id,
                            true,
                            batch.fixtures.iter().filter(|f| !f.completed).count() as i32,
                            0,
                            None,
                        )?;
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => succeeded_sources += 1,
                    Err(e) => {
                        failures.push(format!("NHL: {e}"));
                        if !cli.dry_run
                            && let Some(id) = known_id
                        {
                            db.as_mut().unwrap().event_result(
                                *id,
                                false,
                                0,
                                1,
                                Some(&e.to_string()),
                            )?;
                        }
                    }
                }
                continue;
            }
            if mls::is_feed(feed_url) {
                let _stage_timer = metrics::Stage::new("mls_schedule");
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let batch = mls::fetch(&client, feed_url)?;
                    eprintln!(
                        "MLS season {}: {} schedule observations, {} final results",
                        batch.season,
                        batch.fixtures.len(),
                        batch.results.len()
                    );
                    if !cli.dry_run {
                        let d = db.as_mut().unwrap();
                        let id = match known_id {
                            Some(id) => *id,
                            None => d.ensure_sports_events_type(feed_url, "MLS")?,
                        };
                        d.save_espn_schedule(id, &batch.fixtures)?;
                        d.save_match_results(
                            id,
                            espn_schedule::League::Mls,
                            batch.season,
                            &batch.results,
                        )?;
                        d.event_result(
                            id,
                            true,
                            batch.fixtures.iter().filter(|f| !f.completed).count() as i32,
                            0,
                            None,
                        )?;
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => succeeded_sources += 1,
                    Err(e) => {
                        failures.push(format!("MLS: {e}"));
                        if !cli.dry_run
                            && let Some(id) = known_id
                        {
                            db.as_mut().unwrap().event_result(
                                *id,
                                false,
                                0,
                                1,
                                Some(&e.to_string()),
                            )?;
                        }
                    }
                }
                continue;
            }
            if championship::is_feed(feed_url) {
                let _stage_timer = metrics::Stage::new("championship_schedule");
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let batch = championship::fetch(&client, feed_url)?;
                    eprintln!(
                        "Championship season {}: {} schedule observations, {} final results; Wrexham ID {}",
                        batch.season,
                        batch.fixtures.len(),
                        batch.results.len(),
                        championship::WREXHAM_ID
                    );
                    if !cli.dry_run {
                        let d = db.as_mut().unwrap();
                        let id = match known_id {
                            Some(id) => *id,
                            None => d.ensure_sports_events_type(feed_url, "EFL Championship")?,
                        };
                        d.save_espn_schedule(id, &batch.fixtures)?;
                        d.save_championship_results(id, &batch)?;
                        d.event_result(
                            id,
                            true,
                            batch.fixtures.iter().filter(|f| !f.completed).count() as i32,
                            0,
                            None,
                        )?;
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => succeeded_sources += 1,
                    Err(e) => {
                        failures.push(format!("Championship: {e}"));
                        if !cli.dry_run
                            && let Some(id) = known_id
                        {
                            db.as_mut().unwrap().event_result(
                                *id,
                                false,
                                0,
                                1,
                                Some(&e.to_string()),
                            )?;
                        }
                    }
                }
                continue;
            }
            if nwsl::is_feed(feed_url) || wnba::is_feed(feed_url) || mlb::is_feed(feed_url) {
                let is_mlb = mlb::is_feed(feed_url);
                let is_wnba = wnba::is_feed(feed_url);
                let _stage_timer = metrics::Stage::new(if is_mlb {
                    "mlb_schedule"
                } else if is_wnba {
                    "wnba_schedule"
                } else {
                    "nwsl_schedule"
                });
                let label = if is_mlb {
                    "MLB"
                } else if is_wnba {
                    "WNBA"
                } else {
                    "NWSL"
                };
                let fetched = if is_mlb {
                    mlb::fetch(&client, feed_url)
                } else if is_wnba {
                    wnba::fetch(&client, feed_url)
                } else {
                    nwsl::fetch(&client, feed_url)
                };
                match fetched {
                    Ok(fixtures) => {
                        eprintln!(
                            "{feed_url}: {} {label} fixtures{}",
                            fixtures.len(),
                            if cli.dry_run { " (dry run)" } else { "" }
                        );
                        if cli.dry_run {
                            succeeded_sources += 1;
                            for f in &fixtures {
                                eprintln!(
                                    "{} | {} | {:?} | {}",
                                    f.event.event_date, f.event.title, f.event.starts_at, f.status
                                );
                            }
                        } else {
                            let db = db.as_mut().unwrap();
                            let id = match known_id {
                                Some(id) => *id,
                                None => db.ensure_sports_events_type(
                                    feed_url,
                                    if is_mlb {
                                        "MLB"
                                    } else if is_wnba {
                                        "WNBA"
                                    } else {
                                        "NWSL"
                                    },
                                )?,
                            };
                            match db.save_espn_schedule(id, &fixtures) {
                                Ok(n) => {
                                    succeeded_sources += 1;
                                    db.event_result(id, true, n as i32, 0, None)?;
                                }
                                Err(e) => {
                                    eprintln!("{label} save failed: {e}");
                                    failures.push(format!("{label} save: {e}"));
                                    let _ = db.event_result(id, false, 0, 1, Some(&e.to_string()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        failures.push(format!("{label} {feed_url}: {e}"));
                        eprintln!("{label} schedule {feed_url} failed: {e}");
                        if !cli.dry_run
                            && let Some(id) = known_id
                        {
                            db.as_mut().unwrap().event_result(
                                *id,
                                false,
                                0,
                                1,
                                Some(&e.to_string()),
                            )?;
                        }
                    }
                }
                continue;
            }

            // A broken calendar feed shouldn't stop the rest of the run.
            match events::fetch_events(&client, feed_url, school.as_deref()) {
                Ok(batch) if cli.dry_run => {
                    eprintln!(
                        "{feed_url}: {} events (dry run, not saved)",
                        batch.events.len()
                    );
                    for e in &batch.events {
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
                Ok(batch) => {
                    let db = db.as_mut().unwrap();
                    let type_id = match known_id {
                        Some(id) => *id,
                        None => {
                            let name = url::Url::parse(feed_url)
                                .ok()
                                .and_then(|u| u.host_str().map(str::to_string))
                                .unwrap_or_else(|| feed_url.clone());
                            db.ensure_sports_events_type(feed_url, &name)?
                        }
                    };
                    let n = match db.save_events(type_id, &batch.events) {
                        Ok(n) => n,
                        Err(e) => {
                            failures.push(format!("calendar save {feed_url}: {e}"));
                            let _ = db.event_result(type_id, false, 0, 1, Some(&e.to_string()));
                            continue;
                        }
                    };
                    eprintln!("{feed_url}: saved {n} events to type {type_id}");
                    succeeded_sources += 1;
                    db.event_result(type_id, true, n as i32, 0, None)?;
                    if batch.school_id.is_some() {
                        let corrected =
                            db.refresh_school_team_logo(type_id, batch.team_logo_url.as_deref())?;
                        eprintln!(
                            "{feed_url}: corrected team logos on {corrected} retained events"
                        );
                    }
                }
                Err(e) => {
                    failures.push(format!("calendar {feed_url}: {e}"));
                    eprintln!("events feed {feed_url} failed: {e}");
                    if !cli.dry_run
                        && let Some(id) = known_id
                    {
                        db.as_mut().unwrap().event_result(
                            *id,
                            false,
                            0,
                            1,
                            Some(&e.to_string()),
                        )?;
                    }
                }
            }
        }

        if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
            d.run_heartbeat(id)?;
        }
        // Channels from --youtube-feed (feed row found or created below) and from the lookup table.
        let mut youtube: Vec<(Option<i32>, String)> =
            cli.youtube_feed.iter().map(|u| (None, u.clone())).collect();
        if cli.youtube_from_db {
            for (id, url) in db.as_mut().unwrap().youtube_feeds()? {
                match youtube.iter_mut().find(|(_, u)| *u == url) {
                    Some(existing) => existing.0 = Some(id),
                    None => youtube.push((Some(id), url)),
                }
            }
        }
        let youtube_timer = metrics::Stage::new("youtube");
        for (known_id, feed_url) in &youtube {
            match youtube::fetch_latest(&client, feed_url, cli.youtube_latest as usize) {
                Ok((found, how)) if cli.dry_run => {
                    eprintln!(
                        "{feed_url}: {} videos via {how} (dry run, not saved)",
                        found.len()
                    );
                    for v in &found {
                        eprintln!(
                            "  - {}{} | {} | {} views | {}",
                            v.published_at.to_rfc3339(),
                            if v.published_is_estimate {
                                " (estimated)"
                            } else {
                                ""
                            },
                            v.channel_name.as_deref().unwrap_or("?"),
                            v.views.map_or("?".into(), |n| n.to_string()),
                            v.title
                        );
                    }
                }
                Ok((found, how)) => {
                    let db = db.as_mut().unwrap();
                    let feed_id = match (known_id, found.first()) {
                        (Some(id), _) => *id,
                        (None, Some(v)) => db.ensure_youtube_feed(
                            &v.channel_id,
                            v.channel_name.as_deref(),
                            feed_url,
                        )?,
                        (None, None) => {
                            eprintln!("{feed_url}: the feed has no videos, nothing saved");
                            continue;
                        }
                    };
                    let n = match db.save_videos(feed_id, &found) {
                        Ok(n) => n,
                        Err(e) => {
                            failures.push(format!("YouTube save {feed_url}: {e}"));
                            continue;
                        }
                    };
                    succeeded_sources += 1;
                    let pruned = db.prune_videos(cli.youtube_latest)?;
                    eprintln!(
                        "{feed_url}: saved {n} videos via {how} to feed {feed_id}, pruned {pruned} older (keeping the newest {} per channel)",
                        cli.youtube_latest
                    );
                }
                Err(e) => {
                    failures.push(format!("YouTube {feed_url}: {e}"));
                    eprintln!("youtube feed {feed_url} failed: {e}");
                }
            }
        }

        if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
            d.run_heartbeat(id)?;
        }
        drop(youtube_timer);
        if cli.movies {
            let _stage_timer = metrics::Stage::new("movies");
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                use chrono::Datelike;
                let day = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive();
                let start =
                    day - chrono::Duration::days(day.weekday().num_days_from_monday().into());
                let tracked = db.as_mut().unwrap().tracked_movie_urls()?;
                let enabled = db.as_mut().unwrap().enabled_movie_sources()?;
                let mut report = movies::ingest(start, &tracked, &enabled)?;
                let skipped = movies::retain_current_releases(&mut report, day);
                eprintln!("metrics: movies past_releases_skipped={skipped}");
                let rows = movies::release_rows(&report)?;
                let issues = report["issues"].as_array().map_or(0, Vec::len);
                if cli.dry_run {
                    eprintln!(
                        "movies: {} releases would be saved; {issues} issues (dry run)",
                        rows.len()
                    );
                } else {
                    let n = db.as_mut().unwrap().save_movie_releases(&rows, &report)?;
                    eprintln!(
                        "movies: saved {n} releases (week {start} plus tracked movies); {issues} issues"
                    );
                    let inputs = report["releases"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|r| {
                            Some((
                                r["source_url"].as_str()?.to_owned(),
                                posters::decode(&r["poster_metadata"])?,
                            ))
                        })
                        .collect();
                    match posters::enrich(db.as_mut().unwrap(), &inputs) {
                        Ok(posters) => eprintln!(
                            "movie posters: {} enriched, {} preserved, {} unmatched",
                            posters["enriched_count"],
                            posters["preserved_count"],
                            posters["unmatched"].as_array().map_or(0, Vec::len)
                        ),
                        Err(e) => eprintln!(
                            "movie poster enrichment failed (release ingestion completed): {e}"
                        ),
                    }
                }
                if !cli.dry_run {
                    match movie_commerce::enrich(db.as_mut().unwrap(), &report) {
                        Ok(n) => eprintln!("movie retailer links: checked {n} movies/providers"),
                        Err(e) => {
                            eprintln!("movie retailer link enrichment failed (releases saved): {e}")
                        }
                    }
                }
                if issues == 0 {
                    succeeded_sources += 1;
                }
                if issues > 0 {
                    failures.push(format!("movies: {issues} source issues"));
                    eprintln!("movie source issues: {}", report["issues"]);
                }
                Ok(())
            })();
            if let Err(e) = result {
                failures.push(format!("movies: {e}"));
                eprintln!("movie ingestion failed: {e}");
            }
        }

        if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
            d.run_heartbeat(id)?;
        }
        if cli.tv_from_db {
            let _stage_timer = metrics::Stage::new("tv");
            match tvmaze::collect(db.as_mut().unwrap(), cli.dry_run, cli.tv_force_refresh) {
                Ok(report) => {
                    succeeded_sources += report["series"]
                        .as_array()
                        .map_or(0, |s| s.iter().filter(|s| s["status"] != "failed").count());
                    if report["series"]
                        .as_array()
                        .is_some_and(|s| s.iter().any(|s| s["status"] == "failed"))
                    {
                        failures.push("TVmaze series collection failed".into());
                    }
                    eprintln!("TVmaze collection report: {}", report)
                }
                Err(e) => {
                    failures.push(format!("TVmaze: {e}"));
                    eprintln!("TVmaze collector failed: {e}");
                }
            }
        }

        if !cli.dry_run && (cli.boxoffice || cli.boxoffice_forecasts) {
            let _timer = metrics::Stage::new("cinema_posters");
            match cinema_posters::enrich(db.as_mut().unwrap()) {
                Ok(report) => eprintln!("box-office posters: {report}"),
                Err(e) => failures.push(format!("box-office posters: {e}")),
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
                    .guid(
                        rss::GuidBuilder::default()
                            .value(meta.url)
                            .permalink(true)
                            .build(),
                    )
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
            Some(path) => write_atomic(path, &channel.to_string())?,
            // In loop mode, stdout would fill the log with a feed every run.
            None if cli.every_hours.is_some() => {}
            None => println!("{channel}"),
        }
        Ok(())
    })();
    if !cli.dry_run
        && let Some(d) = db.as_mut()
    {
        let _stage_timer = metrics::Stage::new("retention");
        match d.apply_retention() {
            Ok(()) => eprintln!(
                "retention: past/canceled schedules, past releases/episodes and old polls removed"
            ),
            Err(e) => failures.push(format!("retention cleanup failed: {e}")),
        }
    }
    if let (Some(d), Some(id)) = (db.as_mut(), run_id) {
        let message = outcome
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .or_else(|| (!failures.is_empty()).then(|| failures.join("; ")));
        let status = if outcome.is_err() || (!failures.is_empty() && succeeded_sources == 0) {
            "failed"
        } else if failures.is_empty() {
            "succeeded"
        } else {
            "partial"
        };
        if let Err(e) = d.finish_run(id, status, message.as_deref()) {
            eprintln!("cannot persist run outcome: {e}");
        }
    }
    eprintln!(
        "collection summary: {succeeded_sources} source successes, {} failures",
        failures.len()
    );
    outcome?;
    if !failures.is_empty() {
        return Err(format!(
            "collection incomplete ({} failures): {}",
            failures.len(),
            failures.join("; ")
        )
        .into());
    }

    Ok(())
}

fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(contents.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
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
            (0..24)
                .step_by(hours as usize)
                .map(move |h| date.and_hms_opt(h, 0, 0).unwrap())
        })
        .flat_map(|naive| match tz.from_local_datetime(&naive) {
            chrono::LocalResult::Single(a) => vec![a],
            chrono::LocalResult::Ambiguous(a, b) => vec![a, b],
            chrono::LocalResult::None => vec![],
        })
        .map(|t| t.with_timezone(&Utc))
        .filter(|t| *t > now)
        .min()
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
        Cli::command()
            .error(
                ErrorKind::InvalidValue,
                "--every-hours must divide 24 (1, 2, 3, 4, 6, 8, 12, 24)",
            )
            .exit();
    }
    if !cli.tv_from_db
        && !cli.movies
        && cli.urls.is_empty()
        && cli.urls_file.is_none()
        && !cli.from_db
        && cli.events_feed.is_empty()
        && !cli.events_from_db
        && !cli.f1_from_db
        && !cli.boxoffice
        && !cli.boxoffice_forecasts
        && !cli.standings_from_db
        && !cli.rankings_from_db
        && cli.youtube_feed.is_empty()
        && !cli.youtube_from_db
    {
        Cli::command()
            .error(
                ErrorKind::MissingRequiredArgument,
                "provide at least one URL, --urls-file, --from-db, or a collector such as --standings-from-db",
            )
            .exit();
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
        assert_eq!(
            next_slot(utc("2026-09-20T02:00:00Z"), 3),
            utc("2026-09-20T05:00:00Z")
        );
        // exactly on a slot -> the following slot
        assert_eq!(
            next_slot(utc("2026-09-20T05:00:00Z"), 3),
            utc("2026-09-20T08:00:00Z")
        );
        // just before midnight Central rolls to the next day's 00:00
        assert_eq!(
            next_slot(utc("2026-09-21T04:59:00Z"), 3),
            utc("2026-09-21T05:00:00Z")
        );
    }

    #[test]
    fn slots_follow_daylight_saving() {
        // Fall back (Nov 1 2026): 00:30 CDT -> next slot is 03:00 CST = 09:00 UTC
        assert_eq!(
            next_slot(utc("2026-11-01T05:30:00Z"), 3),
            utc("2026-11-01T09:00:00Z")
        );
        // Spring forward (Mar 8 2026): 00:30 CST -> 03:00 CDT = 08:00 UTC
        assert_eq!(
            next_slot(utc("2026-03-08T06:30:00Z"), 3),
            utc("2026-03-08T08:00:00Z")
        );
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;
    #[test]
    fn one_hour_fallback_includes_repeated_clock_hour() {
        let now = DateTime::parse_from_rfc3339("2026-11-01T06:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(next_slot(now, 1).to_rfc3339(), "2026-11-01T07:00:00+00:00");
    }
    #[test]
    fn retention_limits_reject_signed_overflow() {
        for flag in ["--keep", "--per-source", "--youtube-latest"] {
            assert!(
                Cli::try_parse_from(["scraper", flag, "2147483648", "https://example.com"])
                    .is_err()
            );
            assert!(
                Cli::try_parse_from(["scraper", flag, "2147483647", "https://example.com"]).is_ok()
            );
        }
    }
}
