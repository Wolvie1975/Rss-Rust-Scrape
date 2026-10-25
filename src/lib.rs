//! Collectors and SQL Server persistence shared by the `web_scraper` and `scoreboard` binaries.

pub mod article_identity;
pub mod big12;
pub mod boxoffice;
pub mod boxoffice_forecasts;
pub mod championship;
pub mod cinema_posters;
pub mod db;
pub mod epl;
pub mod espn_schedule;
pub mod events;
pub mod f1;
pub mod feed;
pub mod http;
pub mod match_results;
pub mod metrics;
pub mod mlb;
pub mod mls;
pub mod movie_commerce;
pub mod movie_metadata;
pub mod movies;
pub mod nhl;
pub mod nwsl;
pub mod pages;
pub mod posters;
pub mod rankings;
pub mod scoreboard;
pub mod sports_boxscores;
pub mod sports_periods;
pub mod standings;
pub mod tvmaze;
pub mod wnba;
pub mod youtube;

/// Error type shared by every collector; `Send + Sync` so errors can cross worker threads.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T, E = Error> = std::result::Result<T, E>;
