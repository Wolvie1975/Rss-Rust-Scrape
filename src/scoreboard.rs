//! Persisted ESPN samples. This module never writes schedules or the web results tables.
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Debug, Clone)]
pub struct Coverage {
    pub id: i32,
    pub sport: String,
    pub league: String,
    pub scope: String,
    pub teams: BTreeSet<String>,
    pub season: Option<i32>,
}
pub fn retention_cutoff(now: DateTime<Utc>, previous_days: u32) -> NaiveDate {
    now.with_timezone(&chrono_tz::America::Chicago).date_naive()
        - chrono::Duration::days(i64::from(previous_days))
}
pub fn supported(sport: &str, league: &str) -> bool {
    matches!(
        (sport, league),
        ("football", "nfl" | "college-football")
            | (
                "basketball",
                "nba" | "wnba" | "mens-college-basketball" | "womens-college-basketball"
            )
            | ("baseball", "mlb")
            | ("hockey", "nhl")
            | (
                "soccer",
                "usa.1" | "usa.nwsl" | "eng.1" | "eng.2" | "usa.ncaa.w.1"
            )
            | ("volleyball", "womens-college-volleyball")
    )
}
pub fn request_url(sport: &str, league: &str, day: NaiveDate) -> Result<url::Url> {
    if !supported(sport, league) {
        return Err("unsupported league".into());
    }
    let mut url = url::Url::parse(&format!(
        "https://site.api.espn.com/apis/site/v2/sports/{sport}/{league}/scoreboard"
    ))?;
    url.query_pairs_mut()
        .append_pair("dates", &day.format("%Y%m%d").to_string())
        .append_pair("limit", "1000");
    if league == "college-football" {
        url.query_pairs_mut().append_pair("groups", "80");
    } else if league.contains("college-basketball") {
        url.query_pairs_mut().append_pair("groups", "50");
    }
    Ok(url)
}
fn text(v: &Value, n: usize) -> Result<Option<String>> {
    if v.is_null() {
        return Ok(None);
    }
    let s = v.as_str().ok_or("expected provider text")?;
    if s.encode_utf16().count() > n {
        return Err("provider text exceeds storage limit".into());
    }
    Ok((!s.trim().is_empty()).then(|| s.to_owned()))
}
fn safe(v: &Value) -> Option<String> {
    v.as_str()
        .and_then(|s| crate::http::public_url(s).ok())
        .map(|u| u.to_string())
}
fn timestamp(v: &Value) -> Result<Option<DateTime<Utc>>> {
    let Some(s) = v.as_str() else {
        return if v.is_null() {
            Ok(None)
        } else {
            Err("invalid timestamp".into())
        };
    };
    Ok(Some(
        DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%MZ").map(|d| d.and_utc())
            })?,
    ))
}
fn score(v: &Value) -> Result<Option<f64>> {
    if v.is_null() {
        return Ok(None);
    }
    let v = if v.is_object() { &v["value"] } else { v };
    let n = v
        .as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .ok_or("invalid score")?;
    if !n.is_finite()
        || !(0.0..=999999999.999).contains(&n)
        || (n * 1000.0 - (n * 1000.0).round()).abs() > 0.00001
    {
        return Err("unsupported score precision/range".into());
    }
    Ok(Some(n))
}
fn integer(v: &Value) -> Result<Option<i32>> {
    if v.is_null() {
        return Ok(None);
    }
    let n = v
        .as_i64()
        .or_else(|| v.as_str()?.parse().ok())
        .ok_or("invalid integer detail")?;
    Ok(Some(
        i32::try_from(n)
            .ok()
            .filter(|n| *n >= 0)
            .ok_or("integer detail out of range")?,
    ))
}
pub fn normalized_status(code: &str, state: &str, completed: bool) -> &'static str {
    match code {
        "STATUS_POSTPONED" => "postponed",
        "STATUS_CANCELED" | "STATUS_CANCELLED" | "STATUS_ABANDONED" => "cancelled",
        "STATUS_SUSPENDED" => "suspended",
        "STATUS_DELAYED" | "STATUS_RAIN_DELAY" => "delayed",
        _ if completed
            && (code.starts_with("STATUS_FINAL")
                || code == "STATUS_FULL_TIME"
                || state == "post") =>
        {
            "final"
        }
        _ if state == "in" => "live",
        _ if state == "pre" => "upcoming",
        _ => "unknown",
    }
}
impl Coverage {
    pub fn accepts(&self, e: &Value, row: &Value) -> bool {
        if self
            .season
            .is_some_and(|s| e["season"]["year"].as_i64() != Some(i64::from(s)))
        {
            return false;
        }
        // Team/conference schedules in the web worker deliberately exclude preseason.
        if matches!(self.scope.as_str(), "team" | "conference")
            && self.league != "usa.ncaa.w.1"
            && !matches!(e["season"]["type"].as_i64(), Some(2 | 3))
        {
            return false;
        }
        if self.scope == "league" {
            return true;
        }
        if self.scope == "schedule" && self.teams.is_empty() {
            return true;
        }
        ["homeTeamId", "awayTeamId"]
            .iter()
            .any(|k| row[*k].as_str().is_some_and(|s| self.teams.contains(s)))
    }
}
fn parse_event(e: &Value, sport: &str, day: NaiveDate) -> Result<Value> {
    let id = text(&e["id"], 80)?.ok_or("event identity missing")?;
    let comps = e["competitions"].as_array().ok_or("competition missing")?;
    if comps.len() != 1 {
        return Err("ambiguous event competitions".into());
    }
    let c = &comps[0];
    let competitors = c["competitors"].as_array().ok_or("participants missing")?;
    if competitors.len() != 2 {
        return Err("unsupported matchup".into());
    }
    let side = |name: &str| -> Result<&Value> {
        let mut found = competitors.iter().filter(|v| v["homeAway"] == name);
        let v = found.next().ok_or("home/away designation missing")?;
        if found.next().is_some() {
            return Err("duplicate participant designation".into());
        }
        Ok(v)
    };
    let h = side("home")?;
    let a = side("away")?;
    let status = if c["status"].is_object() {
        &c["status"]
    } else {
        &e["status"]
    };
    let code = text(&status["type"]["name"], 100)?.ok_or("event status missing")?;
    let state = status["type"]["state"].as_str().unwrap_or("");
    let completed = status["type"]["completed"].as_bool().unwrap_or(false);
    let norm = normalized_status(&code, state, completed);
    let raw_start = if c["date"].is_null() {
        &e["date"]
    } else {
        &c["date"]
    };
    let date_only = raw_start
        .as_str()
        .filter(|s| s.len() == 10)
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()?;
    let start = if date_only.is_some() {
        None
    } else {
        timestamp(raw_start)?
    };
    let tbd = c["timeValid"] == false
        || e["timeValid"] == false
        || c["startDateTBD"] == true
        || e["dateTBD"] == true
        || start.is_none();
    let event_day = if tbd {
        date_only.unwrap_or(day)
    } else {
        start
            .map(|d| d.with_timezone(&chrono_tz::America::Chicago).date_naive())
            .unwrap_or(day)
    };
    let (hs, ascore) = if norm == "upcoming" {
        (None, None)
    } else {
        (score(&h["score"])?, score(&a["score"])?)
    };
    if matches!(norm, "live" | "final") && (hs.is_none() || ascore.is_none()) {
        return Err("active/final scores missing".into());
    }
    let team_id = |v: &Value| text(&v["team"]["id"], 80);
    let home_id = team_id(h)?;
    let away_id = team_id(a)?;
    if home_id.is_some() && home_id == away_id {
        return Err("self opponent".into());
    }
    let name = |v: &Value| -> Result<String> {
        Ok(text(&v["team"]["displayName"], 200)?
            .or(text(&v["team"]["name"], 200)?)
            .unwrap_or("TBD".into()))
    };
    let detail =
        text(&status["type"]["detail"], 300)?.or(text(&status["type"]["shortDetail"], 300)?);
    let source = e["links"]
        .as_array()
        .and_then(|ls| ls.iter().find_map(|l| safe(&l["href"])));
    let winner = if norm == "final" {
        match (h["winner"].as_bool(), a["winner"].as_bool()) {
            (Some(true), Some(false)) => Some("home"),
            (Some(false), Some(true)) => Some("away"),
            _ => None,
        }
    } else {
        None
    };
    let broadcasts = c["broadcasts"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|b| b["names"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    if broadcasts.encode_utf16().count() > 1000 {
        return Err("broadcast text exceeds limit".into());
    }
    // Retain only selected scalar live details and per-period scores, never the full provider body.
    let mut live = json!({"statusLabel":detail,"shortLabel":text(&status["type"]["shortDetail"],300)?,"homePeriods":[],"awayPeriods":[]});
    for (key, v) in [("homePeriods", h), ("awayPeriods", a)] {
        let mut lines = vec![];
        for (idx, l) in v["linescores"].as_array().into_iter().flatten().enumerate() {
            if idx >= 100 {
                return Err("too many period scores".into());
            }
            lines.push(json!({"period":l["period"].as_i64().unwrap_or(idx as i64+1),"score":score(&l["value"])?,"displayValue":text(&l["displayValue"],80)?}));
        }
        live[key] = json!(lines);
    }
    if sport == "baseball" {
        live["outs"] = json!(integer(&c["situation"]["outs"])?);
        live["balls"] = json!(integer(&c["situation"]["balls"])?);
        live["strikes"] = json!(integer(&c["situation"]["strikes"])?);
        // The explicit status label (e.g. Bottom 8th) is the authoritative inning-half display.
        for k in ["onFirst", "onSecond", "onThird"] {
            live[k] = json!(c["situation"][k].as_bool());
        }
    }
    let city = text(&c["venue"]["address"]["city"], 100)?;
    live["city"] = json!(city);
    Ok(
        json!({"eventId":id,"providerDate":day.to_string(),"scoreboardDate":event_day.to_string(),
        "startsAtUtc":if tbd{None}else{start.map(|d|d.naive_utc().to_string())},"timeTbd":tbd,
        "homeTeamId":home_id,"awayTeamId":away_id,"homeName":name(h)?,"awayName":name(a)?,
        "homeLogoUrl":safe(&h["team"]["logo"]),"awayLogoUrl":safe(&a["team"]["logo"]),
        "homeScore":hs,"awayScore":ascore,"status":norm,"completed":norm=="final",
        "providerStatus":code,"statusText":detail,"period":integer(&status["period"])?,
        "displayClock":if sport=="baseball"||sport=="volleyball"{None}else{text(&status["displayClock"],80)?},
        "neutralSite":c["neutralSite"].as_bool(),"homeShootout":integer(&h["shootoutScore"])?,"awayShootout":integer(&a["shootoutScore"])?,
        "winnerSide":winner,"venue":text(&c["venue"]["fullName"],300)?,"broadcasts":if broadcasts.is_empty(){None}else{Some(broadcasts)},
        "sourceUrl":source,"liveDetails":live,"providerUpdatedAtUtc":timestamp(&e["lastUpdated"])?.map(|d|d.naive_utc().to_string())}),
    )
}
pub struct Parsed {
    pub rows: Vec<Value>,
    pub issues: usize,
    pub event_count: usize,
}
pub fn parse(
    root: &Value,
    sport: &str,
    league: &str,
    day: NaiveDate,
    coverage: &[Coverage],
) -> Result<Parsed> {
    if !supported(sport, league) {
        return Err("unsupported league".into());
    }
    if !root["leagues"].as_array().is_some_and(|ls| {
        ls.iter().any(|l| {
            l["slug"] == league
                || l["abbreviation"]
                    .as_str()
                    .is_some_and(|a| a.eq_ignore_ascii_case(league))
        })
    }) {
        return Err("wrong/missing provider league".into());
    }
    let es = root["events"].as_array().ok_or("missing events array")?;
    if es.len() >= 1000 {
        return Err("possibly truncated scoreboard".into());
    }
    let mut rows = vec![];
    let mut issues = 0;
    let mut ids = BTreeSet::new();
    for e in es {
        match parse_event(e, sport, day) {
            Ok(mut row) => {
                if !ids.insert(row["eventId"].as_str().unwrap().to_owned()) {
                    return Err("duplicate provider event".into());
                }
                let memberships = coverage
                    .iter()
                    .filter(|s| s.accepts(e, &row))
                    .map(|s| s.id)
                    .collect::<Vec<_>>();
                if !memberships.is_empty() {
                    row["sources"] = json!(memberships);
                    rows.push(row);
                }
            }
            Err(_) => issues += 1,
        }
    }
    Ok(Parsed {
        rows,
        issues,
        event_count: es.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(sport: &str) -> (String, Value) {
        let raw = match sport {
            "football" => include_str!("../tests/fixtures/scoreboard/football.json"),
            "basketball" => include_str!("../tests/fixtures/scoreboard/basketball.json"),
            "baseball" => include_str!("../tests/fixtures/scoreboard/baseball.json"),
            "hockey" => include_str!("../tests/fixtures/scoreboard/hockey.json"),
            "soccer" => include_str!("../tests/fixtures/scoreboard/soccer.json"),
            _ => include_str!("../tests/fixtures/scoreboard/volleyball.json"),
        };
        let root: Value = serde_json::from_str(raw).unwrap();
        (
            root["leagues"][0]["slug"].as_str().unwrap().to_owned(),
            root,
        )
    }
    fn coverage(sport: &str, league: &str) -> Vec<Coverage> {
        vec![Coverage {
            id: 1,
            sport: sport.into(),
            league: league.into(),
            scope: "league".into(),
            teams: BTreeSet::new(),
            season: None,
        }]
    }
    fn sample(root: &Value, sport: &str, league: &str) -> Parsed {
        parse(
            root,
            sport,
            league,
            NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
            &coverage(sport, league),
        )
        .unwrap()
    }
    #[test]
    fn retention_uses_previous_central_calendar_day_at_utc_midnight_and_dst() {
        for (now, wanted) in [
            ("2026-10-24T03:00:00Z", "2026-10-22"),
            ("2026-10-24T05:00:00Z", "2026-10-23"),
            ("2026-11-01T06:30:00Z", "2026-10-31"),
            ("2026-11-01T07:30:00Z", "2026-10-31"),
        ] {
            assert_eq!(
                retention_cutoff(now.parse().unwrap(), 1).to_string(),
                wanted
            );
        }
    }
    #[test]
    fn provider_groups_are_specific_to_each_sport() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        assert!(
            request_url("football", "college-football", day)
                .unwrap()
                .as_str()
                .contains("groups=80")
        );
        assert!(
            request_url("basketball", "mens-college-basketball", day)
                .unwrap()
                .as_str()
                .contains("groups=50")
        );
        assert!(
            !request_url("volleyball", "womens-college-volleyball", day)
                .unwrap()
                .as_str()
                .contains("groups=")
        );
    }
    #[test]
    fn every_sport_transitions_and_preserves_zero_and_missing_scores() {
        for sport in [
            "football",
            "basketball",
            "baseball",
            "hockey",
            "soccer",
            "volleyball",
        ] {
            let (league, mut root) = fixture(sport);
            assert_eq!(sample(&root, sport, &league).issues, 0, "{sport}");
            for (code, state, done, norm) in [
                ("STATUS_SCHEDULED", "pre", false, "upcoming"),
                ("STATUS_IN_PROGRESS", "in", false, "live"),
                ("STATUS_FINAL", "post", true, "final"),
            ] {
                let c = &mut root["events"][0]["competitions"][0];
                c["status"] = json!({"type":{"name":code,"state":state,"completed":done},"period":2,"displayClock":"12:00"});
                for t in c["competitors"].as_array_mut().unwrap() {
                    t["score"] = json!("0");
                }
                let p = sample(&root, sport, &league);
                assert_eq!(p.rows.len(), 1, "{sport}");
                assert_eq!(p.rows[0]["status"], norm);
                assert_eq!(
                    p.rows[0]["homeScore"],
                    if norm == "upcoming" {
                        Value::Null
                    } else {
                        json!(0.0)
                    }
                );
                assert_eq!(p.rows[0]["completed"], done);
            }
            root["events"][0]["competitions"][0]["competitors"][0]["score"] = Value::Null;
            let p = sample(&root, sport, &league);
            assert_eq!(p.issues, 1);
            assert!(p.rows.is_empty());
        }
    }
    #[test]
    fn unusual_states_override_provider_post_and_completion() {
        for (code, wanted) in [
            ("STATUS_POSTPONED", "postponed"),
            ("STATUS_CANCELLED", "cancelled"),
            ("STATUS_SUSPENDED", "suspended"),
            ("STATUS_DELAYED", "delayed"),
            ("STATUS_RAIN_DELAY", "delayed"),
        ] {
            assert_eq!(normalized_status(code, "post", true), wanted);
        }
        assert_eq!(normalized_status("UNRECOGNIZED", "", false), "unknown");
    }
    #[test]
    fn tbd_dates_midnight_dst_and_safe_links() {
        let (league, mut root) = fixture("soccer");
        let c = &mut root["events"][0]["competitions"][0];
        c["date"] = json!("2026-11-01T05:30:00Z");
        c["timeValid"] = json!(true);
        c["competitors"][0]["team"]["logo"] = json!("http://127.0.0.1/secret");
        root["events"][0]["dateTBD"] = json!(false);
        let r = &sample(&root, "soccer", &league).rows[0];
        assert_eq!(r["scoreboardDate"], "2026-11-01");
        assert!(r["homeLogoUrl"].is_null() || r["awayLogoUrl"].is_null());
        root["events"][0]["competitions"][0]["date"] = json!("2026-10-24T03:30:00Z");
        assert_eq!(
            sample(&root, "soccer", &league).rows[0]["scoreboardDate"],
            "2026-10-23"
        );
        root["events"][0]["competitions"][0]["timeValid"] = json!(false);
        assert!(sample(&root, "soccer", &league).rows[0]["startsAtUtc"].is_null());
        root["events"][0]["competitions"][0]["date"] = json!("2026-10-25");
        assert_eq!(
            sample(&root, "soccer", &league).rows[0]["scoreboardDate"],
            "2026-10-25"
        );
    }
    #[test]
    fn scopes_deduplicate_preserve_season_and_do_not_expand_teams() {
        let (league, mut root) = fixture("basketball");
        root["events"][0]["season"] = json!({"year":2026,"type":2});
        let mut c = coverage("basketball", &league);
        c.push(Coverage {
            id: 2,
            scope: "team".into(),
            teams: BTreeSet::from(["not-followed".into()]),
            ..c[0].clone()
        });
        let day = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        let r = parse(&root, "basketball", &league, day, &c).unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0]["sources"], json!([1]));
        c[1].teams = BTreeSet::from([r.rows[0]["homeTeamId"].as_str().unwrap().to_owned()]);
        assert_eq!(
            parse(&root, "basketball", &league, day, &c).unwrap().rows[0]["sources"],
            json!([1, 2])
        );
        c.remove(0);
        c[0].season = Some(2025);
        assert!(
            parse(&root, "basketball", &league, day, &c)
                .unwrap()
                .rows
                .is_empty()
        );
        c[0].season = None;
        root["events"][0]["season"]["type"] = json!(1);
        assert!(
            parse(&root, "basketball", &league, day, &c)
                .unwrap()
                .rows
                .is_empty()
        );
    }
    #[test]
    fn partial_empty_wrong_league_duplicates_and_invalid_precision() {
        let (league, mut root) = fixture("soccer");
        let valid = root["events"][0].clone();
        root["events"] = json!([valid.clone(),{"id":"bad"}]);
        let p = sample(&root, "soccer", &league);
        assert_eq!(p.issues, 1);
        assert_eq!(p.rows.len(), 1);
        root["events"] = json!([valid.clone(), valid]);
        assert!(
            parse(
                &root,
                "soccer",
                &league,
                NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
                &coverage("soccer", &league)
            )
            .is_err()
        );
        root["events"] = json!([]);
        assert_eq!(sample(&root, "soccer", &league).event_count, 0);
        root["leagues"] = json!([]);
        assert!(
            parse(
                &root,
                "soccer",
                &league,
                NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
                &[]
            )
            .is_err()
        );
        assert!(score(&json!("3.1234")).is_err());
        assert_eq!(score(&json!("3.5")).unwrap(), Some(3.5));
    }
    #[test]
    fn live_winner_is_not_inferred_and_sport_details_survive() {
        for sport in ["soccer", "hockey", "volleyball", "baseball"] {
            let (league, mut root) = fixture(sport);
            let c = &mut root["events"][0]["competitions"][0];
            c["status"] = json!({"type":{"name":"STATUS_IN_PROGRESS","state":"in","completed":false,"detail":"Overtime"},"period":4});
            for t in c["competitors"].as_array_mut().unwrap() {
                t["score"] = json!(1);
                t["shootoutScore"] = json!(3);
                t["winner"] = json!(true);
                t["linescores"] = json!([{"value":25,"period":1}]);
            }
            let r = &sample(&root, sport, &league).rows[0];
            assert!(r["winnerSide"].is_null());
            assert_eq!(r["homeShootout"], 3);
            assert_eq!(r["liveDetails"]["homePeriods"][0]["score"], 25.0);
            assert_eq!(r["statusText"], "Overtime");
        }
    }
}
