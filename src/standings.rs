//! Complete, provider-owned standings snapshots; independent of schedules/results.
use crate::Result;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use url::Url;

#[derive(Clone, Debug)]
pub struct Source {
    pub id: i32,
    pub provider: String,
    pub sport: String,
    pub url: String,
    pub minimum_teams: usize,
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub group_id: String,
    pub group_name: String,
    pub team_id: String,
    pub team_name: String,
    pub logo_url: Option<String>,
    pub team_url: Option<String>,
    /// Order in the published table. This is not necessarily a league-wide rank.
    pub position: i32,
    pub rank: Option<i32>,
    pub stats: Value,
    pub note: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub season: String,
    pub season_type: String,
    pub title: String,
    pub source_url: String,
    pub entries: Vec<Entry>,
}
impl Snapshot {
    pub fn validate(&self, minimum: usize) -> Result<()> {
        if minimum == 0 || self.entries.len() < minimum || self.entries.len() > 256 {
            return Err(format!(
                "incomplete standings: {} teams; minimum {minimum}",
                self.entries.len()
            )
            .into());
        }
        let mut ids = BTreeSet::new();
        for row in &self.entries {
            if row.team_id.is_empty()
                || row.team_name.is_empty()
                || row.group_id.is_empty()
                || row.position <= 0
                || !ids.insert((&row.group_id, &row.team_id))
                || !row.stats.as_array().is_some_and(|s| !s.is_empty())
            {
                return Err("invalid/duplicate standings team or missing statistics".into());
            }
        }
        for (s, max) in [
            (&self.season, 80),
            (&self.season_type, 80),
            (&self.title, 300),
            (&self.source_url, 2048),
        ] {
            check_text(s, max)?;
        }
        for r in &self.entries {
            for (s, max) in [
                (&r.group_id, 80),
                (&r.group_name, 200),
                (&r.team_id, 80),
                (&r.team_name, 200),
            ] {
                check_text(s, max)?;
            }
            for s in [&r.logo_url, &r.team_url].into_iter().flatten() {
                check_text(s, 2048)?;
            }
            if let Some(s) = &r.note {
                check_text(s, 1000)?;
            }
        }
        Ok(())
    }
    pub fn rows_json(&self) -> String {
        Value::Array(
            self.entries
                .iter()
                .map(|r| {
                    json!({
                        "groupId":r.group_id,"groupName":r.group_name,"teamId":r.team_id,
                        "teamName":r.team_name,"logoUrl":r.logo_url,"teamUrl":r.team_url,
                        "position":r.position,"rank":r.rank,"stats":r.stats,"note":r.note
                    })
                })
                .collect(),
        )
        .to_string()
    }
}
fn check_text(s: &str, max: usize) -> Result<()> {
    if s.is_empty() || s.encode_utf16().count() > max {
        return Err("standings text missing or exceeds database limit".into());
    }
    Ok(())
}
fn link(raw: Option<&str>, base: &str) -> Option<String> {
    let url = Url::parse(base).ok()?.join(raw?).ok()?;
    crate::http::public_url(url.as_str())
        .ok()
        .map(|u| u.to_string())
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("standings missing {key}").into())
}
pub fn parse_espn(text: &str, url: &str) -> Result<Snapshot> {
    let root: Value = serde_json::from_str(text)?;
    let year = root["season"]["year"]
        .as_i64()
        .ok_or("standings season missing")?;
    let mut out = Snapshot {
        season: year.to_string(),
        season_type: String::new(),
        title: string(&root, "name")?.to_owned(),
        source_url: url.to_owned(),
        entries: vec![],
    };
    fn walk(node: &Value, out: &mut Snapshot, year: i64) -> Result<()> {
        if let Some(table) = node.get("standings") {
            if table["season"].as_i64() != Some(year) {
                return Err("mixed standings seasons".into());
            }
            let season_type = table["seasonType"]
                .as_i64()
                .ok_or("standings season type missing")?
                .to_string();
            if !out.season_type.is_empty() && out.season_type != season_type {
                return Err("mixed standings season types".into());
            }
            out.season_type = season_type;
            // Identity uses the numeric year, independent of changing display labels.
            out.title = string(table, "seasonDisplayName")?.to_owned();
            let group_id = string(node, "id")?;
            let group_name = string(node, "name")?;
            let entries = table["entries"]
                .as_array()
                .filter(|s| !s.is_empty())
                .ok_or("empty standings group")?;
            for (i, row) in entries.iter().enumerate() {
                let team = &row["team"];
                let id = string(team, "id")?;
                if id.parse::<u64>().ok().is_none_or(|n| n == 0) {
                    return Err("invalid ESPN standings team ID".into());
                }
                let stats = row["stats"]
                    .as_array()
                    .filter(|s| !s.is_empty())
                    .ok_or("standings statistics missing")?;
                for stat in stats {
                    string(stat, "name")?;
                    if !stat["displayValue"].is_string() {
                        return Err("standings stat display value missing".into());
                    }
                }
                for required in ["wins", "losses"] {
                    if !stats.iter().any(|stat| {
                        stat["name"] == required
                            && stat["value"]
                                .as_f64()
                                .is_some_and(|v| v.is_finite() && v >= 0.0)
                    }) {
                        return Err(format!("standings missing {required} count").into());
                    }
                }
                let rank = stats
                    .iter()
                    .find(|s| s["name"] == "rank")
                    .and_then(|s| s["value"].as_f64())
                    .filter(|n| *n > 0.0 && n.fract() == 0.0 && *n <= 256.0)
                    .map(|n| n as i32);
                let team_link = team["links"]
                    .as_array()
                    .and_then(|a| {
                        a.iter().find(|v| {
                            v["rel"]
                                .as_array()
                                .is_some_and(|r| r.iter().any(|x| x == "clubhouse"))
                        })
                    })
                    .and_then(|v| v["href"].as_str());
                out.entries.push(Entry {
                    group_id: group_id.to_owned(),
                    group_name: group_name.to_owned(),
                    team_id: id.to_owned(),
                    team_name: string(team, "displayName")?.to_owned(),
                    logo_url: link(team["logos"][0]["href"].as_str(), &out.source_url),
                    team_url: link(team_link, &out.source_url),
                    position: (i + 1) as i32,
                    rank,
                    stats: Value::Array(
                        stats
                            .iter()
                            .map(|s| {
                                let mut s = s.clone();
                                if s["displayName"].as_str().is_none_or(|n| n.is_empty()) {
                                    s["displayName"] = s["name"].clone();
                                }
                                s
                            })
                            .collect(),
                    ),
                    note: row["note"]["description"].as_str().map(str::to_owned),
                });
            }
        }
        if let Some(children) = node["children"].as_array() {
            for child in children {
                walk(child, out, year)?;
            }
        }
        Ok(())
    }
    walk(&root, &mut out, year)?;
    let request = Url::parse(url)?;
    if let Some((_, requested)) = request.query_pairs().find(|(k, _)| k == "season")
        && requested.parse::<i64>()? != year
    {
        return Err("ESPN returned a different season than requested".into());
    }
    out.validate(1)?;
    Ok(out)
}
fn text(el: scraper::ElementRef<'_>) -> String {
    el.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn parse_big12(html: &str, url: &str, sport: &str) -> Result<Snapshot> {
    let document = Html::parse_document(html);
    let selector = |s| Selector::parse(s).unwrap();
    let table = document
        .select(&selector("table.sidearm-standings-table"))
        .next()
        .ok_or("Big 12 standings table missing")?;
    let title = table
        .select(&selector("caption"))
        .next()
        .map(text)
        .ok_or("standings caption missing")?;
    let season = title
        .split_whitespace()
        .next()
        .ok_or("standings season missing")?
        .to_owned();
    if season.len() < 4
        || season
            .get(..4)
            .and_then(|s| s.parse::<u16>().ok())
            .is_none()
        || !title.contains(sport)
    {
        return Err("unexpected Big 12 standings season/sport".into());
    }
    let members = crate::big12::members(html)?;
    let headers = table
        .select(&selector("thead th"))
        .filter(|h| h.value().attr("aria-hidden") != Some("true"))
        .map(text)
        .collect::<Vec<_>>();
    let mut entries = vec![];
    for row in table.select(&selector("tbody tr")) {
        let cells = row
            .select(&selector("td"))
            .filter(|c| c.value().attr("aria-hidden") != Some("true"))
            .collect::<Vec<_>>();
        if cells.len() != headers.len() || cells.len() < 3 {
            return Err("unexpected Big 12 standings columns".into());
        }
        let anchor = cells[0]
            .select(&selector("a"))
            .next()
            .ok_or("Big 12 standings school link missing")?;
        let name = text(anchor);
        let member = members
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| format!("unrecognized Big 12 school {name}"))?;
        let stats=cells.iter().zip(&headers).enumerate().skip(1).map(|(i,(c,h))|json!({"name":format!("column{i}"),"displayName":h,"displayValue":text(*c)})).collect::<Vec<_>>();
        let image = row
            .select(&selector("img"))
            .next()
            .and_then(|i| i.value().attr("src"));
        entries.push(Entry {
            group_id: "big12".into(),
            group_name: "Big 12".into(),
            team_id: member.id.to_string(),
            team_name: member.name.clone(),
            logo_url: link(image, url),
            team_url: link(anchor.value().attr("href"), url),
            position: entries.len() as i32 + 1,
            rank: None,
            stats: json!(stats),
            note: None,
        });
    }
    let out = Snapshot {
        season,
        season_type: "Conference standings".into(),
        title,
        source_url: url.to_owned(),
        entries,
    };
    out.validate(1)?;
    Ok(out)
}
/// Resolve the current soccer link advertised by the conference. Its bare path
/// currently fails; the menu advertises a working explicit standings ID.
fn current_soccer_url(html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("script").unwrap();
    for script in document.select(&selector) {
        let body = script.text().collect::<String>();
        for part in body.split("var component =").skip(1) {
            if let Some(Ok(v)) = serde_json::Deserializer::from_str(part.trim_start())
                .into_iter::<Value>()
                .next()
            {
                fn find(v: &Value) -> Option<String> {
                    if let Some(s) = v.as_str() {
                        let u = Url::parse("https://big12sports.com/").ok()?.join(s).ok()?;
                        if u.host_str() == Some("big12sports.com")
                            && u.path() == "/standings.aspx"
                            && u.query_pairs().any(|(k, v)| k == "path" && v == "soc")
                            && u.query_pairs().any(|(k, _)| k == "standings")
                        {
                            return Some(u.to_string());
                        }
                    }
                    match v {
                        Value::Array(a) => a.iter().find_map(find),
                        Value::Object(o) => o.values().find_map(find),
                        _ => None,
                    }
                }
                if let Some(u) = find(&v) {
                    return Ok(u);
                }
            }
        }
    }
    Err("current Big 12 soccer standings link missing".into())
}
fn validate_nhl(snapshot: &Snapshot) -> Result<()> {
    let ids = snapshot
        .entries
        .iter()
        .map(|e| &e.team_id)
        .collect::<BTreeSet<_>>();
    if snapshot.entries.len() != 32 || ids.len() != 32 {
        return Err("NHL standings require 32 distinct teams".into());
    }
    for entry in &snapshot.entries {
        let stats = entry
            .stats
            .as_array()
            .ok_or("NHL standings statistics missing")?;
        for required in ["points", "gamesPlayed", "otLosses"] {
            if !stats.iter().any(|s| {
                s["name"] == required
                    && s["value"]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && n >= 0.0 && n.fract() == 0.0)
            }) {
                return Err(format!("NHL standings missing {required}").into());
            }
        }
    }
    Ok(())
}
pub fn fetch(client: &Client, source: &Source) -> Result<Snapshot> {
    let mut url = crate::http::public_url(&source.url)?.to_string();
    let body = if source.provider == "big12"
        && source.sport == "Soccer"
        && !Url::parse(&url)?
            .query_pairs()
            .any(|(k, _)| k == "standings")
    {
        let menu = crate::http::get_text(
            client,
            "https://big12sports.com/standings.aspx?path=football",
        )?;
        url = current_soccer_url(&menu)?;
        crate::http::get_text(client, &url)?
    } else {
        crate::http::get_text(client, &url)?
    };
    let out = match source.provider.as_str() {
        "espn-mlb" | "espn-wnba" | "espn-nwsl" | "espn-mls" | "espn-nfl" | "espn-championship"
        | "espn-epl" | "espn-nhl" => {
            let expected = match source.provider.as_str() {
                "espn-nhl" => "l:90",
                "espn-mlb" => "l:10",
                "espn-wnba" => "l:59",
                "espn-mls" => "l:770",
                "espn-nfl" => "l:28",
                "espn-championship" => "l:3914",
                "espn-epl" => "l:700",
                _ => "l:8301",
            };
            let root: Value = serde_json::from_str(&body)?;
            if !root["uid"]
                .as_str()
                .is_some_and(|uid| uid.split('~').any(|part| part == expected))
            {
                return Err("wrong ESPN league in standings response".into());
            }
            let snapshot = parse_espn(&body, &url)?;
            if source.provider == "espn-nhl" {
                validate_nhl(&snapshot)?;
            }
            snapshot
        }
        "big12" => parse_big12(&body, &url, &source.sport)?,
        _ => return Err("unsupported standings provider".into()),
    };
    out.validate(source.minimum_teams)?;
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    #[test]
    fn nhl_divisions_keep_points_overtime_and_all_32_teams() {
        let body = include_str!("../tests/fixtures/standings/nhl.json");
        let snapshot = super::parse_espn(
            body,
            "https://site.api.espn.com/apis/v2/sports/hockey/nhl/standings?level=3",
        )
        .unwrap();
        snapshot.validate(32).unwrap();
        super::validate_nhl(&snapshot).unwrap();
        assert_eq!(snapshot.season, "2027");
        assert_eq!(
            snapshot
                .entries
                .iter()
                .map(|e| &e.group_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        let mut partial = snapshot.clone();
        partial.entries.pop();
        assert!(super::validate_nhl(&partial).is_err());
        let mut duplicate = snapshot.clone();
        duplicate.entries[1].team_id = duplicate.entries[0].team_id.clone();
        assert!(super::validate_nhl(&duplicate).is_err());
        let mut missing = snapshot;
        missing.entries[0]
            .stats
            .as_array_mut()
            .unwrap()
            .retain(|s| s["name"] != "otLosses");
        assert!(super::validate_nhl(&missing).is_err());
    }

    use super::*;
    pub fn professional(provider: &str) -> Snapshot {
        let (data, path) = match provider {
            "mlb" => (
                include_str!("../tests/fixtures/standings/mlb.json"),
                "baseball/mlb",
            ),
            "wnba" => (
                include_str!("../tests/fixtures/standings/wnba.json"),
                "basketball/wnba",
            ),
            "epl" => (
                include_str!("../tests/fixtures/standings/epl.json"),
                "soccer/eng.1",
            ),
            "championship" => (
                include_str!("../tests/fixtures/standings/championship.json"),
                "soccer/eng.2",
            ),
            "nfl" => (
                include_str!("../tests/fixtures/standings/nfl.json"),
                "football/nfl",
            ),
            "mls" => (
                include_str!("../tests/fixtures/standings/mls.json"),
                "soccer/usa.1",
            ),
            _ => (
                include_str!("../tests/fixtures/standings/nwsl.json"),
                "soccer/usa.nwsl",
            ),
        };
        parse_espn(
            data,
            &format!("https://site.api.espn.com/apis/v2/sports/{path}/standings?season=2026"),
        )
        .unwrap()
    }
    #[test]
    fn nfl_divisions_preserve_all_teams_and_provider_order() {
        let s = parse_espn(
            include_str!("../tests/fixtures/standings/nfl-divisions.json"),
            "https://site.api.espn.com/apis/v2/sports/football/nfl/standings?level=3",
        )
        .unwrap();
        s.validate(32).unwrap();
        assert_eq!(s.entries.len(), 32);
        assert_eq!(
            s.entries
                .iter()
                .map(|r| &r.team_id)
                .collect::<BTreeSet<_>>()
                .len(),
            32
        );
        for name in [
            "AFC East",
            "AFC North",
            "AFC South",
            "AFC West",
            "NFC East",
            "NFC North",
            "NFC South",
            "NFC West",
        ] {
            let teams = s
                .entries
                .iter()
                .filter(|r| r.group_name == name)
                .collect::<Vec<_>>();
            assert_eq!(teams.len(), 4, "{name}");
            assert_eq!(
                teams.iter().map(|r| r.position).collect::<Vec<_>>(),
                vec![1, 2, 3, 4]
            );
        }
        assert_eq!(
            s.entries
                .iter()
                .find(|r| r.team_name == "Kansas City Chiefs")
                .unwrap()
                .group_name,
            "AFC West"
        );
    }
    #[test]
    fn full_professional_tables_keep_groups_and_provider_stats() {
        for (provider, count, groups) in [
            ("mlb", 30, 2),
            ("wnba", 15, 2),
            ("nwsl", 16, 1),
            ("mls", 30, 2),
            ("nfl", 32, 2),
            ("championship", 24, 1),
            ("epl", 20, 1),
        ] {
            let s = professional(provider);
            s.validate(count).unwrap();
            assert_eq!(s.entries.len(), count);
            assert_eq!(s.season, "2026");
            assert_eq!(
                s.entries
                    .iter()
                    .map(|r| &r.group_id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                groups
            );
            assert!(
                s.entries
                    .iter()
                    .all(|r| r.logo_url.is_some() && r.team_url.is_some())
            );
            assert!(
                s.entries[0]
                    .stats
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["name"] == "wins")
            );
        }
        let nwsl = professional("nwsl");
        assert!(nwsl.entries.iter().any(|r| r.team_id == "20907"));
        assert!(nwsl.entries.iter().any(|r| r.rank == Some(1)));
        assert!(
            nwsl.entries[0]
                .stats
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["name"] == "points")
        );
    }
    #[test]
    fn all_five_big12_tables_have_stable_member_ids_and_published_seasons() {
        for (html, sport, count, season) in [
            (
                include_str!("../tests/fixtures/standings/fb.html"),
                "Football",
                16,
                "2026",
            ),
            (
                include_str!("../tests/fixtures/standings/mb.html"),
                "Men's Basketball",
                16,
                "2025-26",
            ),
            (
                include_str!("../tests/fixtures/standings/wb.html"),
                "Women's Basketball",
                16,
                "2025-26",
            ),
            (
                include_str!("../tests/fixtures/standings/vb.html"),
                "Volleyball",
                15,
                "2026",
            ),
            (
                include_str!("../tests/fixtures/standings/soc.html"),
                "Soccer",
                16,
                "2026",
            ),
        ] {
            let s = parse_big12(html, "https://big12sports.com/standings.aspx", sport).unwrap();
            s.validate(count).unwrap();
            assert_eq!(s.entries.len(), count);
            assert_eq!(s.season, season);
            assert!(
                s.entries
                    .iter()
                    .all(|r| r.logo_url.is_some() && r.rank.is_none())
            );
            assert_eq!(
                s.entries[0].stats[0]["displayValue"]
                    .as_str()
                    .unwrap()
                    .split('-')
                    .count(),
                if sport == "Soccer" { 3 } else { 2 }
            );
            assert!(
                s.entries[0]
                    .stats
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["displayName"].as_str().unwrap().contains("Overall"))
            );
        }
    }
    #[test]
    fn incomplete_duplicate_or_oversized_snapshots_are_rejected() {
        let mut s = professional("mlb");
        s.entries.truncate(15);
        assert!(s.validate(30).is_err());
        s.entries.push(s.entries[0].clone());
        assert!(s.validate(1).is_err());
        let mut s = professional("nwsl");
        s.entries[0].team_name = "x".repeat(201);
        assert!(s.validate(1).is_err());
        s.entries.clear();
        assert!(s.validate(1).is_err());
    }
    #[test]
    fn malformed_or_wrong_seasons_and_sports_are_rejected() {
        let data = include_str!("../tests/fixtures/standings/nwsl.json");
        assert!(parse_espn(data, "https://site.api.espn.com/standings?season=2027").is_err());
        let mut v: Value = serde_json::from_str(data).unwrap();
        v["children"][0]["standings"]["season"] = json!(2025);
        assert!(parse_espn(&v.to_string(), "https://site.api.espn.com/").is_err());
        assert!(
            parse_big12(
                include_str!("../tests/fixtures/standings/fb.html"),
                "https://big12sports.com/",
                "Soccer"
            )
            .is_err()
        );
        assert!(
            parse_big12(
                "<html>maintenance</html>",
                "https://big12sports.com/",
                "Football"
            )
            .is_err()
        );
    }
    #[test]
    fn unknown_members_are_rejected_and_links_are_safe() {
        let html = include_str!("../tests/fixtures/standings/fb.html")
            .replace(">BYU</a>", ">Unknown school</a>");
        assert!(parse_big12(&html, "https://big12sports.com/", "Football").is_err());
        assert!(link(Some("javascript:alert(1)"), "https://big12sports.com/").is_none());
        assert!(link(Some("http://127.0.0.1/"), "https://big12sports.com/").is_none());
        assert!(
            link(Some("/logo.png"), "https://big12sports.com/")
                .unwrap()
                .starts_with("https://big12sports.com/")
        );
    }
    #[test]
    fn soccer_link_tracks_the_advertised_season() {
        let html = r#"<script>var component = {"type":"navigation","data":[{"url":"/standings.aspx?path=soc&standings=999"}]};</script>"#;
        assert_eq!(
            current_soccer_url(html).unwrap(),
            "https://big12sports.com/standings.aspx?path=soc&standings=999"
        );
        assert!(current_soccer_url("<html></html>").is_err());
    }
}
