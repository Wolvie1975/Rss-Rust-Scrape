//! Full Championship season: future fixtures and completed results stored separately.
use crate::espn_schedule::{self, Fixture, League};
use chrono::{Datelike, Utc};
use serde_json::{Value, json};
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
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
    parse_soccer(root, season, League::Championship, "3914")
}
pub(crate) fn parse_soccer(
    root: &Value,
    season: i32,
    league: League,
    league_id: &str,
) -> Result<(Vec<Fixture>, Vec<Value>)> {
    if !root["leagues"]
        .as_array()
        .is_some_and(|ls| ls.iter().any(|l| l["id"] == league_id))
    {
        return Err("wrong soccer league response".into());
    }
    let fixtures = espn_schedule::parse(root, league, None)?;
    if !fixtures.is_empty()
        && root["leagues"][0]["season"]["year"].as_i64() != Some(i64::from(season))
    {
        return Err("wrong soccer season response".into());
    }
    if root["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["season"]["year"].as_i64() != Some(i64::from(season)))
    {
        return Err("mixed soccer event seasons".into());
    }
    let mut results = Vec::new();
    for fixture in &fixtures {
        if !fixture.completed
            || !(fixture.status == "STATUS_FULL_TIME" || fixture.status.starts_with("STATUS_FINAL"))
        {
            continue;
        }
        let raw = root["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"].as_str().and_then(|s| s.parse::<i32>().ok()) == fixture.event.game_id)
            .ok_or("soccer result missing event")?;
        let competitors = raw["competitions"][0]["competitors"]
            .as_array()
            .ok_or("missing result teams")?;
        let home = competitors
            .iter()
            .find(|c| c["homeAway"] == "home")
            .ok_or("missing home result")?;
        let away = competitors
            .iter()
            .find(|c| c["homeAway"] == "away")
            .ok_or("missing away result")?;
        let score = |v: &Value| -> Result<i32> {
            v["score"]
                .as_str()
                .and_then(|s| s.parse::<i32>().ok())
                .filter(|s| *s >= 0)
                .ok_or_else(|| "final soccer score missing or invalid".into())
        };
        let date = raw["competitions"][0]["date"]
            .as_str()
            .or(raw["date"].as_str())
            .ok_or("result kickoff missing")?;
        let kickoff = chrono::DateTime::parse_from_rfc3339(date)
            .map(|d| d.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%MZ").map(|d| d.and_utc())
            })?;
        let shootout = |v: &Value| -> Result<Option<i32>> {
            if v["shootoutScore"].is_null() {
                return Ok(None);
            }
            let n = v["shootoutScore"]
                .as_i64()
                .or_else(|| v["shootoutScore"].as_str().and_then(|s| s.parse().ok()))
                .ok_or("invalid shootout score")?;
            Ok(Some(
                i32::try_from(n)
                    .ok()
                    .filter(|n| *n >= 0)
                    .ok_or("invalid shootout score")?,
            ))
        };
        let home_pen = shootout(home)?;
        let away_pen = shootout(away)?;
        if fixture.status == "STATUS_FINAL_PEN"
            && (home_pen.is_none() || away_pen.is_none() || home_pen == away_pen)
        {
            return Err("final shootout scores missing or tied".into());
        }
        results.push(json!({"gameId":fixture.event.game_id,"season":season,
            "matchDate":fixture.event.event_date.to_string(),"startsAt":kickoff.to_rfc3339(),
            "homeId":fixture.team_id,"homeName":fixture.team_name,"awayId":fixture.opponent_id,
            "awayName":fixture.event.opponent,"homeScore":score(home)?,"awayScore":score(away)?,"homeShootout":home_pen,"awayShootout":away_pen,
            "homeLogo":fixture.event.team_logo_url,"awayLogo":fixture.event.opponent_logo_url,
            "status":fixture.status,"url":fixture.event.url}));
    }
    Ok((fixtures, results))
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
