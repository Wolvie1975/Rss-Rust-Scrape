//! Optional ESPN game detail, separate from scores and article identity.
use crate::Result;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Game {
    pub sport: String,
    pub league: String,
    pub event_id: String,
    pub home_id: String,
    pub away_id: String,
    pub date: String,
}
#[derive(Debug)]
pub struct Detail {
    pub payload: Option<String>,
    pub summary_url: Option<String>,
    pub boxscore_url: Option<String>,
}
fn array(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or_default()
}
fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
fn slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 60
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
}
fn valid_game(g: &Game) -> bool {
    slug(&g.sport)
        && slug(&g.league)
        && !g.event_id.is_empty()
        && g.event_id.len() <= 20
        && g.event_id.bytes().all(|b| b.is_ascii_digit())
        && !g.home_id.is_empty()
        && !g.away_id.is_empty()
        && g.home_id != g.away_id
}
fn provider_link(header: &Value, rel: &str, event: &str) -> Option<String> {
    array(&header["links"]).iter().find_map(|l| {
        if !array(&l["rel"]).iter().any(|r| r == rel) {
            return None;
        }
        let u = crate::http::public_url(l["href"].as_str()?).ok()?;
        if u.scheme() != "https" || !matches!(u.host_str(), Some("www.espn.com" | "espn.com")) {
            return None;
        }
        let matches_event = u.path_segments().is_some_and(|mut p| p.any(|x| x == event))
            || u.query_pairs().any(|(k, v)| k == "gameId" && v == event);
        (matches_event && u.as_str().len() <= 2048).then(|| u.to_string())
    })
}
fn stats(v: &Value, group: Option<&str>, out: &mut Vec<Value>) {
    for s in array(v) {
        if s["stats"].is_array() {
            stats(&s["stats"], s["name"].as_str().or(group), out);
        } else if let Some(value) = text(&s["displayValue"]).or_else(|| text(&s["value"])) {
            let key = text(&s["name"]).or_else(|| text(&s["abbreviation"]));
            if let Some(key) = key {
                out.push(json!({"group":group,"key":key,"label":text(&s["label"])
                    .or_else(||text(&s["displayName"])).unwrap_or_else(||key.clone()),"value":value}));
            }
        }
    }
}

pub fn parse(root: &Value, game: &Game) -> Result<Detail> {
    if !valid_game(game) {
        return Err("invalid box-score game identity".into());
    }
    let header = &root["header"];
    if text(&header["id"]).as_deref() != Some(&game.event_id) {
        return Err("box-score event identity mismatch".into());
    }
    let competition = array(&header["competitions"])
        .iter()
        .find(|c| text(&c["id"]).as_deref() == Some(&game.event_id))
        .ok_or("box-score competition identity mismatch")?;
    if competition["status"]["type"]["completed"] != true {
        return Err("box-score game is not final".into());
    }
    let competitors = array(&competition["competitors"]);
    if competitors.len() != 2 {
        return Err("box-score requires two competitors".into());
    }
    for (side, id) in [("home", &game.home_id), ("away", &game.away_id)] {
        if !competitors
            .iter()
            .any(|c| c["homeAway"] == side && text(&c["team"]["id"]).as_ref() == Some(id))
        {
            return Err("box-score team identity mismatch".into());
        }
    }
    let mut teams = Vec::new();
    let mut groups = Vec::new();
    for c in competitors {
        let id = text(&c["team"]["id"]).ok_or("missing box-score team ID")?;
        let mut statistics = Vec::new();
        for t in array(&root["boxscore"]["teams"]) {
            let tid = text(&t["team"]["id"]).ok_or("missing statistics team ID")?;
            if tid != game.home_id && tid != game.away_id {
                return Err("foreign statistics team".into());
            }
            if tid == id {
                stats(&t["statistics"], None, &mut statistics);
            }
        }
        let line_scores: Vec<_> = array(&c["linescores"]).iter().map(|l| json!({
            "period":l["period"],"displayValue":text(&l["displayValue"]).or_else(||text(&l["value"]))
        })).collect();
        teams.push(
            json!({"id":id,"name":text(&c["team"]["displayName"]),"homeAway":c["homeAway"],
            "score":text(&c["score"]),"lineScores":line_scores,"statistics":statistics}),
        );
    }
    for p in array(&root["boxscore"]["players"]) {
        let id = text(&p["team"]["id"]).ok_or("missing player team ID")?;
        if id != game.home_id && id != game.away_id {
            return Err("foreign player team".into());
        }
        for s in array(&p["statistics"]) {
            let labels = array(&s["labels"]);
            if labels.is_empty() {
                continue;
            }
            let columns: Vec<_> = labels
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    json!({
                        "key":s["keys"][i],"label":l,"description":s["descriptions"][i]
                    })
                })
                .collect();
            let mut athletes = Vec::new();
            for a in array(&s["athletes"]) {
                let values = array(&a["stats"]);
                // Never attach values to the wrong columns after a provider format change.
                if values.is_empty() {
                    continue;
                }
                if values.len() != columns.len() {
                    return Err("box-score player column/value mismatch".into());
                }
                if text(&a["athlete"]["displayName"]).is_none() {
                    return Err("missing box-score player name".into());
                }
                athletes.push(
                    json!({"id":text(&a["athlete"]["id"]),"name":a["athlete"]["displayName"],
                    "position":a["athlete"]["position"]["abbreviation"],"starter":a["starter"],
                    "stats":values.iter().map(text).collect::<Vec<_>>()}),
                );
            }
            if athletes.is_empty() {
                continue;
            }
            let totals = array(&s["totals"]);
            if !totals.is_empty() && totals.len() != columns.len() {
                return Err("box-score totals column/value mismatch".into());
            }
            groups.push(
                json!({"teamId":id,"name":text(&s["name"]).or_else(||text(&s["type"]))
                .or_else(||text(&s["text"])),"columns":columns,"athletes":athletes,
                "totals":totals.iter().map(text).collect::<Vec<_>>()}),
            );
        }
    }
    let has_team = teams.iter().any(|t| !array(&t["statistics"]).is_empty());
    let has_players = !groups.is_empty();
    let payload = (has_team || has_players).then(||json!({"version":1,"sport":game.sport,
        "league":game.league,"eventId":game.event_id,"hasTeamStats":has_team,"hasPlayerStats":has_players,
        "teams":teams,"playerGroups":groups}).to_string());
    Ok(Detail {
        payload,
        summary_url: provider_link(header, "summary", &game.event_id),
        boxscore_url: provider_link(header, "boxscore", &game.event_id),
    })
}

// Period metadata is optional: new games keep usable stats even if it is unavailable.
// Existing snapshots (including their labels) retain last-good behavior on a failure.
fn period_result(mut detail: Detail, labeled: Result<String>, last_good: bool) -> Result<Detail> {
    match labeled {
        Ok(payload) => detail.payload = Some(payload),
        Err(e) if last_good => return Err(e),
        Err(_) => {}
    }
    Ok(detail)
}

pub fn collect(
    db: &mut crate::db::Db,
    dry: bool,
    limit: i32,
    run_id: Option<i32>,
) -> Result<(usize, usize)> {
    let client = crate::http::client_builder()
        .timeout(Duration::from_secs(12))
        .user_agent("My-Newsfeed/1.0")
        .build()?;
    let games = db.boxscore_candidates(limit)?;
    let mut boards = crate::sports_periods::Boards::new()?;
    let started = Instant::now();
    let (mut available, mut unavailable, mut errors) = (0, 0, 0);
    for game in games {
        if started.elapsed() >= Duration::from_secs(180) {
            break;
        }
        let result = (|| -> Result<Detail> {
            if !valid_game(&game) {
                return Err("invalid box-score game identity".into());
            }
            let u = format!(
                "https://site.api.espn.com/apis/site/v2/sports/{}/{}/summary?event={}",
                game.sport, game.league, game.event_id
            );
            let body = crate::http::get_text(&client, &u)?;
            let mut detail = parse(&serde_json::from_str(&body)?, &game)?;
            if let Some(payload) = detail.payload.as_ref() {
                let labeled = boards
                    .get(&game)
                    .and_then(|b| crate::sports_periods::apply(payload, b, &game));
                let last_good = if labeled.is_err() {
                    db.boxscore_available(&game)?
                } else {
                    false
                };
                if labeled.is_err() && !last_good {
                    eprintln!(
                        "period labels {}/{}/{}: metadata unavailable; retaining valid first-fetch statistics without inferred labels",
                        game.sport, game.league, game.event_id
                    );
                }
                detail = period_result(detail, labeled, last_good)?;
            }
            Ok(detail)
        })();
        match result {
            Ok(detail) => {
                if detail.payload.is_some() {
                    available += 1;
                } else {
                    unavailable += 1;
                }
                eprintln!(
                    "box score {}/{}/{}: {}{}",
                    game.sport,
                    game.league,
                    game.event_id,
                    if detail.payload.is_some() {
                        "available"
                    } else {
                        "unavailable; keep existing fallback"
                    },
                    if dry { " (dry run)" } else { "" }
                );
                if !dry {
                    db.save_boxscore(&game, Some(&detail), None)?;
                }
            }
            Err(e) => {
                errors += 1;
                eprintln!(
                    "box score {}/{}/{}: {e}; preserving existing data/fallback",
                    game.sport, game.league, game.event_id
                );
                if !dry {
                    db.save_boxscore(&game, None, Some(&e.to_string()))?;
                }
            }
        }
        if !dry && let Some(id) = run_id {
            db.run_heartbeat(id)?;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!(
        "box scores: {available} available, {unavailable} unavailable, {errors} errors{}",
        if dry { " (dry run)" } else { "" }
    );
    Ok((available + unavailable, errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(league: &str) -> (Value, Game) {
        let raw = match league {
            "nfl" => include_str!("../tests/fixtures/boxscores/nfl.json"),
            "mlb" => include_str!("../tests/fixtures/boxscores/mlb.json"),
            "usa.nwsl" => include_str!("../tests/fixtures/boxscores/usa.nwsl.json"),
            _ => include_str!("../tests/fixtures/boxscores/womens-college-volleyball.json"),
        };
        let v: Value = serde_json::from_str(raw).unwrap();
        let cs = array(&v["header"]["competitions"][0]["competitors"]);
        let id = |side| {
            cs.iter().find(|c| c["homeAway"] == side).unwrap()["team"]["id"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let g = Game {
            sport: match league {
                "nfl" => "football",
                "mlb" => "baseball",
                "usa.nwsl" => "soccer",
                _ => "volleyball",
            }
            .into(),
            league: league.into(),
            event_id: v["header"]["id"].as_str().unwrap().into(),
            home_id: id("home"),
            away_id: id("away"),
            date: "20261018".into(),
        };
        (v, g)
    }
    #[test]
    fn championship_basketball_matches_complete_frontend_example() {
        let source: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/boxscores/mens-college-basketball.json"
        ))
        .unwrap();
        let g = Game {
            sport: "basketball".into(),
            league: "mens-college-basketball".into(),
            event_id: "401856600".into(),
            home_id: "130".into(),
            away_id: "41".into(),
            date: "20260406".into(),
        };
        let detail = parse(&source, &g).unwrap();
        assert_eq!(
            detail.boxscore_url.as_deref(),
            Some("https://www.espn.com/mens-college-basketball/boxscore/_/gameId/401856600")
        );
        let periods: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/periods/mens-college-basketball.json"
        ))
        .unwrap();
        let labeled = crate::sports_periods::apply(
            detail.payload.as_deref().unwrap(),
            &periods["scoreboard"],
            &g,
        )
        .unwrap();
        let actual: Value = serde_json::from_str(&labeled).unwrap();
        let expected: Value = serde_json::from_str(include_str!(
            "../New Development/mens-college-basketball-boxscore-example.json"
        ))
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(array(&actual["playerGroups"][0]["athletes"]).len(), 9);
        assert_eq!(array(&actual["playerGroups"][1]["athletes"]).len(), 8);
        assert_eq!(actual["playerGroups"][0]["totals"][0], Value::Null);
        // DNP source rows carry no statistics; never manufacture zero-filled players.
        assert!(
            array(&source["boxscore"]["players"][0]["statistics"][0]["athletes"])
                .iter()
                .any(|a| a["didNotPlay"] == true && array(&a["stats"]).is_empty())
        );
    }
    #[test]
    fn missing_optional_period_metadata_preserves_first_stats_and_existing_snapshots() {
        let (v, g) = fixture("nfl");
        let detail = parse(&v, &g).unwrap();
        let old = detail.payload.clone();
        let url = detail.boxscore_url.clone();
        let fallback = period_result(detail, Err("metadata fetch failed".into()), false).unwrap();
        assert_eq!(fallback.payload, old);
        assert_eq!(fallback.boxscore_url, url);
        assert!(
            period_result(
                parse(&v, &g).unwrap(),
                Err("metadata fetch failed".into()),
                true
            )
            .is_err()
        );
    }
    #[test]
    fn actual_sport_formats_preserve_team_player_and_nested_statistics() {
        for league in ["nfl", "mlb", "usa.nwsl", "womens-college-volleyball"] {
            let (v, g) = fixture(league);
            let d = parse(&v, &g).unwrap();
            if league == "womens-college-volleyball" {
                assert!(d.payload.is_none(), "team/player shells are not statistics");
                continue;
            }
            let payload: Value = serde_json::from_str(d.payload.as_deref().expect(league)).unwrap();
            assert_eq!(payload["teams"].as_array().unwrap().len(), 2);
            assert_eq!(payload["hasTeamStats"], true, "{league}");
            assert_eq!(payload["hasPlayerStats"], league != "usa.nwsl", "{league}");
            if league == "mlb" {
                assert!(
                    array(&payload["teams"][0]["statistics"])
                        .iter()
                        .any(|s| s["group"] == "batting")
                );
            }
            if league == "nfl" {
                assert!(d.boxscore_url.as_ref().unwrap().contains("/boxscore/"));
            }
            if league == "usa.nwsl" {
                assert!(d.boxscore_url.is_none());
                assert!(d.summary_url.is_some());
            }
        }
    }
    #[test]
    fn empty_boxscore_never_claims_availability_even_with_advertised_link() {
        let (mut v, g) = fixture("nfl");
        v["boxscore"] = json!({"teams":[],"players":[]});
        let d = parse(&v, &g).unwrap();
        assert!(d.payload.is_none());
        assert!(d.boxscore_url.is_some());
    }
    #[test]
    fn reject_wrong_game_teams_nonfinal_and_column_drift() {
        let (v, g) = fixture("nfl");
        let mut bad = v.clone();
        bad["header"]["id"] = json!("999");
        assert!(parse(&bad, &g).is_err());
        let mut bad_game = g.clone();
        bad_game.home_id = "999".into();
        assert!(parse(&v, &bad_game).is_err());
        let mut bad = v.clone();
        bad["header"]["competitions"][0]["status"]["type"]["completed"] = json!(false);
        assert!(parse(&bad, &g).is_err());
        let mut bad = v.clone();
        bad["boxscore"]["players"][0]["statistics"][0]["athletes"][0]["stats"] = json!(["1"]);
        assert!(parse(&bad, &g).is_err());
        let mut bad = v;
        bad["boxscore"]["teams"][0]["team"]["id"] = json!("999");
        assert!(parse(&bad, &g).is_err());
    }
    #[test]
    fn reject_untrusted_or_wrong_event_provider_links() {
        let (mut v, g) = fixture("nfl");
        for href in [
            "https://evil.example/boxscore/401873007",
            "javascript:alert(1)",
            "https://www.espn.com/nfl/boxscore/_/gameId/999",
            "http://www.espn.com/nfl/boxscore/_/gameId/401873007",
        ] {
            v["header"]["links"] = json!([{"rel":["boxscore"],"href":href}]);
            assert!(parse(&v, &g).unwrap().boxscore_url.is_none());
        }
    }
    #[test]
    fn zero_and_missing_values_are_not_invented() {
        let mut out = vec![];
        stats(
            &json!([{"name":"shots","value":0},{"name":"missing"},{"name":"poss","displayValue":"--"}]),
            None,
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["value"], "0");
        assert_eq!(out[1]["value"], "--");
    }
    #[test]
    fn invalid_path_segments_cannot_change_endpoint() {
        let (_, mut g) = fixture("nfl");
        g.league = "../mlb".into();
        assert!(!valid_game(&g));
        g.league = "nfl".into();
        g.event_id = "1&other=2".into();
        assert!(!valid_game(&g));
    }
}
