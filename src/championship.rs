//! Full Championship season: future fixtures and completed results stored separately.
use crate::Result;
use crate::espn_schedule::{self, Fixture, League};
use chrono::{Datelike, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
pub const WREXHAM_ID: i32 = 352;
pub struct Batch {
    pub season: i32,
    pub fixtures: Vec<Fixture>,
    pub results: Vec<Value>,
}
pub fn is_feed(url: &str) -> bool {
    espn_schedule::is_feed(url, League::Championship)
}
pub fn parse(root: &Value, season: i32) -> Result<(Vec<Fixture>, Vec<Value>)> {
    crate::match_results::parse(root, season, League::Championship, "3914")
}
pub fn fetch(client: &reqwest::blocking::Client, raw: &str) -> Result<Batch> {
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let season = if today.month() >= 7 {
        today.year()
    } else {
        today.year() - 1
    };
    let mut fixtures = BTreeMap::new();
    let mut results = BTreeMap::new();
    let base = crate::http::public_url(raw)?;
    if !is_feed(raw) {
        return Err("invalid Championship feed".into());
    }
    for offset in 0..12 {
        let month = (6 + offset) % 12 + 1;
        let year = season + if month < 7 { 1 } else { 0 };
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
    #[test]
    fn final_scores_and_wrexham_identity_are_preserved() {
        let root: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/championship/october-2026.json"
        ))
        .unwrap();
        let (fixtures, results) = parse(&root, 2026).unwrap();
        assert!(
            fixtures
                .iter()
                .any(|f| f.team_id == WREXHAM_ID || f.opponent_id == WREXHAM_ID)
        );
        assert!(!results.is_empty());
        assert_eq!(results[0]["homeName"], "West Ham United");
        assert_eq!(results[0]["awayName"], "Queens Park Rangers");
        assert_eq!(results[0]["homeScore"], 1);
        assert_eq!(results[0]["awayScore"], 1);
        assert!(
            results
                .iter()
                .all(|r| r["homeScore"].as_i64().is_some() && r["awayScore"].as_i64().is_some())
        );
        let mut bad = root.clone();
        bad["leagues"][0]["id"] = json!("770");
        assert!(parse(&bad, 2026).is_err());
        assert!(parse(&root, 2025).is_err());
        let mut missing = root;
        missing["events"][0]["competitions"][0]["competitors"][0]["score"] = Value::Null;
        assert!(parse(&missing, 2026).is_err());
    }
    #[test]
    fn championship_tbd_dates_follow_london_calendar() {
        for (kickoff, day) in [
            ("2026-11-06T00:00Z", "2026-11-06"),
            ("2026-10-17T23:00Z", "2026-10-18"),
        ] {
            let mut root: Value = serde_json::from_str(include_str!(
                "../tests/fixtures/championship/october-2026.json"
            ))
            .unwrap();
            let c = &mut root["events"][0]["competitions"][0];
            c["date"] = json!(kickoff);
            c["timeValid"] = json!(false);
            c["status"]["type"] =
                json!({"name":"STATUS_SCHEDULED","state":"pre","completed":false});
            let games = espn_schedule::parse(&root, League::Championship, None).unwrap();
            assert_eq!(games[0].event.event_date.to_string(), day);
            assert!(games[0].event.starts_at.is_none());
            assert_eq!(League::Championship.canonical_key(123), "espn:eng.2:123");
        }
    }
}
