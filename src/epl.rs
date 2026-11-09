//! Current/future Premier League fixtures only. No historical result imports.
use crate::Result;
use crate::espn_schedule::{self, Fixture, League};
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
pub fn is_feed(url: &str) -> bool {
    espn_schedule::is_feed(url, League::Epl)
}
pub fn parse(root: &Value, today: NaiveDate, season: i32) -> Result<Vec<Fixture>> {
    if !root["leagues"]
        .as_array()
        .is_some_and(|ls| ls.iter().any(|l| l["id"] == "700"))
    {
        return Err("wrong Premier League response".into());
    }
    let fixtures = espn_schedule::parse(root, League::Epl, None)?;
    if root["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["season"]["year"].as_i64() != Some(i64::from(season)))
    {
        return Err("wrong Premier League season".into());
    }
    // Today's finals reconcile existing schedule rows but are never inserted as results.
    Ok(fixtures
        .into_iter()
        .filter(|f| f.event.event_date >= today)
        .collect())
}
pub fn fetch(client: &reqwest::blocking::Client, raw: &str) -> Result<Vec<Fixture>> {
    if !is_feed(raw) {
        return Err("invalid Premier League feed".into());
    }
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let end_year = if today.month() >= 7 {
        today.year() + 1
    } else {
        today.year()
    };
    let season = end_year - 1;
    let mut month = today.month();
    let mut year = today.year();
    let mut games = BTreeMap::new();
    loop {
        let mut url = crate::http::public_url(raw)?;
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
        for f in parse(&serde_json::from_str(&body)?, today, season)? {
            games.insert(f.event.game_id.unwrap(), f);
        }
        if year == end_year && month == 6 {
            break;
        }
        if month == 12 {
            month = 1;
            year += 1;
        } else {
            month += 1;
        }
    }
    Ok(games.into_values().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn epl_excludes_past_and_preserves_league_identity() {
        let root: Value =
            serde_json::from_str(include_str!("../tests/fixtures/epl/october-2026.json")).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let games = parse(&root, today, 2026).unwrap();
        assert!(!games.is_empty());
        assert!(games.iter().all(|f| f.event.event_date >= today));
        assert_eq!(League::Epl.canonical_key(123), "espn:eng.1:123");
        assert_ne!(
            League::Epl.canonical_key(123),
            League::Championship.canonical_key(123)
        );
        assert!(parse(&root, today, 2025).is_err());
        let mut bad = root;
        bad["leagues"][0]["id"] = serde_json::json!("3914");
        assert!(parse(&bad, today, 2026).is_err());
    }
}
