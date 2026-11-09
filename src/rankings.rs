//! National polls, independent of standings/schedules. ESPN transports the named polls.
use crate::Result;
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct Source {
    pub id: i32,
    pub sport: String,
    pub league_id: String,
    pub poll_id: String,
    pub url: String,
}
#[derive(Clone, Debug)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub rank: i32,
    pub previous: Option<i32>,
    pub previously_unranked: bool,
    pub points: Option<f64>,
    pub first_place_votes: Option<i32>,
    pub record: Option<String>,
    pub logo: Option<String>,
    pub url: Option<String>,
    pub big12_school_id: Option<i32>,
}
#[derive(Clone, Debug)]
pub struct Poll {
    pub season: i32,
    pub season_label: String,
    pub poll_id: String,
    pub name: String,
    pub published: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub period: Option<String>,
    pub phase: Option<String>,
    pub source_url: String,
    pub teams: Vec<Team>,
}
fn bounded(s: &str, max: usize) -> Result<()> {
    if s.is_empty() || s.encode_utf16().count() > max {
        return Err("ranking text missing or exceeds schema limit".into());
    }
    Ok(())
}
impl Poll {
    pub fn validate(&self) -> Result<()> {
        if self.teams.len() < 25 || self.teams.len() > 50 || !self.teams.iter().any(|t| t.rank == 1)
        {
            return Err("incomplete national Top 25".into());
        }
        let mut ids = BTreeSet::new();
        if !(1900..=9999).contains(&self.season) {
            return Err("invalid poll season".into());
        }
        for (s, n) in [
            (&self.season_label, 80),
            (&self.poll_id, 40),
            (&self.name, 200),
            (&self.source_url, 2048),
        ] {
            bounded(s, n)?;
        }
        for p in [&self.period, &self.phase].into_iter().flatten() {
            bounded(p, 100)?;
        }
        for t in &self.teams {
            bounded(&t.id, 80)?;
            bounded(&t.name, 200)?;
            if !ids.insert(&t.id)
                || !(1..=25).contains(&t.rank)
                || t.previous.is_some_and(|r| !(1..=25).contains(&r))
                || t.points.is_some_and(|p| !p.is_finite() || p < 0.0)
                || t.first_place_votes.is_some_and(|v| v < 0)
                || t.big12_school_id.is_some_and(|id| id <= 0)
            {
                return Err("invalid/duplicate national ranking team".into());
            }
            for s in [&t.logo, &t.url].into_iter().flatten() {
                bounded(s, 2048)?;
            }
            if let Some(r) = &t.record {
                bounded(r, 100)?;
            }
        }
        Ok(())
    }
    pub fn rows_json(&self) -> String {
        json!(self.teams.iter().map(|t|json!({"teamId":t.id,"teamName":t.name,"rank":t.rank,"previousRank":t.previous,"previouslyUnranked":t.previously_unranked,"points":t.points,"firstPlaceVotes":t.first_place_votes,"record":t.record,"logoUrl":t.logo,"teamUrl":t.url,"big12SchoolId":t.big12_school_id})).collect::<Vec<_>>()).to_string()
    }
}
fn time(v: &Value) -> Result<DateTime<Utc>> {
    let raw = v.as_str().ok_or("poll timestamp missing")?;
    if let Ok(t) = DateTime::parse_from_rfc3339(raw) {
        return Ok(t.with_timezone(&Utc));
    }
    Ok(chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%MZ")?.and_utc())
}
fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("poll missing {key}").into())
}
fn normalized(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn safe_link(raw: Option<&str>) -> Option<String> {
    crate::http::public_url(raw?).ok().map(|u| u.to_string())
}
pub fn parse(text: &str, source: &Source, members: &[crate::big12::Member]) -> Result<Poll> {
    if members.is_empty() {
        return Err("Big 12 membership missing; retaining previous poll".into());
    }
    let root: Value = serde_json::from_str(text)?;
    if !root["leagues"]
        .as_array()
        .is_some_and(|l| l.iter().any(|v| v["id"] == source.league_id))
    {
        return Err("wrong league for national poll".into());
    }
    let polls = root["rankings"]
        .as_array()
        .ok_or("national rankings array missing")?;
    let matched = polls
        .iter()
        .filter(|p| p["id"] == source.poll_id)
        .collect::<Vec<_>>();
    if matched.len() != 1 {
        return Err("configured national poll missing or duplicated".into());
    }
    let p = matched[0];
    let mut out = Poll {
        season: i32::try_from(p["season"]["year"].as_i64().ok_or("poll season missing")?)?,
        season_label: required(&p["season"], "displayName")?.to_owned(),
        poll_id: source.poll_id.clone(),
        name: required(p, "name")?.to_owned(),
        published: time(&p["date"])?,
        updated: p
            .get("lastUpdated")
            .filter(|v| !v.is_null())
            .map(time)
            .transpose()?,
        period: p["occurrence"]["displayValue"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        phase: p["season"]["type"]["name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        source_url: source.url.clone(),
        teams: vec![],
    };
    for row in p["ranks"].as_array().ok_or("ranked teams missing")? {
        let t = &row["team"];
        let id = required(t, "id")?;
        if id.parse::<u64>().ok().is_none_or(|n| n == 0) {
            return Err("invalid ESPN ranking team ID".into());
        }
        let school = required(t, "location")?;
        let name = t["displayName"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(school);
        // Exact normalized official school name, never numeric cross-provider ID matching.
        let big12 = members
            .iter()
            .find(|m| normalized(&m.name) == normalized(school))
            .map(|m| m.id);
        let previous = row
            .get("previous")
            .filter(|v| !v.is_null())
            .map(|v| v.as_i64().ok_or("invalid previous rank"))
            .transpose()?;
        if previous.is_some_and(|p| !(0..=25).contains(&p)) {
            return Err("invalid previous Top 25 rank".into());
        }
        let votes = row
            .get("firstPlaceVotes")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_i64()
                    .ok_or("invalid first-place votes")
                    .and_then(|v| i32::try_from(v).map_err(|_| "votes exceed integer limit"))
            })
            .transpose()?;
        let points = row
            .get("points")
            .filter(|v| !v.is_null())
            .map(|v| v.as_f64().ok_or("invalid poll points"))
            .transpose()?;
        let link = t["links"]
            .as_array()
            .and_then(|a| {
                a.iter().find(|l| {
                    l["rel"]
                        .as_array()
                        .is_some_and(|r| r.iter().any(|v| v == "clubhouse"))
                })
            })
            .and_then(|l| l["href"].as_str());
        out.teams.push(Team {
            id: id.to_owned(),
            name: name.to_owned(),
            rank: i32::try_from(row["current"].as_i64().ok_or("current poll rank missing")?)?,
            previous: previous.filter(|p| *p > 0).map(|p| p as i32),
            previously_unranked: previous == Some(0),
            points,
            first_place_votes: votes,
            record: row["recordSummary"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            logo: safe_link(
                t["logo"]
                    .as_str()
                    .or_else(|| t["logos"][0]["href"].as_str()),
            ),
            url: safe_link(link),
            big12_school_id: big12,
        });
    }
    out.teams.sort_by_key(|t| t.rank);
    if let Some((_, year)) = url::Url::parse(&source.url)?
        .query_pairs()
        .find(|(k, _)| k == "season")
        && year.parse::<i32>()? != out.season
    {
        return Err("poll does not match requested season".into());
    }
    out.validate()?;
    Ok(out)
}
pub fn fetch(client: &Client, source: &Source, members: &[crate::big12::Member]) -> Result<Poll> {
    let url = crate::http::public_url(&source.url)?;
    let body = crate::http::response_text_limit(
        client.get(url).send()?.error_for_status()?,
        crate::http::JSON_LIMIT,
    )?;
    parse(&body, source, members)
}

/// Official conference member metadata is also published on its standings page.
pub fn current_members(client: &Client) -> Result<Vec<crate::big12::Member>> {
    let html = crate::http::get_text(
        client,
        "https://big12sports.com/standings.aspx?path=football",
    )?;
    crate::big12::members(&html)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn members() -> Vec<crate::big12::Member> {
        let v: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/rankings/big12-members.json"
        ))
        .unwrap();
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| crate::big12::Member {
                id: v["id"].as_i64().unwrap() as i32,
                name: v["name"].as_str().unwrap().into(),
            })
            .collect()
    }
    pub fn fixture(key: &str) -> (&'static str, Source) {
        let (data, sport, league, poll, path) = match key {
            "football" => (
                include_str!("../tests/fixtures/rankings/college-football.json"),
                "Football",
                "23",
                "1",
                "football/college-football",
            ),
            "men" => (
                include_str!("../tests/fixtures/rankings/mens-college-basketball.json"),
                "Men's Basketball",
                "41",
                "1",
                "basketball/mens-college-basketball",
            ),
            "women" => (
                include_str!("../tests/fixtures/rankings/womens-college-basketball.json"),
                "Women's Basketball",
                "54",
                "1",
                "basketball/womens-college-basketball",
            ),
            "soccer" => (
                include_str!("../tests/fixtures/rankings/usa.ncaa.w.1.json"),
                "Soccer",
                "5499",
                "86",
                "soccer/usa.ncaa.w.1",
            ),
            _ => (
                include_str!("../tests/fixtures/rankings/womens-college-volleyball.json"),
                "Volleyball",
                "402",
                "89",
                "volleyball/womens-college-volleyball",
            ),
        };
        (
            data,
            Source {
                id: 1,
                sport: sport.into(),
                league_id: league.into(),
                poll_id: poll.into(),
                url: format!("https://site.api.espn.com/apis/site/v2/sports/{path}/rankings"),
            },
        )
    }
    #[test]
    fn all_five_polls_keep_full_national_rankings_and_real_dates() {
        for key in ["football", "men", "women", "soccer", "volleyball"] {
            let (data, source) = fixture(key);
            let p = parse(data, &source, &members()).unwrap();
            assert_eq!(p.teams.len(), 25);
            assert_eq!(p.teams[0].rank, 1);
            assert!(p.updated.is_some());
            assert!(p.teams.iter().any(|t| t.big12_school_id.is_some()));
            assert!(p.teams.iter().any(|t| t.big12_school_id.is_none()));
            if key == "men" || key == "women" {
                assert_eq!(p.season_label, "2025-26");
                assert_eq!(p.published.date_naive().month(), 4);
            }
        }
    }
    #[test]
    fn soccer_ids_are_not_resolved_via_other_sports_and_former_members_are_not_highlighted() {
        let (data, s) = fixture("soccer");
        let p = parse(data, &s, &members()).unwrap();
        let byu = p.teams.iter().find(|t| t.id == "20516").unwrap();
        assert_eq!(byu.big12_school_id, Some(32));
        let (data, s) = fixture("football");
        let p = parse(data, &s, &members()).unwrap();
        assert!(
            p.teams
                .iter()
                .find(|t| t.name.starts_with("Texas") && t.id == "251")
                .unwrap()
                .big12_school_id
                .is_none()
        );
    }
    #[test]
    fn missing_points_stay_missing_and_unranked_previous_is_distinct_from_unknown() {
        let (data, s) = fixture("soccer");
        let mut v: Value = serde_json::from_str(data).unwrap();
        v["rankings"][0]["ranks"][0]["previous"] = json!(0);
        v["rankings"][0]["ranks"][1]
            .as_object_mut()
            .unwrap()
            .remove("previous");
        let p = parse(&v.to_string(), &s, &members()).unwrap();
        assert!(p.teams[0].points.is_none());
        assert!(p.teams[0].previously_unranked);
        assert!(!p.teams[1].previously_unranked);
        assert!(p.teams[1].previous.is_none());
    }
    #[test]
    fn ties_are_valid_but_missing_and_duplicate_teams_are_not() {
        let (data, s) = fixture("volleyball");
        let p = parse(data, &s, &members()).unwrap();
        let mut tied = p.clone();
        tied.teams[24].rank = 24;
        tied.validate().unwrap();
        let mut bad = p.clone();
        bad.teams.pop();
        assert!(bad.validate().is_err());
        let mut bad = p.clone();
        bad.teams[24].id = bad.teams[0].id.clone();
        assert!(bad.validate().is_err());
        let mut bad = p;
        bad.teams[0].rank = 0;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn wrong_league_poll_or_season_and_missing_membership_are_rejected() {
        let (data, mut s) = fixture("football");
        assert!(parse(data, &s, &[]).is_err());
        s.league_id = "41".into();
        assert!(parse(data, &s, &members()).is_err());
        s.league_id = "23".into();
        s.poll_id = "wrong".into();
        assert!(parse(data, &s, &members()).is_err());
        s.poll_id = "1".into();
        s.url.push_str("?season=2027");
        assert!(parse(data, &s, &members()).is_err());
    }
    #[test]
    fn untrusted_links_and_invalid_timestamp_do_not_become_poll_data() {
        let (data, s) = fixture("football");
        let mut v: Value = serde_json::from_str(data).unwrap();
        v["rankings"][0]["ranks"][0]["team"]["logo"] = json!("javascript:alert(1)");
        let p = parse(&v.to_string(), &s, &members()).unwrap();
        assert!(p.teams[0].logo.is_none());
        v["rankings"][0]["date"] = json!("TBD");
        assert!(parse(&v.to_string(), &s, &members()).is_err());
    }
    use chrono::Datelike;
}
