//! Shared ESPN schedule parsing; scores and results are deliberately ignored.
use crate::events::Event;
use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use chrono_tz::America::{Chicago, New_York};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::BTreeMap;
use url::Url;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum League {
    Nwsl,
    Wnba,
}
impl League {
    pub fn path(self) -> &'static str {
        match self {
            Self::Nwsl => "/apis/site/v2/sports/soccer/usa.nwsl/scoreboard",
            Self::Wnba => "/apis/site/v2/sports/basketball/wnba/scoreboard",
        }
    }
    pub fn provider_key(self) -> &'static str {
        match self {
            Self::Nwsl => "espn-nwsl",
            Self::Wnba => "espn-wnba",
        }
    }
    pub fn canonical_key(self, id: i32) -> String {
        format!(
            "espn:{}:{id}",
            match self {
                Self::Nwsl => "usa.nwsl",
                Self::Wnba => "wnba",
            }
        )
    }
    fn game_url(self, id: i32) -> String {
        match self {
            Self::Nwsl => format!("https://www.espn.com/soccer/match/_/gameId/{id}"),
            Self::Wnba => format!("https://www.espn.com/wnba/game/_/gameId/{id}"),
        }
    }
    fn sport(self) -> &'static str {
        match self {
            Self::Nwsl => "Soccer",
            Self::Wnba => "Basketball",
        }
    }
}
pub struct Fixture {
    pub league: League,
    pub team_name: String,
    pub team_id: i32,
    pub opponent_id: i32,
    pub event: Event,
    pub status: String,
    pub neutral: bool,
    pub completed: bool,
}

pub fn is_feed(raw: &str, league: League) -> bool {
    Url::parse(raw).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some("site.api.espn.com")
            && u.path() == league.path()
    })
}
pub fn fetch(client: &Client, raw: &str, league: League) -> Result<Vec<Fixture>> {
    fetch_from(
        client,
        raw,
        league,
        Utc::now().with_timezone(&Chicago).date_naive(),
    )
}

pub(crate) fn fetch_from(
    client: &Client,
    raw: &str,
    league: League,
    today: NaiveDate,
) -> Result<Vec<Fixture>> {
    let base = Url::parse(raw)?;
    let params: BTreeMap<_, _> = base
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let filter = params.get("team").map(String::as_str);
    if league == League::Nwsl && filter.is_some_and(|team| team != "20907") {
        return Err("NWSL schedule provider supports Kansas City Current (team=20907)".into());
    }
    if let Some(filter) = filter {
        if filter
            .split(',')
            .any(|id| id.parse::<u32>().map_or(true, |n| n == 0))
        {
            return Err("team must contain comma-separated positive ESPN team IDs".into());
        }
    }
    let current = today.year();
    let years = match params.get("season") {
        Some(s) => vec![s.parse::<i32>()?],
        None => vec![current, current + 1],
    };
    let mut games = BTreeMap::new();
    for year in years {
        for month in 1..=12 {
            if year < today.year() || (year == today.year() && month < today.month()) {
                continue;
            }
            let mut url = base.clone();
            url.set_query(None);
            url.query_pairs_mut()
                .append_pair("dates", &format!("{year:04}{month:02}"))
                .append_pair("limit", "1000");
            let body = client
                .get(url)
                .header(reqwest::header::USER_AGENT, "My-Newsfeed/1.0")
                .send()?
                .error_for_status()?
                .text()?;
            for f in parse(&serde_json::from_str(&body)?, league, filter)? {
                games.insert(f.event.game_id.unwrap(), f);
            }
        }
    }
    Ok(games
        .into_values()
        .filter(|f| is_upcoming(f, today))
        .collect())
}
// Retain explicit future cancellations/postponements for reconciliation, but never results.
fn is_upcoming(f: &Fixture, today: NaiveDate) -> bool {
    f.event.event_date >= today && !f.completed
}
fn text(v: &Value, key: &str) -> Option<String> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
}
fn link(v: &Value, relation: &str) -> Option<String> {
    v["links"].as_array()?.iter().find_map(|l| {
        let rel = l["rel"].as_array()?;
        let href = l["href"].as_str()?;
        (rel.iter().any(|r| r.as_str() == Some(relation)) && href.starts_with("https://"))
            .then(|| href.to_owned())
    })
}
pub fn parse(root: &Value, league: League, filter: Option<&str>) -> Result<Vec<Fixture>> {
    let events = root["events"]
        .as_array()
        .ok_or("ESPN response missing events array")?;
    if events.len() >= 1000 {
        return Err("ESPN monthly response may be truncated".into());
    }
    let mut out = Vec::new();
    for raw in events {
        let cs = raw["competitions"]
            .as_array()
            .ok_or("ESPN event missing competitions")?;
        let Some(c) = cs.first() else {
            return Err("ESPN event has no competition".into());
        };
        let competitors = c["competitors"]
            .as_array()
            .ok_or("ESPN competition missing competitors")?;
        let team = if league == League::Nwsl {
            Some("20907")
        } else {
            None
        };
        if let Some(filter) = filter {
            if !competitors.iter().any(|p| {
                p["team"]["id"]
                    .as_str()
                    .is_some_and(|id| filter.split(',').any(|wanted| wanted == id))
            }) {
                continue;
            }
        }
        let Some(own) = competitors.iter().find(|p| match team {
            Some(team) => p["team"]["id"].as_str() == Some(team),
            None => p["homeAway"].as_str() == Some("home"),
        }) else {
            if league == League::Nwsl {
                continue;
            } else {
                return Err("WNBA fixture missing home competitor".into());
            }
        };
        if competitors.len() != 2 {
            return Err("ESPN fixture requires two competitors".into());
        }
        let own_id = own["team"]["id"].as_str().ok_or("missing team ID")?;
        let opp = competitors
            .iter()
            .find(|p| p["team"]["id"].as_str().is_some_and(|id| id != own_id))
            .ok_or("missing opponent")?;
        let team_id = own_id.parse::<i32>()?;
        let opponent_id = opp["team"]["id"]
            .as_str()
            .ok_or("missing opponent team ID")?
            .parse::<i32>()?;
        let id = raw["id"]
            .as_str()
            .ok_or("missing ESPN event ID")?
            .parse::<i32>()?;
        let date = c["date"]
            .as_str()
            .or(raw["date"].as_str())
            .ok_or("missing fixture date")?;
        let instant = DateTime::parse_from_rfc3339(date)
            .map(|d| d.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%MZ").map(|d| d.and_utc())
            })?;
        let status = c["status"]["type"]["name"]
            .as_str()
            .or(raw["status"]["type"]["name"].as_str())
            .ok_or("missing fixture status")?
            .to_owned();
        let status_type = if c["status"]["type"].is_object() {
            &c["status"]["type"]
        } else {
            &raw["status"]["type"]
        };
        let reconciliation = matches!(
            status.as_str(),
            "STATUS_CANCELED"
                | "STATUS_CANCELLED"
                | "STATUS_POSTPONED"
                | "STATUS_DELAYED"
                | "STATUS_SUSPENDED"
        );
        let completed = !reconciliation
            && (status_type["completed"].as_bool().unwrap_or(false)
                || status_type["state"].as_str() == Some("post")
                || status.starts_with("STATUS_FINAL")
                || status == "STATUS_FULL_TIME");
        let neutral = c["neutralSite"].as_bool().unwrap_or(false);
        let away = match own["homeAway"].as_str() {
            Some("away") => Some(true),
            Some("home") => Some(false),
            _ => return Err("missing home/away designation".into()),
        };
        let time_valid = c["timeValid"].as_bool().or(raw["timeValid"].as_bool()) == Some(true);
        let tbd = !time_valid
            || matches!(
                status.as_str(),
                "STATUS_POSTPONED"
                    | "STATUS_CANCELED"
                    | "STATUS_CANCELLED"
                    | "STATUS_DELAYED"
                    | "STATUS_SUSPENDED"
            );
        let opponent = text(&opp["team"], "displayName").ok_or("missing opponent name")?;
        let name = text(&own["team"], "displayName").ok_or("missing team name")?;
        let tv = c["broadcasts"]
            .as_array()
            .map(|bs| {
                bs.iter()
                    .flat_map(|b| {
                        b["names"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty());
        let logo =
            |p: &Value| text(&p["team"], "logo").or_else(|| text(&p["team"]["logos"][0], "href"));
        // ESPN's unconfirmed dates use midnight Eastern (04:00Z in summer,
        // 05:00Z in winter). This is a date placeholder, not a Central-night kickoff.
        let eastern = instant.with_timezone(&New_York);
        let event_date =
            if !time_valid && eastern.hour() == 0 && eastern.minute() == 0 && eastern.second() == 0
            {
                eastern.date_naive()
            } else {
                instant.with_timezone(&Chicago).date_naive()
            };
        out.push(Fixture {
            league,
            team_name: name.clone(),
            team_id,
            opponent_id,
            status,
            neutral,
            completed,
            event: Event {
                // A canonical ID-based URL stays constant even if ESPN changes the slug or link type.
                url: league.game_url(id),
                game_id: Some(id),
                title: format!(
                    "{name} {} {opponent}",
                    if neutral {
                        "vs"
                    } else if away == Some(true) {
                        "at"
                    } else {
                        "vs"
                    }
                ),
                sport: Some(league.sport().into()),
                opponent: Some(opponent),
                is_away: if neutral { None } else { away },
                location: text(&c["venue"], "fullName")
                    .or_else(|| text(&raw["venue"], "displayName")),
                event_date,
                starts_at: if tbd { None } else { Some(instant) },
                ends_at: None,
                time_tbd: tbd,
                tv,
                stream_url: link(c, "watch").or_else(|| link(raw, "watch")),
                live_stats_url: link(raw, "summary"),
                team_logo_url: logo(own),
                opponent_logo_url: logo(opp),
            },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod upcoming_tests {
    use super::*;
    #[test]
    fn both_leagues_include_today_and_future_but_exclude_past_and_results() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        for (root, league) in [
            (crate::nwsl::tests::sample(), League::Nwsl),
            (crate::wnba::tests::sample(), League::Wnba),
        ] {
            let mut f = parse(&root, league, None).unwrap().remove(0);
            f.completed = false;
            f.status = "STATUS_SCHEDULED".into();
            f.event.event_date = today.pred_opt().unwrap();
            assert!(!is_upcoming(&f, today));
            f.event.event_date = today;
            assert!(is_upcoming(&f, today));
            f.event.event_date = today.succ_opt().unwrap();
            assert!(is_upcoming(&f, today));
            f.completed = true;
            assert!(!is_upcoming(&f, today));
            f.completed = false;
            f.status = "STATUS_CANCELED".into();
            assert!(is_upcoming(&f, today)); // Existing future games still receive cancellation updates.
        }
    }
    #[test]
    fn cutoff_uses_central_calendar_date_including_unknown_start_times() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let mut root = crate::nwsl::tests::sample();
        let c = &mut root["events"][1]["competitions"][0];
        c["date"] = "2026-10-09T00:30Z".into();
        c["timeValid"] = false.into();
        let f = parse(&root, League::Nwsl, None).unwrap().remove(1);
        assert!(!is_upcoming(&f, today));
        root["events"][1]["competitions"][0]["date"] = "2026-10-10T00:30Z".into();
        let f = parse(&root, League::Nwsl, None).unwrap().remove(1);
        assert!(is_upcoming(&f, today));
        assert!(f.event.starts_at.is_none());
    }
}

#[cfg(test)]
mod request_window_tests {
    use super::*;
    #[test]
    fn past_season_performs_no_network_requests() {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        for league in [League::Wnba, League::Nwsl] {
            assert!(
                fetch_from(
                    &client,
                    "http://127.0.0.1:1/scoreboard?season=2025",
                    league,
                    today
                )
                .unwrap()
                .is_empty()
            );
        }
    }
}

#[cfg(test)]
mod tbd_date_tests {
    use super::*;
    #[test]
    fn espn_midnight_eastern_placeholders_keep_advertised_date_and_cutoff() {
        for (utc, day) in [
            (
                "2026-10-09T04:00Z",
                NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
            ),
            (
                "2026-12-09T05:00Z",
                NaiveDate::from_ymd_opt(2026, 12, 9).unwrap(),
            ),
        ] {
            for (mut root, league) in [
                (crate::wnba::tests::sample(), League::Wnba),
                (crate::nwsl::tests::sample(), League::Nwsl),
            ] {
                let c = &mut root["events"][0]["competitions"][0];
                c["date"] = utc.into();
                c["timeValid"] = false.into();
                c["status"]["type"] =
                    serde_json::json!({"name":"STATUS_SCHEDULED","state":"pre","completed":false});
                let f = parse(&root, league, None).unwrap().remove(0);
                assert_eq!(f.event.event_date, day);
                assert!(is_upcoming(&f, day));
                assert!(f.event.starts_at.is_none());
                root["events"][0]["competitions"][0]["timeValid"] = true.into();
                let f = parse(&root, league, None).unwrap().remove(0);
                assert_eq!(f.event.event_date, day.pred_opt().unwrap());
                assert!(!is_upcoming(&f, day));
            }
        }
    }
}
