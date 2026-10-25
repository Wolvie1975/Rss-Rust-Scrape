//! Shared ESPN final-score parsing for season-wide collectors.
use crate::Result;
use crate::espn_schedule::{self, Fixture, League};
use chrono::Utc;
use serde_json::{Value, json};
pub fn parse(
    root: &Value,
    season: i32,
    league: League,
    league_id: &str,
) -> Result<(Vec<Fixture>, Vec<Value>)> {
    if !root["leagues"]
        .as_array()
        .is_some_and(|ls| ls.iter().any(|l| l["id"] == league_id))
    {
        return Err("wrong match league response".into());
    }
    let fixtures = espn_schedule::parse(root, league, None)?;
    // ESPN NHL preseason responses can retain the previous season in the league
    // header. Each event season below is authoritative and must still match.
    if league != League::Nhl
        && !fixtures.is_empty()
        && root["leagues"][0]["season"]["year"].as_i64() != Some(i64::from(season))
    {
        return Err("wrong match season response".into());
    }
    if root["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["season"]["year"].as_i64() != Some(i64::from(season)))
    {
        return Err("mixed match event seasons".into());
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
            .ok_or("match result missing event")?;
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
                .ok_or_else(|| "final score missing or invalid".into())
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
        if league == League::Nhl && score(home)? == score(away)? {
            return Err("final NHL score cannot be tied".into());
        }
        for (value, limit) in [
            (fixture.team_name.as_str(), 200),
            (
                fixture
                    .event
                    .opponent
                    .as_deref()
                    .ok_or("result opponent missing")?,
                200,
            ),
            (fixture.status.as_str(), 100),
            (fixture.event.url.as_str(), 2048),
        ]
        .into_iter()
        .chain(
            [
                fixture.event.team_logo_url.as_deref(),
                fixture.event.opponent_logo_url.as_deref(),
            ]
            .into_iter()
            .flatten()
            .map(|s| (s, 2048)),
        ) {
            if value.encode_utf16().count() > limit {
                return Err("match result exceeds SQL text limit".into());
            }
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
