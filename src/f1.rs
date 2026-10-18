//! Jolpica seasonal F1 snapshots. Pagination counts classifications, not races.
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Duration};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const BASE: &str = "https://api.jolpi.ca/ergast/f1/";

#[derive(Debug)]
pub struct Snapshot {
    pub season: i32,
    pub standings_round: i32,
    pub races: Vec<Value>,
    pub results: Vec<Value>,
    pub sprint: Vec<Value>,
    pub drivers: Vec<Value>,
    pub constructors: Vec<Value>,
}
impl Snapshot {
    pub fn payload(&self) -> String {
        json!({"races":self.races,"results":self.results,"sprint":self.sprint,
            "drivers":self.drivers,"constructors":self.constructors})
        .to_string()
    }
    pub fn summary(&self) -> String {
        format!(
            "F1 {}: {} races, {} Grand Prix classifications, {} sprint classifications, {} drivers, {} constructors; standings round {}",
            self.season,
            self.races.len(),
            self.results.len(),
            self.sprint.len(),
            self.drivers.len(),
            self.constructors.len(),
            self.standings_round
        )
    }
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("F1 missing {k}").into())
}
fn integer(v: &Value, k: &str) -> Result<i32> {
    let n: i32 = text(v, k)?.parse()?;
    if n < 0 {
        return Err(format!("F1 negative {k}").into());
    }
    Ok(n)
}
fn metadata(v: &Value) -> Result<(usize, usize, usize)> {
    if v["series"] != "f1" {
        return Err("not an F1 response".into());
    }
    let total = usize::try_from(integer(v, "total")?)?;
    let offset = usize::try_from(integer(v, "offset")?)?;
    let limit = usize::try_from(integer(v, "limit")?)?;
    if total > 5000 || limit == 0 || limit > 100 {
        return Err("F1 pagination exceeds bounds".into());
    }
    Ok((total, offset, limit))
}
/// Merge result pages at classification level: a page can split one race.
fn page_items(data: &Value, kind: &str, season: i32) -> Result<(Vec<Value>, i32)> {
    if matches!(kind, "drivers" | "constructors") {
        let table = &data["StandingsTable"];
        if integer(table, "season")? != season {
            return Err("F1 standings season mismatch".into());
        }
        let lists = table["StandingsLists"]
            .as_array()
            .ok_or("missing standings lists")?;
        if lists.is_empty() {
            return Ok((vec![], 0));
        }
        if lists.len() != 1 || integer(&lists[0], "season")? != season {
            return Err("ambiguous standings snapshot".into());
        }
        let round = integer(&lists[0], "round")?;
        let key = if kind == "drivers" {
            "DriverStandings"
        } else {
            "ConstructorStandings"
        };
        return Ok((
            lists[0][key]
                .as_array()
                .ok_or("missing F1 standings")?
                .clone(),
            round,
        ));
    }
    let table = &data["RaceTable"];
    if integer(table, "season")? != season {
        return Err("F1 race season mismatch".into());
    }
    let mut out = vec![];
    for race in table["Races"].as_array().ok_or("missing F1 races")? {
        if integer(race, "season")? != season || integer(race, "round")? == 0 {
            return Err("invalid F1 race identity".into());
        }
        if kind == "schedule" {
            out.push(race.clone());
            continue;
        }
        let key = if kind == "results" {
            "Results"
        } else {
            "SprintResults"
        };
        for result in race[key].as_array().ok_or("missing race classifications")? {
            let mut row = result.clone();
            row["round"] = race["round"].clone();
            out.push(row);
        }
    }
    Ok((out, 0))
}
fn fetch_pages(client: &Client, path: &str, kind: &str, season: i32) -> Result<(Vec<Value>, i32)> {
    let mut offset = 0;
    let mut expected_total = None;
    let mut expected_round = None;
    let mut out = vec![];
    loop {
        // Less than the provider's four requests/second burst limit, no retry storm.
        std::thread::sleep(Duration::from_millis(350));
        let url = format!("{BASE}{path}?limit=100&offset={offset}");
        let response = client
            .get(crate::http::public_url(&url)?)
            .header(reqwest::header::USER_AGENT, "My-Newsfeed/0.1.0")
            .send()?
            .error_for_status()?;
        let body = crate::http::response_text_limit(response, crate::http::JSON_LIMIT)?;
        let root: Value = serde_json::from_str(&body)?;
        let data = &root["MRData"];
        let (total, returned_offset, limit) = metadata(data)?;
        if returned_offset != offset || expected_total.is_some_and(|n| n != total) {
            return Err("F1 pagination changed during collection".into());
        }
        expected_total = Some(total);
        let (rows, round) = page_items(data, kind, season)?;
        if expected_round.is_some_and(|n| n != round)
            || rows.len() > limit
            || rows.len() > total.saturating_sub(offset)
        {
            return Err("F1 inconsistent page".into());
        }
        expected_round = Some(round);
        let count = rows.len();
        out.extend(rows);
        if out.len() == total {
            break;
        }
        if count == 0 {
            return Err("F1 truncated pagination".into());
        }
        offset += count;
    }
    Ok((out, expected_round.unwrap_or(0)))
}
fn identity(row: &Value, key: &str, field: &str) -> Result<String> {
    Ok(text(&row[key], field)?.to_owned())
}
fn validate_rows(rows: &[Value], kind: &str, rounds: &BTreeSet<i32>) -> Result<()> {
    let mut ids = BTreeSet::new();
    let mut positions = BTreeSet::new();
    for row in rows {
        let position = integer(row, "position")?;
        let points: f64 = text(row, "points")?.parse()?;
        if position == 0 || !points.is_finite() || points < 0.0 {
            return Err("invalid F1 position/points".into());
        }
        let round = if matches!(kind, "results" | "sprint") {
            integer(row, "round")?
        } else {
            0
        };
        let id = if kind == "constructors" {
            text(&row["Constructor"], "name")?;
            identity(row, "Constructor", "constructorId")?
        } else {
            text(&row["Driver"], "givenName")?;
            text(&row["Driver"], "familyName")?;
            identity(row, "Driver", "driverId")?
        };
        if !ids.insert((round, id)) || !positions.insert((round, position)) {
            return Err("duplicate F1 classification".into());
        }
        if round > 0 {
            if !rounds.contains(&round) {
                return Err("F1 results refer to unknown race".into());
            }
            integer(row, "grid")?;
            integer(row, "laps")?;
            text(row, "status")?;
            identity(row, "Constructor", "constructorId")?;
            text(&row["Constructor"], "name")?;
        } else {
            integer(row, "wins")?;
            if kind == "drivers" {
                let teams = row["Constructors"]
                    .as_array()
                    .filter(|a| !a.is_empty())
                    .ok_or("missing driver constructors")?;
                for team in teams {
                    text(team, "constructorId")?;
                    text(team, "name")?;
                }
            }
        }
    }
    Ok(())
}
pub fn validate(s: &Snapshot) -> Result<()> {
    if !(1950..=2200).contains(&s.season) || s.races.is_empty() || s.races.len() > 40 {
        return Err("invalid F1 season/calendar".into());
    }
    let mut rounds = BTreeSet::new();
    for race in &s.races {
        let round = integer(race, "round")?;
        if integer(race, "season")? != s.season || round == 0 || !rounds.insert(round) {
            return Err("duplicate/wrong F1 season or round".into());
        }
        text(race, "raceName")?;
        chrono::NaiveDate::parse_from_str(text(race, "date")?, "%Y-%m-%d")?;
        text(&race["Circuit"], "circuitId")?;
        text(&race["Circuit"], "circuitName")?;
        for key in [
            "Race",
            "FirstPractice",
            "SecondPractice",
            "ThirdPractice",
            "Qualifying",
            "Sprint",
            "SprintQualifying",
            "SprintShootout",
        ] {
            let session = if key == "Race" { race } else { &race[key] };
            if session.is_null() {
                continue;
            }
            let date = text(session, "date")?;
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
            if let Some(time) = session["time"].as_str() {
                chrono::DateTime::parse_from_rfc3339(&format!("{date}T{time}"))?;
            }
        }
    }
    if s.standings_round < 0 || (s.standings_round > 0 && !rounds.contains(&s.standings_round)) {
        return Err("invalid F1 standings round".into());
    }
    for (rows, kind) in [
        (&s.results, "results"),
        (&s.sprint, "sprint"),
        (&s.drivers, "drivers"),
        (&s.constructors, "constructors"),
    ] {
        validate_rows(rows, kind, &rounds)?;
    }
    if s.drivers.is_empty() != s.constructors.is_empty()
        || (!s.results.is_empty() && (s.drivers.is_empty() || s.standings_round == 0))
    {
        return Err("missing F1 championship standings".into());
    }
    Ok(())
}
pub fn fetch(client: &Client) -> Result<Snapshot> {
    // Resolve current once, then pin every paginated request to that season.
    let response = client
        .get(format!("{BASE}current/?limit=1"))
        .header(reqwest::header::USER_AGENT, "My-Newsfeed/0.1.0")
        .send()?
        .error_for_status()?;
    let body = crate::http::response_text_limit(response, crate::http::JSON_LIMIT)?;
    let root: Value = serde_json::from_str(&body)?;
    metadata(&root["MRData"])?;
    let season = integer(&root["MRData"]["RaceTable"], "season")?;
    let (races, _) = fetch_pages(client, &format!("{season}/"), "schedule", season)?;
    let (results, _) = fetch_pages(client, &format!("{season}/results/"), "results", season)?;
    let (sprint, _) = fetch_pages(client, &format!("{season}/sprint/"), "sprint", season)?;
    let (drivers, driver_round) = fetch_pages(
        client,
        &format!("{season}/driverstandings/"),
        "drivers",
        season,
    )?;
    let (constructors, constructor_round) = fetch_pages(
        client,
        &format!("{season}/constructorstandings/"),
        "constructors",
        season,
    )?;
    if driver_round != constructor_round {
        return Err("F1 championship rounds disagree; retaining previous snapshot".into());
    }
    let s = Snapshot {
        season,
        standings_round: driver_round,
        races,
        results,
        sprint,
        drivers,
        constructors,
    };
    validate(&s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> Value {
        serde_json::from_str(match name {
            "schedule" => include_str!("../tests/fixtures/f1/schedule.json"),
            "results" => include_str!("../tests/fixtures/f1/results.json"),
            "sprint" => include_str!("../tests/fixtures/f1/sprint.json"),
            "drivers" => include_str!("../tests/fixtures/f1/drivers.json"),
            _ => include_str!("../tests/fixtures/f1/constructors.json"),
        })
        .unwrap()
    }
    fn sample() -> Snapshot {
        let rows = |name| page_items(&fixture(name)["MRData"], name, 2026).unwrap().0;
        Snapshot {
            season: 2026,
            standings_round: 17,
            races: rows("schedule"),
            results: rows("results"),
            sprint: rows("sprint"),
            drivers: rows("drivers"),
            constructors: rows("constructors"),
        }
    }
    #[test]
    fn real_provider_fields_preserve_sprints_fractional_points_and_statuses() {
        let mut s = sample();
        validate(&s).unwrap();
        assert!(s.races.iter().any(|r| r["Sprint"].is_object()));
        assert!(s.results.iter().any(|r| r["status"] != "Finished"));
        s.drivers[0]["points"] = json!("12.5");
        validate(&s).unwrap();
        assert!(s.payload().contains("12.5"));
    }
    #[test]
    fn rejects_duplicate_wrong_season_missing_classification_and_bad_dates() {
        let mut s = sample();
        s.results.push(s.results[0].clone());
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.races[0]["season"] = json!("2025");
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.results[0]["Driver"] = Value::Null;
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.races[0]["date"] = json!("invalid");
        assert!(validate(&s).is_err());
        let mut s = sample();
        s.constructors.clear();
        assert!(validate(&s).is_err());
    }
    #[test]
    fn split_result_pages_flatten_without_losing_a_race() {
        let root = fixture("results");
        let data = &root["MRData"];
        let (rows, _) = page_items(data, "results", 2026).unwrap();
        let count: usize = data["RaceTable"]["Races"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["Results"].as_array().unwrap().len())
            .sum();
        assert_eq!(rows.len(), count);
        assert_eq!(count, 100);
        assert!(
            metadata(&json!({"series":"f1","total":"10001","offset":"0","limit":"100"})).is_err()
        );
        assert!(page_items(data, "results", 2025).is_err());
    }
    #[test]
    fn preseason_allows_empty_classifications_and_unknown_times() {
        let mut s = sample();
        s.results.clear();
        s.sprint.clear();
        s.drivers.clear();
        s.constructors.clear();
        s.standings_round = 0;
        s.races[0].as_object_mut().unwrap().remove("time");
        validate(&s).unwrap();
    }
}
