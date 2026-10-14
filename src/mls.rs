//! League-wide current calendar-season MLS fixtures and final results.
use crate::espn_schedule::{Fixture, League};
use chrono::{Datelike, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct Batch {
    pub season: i32,
    pub fixtures: Vec<Fixture>,
    pub results: Vec<Value>,
}
pub fn is_feed(url: &str) -> bool {
    crate::espn_schedule::is_feed(url, League::Mls)
}
pub fn parse(root: &Value, season: i32) -> Result<(Vec<Fixture>, Vec<Value>)> {
    crate::match_results::parse(root, season, League::Mls, "770")
}
pub fn fetch(client: &reqwest::blocking::Client, raw: &str) -> Result<Batch> {
    if !is_feed(raw) {
        return Err("invalid MLS schedule URL".into());
    }
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let season = today.year();
    let mut fixtures = BTreeMap::new();
    let mut results = BTreeMap::new();
    for month in 1..=12 {
        let mut url = crate::http::public_url(raw)?;
        url.set_query(None);
        url.query_pairs_mut()
            .append_pair("dates", &format!("{season:04}{month:02}"))
            .append_pair("limit", "1000");
        let response = client
            .get(url)
            .header(reqwest::header::USER_AGENT, "My-Newsfeed/1.0")
            .send()?
            .error_for_status()?;
        let body = crate::http::response_text_limit(response, crate::http::JSON_LIMIT)?;
        let (games, scores) = parse(&serde_json::from_str(&body)?, season)?;
        for f in games {
            fixtures.insert(f.event.game_id.unwrap(), f);
        }
        for r in scores {
            results.insert(r["gameId"].as_i64().unwrap(), r);
        }
    }
    Ok(Batch {
        season,
        fixtures: fixtures
            .into_values()
            .filter(|f| f.event.event_date >= today || f.completed)
            .collect(),
        results: results.into_values().collect(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn league_season_identity_and_scores() {
        let root: Value =
            serde_json::from_str(include_str!("../tests/fixtures/mls/october-2026.json")).unwrap();
        let (f, r) = parse(&root, 2026).unwrap();
        assert!(!r.is_empty());
        assert!(f.iter().all(|f| f.league == League::Mls));
        assert!(r.iter().all(|r| r["homeScore"].as_i64().unwrap() >= 0));
        assert!(parse(&root, 2025).is_err());
        assert_eq!(League::Mls.canonical_key(123), "espn:usa.1:123");
        let mut bad = root;
        bad["leagues"][0]["id"] = serde_json::json!("700");
        assert!(parse(&bad, 2026).is_err());
    }
    #[test]
    fn shootouts_keep_goals_and_penalties_separate() {
        let root: Value =
            serde_json::from_str(include_str!("../tests/fixtures/mls/playoffs-2025.json")).unwrap();
        let (_, r) = parse(&root, 2025).unwrap();
        let pens = r
            .iter()
            .find(|r| r["status"] == "STATUS_FINAL_PEN")
            .unwrap();
        assert_ne!(pens["homeShootout"], pens["awayShootout"]);
        let mut bad = root;
        for e in bad["events"].as_array_mut().unwrap() {
            if e["status"]["type"]["name"] == "STATUS_FINAL_PEN" {
                e["competitions"][0]["competitors"][0]["shootoutScore"] = Value::Null;
            }
        }
        assert!(parse(&bad, 2025).is_err());
    }
}
