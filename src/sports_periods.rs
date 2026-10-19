//! Labels come from explicit ESPN scoreboard period identities, never array length.
use crate::sports_boxscores::Game;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn items(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or_default()
}
fn value(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
fn ordinal(n: u64) -> String {
    let suffix = if (11..=13).contains(&(n % 100)) {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{n}{suffix}")
}
fn ot(n: u64) -> String {
    if n == 1 {
        "OT".into()
    } else {
        format!("{n}OT")
    }
}
fn number(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|n| *n > 0 && *n <= 100)
}
fn penalty(status: &Value) -> bool {
    status["name"].as_str().is_some_and(|s| s.contains("PEN"))
        || ["shortDetail", "detail"].iter().any(|k| {
            status[*k]
                .as_str()
                .is_some_and(|s| s.to_ascii_lowercase().contains("pen"))
        })
}
fn shootout(status: &Value) -> bool {
    status["name"].as_str().is_some_and(|s| s.ends_with("_SO"))
        || ["shortDetail", "detail", "altDetail"].iter().any(|k| {
            status[*k]
                .as_str()
                .is_some_and(|s| s == "SO" || s.ends_with("/SO"))
        })
}
fn label(raw: &Value, g: &Game, status: &Value, season: Option<u64>, last: u64) -> Option<String> {
    if let Some(s) = raw
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.parse::<f64>().is_err())
    {
        // Explicit provider labels are authoritative; do not collapse duplicate unknown labels.
        return Some(s.to_string());
    }
    let p = number(raw)?;
    match (g.sport.as_str(), g.league.as_str()) {
        ("baseball", "mlb") => Some(format!("{} inning", ordinal(p))),
        ("football", "nfl" | "college-football") => {
            Some(if p <= 4 { format!("Q{p}") } else { ot(p - 4) })
        }
        ("basketball", "mens-college-basketball") => Some(if p <= 2 {
            format!("{} half", ordinal(p))
        } else {
            ot(p - 2)
        }),
        ("basketball", "nba" | "wnba") => Some(if p <= 4 { format!("Q{p}") } else { ot(p - 4) }),
        ("hockey", "nhl") => {
            if p <= 3 {
                Some(format!("{} period", ordinal(p)))
            } else if p == 5 && shootout(status) && season != Some(3) {
                Some("SO".into())
            } else if p == 4 || (season == Some(3) && !shootout(status)) {
                Some(ot(p - 3))
            } else {
                None
            }
        }
        ("soccer", _) => match p {
            1 | 2 => Some(format!("{} half", ordinal(p))),
            3 | 4 if !penalty(status) || last == 5 => Some(format!("ET{}", p - 2)),
            5 if penalty(status) => Some("SO".into()),
            _ => None,
        },
        _ => None,
    }
}

pub struct Boards {
    client: reqwest::blocking::Client,
    boards: HashMap<String, Value>,
}
impl Boards {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: crate::http::client_builder()
                .timeout(Duration::from_secs(12))
                .user_agent("My-Newsfeed/1.0")
                .build()?,
            boards: HashMap::new(),
        })
    }
    pub fn get(&mut self, g: &Game) -> Result<&Value> {
        if [&g.event_id, &g.home_id, &g.away_id]
            .iter()
            .any(|id| id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()))
            || g.date.len() != 8
            || !g.date.bytes().all(|b| b.is_ascii_digit())
            || [&g.sport, &g.league].iter().any(|s| {
                s.is_empty()
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
            })
        {
            return Err("invalid period metadata request identity".into());
        }
        let key = format!("{}/{}/{}", g.sport, g.league, g.date);
        if !self.boards.contains_key(&key) {
            std::thread::sleep(Duration::from_millis(250));
            let url = format!(
                "https://site.api.espn.com/apis/site/v2/sports/{}/{}/scoreboard?dates={}&limit=1000",
                g.sport, g.league, g.date
            );
            let root: Value = serde_json::from_str(&crate::http::get_text(&self.client, &url)?)?;
            if !root["events"].is_array() {
                return Err("period scoreboard has no events array".into());
            }
            self.boards.insert(key.clone(), root);
        }
        // Soccer's site scoreboards omit line scores. Core line-score collections
        // provide explicit period numbers; never follow their HTTP/private $ref links.
        // Some date scoreboards omit otherwise published final games (e.g. unranked
        // college teams). A verified event summary can anchor the core metadata.
        if !items(&self.boards[&key]["events"])
            .iter()
            .any(|e| e["id"] == g.event_id)
        {
            std::thread::sleep(Duration::from_millis(250));
            let url = format!(
                "https://site.api.espn.com/apis/site/v2/sports/{}/{}/summary?event={}",
                g.sport, g.league, g.event_id
            );
            let root: Value = serde_json::from_str(&crate::http::get_text(&self.client, &url)?)?;
            let event = summary_anchor(&root, g)?;
            self.boards.get_mut(&key).unwrap()["events"]
                .as_array_mut()
                .unwrap()
                .push(event);
        }
        let event_index = items(&self.boards[&key]["events"])
            .iter()
            .position(|e| e["id"] == g.event_id)
            .ok_or("period event anchor missing")?;
        let comp_index = items(&self.boards[&key]["events"][event_index]["competitions"])
            .iter()
            .position(|c| c["id"] == g.event_id)
            .ok_or("period competition not found")?;
        let cs = items(
            &self.boards[&key]["events"][event_index]["competitions"][comp_index]["competitors"],
        );
        let missing: Vec<_> = cs
            .iter()
            .enumerate()
            .filter(|(_, c)| !c["linescores"].is_array())
            .map(|(i, c)| (i, c["team"]["id"].as_str().unwrap_or("").to_string()))
            .collect();
        for (index, id) in missing {
            if ![&g.home_id, &g.away_id].contains(&&id)
                || id.is_empty()
                || !id.bytes().all(|b| b.is_ascii_digit())
            {
                return Err("invalid core period team identity".into());
            }
            std::thread::sleep(Duration::from_millis(250));
            let url = format!(
                "https://sports.core.api.espn.com/v2/sports/{}/leagues/{}/events/{}/competitions/{}/competitors/{}/linescores?lang=en&region=us&limit=100",
                g.sport, g.league, g.event_id, g.event_id, id
            );
            let root: Value = serde_json::from_str(&crate::http::get_text(&self.client, &url)?)?;
            let lines = core_lines(&root, g, &id)?;
            self.boards.get_mut(&key).unwrap()["events"][event_index]["competitions"][comp_index]
                ["competitors"][index]["linescores"] = Value::Array(lines);
            self.boards.get_mut(&key).unwrap()["events"][event_index]["periodCoreMetadata"] =
                json!(true);
        }
        Ok(&self.boards[&key])
    }
}

fn summary_anchor(root: &Value, g: &Game) -> Result<Value> {
    let h = &root["header"];
    if h["id"] != g.event_id {
        return Err("period summary event mismatch".into());
    }
    let c = items(&h["competitions"])
        .iter()
        .find(|c| c["id"] == g.event_id)
        .ok_or("period summary competition mismatch")?;
    if c["status"]["type"]["completed"] != true {
        return Err("period summary is not final".into());
    }
    let mut cs = items(&c["competitors"]).to_vec();
    if cs.len() != 2 {
        return Err("period summary requires two teams".into());
    }
    for (side, id) in [("home", &g.home_id), ("away", &g.away_id)] {
        if !cs
            .iter()
            .any(|c| c["homeAway"] == side && c["team"]["id"] == *id)
        {
            return Err("period summary team mismatch".into());
        }
    }
    for team in &mut cs {
        team.as_object_mut()
            .ok_or("invalid period summary team")?
            .remove("linescores");
    }
    Ok(
        json!({"id":g.event_id,"season":h["season"],"periodCoreMetadata":true,"periodSummaryAnchor":true,
        "competitions":[{"id":g.event_id,"status":c["status"],"competitors":cs}]}),
    )
}

fn core_lines(root: &Value, g: &Game, team: &str) -> Result<Vec<Value>> {
    let lines = root["items"]
        .as_array()
        .ok_or("missing core line-score items")?;
    if root["count"].as_u64() != Some(lines.len() as u64) || root["pageCount"].as_u64() != Some(1) {
        return Err("partial core line-score response".into());
    }
    let prefix = format!(
        "/v2/sports/{}/leagues/{}/events/{}/competitions/{}/competitors/{}/linescores/",
        g.sport, g.league, g.event_id, g.event_id, team
    );
    for l in lines {
        let reference =
            url::Url::parse(l["$ref"].as_str().ok_or("core period reference missing")?)?;
        if reference.host_str() != Some("sports.core.api.espn.com")
            || !reference.path().starts_with(&prefix)
        {
            return Err("foreign core period reference".into());
        }
    }
    Ok(lines.clone())
}

/// Repair only period fields and normalization metadata. All values and statistics stay intact.
pub fn apply(payload: &str, board: &Value, g: &Game) -> Result<String> {
    let mut data: Value = serde_json::from_str(payload)?;
    if data["version"] != 1
        || data["sport"] != g.sport
        || data["league"] != g.league
        || data["eventId"] != g.event_id
    {
        return Err("period cache identity/version mismatch".into());
    }
    let event = items(&board["events"])
        .iter()
        .find(|e| e["id"] == g.event_id)
        .ok_or("period metadata event not found")?;
    let competition = items(&event["competitions"])
        .iter()
        .find(|c| c["id"] == g.event_id)
        .ok_or("period competition mismatch")?;
    if competition["status"]["type"]["completed"] != true {
        return Err("period metadata event is not final".into());
    }
    let cs = items(&competition["competitors"]);
    if cs.len() != 2 {
        return Err("period metadata requires two competitors".into());
    }
    let teams = data["teams"]
        .as_array_mut()
        .ok_or("period payload has no teams")?;
    if teams.len() != 2 {
        return Err("period cache requires two teams".into());
    }
    for (side, id) in [("home", &g.home_id), ("away", &g.away_id)] {
        if !cs
            .iter()
            .any(|c| c["homeAway"] == side && c["team"]["id"] == *id)
            || !teams
                .iter()
                .any(|t| t["homeAway"] == side && t["id"] == *id)
        {
            return Err("period metadata team mismatch".into());
        }
    }
    let mut unknown = 0;
    for team in teams {
        let c = cs
            .iter()
            .find(|c| c["team"]["id"] == team["id"])
            .ok_or("missing period team")?;
        let source = items(&c["linescores"]);
        let lines = team["lineScores"]
            .as_array_mut()
            .ok_or("missing cached line array")?;
        if lines.len() != source.len() {
            return Err(
                "period metadata/cached line count mismatch; preserving cached values".into(),
            );
        }
        let mut period_ids = std::collections::BTreeSet::new();
        let last = source
            .iter()
            .filter_map(|l| number(&l["period"]))
            .max()
            .unwrap_or(0);
        // Every mapping uses the corresponding provider period, including sparse arrays.
        // A missing identifier stays null; no 1..N period sequence is synthesized.
        for (line, raw) in lines.iter_mut().zip(source) {
            let v = value(&raw["displayValue"]).or_else(|| value(&raw["value"]));
            if value(&line["displayValue"]) != v {
                return Err(
                    "period metadata/cached value mismatch; preserving cached values".into(),
                );
            }
            // Duplicate numeric identifiers are ambiguous, not a license to deduplicate entries.
            if let Some(p) = number(&raw["period"])
                && !period_ids.insert(p)
            {
                return Err("duplicate provider period identity".into());
            }
            let identity = if line["period"].is_null()
                || line["period"].as_str().is_some_and(|s| s.trim().is_empty())
            {
                &raw["period"]
            } else {
                &line["period"]
            };
            let mapped = label(
                identity,
                g,
                &competition["status"]["type"],
                event["season"]["type"].as_u64(),
                last,
            );
            if mapped.is_none() {
                unknown += 1;
            }
            line["period"] = mapped.map(Value::String).unwrap_or(Value::Null);
        }
    }
    data["periodLabelsVersion"] = json!(1);
    data["periodLabelSource"] = json!(if event["periodSummaryAnchor"] == true {
        "espn-summary+core"
    } else if event["periodCoreMetadata"] == true {
        "espn-scoreboard+core"
    } else {
        "espn-scoreboard"
    });
    data["periodLabelStatus"] = json!(if unknown == 0 { "complete" } else { "partial" });
    if unknown > 0 {
        eprintln!(
            "period labels {}/{}/{}: {unknown} ambiguous entries retained",
            g.sport, g.league, g.event_id
        );
    }
    Ok(data.to_string())
}

pub fn repair(
    db: &mut crate::db::Db,
    dry: bool,
    limit: i32,
    run_id: Option<i32>,
) -> Result<(usize, usize)> {
    let games = db.period_candidates(limit)?;
    let mut boards = Boards::new()?;
    let start = std::time::Instant::now();
    let (mut ok, mut errors, mut changed) = (0, 0, 0);
    for (game, old) in games {
        if start.elapsed() >= Duration::from_secs(180) {
            break;
        }
        match boards
            .get(&game)
            .and_then(|board| apply(&old, board, &game))
        {
            Ok(new) => {
                if dry || db.update_periods(&game, &old, &new)? {
                    changed += 1;
                }
                ok += 1;
            }
            Err(e) => {
                errors += 1;
                eprintln!(
                    "period repair {}/{}/{}: {e}; retained last-good cache",
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
    }
    eprintln!(
        "period repair: {ok} validated, {changed} updated{}, {errors} errors",
        if dry { " (dry run)" } else { "" }
    );
    Ok((ok, errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(league: &str) -> (Value, Game, Value) {
        let raw = match league {
            "mlb" => include_str!("../tests/fixtures/periods/mlb.json"),
            "nfl" => include_str!("../tests/fixtures/periods/nfl.json"),
            "college-football" => include_str!("../tests/fixtures/periods/college-football.json"),
            "nhl" => include_str!("../tests/fixtures/periods/nhl.json"),
            "mens-college-basketball" => {
                include_str!("../tests/fixtures/periods/mens-college-basketball.json")
            }
            "fifa.world" => include_str!("../tests/fixtures/periods/fifa.world.json"),
            "usa.nwsl" => include_str!("../tests/fixtures/periods/usa.nwsl.json"),
            "eng.2" => include_str!("../tests/fixtures/periods/eng.2.json"),
            _ => include_str!("../tests/fixtures/periods/usa.1.json"),
        };
        let v: Value = serde_json::from_str(raw).unwrap();
        let cs = items(&v["summaryTeams"]);
        let id = |side| {
            cs.iter().find(|c| c["homeAway"] == side).unwrap()["team"]["id"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let g = Game {
            sport: v["game"]["sport"].as_str().unwrap().into(),
            league: league.into(),
            event_id: v["game"]["eventId"].as_str().unwrap().into(),
            home_id: id("home"),
            away_id: id("away"),
            date: v["game"]["date"].as_str().unwrap().into(),
        };
        let teams:Vec<_>=cs.iter().map(|c|json!({"id":c["team"]["id"],"homeAway":c["homeAway"],"score":c["score"],
            "name":c["team"]["displayName"],"statistics":[{"key":"unchanged","value":"0"}],
            "lineScores":items(&c["linescores"]).iter().map(|l|json!({"period":null,"displayValue":l["displayValue"]})).collect::<Vec<_>>()})).collect();
        let payload = json!({"version":1,"sport":g.sport,"league":g.league,"eventId":g.event_id,"teams":teams,"playerGroups":[{"name":"retained","stats":["0","--"]}]});
        (v["scoreboard"].clone(), g, payload)
    }
    fn apply_json(p: &Value, b: &Value, g: &Game) -> Value {
        serde_json::from_str(&apply(&p.to_string(), b, g).unwrap()).unwrap()
    }
    fn labels(v: &Value) -> Vec<Value> {
        items(&v["teams"][0]["lineScores"])
            .iter()
            .map(|x| x["period"].clone())
            .collect()
    }
    fn strip(v: &mut Value) {
        for k in [
            "periodLabelsVersion",
            "periodLabelSource",
            "periodLabelStatus",
        ] {
            v.as_object_mut().unwrap().remove(k);
        }
        for t in v["teams"].as_array_mut().unwrap() {
            for l in t["lineScores"].as_array_mut().unwrap() {
                l["period"] = Value::Null;
            }
        }
    }
    #[test]
    fn recorded_omitted_college_game_core_metadata_matches_cached_summary_values() {
        let v: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/periods/college-football-omitted-event.json"
        ))
        .unwrap();
        let teams = items(&v["teams"]);
        let id = |side| {
            teams.iter().find(|t| t["homeAway"] == side).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let g = Game {
            sport: "football".into(),
            league: "college-football".into(),
            event_id: v["eventId"].as_str().unwrap().into(),
            date: v["date"].as_str().unwrap().into(),
            home_id: id("home"),
            away_id: id("away"),
        };
        for t in teams {
            let rows = core_lines(&t["core"], &g, t["id"].as_str().unwrap()).unwrap();
            assert_eq!(rows.len(), items(&t["summaryLines"]).len());
            for (raw, summary) in rows.iter().zip(items(&t["summaryLines"])) {
                assert_eq!(raw["displayValue"], summary["displayValue"]);
                assert!(
                    label(&raw["period"], &g, &v["status"], Some(2), 4)
                        .unwrap()
                        .starts_with('Q')
                );
            }
        }
    }
    #[test]
    fn invalid_metadata_identity_is_rejected_before_http() {
        let (_, mut g, _) = fixture("nfl");
        g.event_id = "1&other=2".into();
        assert!(Boards::new().unwrap().get(&g).is_err());
    }
    #[test]
    fn missing_day_events_can_use_verified_summary_anchor_without_inferred_periods() {
        let (b, g, p) = fixture("nfl");
        let comp = b["events"][0]["competitions"][0].clone();
        let summary = json!({"header":{"id":g.event_id,"season":{"type":2},"competitions":[comp]}});
        let event = summary_anchor(&summary, &g).unwrap();
        assert_eq!(event["periodSummaryAnchor"], true);
        assert!(
            event["competitions"][0]["competitors"][0]
                .get("linescores")
                .is_none()
        );
        assert!(apply(&p.to_string(), &json!({"events":[event]}), &g).is_err());
        let mut bad = summary.clone();
        bad["header"]["id"] = json!("wrong");
        assert!(summary_anchor(&bad, &g).is_err());
        let mut bad = summary;
        bad["header"]["competitions"][0]["competitors"][0]["team"]["id"] = json!("wrong");
        assert!(summary_anchor(&bad, &g).is_err());
    }
    #[test]
    fn core_metadata_requires_complete_page_and_matching_game_team_references() {
        let (b, g, _) = fixture("usa.nwsl");
        let team = &g.home_id;
        let cs = items(&b["events"][0]["competitions"][0]["competitors"]);
        let rows = cs.iter().find(|c| c["team"]["id"] == *team).unwrap()["linescores"].clone();
        let mut root = json!({"count":items(&rows).len(),"pageCount":1,"items":rows});
        assert!(core_lines(&root, &g, team).is_ok());
        root["pageCount"] = json!(2);
        assert!(core_lines(&root, &g, team).is_err());
        root["pageCount"] = json!(1);
        root["items"][0]["$ref"] = json!(
            "http://sports.core.api.espn.com/v2/sports/soccer/leagues/usa.nwsl/events/999/competitions/999/competitors/999/linescores/38/1"
        );
        assert!(core_lines(&root, &g, team).is_err());
    }
    #[test]
    fn wnba_uses_quarters_not_college_halves() {
        let (b, mut g, mut p) = synthetic(
            "mens-college-basketball",
            &[json!(1), json!(2), json!(3), json!(4), json!(5)],
            &[json!("0"), json!("0"), json!("0"), json!("0"), json!("2")],
        );
        g.league = "wnba".into();
        p["league"] = json!("wnba");
        assert_eq!(
            labels(&apply_json(&p, &b, &g)),
            vec![
                json!("Q1"),
                json!("Q2"),
                json!("Q3"),
                json!("Q4"),
                json!("OT")
            ]
        );
    }
    #[test]
    fn recorded_provider_formats_and_label_only_idempotence() {
        for league in [
            "mlb",
            "nfl",
            "college-football",
            "nhl",
            "mens-college-basketball",
            "fifa.world",
            "usa.nwsl",
            "eng.2",
            "usa.1",
        ] {
            let (b, g, p) = fixture(league);
            let out = apply_json(&p, &b, &g);
            assert_eq!(out["version"], 1);
            assert_eq!(out["periodLabelStatus"], "complete", "{league}");
            assert_eq!(apply_json(&out, &b, &g), out, "{league}");
            let mut untouched = out.clone();
            strip(&mut untouched);
            assert_eq!(untouched, p, "nonlabel content changed in {league}");
            assert!(labels(&out).iter().all(|x| x.is_string()));
            match league {
                "mlb" => {
                    assert_eq!(labels(&out)[0], "1st inning");
                    assert_eq!(labels(&out)[8], "9th inning");
                }
                "nfl" => {
                    assert_eq!(labels(&out)[3], "Q4");
                    assert_eq!(labels(&out)[4], "OT");
                }
                "nhl" => {
                    assert_eq!(
                        labels(&out),
                        vec![
                            json!("1st period"),
                            json!("2nd period"),
                            json!("3rd period"),
                            json!("OT"),
                            json!("SO")
                        ]
                    );
                }
                "mens-college-basketball" => {
                    assert_eq!(labels(&out), vec![json!("1st half"), json!("2nd half")])
                }
                "fifa.world" => assert_eq!(
                    labels(&out),
                    vec![
                        json!("1st half"),
                        json!("2nd half"),
                        json!("ET1"),
                        json!("ET2"),
                        json!("SO")
                    ]
                ),
                _ => {}
            }
        }
    }
    // Synthetic edges are explicit, not claimed as observations of retained production games.
    fn synthetic(league: &str, ids: &[Value], values: &[Value]) -> (Value, Game, Value) {
        let (mut b, g, mut p) = fixture(league);
        for (i, t) in p["teams"].as_array_mut().unwrap().iter_mut().enumerate() {
            t["lineScores"] = Value::Array(
                values
                    .iter()
                    .map(|v| json!({"period":null,"displayValue":v}))
                    .collect(),
            );
            b["events"][0]["competitions"][0]["competitors"][i]["linescores"] = Value::Array(
                ids.iter()
                    .zip(values)
                    .map(|(n, v)| json!({"period":n,"displayValue":v}))
                    .collect(),
            );
        }
        (b, g, p)
    }
    #[test]
    fn synthetic_extra_innings_shortened_games_unplayed_and_zero() {
        for n in [7, 11] {
            let ids: Vec<_> = (1..=n).map(|x| json!(x)).collect();
            let mut vals = vec![json!("0"); n];
            vals[n - 1] = json!("-");
            let (b, g, p) = synthetic("mlb", &ids, &vals);
            let out = apply_json(&p, &b, &g);
            assert_eq!(labels(&out).len(), n);
            assert_eq!(labels(&out)[n - 1], format!("{} inning", ordinal(n as u64)));
            assert_eq!(out["teams"][0]["lineScores"][n - 1]["displayValue"], "-");
            assert_eq!(out["teams"][0]["lineScores"][0]["displayValue"], "0");
        }
    }
    #[test]
    fn synthetic_multiple_overtime_is_sport_specific() {
        for (league, ids, expected) in [
            ("college-football", vec![1, 2, 3, 4, 5, 6], "2OT"),
            ("mens-college-basketball", vec![1, 2, 3, 4], "2OT"),
            ("nhl", vec![1, 2, 3, 4, 5, 6], "3OT"),
        ] {
            let ids: Vec<_> = ids.iter().map(|n| json!(n)).collect();
            let (b, mut g, p) = synthetic(league, &ids, &vec![json!("0"); ids.len()]);
            let mut b = b;
            if league == "nhl" {
                b["events"][0]["season"]["type"] = json!(3);
                b["events"][0]["competitions"][0]["status"]["type"] =
                    json!({"completed":true,"name":"STATUS_FINAL","shortDetail":"Final/3OT"});
            }
            g.league = league.into();
            let out = apply_json(&p, &b, &g);
            assert_eq!(labels(&out).last().unwrap(), expected);
        }
    }
    #[test]
    fn sparse_arrays_use_explicit_identity_and_missing_values_stay_missing() {
        let (b, g, p) = synthetic("nfl", &[json!(1), json!(3)], &[json!("0"), Value::Null]);
        let out = apply_json(&p, &b, &g);
        assert_eq!(labels(&out), vec![json!("Q1"), json!("Q3")]);
        assert!(out["teams"][0]["lineScores"][1]["displayValue"].is_null());
    }
    #[test]
    fn absent_metadata_does_not_get_guessed_from_length() {
        let (b, g, p) = synthetic(
            "nfl",
            &[Value::Null, Value::Null],
            &[json!("0"), json!("7")],
        );
        let out = apply_json(&p, &b, &g);
        assert_eq!(labels(&out), vec![Value::Null, Value::Null]);
        assert_eq!(out["periodLabelStatus"], "partial");
    }
    #[test]
    fn unknown_duplicate_labels_preserve_all_entries_and_explicit_text() {
        let (b, g, p) = synthetic(
            "nfl",
            &[json!("Unspecified"), json!("Unspecified")],
            &[json!("0"), json!("7")],
        );
        assert_eq!(
            labels(&apply_json(&p, &b, &g)),
            vec![json!("Unspecified"), json!("Unspecified")]
        );
    }
    #[test]
    fn bad_identity_values_column_counts_or_nonfinal_preserve_last_good_via_error() {
        let (b, g, p) = fixture("mlb");
        let mut bad = b.clone();
        bad["events"][0]["id"] = json!("wrong");
        assert!(apply(&p.to_string(), &bad, &g).is_err());
        let mut bad = b.clone();
        bad["events"][0]["competitions"][0]["competitors"][0]["team"]["id"] = json!("wrong");
        assert!(apply(&p.to_string(), &bad, &g).is_err());
        let mut bad = p.clone();
        bad["teams"][0]["lineScores"][0]["displayValue"] = json!("999");
        assert!(apply(&bad.to_string(), &b, &g).is_err());
        let mut bad = b.clone();
        bad["events"][0]["competitions"][0]["competitors"][0]["linescores"] = json!([]);
        assert!(apply(&p.to_string(), &bad, &g).is_err());
        let mut bad = b;
        bad["events"][0]["competitions"][0]["status"]["type"]["completed"] = json!(false);
        assert!(apply(&p.to_string(), &bad, &g).is_err());
    }
    #[test]
    fn duplicate_numeric_ids_and_ambiguous_shootouts_are_not_relabeled_as_ot() {
        let (b, g, p) = synthetic("nfl", &[json!(1), json!(1)], &[json!("0"), json!("0")]);
        assert!(apply(&p.to_string(), &b, &g).is_err());
        let (mut b, g, p) = synthetic("nhl", &[json!(5)], &[json!("0")]);
        b["events"][0]["competitions"][0]["status"]["type"] =
            json!({"completed":true,"name":"STATUS_FINAL"});
        assert_eq!(labels(&apply_json(&p, &b, &g)), vec![Value::Null]);
        let (mut b, g, p) = synthetic(
            "fifa.world",
            &[json!(1), json!(2), json!(3)],
            &[json!("0"), json!("0"), json!("4")],
        );
        b["events"][0]["competitions"][0]["status"]["type"] =
            json!({"completed":true,"name":"STATUS_FINAL_PEN"});
        assert!(labels(&apply_json(&p, &b, &g))[2].is_null());
    }
}
