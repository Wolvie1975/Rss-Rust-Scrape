//! Big 12 observations retain feed perspective while explicit provider aliases identify one game.
use crate::{
    db::EventFeed,
    events::{self, Event},
};
use chrono::{Datelike, NaiveDate};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const BASE: &str = "https://big12sports.com/";
pub const SPORTS: [(i32, &str); 5] = [
    (4, "Football"),
    (7, "Men's Basketball"),
    (15, "Women's Basketball"),
    (31, "Soccer"),
    (37, "Volleyball"),
];
#[derive(Debug, Clone)]
pub struct Member {
    pub id: i32,
    pub name: String,
}
pub struct FeedResult {
    pub id: i32,
    pub school: String,
    pub observations: Vec<Value>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

pub fn is_provider_feed(raw: &str) -> bool {
    url::Url::parse(raw).ok().is_some_and(|u| {
        u.host_str() == Some("big12sports.com")
            && u.path() == "/services/responsive-calendar-subscription.ashx/calendar.rss"
    })
}
pub fn school_id(raw: &str) -> Option<i32> {
    let u = url::Url::parse(raw).ok()?;
    if u.host_str()? != "big12sports.com"
        || u.path() != "/services/responsive-calendar-subscription.ashx/calendar.rss"
    {
        return None;
    }
    u.query_pairs()
        .find(|(k, _)| k == "school_id")
        .and_then(|(_, v)| v.parse().ok())
        .filter(|id| *id > 0)
}
pub fn feed_url(id: i32) -> String {
    format!(
        "{BASE}services/responsive-calendar-subscription.ashx/calendar.rss?sport_id=0&school_id={id}&schedule_id=0"
    )
}
fn fetch(client: &Client, url: &str) -> Result<String> {
    Ok(client.get(url).send()?.error_for_status()?.text()?)
}
pub fn members(html: &str) -> Result<Vec<Member>> {
    let doc = Html::parse_document(html);
    let selector = Selector::parse("script").unwrap();
    for script in doc.select(&selector) {
        let text = script.text().collect::<String>();
        for part in text.split("var component =").skip(1) {
            let Some(Ok(c)) = serde_json::Deserializer::from_str(part.trim_start())
                .into_iter::<Value>()
                .next()
            else {
                continue;
            };
            if c["type"] != "members" {
                continue;
            }
            let mut out = Vec::new();
            let mut ids = BTreeSet::new();
            for r in c["data"].as_array().ok_or("member list missing")? {
                if r["school_active"] != true || r["member_type"] != "F" {
                    continue;
                }
                let id = i32::try_from(r["id"].as_i64().ok_or("member ID missing")?)?;
                let name = r["title"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or("member name missing")?
                    .to_owned();
                if id <= 0 || !ids.insert(id) {
                    return Err("invalid or duplicate member ID".into());
                }
                out.push(Member { id, name });
            }
            if out.is_empty() {
                return Err("no current full members found".into());
            }
            return Ok(out);
        }
    }
    Err("calendar members metadata missing".into())
}
pub fn current_members(client: &Client) -> Result<Vec<Member>> {
    members(&fetch(client, &format!("{BASE}calendar.aspx"))?)
}
fn string(v: &Value) -> Option<String> {
    v.as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
fn id(v: &Value) -> Option<i32> {
    v.as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .filter(|n| *n > 0)
}
fn media_url(v: &Value) -> Option<String> {
    string(&v["url"])
        .and_then(|u| url::Url::parse(BASE).ok()?.join(&u).ok())
        .filter(|u| matches!(u.scheme(), "https" | "http"))
        .map(|u| u.to_string())
}
pub fn event_json(e: &Event) -> Value {
    json!({"url":e.url,"game_id":e.game_id,"title":e.title,"sport":e.sport,"opponent":e.opponent,"is_away":e.is_away,"location":e.location,"event_date":e.event_date.to_string(),"starts_at":e.starts_at.map(|d|d.to_rfc3339()),"ends_at":e.ends_at.map(|d|d.to_rfc3339()),"time_tbd":e.time_tbd,"tv":e.tv,"stream_url":e.stream_url,"live_stats_url":e.live_stats_url,"team_logo_url":e.team_logo_url,"opponent_logo_url":e.opponent_logo_url})
}

pub fn enrich(
    mut e: Event,
    school: &Member,
    catalog: &[Member],
    metadata: Option<&Value>,
) -> Value {
    let game = e.game_id;
    let mut issue = None;
    let mut warning = None;
    let mut partner = None;
    let mut opponent_id = None;
    let mut neutral = None;
    let mut timezone = None;
    let mut radio = None;
    let mut audio = None;
    let sport_id = SPORTS
        .iter()
        .find(|(_, name)| e.sport.as_deref() == Some(*name))
        .map(|(id, _)| *id);
    let fail = |s: &str| Some(s.to_owned());
    if game.is_none() {
        issue = fail("missing provider game ID");
    }
    if let Some(m) = metadata {
        let direct = id(&m["id"]) == game;
        let (own, opp) = if direct {
            (&m["school"], &m["opponent"])
        } else {
            (&m["opponent"], &m["school"])
        };
        opponent_id = id(&opp["id"]);
        partner = if direct {
            id(&m["opp_game_id"])
        } else {
            id(&m["id"])
        };
        if id(&own["id"]) != Some(school.id) || id(&m["sport"]["id"]) != sport_id {
            issue = fail("metadata school/sport disagrees with RSS");
        }
        if string(&opp["title"]).as_deref() != e.opponent.as_deref() {
            issue = fail("metadata opponent disagrees with RSS");
        }
        if opponent_id == Some(school.id) || e.opponent.as_deref() == Some(&school.name) {
            issue = fail("self-opponent provider record");
        }
        if partner == game && partner.is_some() {
            issue = fail("self-referencing game alias");
        }
        if issue.is_none()
            && opponent_id.is_some_and(|i| catalog.iter().any(|s| s.id == i))
            && partner.is_none()
        {
            issue = fail("member opponent has no verified game alias");
        }
        let indicator = m["location_indicator"].as_str();
        neutral = match indicator {
            Some("N") => Some(true),
            Some("A" | "H") => Some(false),
            _ => None,
        };
        e.is_away = match (indicator, direct) {
            (Some("N"), _) => None,
            (Some("A"), true) | (Some("H"), false) => Some(true),
            (Some("H"), true) | (Some("A"), false) => Some(false),
            _ => None,
        };
        timezone = string(&m["timezone"]);
        if let (Some(start), Some(utc)) = (e.starts_at, m["date_utc"].as_str()) {
            if start.format("%Y-%m-%dT%H:%M:%S").to_string() != utc.get(..19).unwrap_or(utc) {
                warning = fail("RSS/JSON UTC disagreement; RSS timing retained");
            }
        }
        let media = &m["media"];
        e.tv = e.tv.or_else(|| string(&media["tv"]));
        e.stream_url = e.stream_url.or_else(|| media_url(&media["video"]));
        e.live_stats_url = e.live_stats_url.or_else(|| media_url(&media["stats"]));
        radio = string(&media["radio"]);
        audio = media_url(&media["audio"]);
    } else {
        issue = fail("provider identity metadata unavailable");
    }
    let canonical = game.map(|g| match partner {
        Some(p) => format!("big12:pair:{}:{}", g.min(p), g.max(p)),
        None => format!("big12:game:{g}"),
    });
    json!({"provider_key":"big12","provider_game_id":game,"paired_game_id":partner,"canonical_key":canonical,"school_id":school.id,"school_name":school.name,"opponent_school_id":opponent_id,"sport_id":sport_id,"is_neutral":neutral,"calendar_timezone":timezone,"radio":radio,"audio_url":audio,"issue":issue,"warning":warning,"event":event_json(&e),"metadata":metadata})
}

pub fn collect(client: &Client, feeds: &[EventFeed]) -> Result<Vec<FeedResult>> {
    let html = fetch(client, &format!("{BASE}calendar.aspx"))?;
    let catalog = members(&html)?;
    let mut batches = Vec::new();
    let mut months = BTreeSet::new();
    for f in feeds {
        let sid = school_id(&f.url).ok_or("Big 12 feed has no school ID")?;
        let Some(school) = catalog.iter().find(|m| m.id == sid) else {
            batches.push((
                f.id,
                Member {
                    id: sid,
                    name: f.school.clone().unwrap_or_default(),
                },
                Err::<Vec<Event>, String>("school is not a current full member".into()),
            ));
            continue;
        };
        let events = fetch(client, &f.url)
            .and_then(|xml| events::parse_feed(&xml, &f.url, Some(&school.name), Some(&html)))
            .map(|b| {
                b.events
                    .into_iter()
                    .filter(|e| SPORTS.iter().any(|(_, s)| e.sport.as_deref() == Some(*s)))
                    .collect::<Vec<_>>()
            })
            .map_err(|e| e.to_string());
        if let Ok(events) = &events {
            for e in events {
                months.insert((e.event_date.year(), e.event_date.month()));
            }
        }
        batches.push((f.id, school.clone(), events));
    }
    let mut metadata = BTreeMap::new();
    let mut metadata_errors = Vec::new();
    for (year, month) in months {
        let start = NaiveDate::from_ymd_opt(year, month, 1).ok_or("invalid calendar month")?;
        let next = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or("invalid next month")?;
        let end = next.pred_opt().ok_or("invalid month end")?;
        let mut url = url::Url::parse(&format!("{BASE}services/responsive-calendar.ashx"))?;
        url.query_pairs_mut()
            .append_pair("start", &start.to_string())
            .append_pair("end", &format!("{end} 23:59:59"))
            .append_pair("sport_id", "0")
            .append_pair("school_id", "0");
        match fetch(client, url.as_str()).and_then(|s| Ok(serde_json::from_str::<Vec<Value>>(&s)?))
        {
            Ok(rows) => {
                for row in rows {
                    if let Some(g) = id(&row["id"]) {
                        metadata.insert(g, row.clone());
                    }
                    if let Some(g) = id(&row["opp_game_id"]) {
                        metadata.insert(g, row);
                    }
                }
            }
            Err(e) => metadata_errors.push(format!("{start}: metadata fetch failed: {e}")),
        }
    }
    let mut results: Vec<FeedResult> = batches
        .into_iter()
        .map(|(id, school, events)| match events {
            Err(error) => FeedResult {
                id,
                school: school.name,
                observations: Vec::new(),
                error: Some(error),
                warnings: Vec::new(),
            },
            Ok(events) => {
                let observations: Vec<_> = events
                    .into_iter()
                    .map(|e| {
                        let m = e.game_id.and_then(|g| metadata.get(&g));
                        enrich(e, &school, &catalog, m)
                    })
                    .collect();
                let mut warnings = metadata_errors.clone();
                for o in &observations {
                    if let Some(w) = o["warning"].as_str() {
                        warnings.push(format!("{}: {w}", o["provider_game_id"]));
                    }
                }
                FeedResult {
                    id,
                    school: school.name,
                    observations,
                    error: None,
                    warnings,
                }
            }
        })
        .collect();
    let mut timings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &results {
        for o in &r.observations {
            if o["issue"].is_null() {
                if let Some(key) = o["canonical_key"].as_str() {
                    timings.entry(key.to_owned()).or_default().insert(format!(
                        "{}|{}|{}",
                        o["event"]["event_date"], o["event"]["starts_at"], o["event"]["time_tbd"]
                    ));
                }
            }
        }
    }
    for r in &mut results {
        for o in &mut r.observations {
            if o["canonical_key"]
                .as_str()
                .is_some_and(|key| timings.get(key).is_some_and(|s| s.len() > 1))
            {
                let message = "paired RSS timing disagreement; primary source retained";
                let previous = o["warning"].as_str().unwrap_or("");
                o["warning"] = json!(if previous.is_empty() {
                    message.to_owned()
                } else {
                    format!("{previous}; {message}")
                });
                r.warnings
                    .push(format!("{}: {message}", o["provider_game_id"]));
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixture(school_id: i32) -> Value {
        let metadata: Vec<Value> =
            serde_json::from_str(include_str!("../tests/fixtures/big12-game-aliases.json"))
                .unwrap();
        let catalog = members(include_str!("../tests/fixtures/big12-members.html")).unwrap();
        let school = catalog.iter().find(|s| s.id == school_id).unwrap();
        let xml = if school_id == 3 {
            include_str!("../tests/fixtures/big12-kansas-calendar.xml")
        } else {
            include_str!("../tests/fixtures/big12-utah-calendar.xml")
        };
        let e = events::parse_feed(
            xml,
            &feed_url(school_id),
            Some(&school.name),
            Some(include_str!("../tests/fixtures/big12-members.html")),
        )
        .unwrap()
        .events
        .into_iter()
        .next()
        .unwrap();
        let m = metadata.iter().find(|m| id(&m["id"]) == e.game_id).unwrap();
        enrich(e, school, &catalog, Some(m))
    }
    #[test]
    fn mirrored_ids_share_identity_but_keep_perspective_and_rss_time() {
        let kansas = fixture(3);
        let utah = fixture(36);
        assert_eq!(kansas["canonical_key"], utah["canonical_key"]);
        assert_eq!(kansas["canonical_key"], "big12:pair:179871:179872");
        assert_eq!(kansas["event"]["is_away"], true);
        assert_eq!(utah["event"]["is_away"], false);
        assert_eq!(kansas["event"]["opponent"], "Utah");
        assert_eq!(utah["event"]["opponent"], "Kansas");
        assert_eq!(kansas["event"]["starts_at"], "2026-10-11T02:15:00+00:00");
        assert!(kansas["warning"].is_string());
        assert!(kansas["issue"].is_null());
        assert!(
            kansas["event"]["team_logo_url"]
                .as_str()
                .unwrap()
                .contains("jhwk")
        );
    }
    #[test]
    fn neutral_tbd_does_not_invent_a_home_side_or_time() {
        let catalog = members(include_str!("../tests/fixtures/big12-members.html")).unwrap();
        let school = catalog.iter().find(|s| s.id == 3).unwrap();
        let mut e = events::parse_feed(
            include_str!("../tests/fixtures/big12-kansas-calendar.xml"),
            &feed_url(3),
            Some("Kansas"),
            None,
        )
        .unwrap()
        .events
        .remove(0);
        let mut m = fixture(3)["metadata"].clone();
        m["location_indicator"] = json!("N");
        e.starts_at = None;
        e.ends_at = None;
        e.time_tbd = true;
        let o = enrich(e, school, &catalog, Some(&m));
        assert_eq!(o["is_neutral"], true);
        assert!(o["event"]["is_away"].is_null());
        assert!(o["event"]["starts_at"].is_null());
        assert_eq!(o["event"]["time_tbd"], true);
    }
    #[test]
    fn missing_metadata_and_self_opponents_are_quarantined() {
        let catalog = members(include_str!("../tests/fixtures/big12-members.html")).unwrap();
        let school = catalog.iter().find(|s| s.id == 3).unwrap();
        let e = events::parse_feed(
            include_str!("../tests/fixtures/big12-kansas-calendar.xml"),
            &feed_url(3),
            Some("Kansas"),
            None,
        )
        .unwrap()
        .events
        .remove(0);
        assert!(enrich(e.clone(), school, &catalog, None)["issue"].is_string());
        let mut m = fixture(3)["metadata"].clone();
        m["opponent"] = m["school"].clone();
        let mut e = e;
        e.opponent = Some("Kansas".into());
        assert!(enrich(e, school, &catalog, Some(&m))["issue"].is_string());
    }
    #[test]
    fn doubleheaders_do_not_merge_by_teams_and_date() {
        let catalog = members(include_str!("../tests/fixtures/big12-members.html")).unwrap();
        let school = catalog.iter().find(|s| s.id == 3).unwrap();
        let e = events::parse_feed(
            include_str!("../tests/fixtures/big12-kansas-calendar.xml"),
            &feed_url(3),
            Some("Kansas"),
            None,
        )
        .unwrap()
        .events
        .remove(0);
        let mut m = fixture(3)["metadata"].clone();
        let mut a = e.clone();
        a.game_id = Some(9001);
        m["id"] = json!(9001);
        m["opp_game_id"] = json!(9002);
        let a = enrich(a, school, &catalog, Some(&m));
        let mut b = e;
        b.game_id = Some(9003);
        m["id"] = json!(9003);
        m["opp_game_id"] = json!(9004);
        let b = enrich(b, school, &catalog, Some(&m));
        assert_ne!(a["canonical_key"], b["canonical_key"]);
        assert!(a["issue"].is_null());
        assert!(b["issue"].is_null());
    }
    #[test]
    fn provider_members_exclude_affiliates_and_feed_order_is_irrelevant() {
        let catalog = members(include_str!("../tests/fixtures/big12-members.html")).unwrap();
        assert_eq!(catalog.len(), 16);
        assert!(catalog.iter().any(|s| s.name == "Oklahoma State"));
        assert_eq!(
            school_id(
                "https://big12sports.com/services/responsive-calendar-subscription.ashx/calendar.rss?school_id=3&schedule_id=0&sport_id=0"
            ),
            Some(3)
        );
        assert_eq!(
            school_id(
                "https://unrelated.test/services/responsive-calendar-subscription.ashx/calendar.rss?school_id=3"
            ),
            None
        );
    }
}
