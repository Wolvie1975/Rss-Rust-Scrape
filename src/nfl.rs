//! League-wide NFL schedule; covers regular season weeks 1-18 and postseason weeks 1-5.
use crate::Result;
use crate::espn_schedule::{self, Fixture, League};
use chrono::{Datelike, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn is_feed(raw: &str) -> bool {
    espn_schedule::is_feed(raw, League::Nfl)
}

pub fn season_for(today: NaiveDate) -> i32 {
    if today.month() >= 7 {
        today.year()
    } else {
        today.year() - 1
    }
}

pub fn parse(root: &Value, season: i32) -> Result<Vec<Fixture>> {
    if !root["leagues"]
        .as_array()
        .is_some_and(|ls| ls.iter().any(|l| l["id"] == "28" || l["slug"] == "nfl"))
    {
        return Err("wrong NFL response".into());
    }
    let events = root["events"]
        .as_array()
        .ok_or("ESPN response missing events array")?;
    for e in events {
        if e["season"]["year"].as_i64() != Some(i64::from(season)) {
            return Err("wrong NFL season".into());
        }
        if !matches!(e["season"]["type"].as_i64(), Some(2 | 3)) {
            return Err("unexpected NFL season type".into());
        }
    }
    espn_schedule::parse(root, League::Nfl, None)
}

pub fn fetch(client: &Client, raw: &str) -> Result<Vec<Fixture>> {
    if !is_feed(raw) {
        return Err("invalid NFL feed".into());
    }
    let _ = crate::http::public_url(raw)?;
    fetch_from(
        client,
        raw,
        Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive(),
    )
}

pub(crate) fn fetch_from(client: &Client, raw: &str, today: NaiveDate) -> Result<Vec<Fixture>> {
    let base = url::Url::parse(raw)?;
    let params: BTreeMap<_, _> = base
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let season = match params.get("season") {
        Some(s) => s.parse::<i32>()?,
        None => season_for(today),
    };
    let reconcile_from = today - chrono::Duration::days(7);
    let mut games = BTreeMap::new();
    let weeks = (1..=18)
        .map(|w| (2, w))
        .chain((1..=5).map(|w| (3, w)));
    for (season_type, week) in weeks {
        let mut url = base.clone();
        url.set_query(None);
        url.query_pairs_mut()
            .append_pair("dates", &season.to_string())
            .append_pair("seasontype", &season_type.to_string())
            .append_pair("week", &week.to_string())
            .append_pair("limit", "1000");
        let response = client
            .get(url)
            .header(reqwest::header::USER_AGENT, "My-Newsfeed/1.0")
            .send()?
            .error_for_status()?;
        let body = crate::http::response_text_limit(response, crate::http::JSON_LIMIT)?;
        for f in parse(&serde_json::from_str(&body)?, season)? {
            games.insert(f.event.game_id.unwrap(), f);
        }
    }
    Ok(games
        .into_values()
        .filter(|f| {
            (f.event.event_date >= today && !f.completed) || f.event.event_date >= reconcile_from
        })
        .collect())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    pub fn sample() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/nfl/week-10-2026.json")).unwrap()
    }

    #[test]
    fn nfl_season_crosses_new_year_without_stopping_at_december() {
        for (date, season) in [
            ("2026-09-01", 2026),
            ("2026-11-06", 2026),
            ("2026-12-31", 2026),
            ("2027-01-15", 2026),
            ("2027-02-14", 2026),
            ("2027-06-30", 2026),
            ("2027-07-01", 2027),
        ] {
            assert_eq!(season_for(date.parse().unwrap()), season, "{date}");
        }
    }

    #[test]
    fn regular_season_week_maps_home_perspective_canonical_key_central_dates_and_tv() {
        let games = parse(&sample(), 2026).unwrap();
        // 5 events in fixture: 4 real games + 1 TBD postseason placeholder (skipped).
        assert_eq!(games.len(), 4);

        // Thursday night game: 2026-11-13T01:15Z -> 2026-11-12 in America/Chicago
        let thursday = &games[0];
        assert_eq!(thursday.team_name, "New York Giants");
        assert_eq!(thursday.team_id, 19);
        assert_eq!(
            thursday.event.opponent.as_deref(),
            Some("Washington Commanders")
        );
        assert_eq!(thursday.opponent_id, 28);
        assert_eq!(thursday.event.is_away, Some(false));
        assert!(!thursday.neutral);
        assert_eq!(
            thursday.event.title,
            "New York Giants vs Washington Commanders"
        );
        assert_eq!(thursday.event.sport.as_deref(), Some("Football"));
        assert_eq!(thursday.event.location.as_deref(), Some("MetLife Stadium"));
        assert_eq!(thursday.event.event_date.to_string(), "2026-11-12");
        assert_eq!(
            thursday.event.starts_at.unwrap().to_rfc3339(),
            "2026-11-13T01:15:00+00:00"
        );
        assert!(!thursday.event.time_tbd);
        assert_eq!(thursday.event.tv.as_deref(), Some("Prime Video"));
        assert_eq!(
            thursday.event.url,
            "https://www.espn.com/nfl/game/_/gameId/401873052"
        );
        assert_eq!(
            thursday.event.live_stats_url.as_deref(),
            Some("https://www.espn.com/nfl/game/_/gameId/401873052/commanders-giants")
        );
        assert_eq!(
            thursday.league.canonical_key(401873052),
            "espn:nfl:401873052"
        );
        assert_eq!(thursday.league.provider_key(), "espn-nfl");
        assert!(thursday.event.ends_at.is_none());

        // Sunday night game: 2026-11-16T01:20Z -> 2026-11-15 in America/Chicago
        let snf = &games[2];
        assert_eq!(snf.team_name, "Cincinnati Bengals");
        assert_eq!(
            snf.event.opponent.as_deref(),
            Some("Pittsburgh Steelers")
        );
        assert_eq!(snf.event.event_date.to_string(), "2026-11-15");
        assert_eq!(
            snf.event.starts_at.unwrap().to_rfc3339(),
            "2026-11-16T01:20:00+00:00"
        );
        assert_eq!(snf.event.tv.as_deref(), Some("NBC"));
    }

    #[test]
    fn neutral_site_and_tbd_kickoff_follow_nfl_rules() {
        let games = parse(&sample(), 2026).unwrap();

        // International neutral-site game (Munich): IsNeutral = 1, IsAway = 0 (home perspective)
        let munich = &games[1];
        assert_eq!(munich.team_name, "Detroit Lions");
        assert_eq!(
            munich.event.opponent.as_deref(),
            Some("New England Patriots")
        );
        assert!(munich.neutral);
        assert_eq!(munich.event.is_away, Some(false));
        assert_eq!(
            munich.event.title,
            "Detroit Lions vs New England Patriots"
        );
        assert_eq!(
            munich.event.location.as_deref(),
            Some("FC Bayern Munich Stadium")
        );

        // Week 18 flexed TBD game: 2027-01-10T05:00Z (midnight Eastern) -> keeps 2027-01-10, TimeTbd = 1
        let tbd = &games[3];
        assert_eq!(tbd.team_name, "Buffalo Bills");
        assert_eq!(tbd.event.opponent.as_deref(), Some("New York Jets"));
        assert!(tbd.event.time_tbd);
        assert!(tbd.event.starts_at.is_none());
        assert_eq!(tbd.event.event_date.to_string(), "2027-01-10");
        assert!(tbd.event.tv.is_none());
    }

    #[test]
    fn postseason_placeholders_and_pro_bowl_are_skipped_until_real_teams_set() {
        let mut root = sample();
        // Fixture event 4 is a postseason placeholder (-1 TBD vs -2 TBD); verify it was skipped.
        let initial = parse(&root, 2026).unwrap();
        assert!(!initial.iter().any(|g| g.event.game_id == Some(401872910)));

        // Pro Bowl event (AFC vs NFC, IDs 31 and 32) in seasontype=3 is also skipped.
        let mut pro_bowl = root["events"][4].clone();
        pro_bowl["id"] = json!("401831718");
        pro_bowl["competitions"][0]["id"] = json!("401831718");
        pro_bowl["competitions"][0]["competitors"][0]["team"] = json!({
            "id": "31",
            "abbreviation": "AFC",
            "displayName": "AFC"
        });
        pro_bowl["competitions"][0]["competitors"][1]["team"] = json!({
            "id": "32",
            "abbreviation": "NFC",
            "displayName": "NFC"
        });
        root["events"].as_array_mut().unwrap().push(pro_bowl);
        let with_pro_bowl = parse(&root, 2026).unwrap();
        assert!(!with_pro_bowl.iter().any(|g| g.event.game_id == Some(401831718)));

        // Once the postseason matchup is decided with two real NFL teams, it is stored.
        let c = &mut root["events"][4]["competitions"][0];
        c["date"] = json!("2027-01-16T21:30Z");
        c["timeValid"] = json!(true);
        c["competitors"][0]["team"] = json!({
            "id": "12",
            "abbreviation": "KC",
            "displayName": "Kansas City Chiefs",
            "logo": "https://a.espncdn.com/i/teamlogos/nfl/500/scoreboard/kc.png"
        });
        c["competitors"][1]["team"] = json!({
            "id": "33",
            "abbreviation": "BAL",
            "displayName": "Baltimore Ravens",
            "logo": "https://a.espncdn.com/i/teamlogos/nfl/500/scoreboard/bal.png"
        });
        let decided = parse(&root, 2026).unwrap();
        let playoff = decided
            .iter()
            .find(|g| g.event.game_id == Some(401872910))
            .expect("decided playoff game should be stored");
        assert_eq!(playoff.team_name, "Kansas City Chiefs");
        assert_eq!(playoff.team_id, 12);
        assert_eq!(playoff.event.opponent.as_deref(), Some("Baltimore Ravens"));
        assert_eq!(playoff.opponent_id, 33);
        assert_eq!(playoff.event.event_date.to_string(), "2027-01-16");
        assert!(!playoff.event.time_tbd);
    }

    #[test]
    fn preseason_wrong_league_and_wrong_season_are_rejected() {
        assert!(parse(&sample(), 2025).is_err());

        let mut wrong_league = sample();
        wrong_league["leagues"][0]["id"] = json!("90");
        wrong_league["leagues"][0]["slug"] = json!("nhl");
        assert!(parse(&wrong_league, 2026).is_err());

        let mut preseason = sample();
        preseason["events"][0]["season"]["type"] = json!(1);
        assert!(parse(&preseason, 2026).is_err());
    }

    #[test]
    fn weekly_fetch_collects_regular_and_postseason_across_new_year() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..23 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut req = Vec::new();
                let mut buf = [0; 1024];
                while !req.windows(4).any(|b| b == b"\r\n\r\n") {
                    let n = socket.read(&mut buf).unwrap();
                    assert!(n > 0);
                    req.extend_from_slice(&buf[..n]);
                }
                let req_str = String::from_utf8(req).unwrap();
                let first_line = req_str.lines().next().unwrap().to_string();
                requests.push(first_line.clone());

                let body = if first_line.contains("seasontype=2&week=10") {
                    sample().to_string()
                } else if first_line.contains("seasontype=3&week=5") {
                    let mut sb = sample();
                    let mut e = sb["events"][1].clone();
                    e["id"] = json!("401873270");
                    e["season"] = json!({"year": 2026, "type": 3, "slug": "post-season"});
                    e["competitions"][0]["id"] = json!("401873270");
                    e["competitions"][0]["date"] = json!("2027-02-14T23:30Z");
                    e["competitions"][0]["neutralSite"] = json!(true);
                    sb["events"] = json!([e]);
                    sb.to_string()
                } else {
                    let mut empty = sample();
                    empty["events"] = json!([]);
                    empty.to_string()
                };
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
            requests
        });

        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!("http://{address}/apis/site/v2/sports/football/nfl/scoreboard");
        let today = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        let games = fetch_from(&client, &url, today).unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 23);
        assert!(requests[0].contains("dates=2026&seasontype=2&week=1&limit=1000"));
        assert!(requests[17].contains("dates=2026&seasontype=2&week=18&limit=1000"));
        assert!(requests[18].contains("dates=2026&seasontype=3&week=1&limit=1000"));
        assert!(requests[22].contains("dates=2026&seasontype=3&week=5&limit=1000"));

        // 4 games from week 10/18 + 1 Super Bowl in February 2027 = 5 games
        assert_eq!(games.len(), 5);
        let sb = games
            .iter()
            .find(|g| g.event.game_id == Some(401873270))
            .expect("February Super Bowl should be collected");
        assert_eq!(sb.event.event_date.to_string(), "2027-02-14");
        assert!(sb.neutral);
        assert_eq!(sb.event.is_away, Some(false));
    }
}
