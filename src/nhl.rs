//! League-wide NHL fixtures and final scores; seasons use ESPN's ending year.
use crate::Result;
use crate::espn_schedule::{self, Fixture, League};
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
pub struct Batch {
    pub season: i32,
    pub fixtures: Vec<Fixture>,
    pub results: Vec<Value>,
}
pub fn is_feed(url: &str) -> bool {
    espn_schedule::is_feed(url, League::Nhl)
}
fn season_for(today: NaiveDate) -> i32 {
    today.year() + i32::from(today.month() >= 7)
}
pub fn parse(root: &Value, season: i32) -> Result<(Vec<Fixture>, Vec<Value>)> {
    crate::match_results::parse(root, season, League::Nhl, "90")
}
pub fn fetch(client: &reqwest::blocking::Client, raw: &str) -> Result<Batch> {
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let season = season_for(today);
    let mut fixtures = BTreeMap::new();
    let mut results = BTreeMap::new();
    let base = crate::http::public_url(raw)?;
    if !is_feed(raw) {
        return Err("invalid NHL feed".into());
    }
    for offset in 0..12 {
        let month = (6 + offset) % 12 + 1;
        let year = season - if month >= 7 { 1 } else { 0 };
        let mut url = base.clone();
        url.set_query(None);
        url.query_pairs_mut()
            .append_pair("dates", &format!("{year:04}{month:02}"))
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
    // Keep completed observations for schedule reconciliation; past rows are not inserted.
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
    use serde_json::json;
    fn sample() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/nhl/october-2026.json")).unwrap()
    }
    #[test]
    fn ending_year_rolls_over_in_july() {
        for (date, year) in [
            ("2026-10-14", 2027),
            ("2027-01-01", 2027),
            ("2027-06-30", 2027),
            ("2027-07-01", 2028),
        ] {
            assert_eq!(season_for(date.parse().unwrap()), year);
        }
    }
    #[test]
    fn home_perspective_scores_and_finish_are_preserved() {
        let (games, results) = parse(&sample(), 2027).unwrap();
        assert_eq!(games.len(), 4);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0]["homeName"], "Columbus Blue Jackets");
        assert_eq!(results[0]["awayName"], "Buffalo Sabres");
        assert_eq!(results[0]["homeScore"], 6);
        assert_eq!(results[0]["awayScore"], 3);
        assert_eq!(results[1]["status"], "STATUS_FINAL_OT");
        assert_eq!(results[2]["status"], "STATUS_FINAL_SO");
        assert!(
            games
                .iter()
                .all(|g| g.event.sport.as_deref() == Some("Hockey") && g.league == League::Nhl)
        );
        assert_eq!(League::Nhl.canonical_key(123), "espn:nhl:123");
        assert!(
            games[0]
                .event
                .url
                .starts_with("https://www.espn.com/nhl/game/")
        );
        assert!(results.iter().all(|r| r["homeScore"] != r["awayScore"]));
    }
    #[test]
    fn malformed_wrong_league_and_wrong_season_are_rejected() {
        assert!(parse(&sample(), 2026).is_err());
        let mut bad = sample();
        bad["leagues"][0]["id"] = json!("10");
        assert!(parse(&bad, 2027).is_err());
        let mut bad = sample();
        bad["events"][0]["season"]["year"] = json!(2026);
        assert!(parse(&bad, 2027).is_err());
        for score in [Value::Null, json!("-1"), json!("3")] {
            let mut bad = sample();
            bad["events"][0]["competitions"][0]["competitors"][0]["score"] = score;
            assert!(parse(&bad, 2027).is_err());
        }
        let mut bad = sample();
        bad["events"][0]["competitions"][0]["competitors"][0]["team"]["displayName"] =
            json!("x".repeat(201));
        assert!(parse(&bad, 2027).is_err());
    }
    #[test]
    fn preseason_uses_event_season_when_league_header_lags() {
        let mut root = sample();
        root["leagues"][0]["season"]["year"] = json!(2026);
        assert!(parse(&root, 2027).is_ok());
        root["events"][0]["season"]["year"] = json!(2026);
        assert!(parse(&root, 2027).is_err());
    }
    #[test]
    fn cancellations_reconcile_without_becoming_results() {
        let mut root = sample();
        root["events"] = json!([root["events"][3].clone()]);
        root["events"][0]["competitions"][0]["status"]["type"] =
            json!({"name":"STATUS_POSTPONED","state":"post","completed":true});
        let (games, results) = parse(&root, 2027).unwrap();
        assert_eq!(games.len(), 1);
        assert!(results.is_empty());
        assert!(!games[0].completed);
        assert!(games[0].event.time_tbd);
        assert!(!is_feed(
            "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard"
        ));
    }
    #[test]
    #[ignore = "read-only network verification against live ESPN NHL feeds"]
    fn live_nhl_feeds() {
        let client = crate::http::client_builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        let batch = fetch(
            &client,
            "https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/scoreboard",
        )
        .unwrap();
        assert!(!batch.fixtures.is_empty());
        let standings = crate::standings::fetch(
            &client,
            &crate::standings::Source {
                id: 0,
                provider: "espn-nhl".into(),
                sport: "Hockey".into(),
                url: "https://site.api.espn.com/apis/v2/sports/hockey/nhl/standings?level=3".into(),
                minimum_teams: 32,
            },
        )
        .unwrap();
        eprintln!(
            "NHL live verification: season={} schedule_observations={} final_results={} standings_teams={}",
            batch.season,
            batch.fixtures.len(),
            batch.results.len(),
            standings.entries.len()
        );
    }
}
