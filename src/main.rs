use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use rss::{ChannelBuilder, ItemBuilder};
use web_scraper::Result;
use web_scraper::pages::{self, PageMeta};
use web_scraper::{
    article_identity, big12, boxoffice, boxoffice_forecasts, championship, cinema_posters, db, epl,
    espn_schedule, events, f1, http, metrics, mlb, mls, movie_commerce, movies, nhl, nwsl, posters,
    rankings, sports_boxscores, sports_periods, standings, tvmaze, wnba, youtube,
};

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
    /// Apply the additive optional game box-score schema with migration credentials.
    #[arg(long, conflicts_with_all=["dry_run","every_hours"])]
    configure_boxscores: bool,
    /// Enrich stored finished games with optional ESPN box scores.
    #[arg(long)]
    boxscores_from_db: bool,
    /// Backfill labels on existing box-score payloads without replacing statistics or values.
    #[arg(long)]
    repair_boxscore_periods: bool,
    /// Maximum optional box-score requests per run (also included by --events-from-db).
    #[arg(long, default_value_t=100, value_parser=clap::value_parser!(i32).range(1..=500))]
    boxscores_limit: i32,
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

    /// Monday-Sunday weeks of movie releases to discover, starting with this week
    #[arg(long, value_name = "N", default_value_t = 4, value_parser = clap::value_parser!(u32).range(1..=12))]
    movie_weeks: u32,

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

impl Cli {
    /// Whether any requested option reads or writes SQL Server.
    fn uses_db(&self) -> bool {
        self.health_check
            || self.initialize_db
            || self.configure_rankings
            || self.rankings_from_db
            || self.configure_movieinsider
            || self.configure_standings
            || self.standings_from_db
            || self.configure_mlb
            || self.configure_wnba
            || self.configure_championship
            || self.boxoffice
            || self.boxoffice_forecasts
            || self.configure_boxoffice
            || self.configure_mls
            || self.configure_nhl
            || self.configure_boxscores
            || self.repair_boxscore_periods
            || self.boxscores_from_db
            || self.configure_f1
            || self.f1_from_db
            || self.configure_epl
            || self.configure_nwsl
            || self.configure_tv
            || self.tv_from_db
            || self.movie_posters
            || self.configure_big12
            || self.enable_big12
            || self.movies
            || self.db
            || self.from_db
            || self.save_sources
            || !self.events_feed.is_empty()
            || self.events_from_db
            || !self.youtube_feed.is_empty()
            || self.youtube_from_db
    }

    /// Whether a schema/registration command was requested; these use migration credentials.
    fn is_setup(&self) -> bool {
        self.configure_boxscores
            || self.configure_rankings
            || self.configure_movieinsider
            || self.configure_standings
            || self.initialize_db
            || self.configure_mlb
            || self.configure_wnba
            || self.configure_championship
            || self.configure_boxoffice
            || self.configure_mls
            || self.configure_nhl
            || self.configure_f1
            || self.configure_epl
            || self.configure_nwsl
            || self.configure_tv
            || self.configure_big12
            || self.enable_big12
    }

    fn dry_suffix(&self) -> &'static str {
        if self.dry_run { " (dry run)" } else { "" }
    }
}

fn connect_db(cli: &Cli) -> Result<Option<db::Db>> {
    if !cli.uses_db() {
        return Ok(None);
    }
    let setup = cli.is_setup();
    let ado = if setup {
        std::env::var("MSSQL_MIGRATION_CONNECTION_STRING")
            .or_else(|_| std::env::var("MSSQL_CONNECTION_STRING"))
    } else {
        std::env::var("MSSQL_CONNECTION_STRING")
    }
    .map_err(|_| "database options require MSSQL_CONNECTION_STRING to be set")?;
    // F1 and box-score setup apply their own additive migrations to an existing database.
    let db = if setup && !cli.configure_f1 && !cli.configure_boxscores {
        db::Db::open_named(&ado, &cli.database)?
    } else {
        db::Db::connect_named(&ado, &cli.database)?
    };
    Ok(Some(db))
}

fn require(db: Option<&mut db::Db>) -> Result<&mut db::Db> {
    db.ok_or_else(|| "this option requires a database connection".into())
}

/// Runs a one-shot health, schema or registration command. Returns false when none was requested.
fn run_setup(cli: &Cli, db: Option<&mut db::Db>) -> Result<bool> {
    let message = if cli.health_check {
        require(db)?.check_health()?;
        return Ok(true);
    } else if cli.initialize_db {
        require(db)?.setup_tv(false)?;
        "database schema initialized"
    } else if cli.configure_rankings {
        require(db)?.configure_rankings()?;
        "National Top 25 sources registered disabled; enable SportsRankingSources to import"
    } else if cli.configure_movieinsider {
        require(db)?.configure_movieinsider()?;
        "Movie Insider digital source registered disabled; enable MovieReleaseSources to import"
    } else if cli.configure_standings {
        require(db)?.configure_standings()?;
        "Standings sources registered disabled; enable SportsStandingsSources to import"
    } else if cli.configure_mlb {
        require(db)?.configure_mlb()?;
        "MLB schedule registered; enable it in SportsEventsType to import"
    } else if cli.configure_boxoffice {
        require(db)?.configure_boxoffice()?;
        "Box-office chart table configured"
    } else if cli.configure_boxscores {
        require(db)?.configure_boxscores()?;
        "configured optional sports box-score cache and read view"
    } else if cli.configure_f1 {
        require(db)?.configure_f1()?;
        "F1 schedule/results and driver/constructor championships configured disabled"
    } else if cli.configure_nhl {
        require(db)?.configure_nhl()?;
        "NHL schedule/results and standings registered disabled"
    } else if cli.configure_mls {
        require(db)?.configure_mls()?;
        "MLS schedule/results registered disabled"
    } else if cli.configure_epl {
        require(db)?.configure_epl()?;
        "Premier League sources registered disabled"
    } else if cli.configure_championship {
        require(db)?.configure_championship()?;
        "EFL Championship sources registered disabled"
    } else if cli.configure_wnba {
        require(db)?.configure_wnba()?;
        "WNBA schedule registered; enable it in SportsEventsType to import"
    } else if cli.configure_nwsl {
        require(db)?.configure_nwsl()?;
        "League-wide NWSL schedule registered; existing Enabled setting preserved"
    } else if cli.configure_tv {
        require(db)?.setup_tv(true)?;
        "TVmaze schema ready; five tracking rows seeded without changing existing Enabled settings"
    } else if cli.configure_big12 || cli.enable_big12 {
        let client = http::client_builder()
            .timeout(Duration::from_secs(20))
            .user_agent(USER_AGENT)
            .build()?;
        let members = big12::current_members(&client)?;
        require(db)?.configure_big12(&members, cli.enable_big12)?;
        eprintln!(
            "configured {} current Big 12 members; new feeds {}",
            members.len(),
            if cli.enable_big12 {
                "enabled"
            } else {
                "disabled"
            }
        );
        return Ok(true);
    } else if cli.movie_posters {
        let db = require(db)?;
        if cli.retry_missing_posters {
            db.retry_missing_posters()?;
        }
        let report = posters::enrich(db, &BTreeMap::new())?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        eprintln!("box-office posters: {}", cinema_posters::enrich(db)?);
        return Ok(true);
    } else {
        return Ok(false);
    };
    eprintln!("{message}");
    Ok(true)
}

const USER_AGENT: &str = "Mozilla/5.0 (compatible; web_scraper/0.1)";

fn json_client() -> Result<reqwest::blocking::Client> {
    Ok(http::client_builder()
        .user_agent("My-Newsfeed/1.0")
        .timeout(Duration::from_secs(30))
        .build()?)
}

/// A calendar feed to ingest: its `SportsEventsType` row (if registered), URL and school name.
struct EventFeedRef {
    known_id: Option<i32>,
    url: String,
    school: Option<String>,
}

/// Provider-specific labels for an ESPN-backed league feed.
struct League {
    stage: &'static str,
    /// Prefix for failure messages.
    label: &'static str,
    /// `SportsEventsType` name used when an unregistered feed URL is first saved.
    type_name: &'static str,
}

/// Per Big 12 feed: saved, quarantined, issues, and whether the feed succeeded overall.
struct Big12Tally {
    saved: i32,
    quarantined: i32,
    errors: Vec<String>,
    success: bool,
}

/// State of one collection run: what succeeded, what failed, and the run-log row to heartbeat.
struct Run<'a> {
    cli: &'a Cli,
    db: Option<db::Db>,
    run_id: Option<i32>,
    failures: Vec<String>,
    succeeded: usize,
}

impl Run<'_> {
    fn db(&mut self) -> Result<&mut db::Db> {
        require(self.db.as_mut())
    }

    fn heartbeat(&mut self) -> Result<()> {
        if let (Some(d), Some(id)) = (self.db.as_mut(), self.run_id) {
            d.run_heartbeat(id)?;
        }
        Ok(())
    }

    fn collect(&mut self) -> Result<()> {
        let cli = self.cli;
        let mut urls = cli.urls.clone();
        if let Some(path) = &cli.urls_file {
            urls.extend(read_urls_file(path)?);
        }
        if cli.save_sources {
            let added = self.db()?.add_sources(&urls)?;
            eprintln!(
                "added {added} new sources ({} already present)",
                urls.len() - added
            );
        }
        if cli.from_db {
            for url in self.db()?.enabled_sources()? {
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
            && !cli.repair_boxscore_periods
            && !cli.boxscores_from_db
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
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(15))
            .build()?;

        if cli.f1_from_db || cli.events_from_db {
            self.collect_f1()?;
        }
        if cli.rankings_from_db {
            self.collect_rankings()?;
        }
        if cli.boxoffice {
            self.collect_boxoffice();
        }
        if cli.boxoffice_forecasts {
            self.collect_boxoffice_forecasts()?;
        }
        if cli.standings_from_db {
            self.collect_standings()?;
        }
        let pages = self.collect_news(&client, urls)?;
        self.collect_event_feeds(&client)?;
        self.heartbeat()?;
        self.collect_youtube(&client)?;
        self.heartbeat()?;
        if cli.movies {
            self.collect_movies();
        }
        self.heartbeat()?;
        if cli.tv_from_db {
            self.collect_tv()?;
        }
        if !cli.dry_run && (cli.boxoffice || cli.boxoffice_forecasts) {
            let _timer = metrics::Stage::new("cinema_posters");
            match cinema_posters::enrich(self.db()?) {
                Ok(report) => eprintln!("box-office posters: {report}"),
                Err(e) => self.failures.push(format!("box-office posters: {e}")),
            }
        }
        if cli.repair_boxscore_periods {
            let _timer = metrics::Stage::new("sports_periods");
            let run_id = self.run_id;
            let (ok, errors) =
                sports_periods::repair(self.db()?, cli.dry_run, cli.boxscores_limit, run_id)?;
            self.succeeded += ok;
            if errors > 0 {
                self.failures.push(format!(
                    "period repair: {errors} metadata/validation errors; cached values preserved"
                ));
            }
        }
        if cli.boxscores_from_db || cli.events_from_db {
            let _timer = metrics::Stage::new("sports_boxscores");
            let run_id = self.run_id;
            match sports_boxscores::collect(self.db()?, cli.dry_run, cli.boxscores_limit, run_id) {
                Ok((ok, errors)) => {
                    self.succeeded += ok;
                    if errors > 0 {
                        self.failures.push(format!("optional box scores: {errors} fetch/validation errors; existing results preserved"));
                    }
                }
                Err(e) => self.failures.push(format!("optional box scores: {e}")),
            }
        }
        write_feed(cli, pages)
    }

    fn collect_f1(&mut self) -> Result<()> {
        let _timer = metrics::Stage::new("f1");
        let dry = self.cli.dry_run;
        if !self.db()?.f1_enabled()? {
            if self.cli.f1_from_db {
                self.failures
                    .push("F1 source is disabled or not configured".to_string());
            }
            return Ok(());
        }
        if !dry {
            self.db()?.f1_status(None)?;
        }
        let result = (|| -> Result<()> {
            let f1_client = http::client_builder()
                .timeout(Duration::from_secs(30))
                .build()?;
            let snapshot = f1::fetch(&f1_client)?;
            eprintln!("{}{}", snapshot.summary(), self.cli.dry_suffix());
            if !dry {
                self.db()?.save_f1(&snapshot)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.succeeded += 1,
            Err(e) => {
                if !dry {
                    self.db()?.f1_status(Some(&e.to_string()))?;
                }
                self.failures.push(format!("F1: {e}"));
            }
        }
        Ok(())
    }

    fn collect_rankings(&mut self) -> Result<()> {
        let _stage_timer = metrics::Stage::new("rankings");
        let dry = self.cli.dry_run;
        let client = json_client()?;
        let sources = self.db()?.ranking_sources()?;
        if sources.is_empty() {
            return Ok(());
        }
        let members = rankings::current_members(&client);
        for source in sources {
            if !dry {
                self.db()?.ranking_attempt(source.id)?;
            }
            let result = (|| -> Result<()> {
                let members = members
                    .as_ref()
                    .map_err(|e| format!("Big 12 membership: {e}"))?;
                let poll = rankings::fetch(&client, &source, members)?;
                if !dry {
                    self.db()?.save_ranking(&source, &poll)?;
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
                    self.cli.dry_suffix()
                );
                Ok(())
            })();
            self.heartbeat()?;
            match result {
                Ok(()) => self.succeeded += 1,
                Err(e) => {
                    let message = format!("{} rankings: {e}", source.sport);
                    eprintln!("{message}");
                    if !dry && let Err(e) = self.db()?.ranking_error(source.id, &message) {
                        self.failures.push(format!("ranking error tracking: {e}"));
                    }
                    self.failures.push(message);
                }
            }
        }
        Ok(())
    }

    fn collect_boxoffice(&mut self) {
        let _timer = metrics::Stage::new("boxoffice");
        let result = (|| -> Result<()> {
            let chart = boxoffice::fetch(&json_client()?)?;
            if !self.cli.dry_run {
                self.db()?.save_boxoffice(&chart)?;
            }
            eprintln!(
                "Box office: 10 movies, weekend {}{}",
                chart.weekend,
                self.cli.dry_suffix()
            );
            Ok(())
        })();
        match result {
            Ok(()) => self.succeeded += 1,
            Err(e) => self.failures.push(format!("Box office: {e}")),
        }
    }

    fn collect_boxoffice_forecasts(&mut self) -> Result<()> {
        let _timer = metrics::Stage::new("boxoffice_forecasts");
        let dry = self.cli.dry_run;
        if !dry {
            self.db()?.boxoffice_forecast_status(None)?;
        }
        let result = (|| -> Result<()> {
            let snapshots = boxoffice_forecasts::fetch(&json_client()?)?;
            if !dry {
                self.db()?.save_boxoffice_forecasts(&snapshots)?;
            }
            for snapshot in &snapshots {
                eprintln!(
                    "Boxoffice Pro {}: {} current/future movies{}",
                    snapshot.kind,
                    snapshot.rows.len(),
                    self.cli.dry_suffix()
                );
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.succeeded += 1,
            Err(e) => {
                let message = format!("Boxoffice Pro forecasts: {e}");
                if !dry {
                    self.db()?.boxoffice_forecast_status(Some(&message))?;
                }
                self.failures.push(message);
            }
        }
        Ok(())
    }

    fn collect_standings(&mut self) -> Result<()> {
        let _stage_timer = metrics::Stage::new("standings");
        let dry = self.cli.dry_run;
        let client = json_client()?;
        for source in self.db()?.standings_sources()? {
            if !dry {
                self.db()?.standings_attempt(source.id)?;
            }
            let result = standings::fetch(&client, &source).and_then(|snapshot| {
                if !dry {
                    self.db()?.save_standings(&source, &snapshot)?;
                }
                eprintln!(
                    "{} {} standings: {} teams, season {}{}",
                    source.provider,
                    source.sport,
                    snapshot.entries.len(),
                    snapshot.season,
                    self.cli.dry_suffix()
                );
                Ok(())
            });
            self.heartbeat()?;
            match result {
                Ok(()) => self.succeeded += 1,
                Err(e) => {
                    let message = format!("{} {} standings: {e}", source.provider, source.sport);
                    eprintln!("{message}");
                    if !dry && let Err(e) = self.db()?.standings_error(source.id, &message) {
                        self.failures.push(format!("standings error tracking: {e}"));
                    }
                    self.failures.push(message);
                }
            }
        }
        Ok(())
    }

    /// Scrapes every news source, then filters, saves and prunes. Returns the pages for the RSS output.
    fn collect_news(
        &mut self,
        client: &reqwest::blocking::Client,
        mut urls: Vec<String>,
    ) -> Result<Vec<PageMeta>> {
        let _news_timer = metrics::Stage::new("news");
        let cli = self.cli;
        let mut pages = Vec::new();
        let mut fetched = Vec::new();
        let identity_started = std::time::Instant::now();
        let identity_client = article_identity::client()?;
        // Rotate the identity work order so the bounded HTTP budget cannot starve later sources.
        if !urls.is_empty() {
            let offset = (Utc::now().timestamp().div_euclid(3600) as usize) % urls.len();
            urls.rotate_left(offset);
        }
        for url in &urls {
            let result = pages::scrape_source(client, url);
            self.heartbeat()?;
            match result {
                Ok((mut found, how)) => {
                    fetched.push(url.clone());
                    if cli.dry_run || self.db.is_none() {
                        self.succeeded += 1;
                    }
                    let total = found.len();
                    if let Some(cap) = cli.per_source {
                        found.sort_by_key(|p| std::cmp::Reverse(p.published));
                        found.truncate(cap as usize);
                    }
                    let recent = match self.db.as_mut() {
                        Some(d) => d.recent_articles(url)?,
                        None => vec![],
                    };
                    pages::enrich_article_identities(
                        &identity_client,
                        &mut found,
                        recent,
                        identity_started,
                        self.db.as_mut(),
                        cli.dry_run,
                    )?;
                    if how == "feed" {
                        let missing = found.iter().filter(|p| p.image_url.is_none()).count();
                        pages::fill_missing_images(client, &mut found);
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
                    self.failures.push(format!("news {url}: {e}"));
                    if let Some(d) = self.db.as_mut().filter(|_| !cli.dry_run) {
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
        } else if !urls.is_empty()
            && let Some(db) = self.db.as_mut()
        {
            let mut n = 0;
            for url in &urls {
                if !fetched.contains(url) {
                    continue;
                }
                let source_pages: Vec<_> = pages.iter().filter(|p| p.source == *url).collect();
                match db.save_page_refs(&source_pages) {
                    Ok(count) => {
                        n += count;
                        self.succeeded += 1;
                        db.record_result(url, None)?;
                    }
                    Err(e) => {
                        self.failures.push(format!("save news {url}: {e}"));
                        let _ = db.record_result(url, Some(&e.to_string()));
                    }
                }
                if let Some(id) = self.run_id {
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
        Ok(pages)
    }

    /// Calendar feeds from --events-feed and from the lookup table, each with its known type row
    /// (if any) and school name. A flagged URL that is already in the table reuses that row.
    fn event_feeds(&mut self) -> Result<Vec<EventFeedRef>> {
        let cli = self.cli;
        let mut feeds: Vec<EventFeedRef> = Vec::new();
        if cli.events_feed.is_empty() && !cli.events_from_db {
            return Ok(feeds);
        }
        let known = self.db()?.sports_event_feeds()?;
        for url in &cli.events_feed {
            let row = known.iter().find(|f| f.url == *url);
            feeds.push(EventFeedRef {
                known_id: row.map(|f| f.id),
                url: url.clone(),
                school: row.and_then(|f| f.school.clone()),
            });
        }
        if cli.events_from_db {
            for f in known {
                if !feeds.iter().any(|e| e.url == f.url) {
                    feeds.push(EventFeedRef {
                        known_id: Some(f.id),
                        url: f.url,
                        school: f.school,
                    });
                }
            }
        }
        Ok(feeds)
    }

    fn collect_event_feeds(&mut self, client: &reqwest::blocking::Client) -> Result<()> {
        let feeds = self.event_feeds()?;
        self.heartbeat()?;
        let big12_feeds: Vec<_> = feeds
            .iter()
            .filter(|f| big12::school_id(&f.url).is_some())
            .filter_map(|f| {
                f.known_id.map(|id| db::EventFeed {
                    id,
                    url: f.url.clone(),
                    school: f.school.clone(),
                })
            })
            .collect();
        if !big12_feeds.is_empty() {
            self.collect_big12(client, &big12_feeds)?;
        }
        self.heartbeat()?;
        for feed in &feeds {
            self.collect_event_feed(client, feed)?;
        }
        Ok(())
    }

    fn collect_big12(
        &mut self,
        client: &reqwest::blocking::Client,
        feeds: &[db::EventFeed],
    ) -> Result<()> {
        let _stage_timer = metrics::Stage::new("big12_schedules");
        let dry = self.cli.dry_run;
        if !dry {
            for f in feeds {
                self.db()?.event_attempt(f.id)?;
            }
        }
        let collection = {
            let _timer = metrics::Stage::new("big12_http");
            big12::collect(client, feeds)
        };
        let results = match collection {
            Ok(results) => results,
            Err(e) => {
                self.failures.push(format!("Big12: {e}"));
                eprintln!("Big 12 collection failed: {e}");
                if !dry {
                    for f in feeds {
                        self.db()?
                            .event_result(f.id, false, 0, 1, Some(&e.to_string()))?;
                    }
                }
                return Ok(());
            }
        };
        let _sql_timer = metrics::Stage::new("big12_sql");
        let mut tallies: BTreeMap<i32, Big12Tally> = results
            .iter()
            .map(|r| {
                let mut errors = r.warnings.clone();
                errors.extend(r.error.clone());
                let tally = Big12Tally {
                    saved: 0,
                    quarantined: 0,
                    errors,
                    success: r.error.is_none(),
                };
                (r.id, tally)
            })
            .collect();
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
            let tally = tallies
                .get_mut(&feed)
                .ok_or("Big 12 observation for an unknown feed")?;
            if let Some(issue) = observation["issue"].as_str() {
                tally
                    .errors
                    .push(format!("{}: {issue}", observation["provider_game_id"]));
            }
            if dry {
                if observation["issue"].is_null() {
                    tally.saved += 1;
                } else {
                    tally.quarantined += 1;
                }
                continue;
            }
            match require(self.db.as_mut())?.save_big12_observation(feed, observation) {
                Ok(outcome) if outcome == "saved" => tally.saved += 1,
                Ok(outcome) => {
                    tally.quarantined += 1;
                    if observation["issue"].is_null() {
                        tally
                            .errors
                            .push(format!("{}: {outcome}", observation["provider_game_id"]));
                    }
                }
                Err(e) => {
                    tally.quarantined += 1;
                    tally.success = false;
                    tally.errors.push(e.to_string());
                }
            }
        }
        for result in results {
            let tally = tallies
                .remove(&result.id)
                .ok_or("Big 12 result reported twice")?;
            if tally.success {
                self.succeeded += 1;
            } else {
                self.failures
                    .push(format!("Big12 {} failed", result.school));
            }
            eprintln!(
                "Big 12 {}: {} source observations saved, {} quarantined, {} issues{}",
                result.school,
                tally.saved,
                tally.quarantined,
                tally.errors.len(),
                self.cli.dry_suffix()
            );
            if !dry {
                let error = (!tally.errors.is_empty()).then(|| tally.errors.join("; "));
                self.db()?.event_result(
                    result.id,
                    tally.success,
                    result.observations.len() as i32,
                    tally.errors.len() as i32,
                    error.as_deref(),
                )?;
            }
        }
        Ok(())
    }

    /// Records a failed feed on its registered row (unregistered feeds have none yet).
    fn event_feed_failed(&mut self, known_id: Option<i32>, error: &str) -> Result<()> {
        if !self.cli.dry_run
            && let Some(id) = known_id
        {
            self.db()?.event_result(id, false, 0, 1, Some(error))?;
        }
        Ok(())
    }

    fn collect_event_feed(
        &mut self,
        client: &reqwest::blocking::Client,
        feed: &EventFeedRef,
    ) -> Result<()> {
        let url = feed.url.as_str();
        if big12::is_provider_feed(url) {
            if feed.known_id.is_none() {
                eprintln!(
                    "{url}: register and enable this school using --configure-big12 / --enable-big12"
                );
            } else if big12::school_id(url).is_none() {
                eprintln!(
                    "{url}: conference-wide RSS is not supported for ingestion; use registered school feeds"
                );
            }
            return Ok(());
        }
        if !self.cli.dry_run
            && let Some(id) = feed.known_id
        {
            self.db()?.event_attempt(id)?;
        }

        if epl::is_feed(url) {
            const EPL: League = League {
                stage: "epl_schedule",
                label: "Premier League",
                type_name: "Premier League",
            };
            return self.collect_league(
                &EPL,
                feed,
                || {
                    let fixtures = epl::fetch(client, url)?;
                    eprintln!(
                        "Premier League: {} current/future schedule observations",
                        fixtures.len()
                    );
                    Ok(fixtures)
                },
                |d, id, fixtures| Ok(d.save_espn_schedule(id, fixtures)? as i32),
            );
        }
        if nhl::is_feed(url) {
            const NHL: League = League {
                stage: "nhl_schedule",
                label: "NHL",
                type_name: "NHL",
            };
            return self.collect_league(
                &NHL,
                feed,
                || {
                    let batch = nhl::fetch(client, url)?;
                    eprintln!(
                        "NHL season {}: {} schedule observations, {} final results",
                        batch.season,
                        batch.fixtures.len(),
                        batch.results.len()
                    );
                    Ok(batch)
                },
                |d, id, batch| {
                    d.save_espn_schedule(id, &batch.fixtures)?;
                    d.save_match_results(
                        id,
                        espn_schedule::League::Nhl,
                        batch.season,
                        &batch.results,
                    )?;
                    Ok(upcoming(&batch.fixtures))
                },
            );
        }
        if mls::is_feed(url) {
            const MLS: League = League {
                stage: "mls_schedule",
                label: "MLS",
                type_name: "MLS",
            };
            return self.collect_league(
                &MLS,
                feed,
                || {
                    let batch = mls::fetch(client, url)?;
                    eprintln!(
                        "MLS season {}: {} schedule observations, {} final results",
                        batch.season,
                        batch.fixtures.len(),
                        batch.results.len()
                    );
                    Ok(batch)
                },
                |d, id, batch| {
                    d.save_espn_schedule(id, &batch.fixtures)?;
                    d.save_match_results(
                        id,
                        espn_schedule::League::Mls,
                        batch.season,
                        &batch.results,
                    )?;
                    Ok(upcoming(&batch.fixtures))
                },
            );
        }
        if championship::is_feed(url) {
            const CHAMPIONSHIP: League = League {
                stage: "championship_schedule",
                label: "Championship",
                type_name: "EFL Championship",
            };
            return self.collect_league(
                &CHAMPIONSHIP,
                feed,
                || {
                    let batch = championship::fetch(client, url)?;
                    eprintln!(
                        "Championship season {}: {} schedule observations, {} final results; Wrexham ID {}",
                        batch.season,
                        batch.fixtures.len(),
                        batch.results.len(),
                        championship::WREXHAM_ID
                    );
                    Ok(batch)
                },
                |d, id, batch| {
                    d.save_espn_schedule(id, &batch.fixtures)?;
                    d.save_championship_results(id, batch)?;
                    Ok(upcoming(&batch.fixtures))
                },
            );
        }
        let league = if mlb::is_feed(url) {
            Some(("mlb_schedule", "MLB", mlb::fetch as FixtureFetch))
        } else if wnba::is_feed(url) {
            Some(("wnba_schedule", "WNBA", wnba::fetch as FixtureFetch))
        } else if nwsl::is_feed(url) {
            Some(("nwsl_schedule", "NWSL", nwsl::fetch as FixtureFetch))
        } else {
            None
        };
        if let Some((stage, label, fetch)) = league {
            let _stage_timer = metrics::Stage::new(stage);
            return self.collect_schedule_only(feed, label, fetch(client, url));
        }
        self.collect_calendar(client, feed)
    }

    /// Fetch, persist and record one ESPN-backed league feed. Provider and save failures are
    /// recorded against the feed and the run continues.
    fn collect_league<T>(
        &mut self,
        league: &League,
        feed: &EventFeedRef,
        fetch: impl FnOnce() -> Result<T>,
        save: impl FnOnce(&mut db::Db, i32, &T) -> Result<i32>,
    ) -> Result<()> {
        let _stage_timer = metrics::Stage::new(league.stage);
        let result = (|| -> Result<()> {
            let data = fetch()?;
            if !self.cli.dry_run {
                let d = self.db()?;
                let id = match feed.known_id {
                    Some(id) => id,
                    None => d.ensure_sports_events_type(&feed.url, league.type_name)?,
                };
                let n = save(d, id, &data)?;
                d.event_result(id, true, n, 0, None)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.succeeded += 1,
            Err(e) => {
                self.failures.push(format!("{}: {e}", league.label));
                self.event_feed_failed(feed.known_id, &e.to_string())?;
            }
        }
        Ok(())
    }

    /// MLB, WNBA and NWSL league schedules (fixtures only; results come from other paths).
    fn collect_schedule_only(
        &mut self,
        feed: &EventFeedRef,
        label: &str,
        fetched: Result<Vec<espn_schedule::Fixture>>,
    ) -> Result<()> {
        let url = feed.url.as_str();
        let fixtures = match fetched {
            Ok(fixtures) => fixtures,
            Err(e) => {
                self.failures.push(format!("{label} {url}: {e}"));
                eprintln!("{label} schedule {url} failed: {e}");
                return self.event_feed_failed(feed.known_id, &e.to_string());
            }
        };
        eprintln!(
            "{url}: {} {label} fixtures{}",
            fixtures.len(),
            self.cli.dry_suffix()
        );
        if self.cli.dry_run {
            self.succeeded += 1;
            for f in &fixtures {
                eprintln!(
                    "{} | {} | {:?} | {}",
                    f.event.event_date, f.event.title, f.event.starts_at, f.status
                );
            }
            return Ok(());
        }
        let db = require(self.db.as_mut())?;
        let id = match feed.known_id {
            Some(id) => id,
            None => db.ensure_sports_events_type(url, label)?,
        };
        match db.save_espn_schedule(id, &fixtures) {
            Ok(n) => {
                self.succeeded += 1;
                db.event_result(id, true, n as i32, 0, None)?;
            }
            Err(e) => {
                eprintln!("{label} save failed: {e}");
                self.failures.push(format!("{label} save: {e}"));
                let _ = db.event_result(id, false, 0, 1, Some(&e.to_string()));
            }
        }
        Ok(())
    }

    /// A Sidearm-style school calendar. A broken feed shouldn't stop the rest of the run.
    fn collect_calendar(
        &mut self,
        client: &reqwest::blocking::Client,
        feed: &EventFeedRef,
    ) -> Result<()> {
        let url = feed.url.as_str();
        let batch = match events::fetch_events(client, url, feed.school.as_deref()) {
            Ok(batch) => batch,
            Err(e) => {
                self.failures.push(format!("calendar {url}: {e}"));
                eprintln!("events feed {url} failed: {e}");
                return self.event_feed_failed(feed.known_id, &e.to_string());
            }
        };
        if self.cli.dry_run {
            eprintln!("{url}: {} events (dry run, not saved)", batch.events.len());
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
            return Ok(());
        }
        let db = require(self.db.as_mut())?;
        let type_id = match feed.known_id {
            Some(id) => id,
            None => {
                let name = url::Url::parse(url)
                    .ok()
                    .and_then(|u| u.host_str().map(str::to_string))
                    .unwrap_or_else(|| url.to_string());
                db.ensure_sports_events_type(url, &name)?
            }
        };
        let n = match db.save_events(type_id, &batch.events) {
            Ok(n) => n,
            Err(e) => {
                self.failures.push(format!("calendar save {url}: {e}"));
                let _ = db.event_result(type_id, false, 0, 1, Some(&e.to_string()));
                return Ok(());
            }
        };
        eprintln!("{url}: saved {n} events to type {type_id}");
        self.succeeded += 1;
        db.event_result(type_id, true, n as i32, 0, None)?;
        if batch.school_id.is_some() {
            let corrected = db.refresh_school_team_logo(type_id, batch.team_logo_url.as_deref())?;
            eprintln!("{url}: corrected team logos on {corrected} retained events");
        }
        Ok(())
    }

    /// Channels from --youtube-feed (feed row found or created below) and from the lookup table.
    fn collect_youtube(&mut self, client: &reqwest::blocking::Client) -> Result<()> {
        let cli = self.cli;
        let mut feeds: Vec<(Option<i32>, String)> =
            cli.youtube_feed.iter().map(|u| (None, u.clone())).collect();
        if cli.youtube_from_db {
            for (id, url) in self.db()?.youtube_feeds()? {
                match feeds.iter_mut().find(|(_, u)| *u == url) {
                    Some(existing) => existing.0 = Some(id),
                    None => feeds.push((Some(id), url)),
                }
            }
        }
        let _youtube_timer = metrics::Stage::new("youtube");
        for (known_id, feed_url) in &feeds {
            let (found, how) =
                match youtube::fetch_latest(client, feed_url, cli.youtube_latest as usize) {
                    Ok(fetched) => fetched,
                    Err(e) => {
                        self.failures.push(format!("YouTube {feed_url}: {e}"));
                        eprintln!("youtube feed {feed_url} failed: {e}");
                        continue;
                    }
                };
            if cli.dry_run {
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
                continue;
            }
            let db = require(self.db.as_mut())?;
            let feed_id = match (known_id, found.first()) {
                (Some(id), _) => *id,
                (None, Some(v)) => {
                    db.ensure_youtube_feed(&v.channel_id, v.channel_name.as_deref(), feed_url)?
                }
                (None, None) => {
                    eprintln!("{feed_url}: the feed has no videos, nothing saved");
                    continue;
                }
            };
            let n = match db.save_videos(feed_id, &found) {
                Ok(n) => n,
                Err(e) => {
                    self.failures.push(format!("YouTube save {feed_url}: {e}"));
                    continue;
                }
            };
            self.succeeded += 1;
            let pruned = db.prune_videos(cli.youtube_latest)?;
            eprintln!(
                "{feed_url}: saved {n} videos via {how} to feed {feed_id}, pruned {pruned} older (keeping the newest {} per channel)",
                cli.youtube_latest
            );
        }
        Ok(())
    }

    fn collect_movies(&mut self) {
        let _stage_timer = metrics::Stage::new("movies");
        if let Err(e) = self.try_collect_movies() {
            self.failures.push(format!("movies: {e}"));
            eprintln!("movie ingestion failed: {e}");
        }
    }

    fn try_collect_movies(&mut self) -> Result<()> {
        use chrono::Datelike;
        let dry = self.cli.dry_run;
        let day = Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive();
        let start = day - chrono::Duration::days(day.weekday().num_days_from_monday().into());
        let db = self.db()?;
        let tracked = db.tracked_movie_urls()?;
        let enabled = db.enabled_movie_sources()?;
        let disc_disabled = db.disc_disabled_movie_sources()?;
        let window = movies::Window {
            start,
            weeks: self.cli.movie_weeks,
        };
        let mut report =
            movies::ingest_with_disc_policy(window, &tracked, &enabled, &disc_disabled)?;
        let skipped = movies::retain_current_releases(&mut report, day);
        eprintln!("metrics: movies past_releases_skipped={skipped}");
        let rows = movies::release_rows(&report)?;
        let issues = report["issues"].as_array().map_or(0, Vec::len);
        if dry {
            eprintln!(
                "movies: {} releases would be saved; {issues} issues (dry run)",
                rows.len()
            );
        } else {
            let db = self.db()?;
            let n = db.save_movie_releases(&rows, &report)?;
            eprintln!(
                "movies: saved {n} releases ({} weeks from {start} plus tracked movies); {issues} issues",
                window.weeks
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
            match posters::enrich(db, &inputs) {
                Ok(posters) => eprintln!(
                    "movie posters: {} enriched, {} preserved, {} unmatched",
                    posters["enriched_count"],
                    posters["preserved_count"],
                    posters["unmatched"].as_array().map_or(0, Vec::len)
                ),
                Err(e) => {
                    eprintln!("movie poster enrichment failed (release ingestion completed): {e}")
                }
            }
            match movie_commerce::enrich(db, &report) {
                Ok(n) => eprintln!("movie retailer links: checked {n} movies/providers"),
                Err(e) => eprintln!("movie retailer link enrichment failed (releases saved): {e}"),
            }
        }
        if issues == 0 {
            self.succeeded += 1;
        } else {
            self.failures
                .push(format!("movies: {issues} source issues"));
            eprintln!("movie source issues: {}", report["issues"]);
        }
        Ok(())
    }

    fn collect_tv(&mut self) -> Result<()> {
        let _stage_timer = metrics::Stage::new("tv");
        let (dry, force) = (self.cli.dry_run, self.cli.tv_force_refresh);
        match tvmaze::collect(self.db()?, dry, force) {
            Ok(report) => {
                let series = report["series"].as_array();
                self.succeeded +=
                    series.map_or(0, |s| s.iter().filter(|s| s["status"] != "failed").count());
                if series.is_some_and(|s| s.iter().any(|s| s["status"] == "failed")) {
                    self.failures.push("TVmaze series collection failed".into());
                }
                eprintln!("TVmaze collection report: {report}");
            }
            Err(e) => {
                self.failures.push(format!("TVmaze: {e}"));
                eprintln!("TVmaze collector failed: {e}");
            }
        }
        Ok(())
    }

    /// Applies retention, records the run outcome, and folds partial failures into the result.
    fn finish(mut self, outcome: Result<()>) -> Result<()> {
        if !self.cli.dry_run
            && let Some(d) = self.db.as_mut()
        {
            let _stage_timer = metrics::Stage::new("retention");
            match d.apply_retention() {
                Ok(()) => eprintln!(
                    "retention: past/canceled schedules, past releases/episodes and old polls removed"
                ),
                Err(e) => self.failures.push(format!("retention cleanup failed: {e}")),
            }
        }
        if let (Some(d), Some(id)) = (self.db.as_mut(), self.run_id) {
            let message = outcome
                .as_ref()
                .err()
                .map(|e| e.to_string())
                .or_else(|| (!self.failures.is_empty()).then(|| self.failures.join("; ")));
            let status = if outcome.is_err() || (!self.failures.is_empty() && self.succeeded == 0) {
                "failed"
            } else if self.failures.is_empty() {
                "succeeded"
            } else {
                "partial"
            };
            if let Err(e) = d.finish_run(id, status, message.as_deref()) {
                eprintln!("cannot persist run outcome: {e}");
            }
        }
        eprintln!(
            "collection summary: {} source successes, {} failures",
            self.succeeded,
            self.failures.len()
        );
        outcome?;
        if !self.failures.is_empty() {
            return Err(format!(
                "collection incomplete ({} failures): {}",
                self.failures.len(),
                self.failures.join("; ")
            )
            .into());
        }
        Ok(())
    }
}

type FixtureFetch = fn(&reqwest::blocking::Client, &str) -> Result<Vec<espn_schedule::Fixture>>;

/// Count of not-yet-completed fixtures, recorded as the feed's event count.
fn upcoming(fixtures: &[espn_schedule::Fixture]) -> i32 {
    fixtures.iter().filter(|f| !f.completed).count() as i32
}

fn write_feed(cli: &Cli, pages: Vec<PageMeta>) -> Result<()> {
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
}

/// One full scrape: collect sources, scrape, filter, save, prune, and write the RSS feed.
fn run_once(cli: &Cli) -> Result<()> {
    let mut db = connect_db(cli)?;
    if run_setup(cli, db.as_mut())? {
        return Ok(());
    }
    let run_id = match db.as_mut() {
        Some(d) if !cli.dry_run => Some(d.start_run()?),
        _ => None,
    };
    let mut run = Run {
        cli,
        db,
        run_id,
        failures: Vec::new(),
        succeeded: 0,
    };
    let _run_timer = metrics::Stage::new("run");
    let outcome = run.collect();
    run.finish(outcome)
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

fn main() -> Result<()> {
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
        && !cli.repair_boxscore_periods
        && !cli.boxscores_from_db
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
