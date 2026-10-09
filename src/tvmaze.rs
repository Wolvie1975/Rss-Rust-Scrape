//! TVmaze original scheduling, not country-specific streaming availability.
use crate::db::Db;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[cfg(test)]
pub const SEEDS: [i32; 5] = [83073, 64950, 45039, 33352, 90632];
fn positive(v: &Value) -> Option<i32> {
    v.as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .filter(|n| *n > 0)
}
fn raw(v: &Value) -> Option<&str> {
    v.as_str()
}
fn date(v: &Value) -> std::result::Result<Option<NaiveDate>, String> {
    match raw(v).filter(|s| !s.is_empty()) {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| format!("invalid airdate: {s}")),
        None if v.is_null() || v == "" => Ok(None),
        None => Err("airdate must be a string or null".into()),
    }
}
fn https(v: &Value) -> Option<String> {
    let u = url::Url::parse(v.as_str()?).ok()?;
    (u.scheme() == "https" && u.username().is_empty() && u.password().is_none())
        .then(|| u.to_string())
}
fn source_url(v: &Value, kind: &str, id: i32) -> std::result::Result<String, String> {
    let u = url::Url::parse(v.as_str().ok_or("missing source URL")?).map_err(|e| e.to_string())?;
    if u.scheme() != "https"
        || u.host_str() != Some("www.tvmaze.com")
        || u.path().split('/').nth(1) != Some(kind)
        || u.path().split('/').nth(2) != Some(&id.to_string())
    {
        return Err("source URL identity disagrees with provider ID".into());
    }
    Ok(u.to_string())
}
fn number(v: &Value) -> std::result::Result<Option<i32>, String> {
    if v.is_null() {
        return Ok(None);
    }
    v.as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .filter(|n| *n >= 0)
        .map(Some)
        .ok_or_else(|| "invalid numeric episode field".into())
}
/// Validate the entire snapshot before any series/episode write.
pub fn normalize(
    show_id: i32,
    show: &Value,
    episodes: &Value,
) -> std::result::Result<(Value, Vec<String>), String> {
    if positive(&show["id"]) != Some(show_id) {
        return Err("show ID mismatch; no name-based fallback is allowed".into());
    }
    for key in [
        "id",
        "name",
        "url",
        "network",
        "webChannel",
        "status",
        "premiered",
        "image",
        "externals",
    ] {
        if show.get(key).is_none() {
            return Err(format!("incomplete show response: missing {key}"));
        }
    }
    let title = raw(&show["name"])
        .filter(|s| !s.trim().is_empty())
        .ok_or("show title missing")?;
    let premiere = date(&show["premiered"])?;
    let source = source_url(&show["url"], "shows", show_id)?;
    let rows = episodes
        .as_array()
        .ok_or("episode response is not a complete list")?;
    let mut ids = BTreeSet::new();
    let mut normalized = Vec::new();
    let mut warnings = Vec::new();
    for e in rows {
        for key in [
            "id", "url", "name", "season", "number", "type", "airdate", "airtime", "airstamp",
            "runtime", "image",
        ] {
            if e.get(key).is_none() {
                return Err(format!("incomplete episode response: missing {key}"));
            }
        }
        let id = positive(&e["id"]).ok_or("episode ID missing/invalid")?;
        if !ids.insert(id) {
            return Err("duplicate episode IDs in provider response".into());
        }
        if let Some(href) = e["_links"]["show"]["href"].as_str() {
            if href != format!("https://api.tvmaze.com/shows/{show_id}") {
                return Err("episode belongs to a different show".into());
            }
        }
        let airdate = date(&e["airdate"])?;
        let airtime = match raw(&e["airtime"]) {
            Some(s) if !s.trim().is_empty() => {
                Some(NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| "invalid episode airtime")?)
            }
            Some(_) | None if e["airtime"].is_null() || e["airtime"].is_string() => None,
            _ => return Err("airtime must be a string or null".into()),
        };
        let stamp = match raw(&e["airstamp"]).filter(|s| !s.is_empty()) {
            Some(s) => Some(
                DateTime::parse_from_rfc3339(s)
                    .map_err(|_| "invalid raw airstamp")?
                    .with_timezone(&Utc),
            ),
            None if e["airstamp"].is_null() || e["airstamp"] == "" => None,
            None => return Err("airstamp must be a string or null".into()),
        };
        let starts = if airdate.is_some() && airtime.is_some() {
            stamp
        } else {
            None
        };
        if airtime.is_some() && starts.is_none() {
            warnings.push(format!(
                "episode {id}: local airtime has no meaningful UTC stamp"
            ));
        }
        let kind = raw(&e["type"]).ok_or("episode type missing")?;
        normalized.push(json!({"id":id,"season":number(&e["season"] )?,"number":number(&e["number"] )?,"title":e["name"],"type":kind,"special":kind!="regular","airdate":airdate.map(|d|d.to_string()),"airtime":airtime.filter(|_|airdate.is_some()).map(|t|t.format("%H:%M:%S").to_string()),"starts_at_utc":starts.map(|d|d.to_rfc3339()),"date_only":airtime.is_none() || airdate.is_none(),"runtime":number(&e["runtime"] )?,"image_url":https(&e["image"]["original"]).or_else(||https(&e["image"]["medium"])),"source_url":source_url(&e["url"],"episodes",id)?,"raw_airdate":e["airdate"],"raw_airtime":e["airtime"],"raw_airstamp":e["airstamp"],"raw":e}));
    }
    Ok((
        json!({"show_id":show_id,"title":title,"network_id":positive(&show["network"]["id"]),"network_name":show["network"]["name"],"network_country":show["network"]["country"]["code"],"network_timezone":show["network"]["country"]["timezone"],"web_channel_id":positive(&show["webChannel"]["id"]),"web_channel_name":show["webChannel"]["name"],"web_channel_country":show["webChannel"]["country"]["code"],"web_channel_timezone":show["webChannel"]["country"]["timezone"],"status":show["status"],"premiere":premiere.map(|d|d.to_string()),"poster_url":https(&show["image"]["original"]).or_else(||https(&show["image"]["medium"])),"source_url":source,"imdb_id":show["externals"]["imdb"],"thetvdb_id":positive(&show["externals"]["thetvdb"]),"tvrage_id":positive(&show["externals"]["tvrage"]),"raw_show":show,"raw_episodes":episodes,"episodes":normalized,"issue_count":warnings.len()}),
        warnings,
    ))
}
#[derive(Debug)]
struct RequestError {
    message: String,
    retry_seconds: i32,
}
struct Api {
    base: String,
    pacing: Duration,
    client: Client,
    last: Option<Instant>,
}
impl Api {
    fn get(&mut self, path: &str) -> std::result::Result<Value, RequestError> {
        for attempt in 0..3 {
            if let Some(last) = self.last {
                let wait = self.pacing.saturating_sub(last.elapsed());
                std::thread::sleep(wait);
            }
            self.last = Some(Instant::now());
            let url = format!("{}{path}", self.base);
            let result = self.client.get(&url).send();
            let (message, delay) = match result {
                Ok(response) if response.status().is_success() => return response.json_value(),
                Ok(response) => {
                    let code = response.status();
                    let retry = retry_delay(
                        response
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|v| v.to_str().ok()),
                        attempt,
                    );
                    if !(code.as_u16() == 429 || code.as_u16() == 408 || code.is_server_error()) {
                        return Err(RequestError {
                            message: format!("TVmaze {code} for {path}"),
                            retry_seconds: 3600,
                        });
                    }
                    (format!("TVmaze {code} for {path}"), retry)
                }
                Err(e) => (
                    format!("TVmaze transport failure: {e}"),
                    2_i32.pow(attempt + 1),
                ),
            };
            if attempt == 2 || delay > 60 {
                return Err(RequestError {
                    message,
                    retry_seconds: delay.max(900),
                });
            }
            std::thread::sleep(Duration::from_secs(delay as u64));
        }
        unreachable!()
    }
}
trait JsonResponse {
    fn json_value(self) -> std::result::Result<Value, RequestError>;
}
impl JsonResponse for reqwest::blocking::Response {
    fn json_value(self) -> std::result::Result<Value, RequestError> {
        let text = self.text().map_err(|e| RequestError {
            message: e.to_string(),
            retry_seconds: 900,
        })?;
        serde_json::from_str(&text).map_err(|e| RequestError {
            message: format!("invalid JSON: {e}"),
            retry_seconds: 900,
        })
    }
}
pub fn retry_delay(header: Option<&str>, attempt: u32) -> i32 {
    header
        .and_then(|s| {
            s.parse::<i32>().ok().filter(|n| *n >= 0).or_else(|| {
                DateTime::parse_from_rfc2822(s).ok().map(|t| {
                    (t.with_timezone(&Utc) - Utc::now())
                        .num_seconds()
                        .max(1)
                        .min(i64::from(i32::MAX)) as i32
                })
            })
        })
        .unwrap_or_else(|| 2_i32.pow(attempt + 1))
        .max(1)
}
pub fn collect(db: &mut Db, dry_run: bool, force: bool) -> Result<Value> {
    let mut api = Api {
        base: "https://api.tvmaze.com".into(),
        pacing: Duration::from_millis(600),
        client: Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("MyNewsFeed-web_scraper/0.1 (TVmaze episode tracking)")
            .build()?,
        last: None,
    };
    let jobs = db.tv_jobs()?;
    let mut results = Vec::new();
    for (id, title) in jobs {
        let lease = if dry_run {
            Some("dry-run".to_owned())
        } else {
            db.claim_tv_series(id, force)?
        };
        let Some(lease) = lease else {
            results.push(json!({"show_id":id,"title":title,"status":"cached_or_leased"}));
            continue;
        };
        let result = (|| -> std::result::Result<(Value, Vec<String>), RequestError> {
            let show = api.get(&format!("/shows/{id}"))?;
            let episodes = api.get(&format!("/shows/{id}/episodes?specials=1"))?;
            normalize(id, &show, &episodes).map_err(|message| RequestError {
                message,
                retry_seconds: 900,
            })
        })();
        match result {
            Ok((payload, warnings)) => {
                let n = payload["episodes"].as_array().unwrap().len();
                let saved = if dry_run {
                    Ok("dry_run".to_owned())
                } else {
                    db.save_tv_snapshot(id, &lease, &payload)
                };
                match saved {
                    Ok(outcome) => {
                        eprintln!(
                            "TVmaze {id} {title}: {n} episodes, {} issues ({outcome})",
                            warnings.len()
                        );
                        results.push(json!({"show_id":id,"title":title,"status":outcome,"episodes":n,"warnings":warnings}));
                    }
                    Err(e) => {
                        let message = e.to_string();
                        db.fail_tv_series(id, &lease, &message, 900)?;
                        results.push(
                            json!({"show_id":id,"title":title,"status":"failed","error":message}),
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("TVmaze {id} {title}: {}", e.message);
                if !dry_run {
                    db.fail_tv_series(id, &lease, &e.message, e.retry_seconds)?;
                }
                results
                    .push(json!({"show_id":id,"title":title,"status":"failed","error":e.message}));
            }
        }
    }
    Ok(
        json!({"series":results,"schedule_scope":"original broadcast/release; not regional streaming availability"}),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixtures(id: i32) -> (Value, Value) {
        let (show, episodes) = match id {
            83073 => (
                include_str!("../tests/fixtures/tvmaze/83073-show.json"),
                include_str!("../tests/fixtures/tvmaze/83073-episodes.json"),
            ),
            64950 => (
                include_str!("../tests/fixtures/tvmaze/64950-show.json"),
                include_str!("../tests/fixtures/tvmaze/64950-episodes.json"),
            ),
            45039 => (
                include_str!("../tests/fixtures/tvmaze/45039-show.json"),
                include_str!("../tests/fixtures/tvmaze/45039-episodes.json"),
            ),
            33352 => (
                include_str!("../tests/fixtures/tvmaze/33352-show.json"),
                include_str!("../tests/fixtures/tvmaze/33352-episodes.json"),
            ),
            90632 => (
                include_str!("../tests/fixtures/tvmaze/90632-show.json"),
                include_str!("../tests/fixtures/tvmaze/90632-episodes.json"),
            ),
            _ => panic!("fixture ID"),
        };
        (
            serde_json::from_str(show).unwrap(),
            serde_json::from_str(episodes).unwrap(),
        )
    }
    #[test]
    fn transient_503_is_retried_before_success() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            for status in ["503 Service Unavailable", "200 OK"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut req = [0; 1024];
                let _ = stream.read(&mut req);
                let body = "{\"id\":90632}";
                write!(stream,"HTTP/1.1 {status}\r\nRetry-After: 1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let mut api = Api {
            base: format!("http://{addr}"),
            pacing: Duration::ZERO,
            client: Client::builder().no_proxy().build().unwrap(),
            last: None,
        };
        assert_eq!(api.get("/retry").unwrap()["id"], 90632);
        thread.join().unwrap();
    }
    #[test]
    fn explicit_ids_include_2026_nbc_not_2003_line_of_fire() {
        for id in SEEDS {
            let (s, e) = fixtures(id);
            assert_eq!(normalize(id, &s, &e).unwrap().0["show_id"], id);
        }
        let (mut s, e) = fixtures(90632);
        assert_eq!(s["network"]["name"], "NBC");
        assert!(s["premiered"].as_str().unwrap().starts_with("2026"));
        s["id"] = json!(2003);
        assert!(normalize(90632, &s, &e).is_err());
    }
    #[test]
    fn blank_airtime_never_becomes_noon_release_time() {
        for id in [83073, 64950, 45039, 33352] {
            let (s, e) = fixtures(id);
            let (p, _) = normalize(id, &s, &e).unwrap();
            for ep in p["episodes"].as_array().unwrap() {
                assert_eq!(ep["date_only"], true);
                assert!(ep["airtime"].is_null());
                assert!(ep["starts_at_utc"].is_null());
                assert_eq!(ep["raw_airtime"], "");
                assert!(ep["raw_airstamp"].is_string());
            }
        }
    }
    #[test]
    fn batches_and_specials_keep_unique_episode_ids() {
        let (s, e) = fixtures(83073);
        let (p, _) = normalize(83073, &s, &e).unwrap();
        let same: Vec<_> = p["episodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["airdate"] == "2026-10-09")
            .collect();
        assert_eq!(same.len(), 3);
        assert_eq!(
            same.iter()
                .map(|e| e["id"].as_i64().unwrap())
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
        let (s, e) = fixtures(33352);
        let (p, _) = normalize(33352, &s, &e).unwrap();
        assert_eq!(
            p["episodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["special"] == true)
                .count(),
            2
        );
    }
    #[test]
    fn timed_nbc_stamp_is_utc_without_peacock_inference() {
        let (s, e) = fixtures(90632);
        let (p, _) = normalize(90632, &s, &e).unwrap();
        assert_eq!(
            p["episodes"][0]["starts_at_utc"],
            "2026-09-22T02:00:00+00:00"
        );
        assert_eq!(p["episodes"][0]["airtime"], "22:00:00");
        assert_eq!(p["episodes"][0]["date_only"], false);
        assert!(p["web_channel_name"].is_null());
        assert_eq!(p["network_name"], "NBC");
    }
    #[test]
    fn malformed_duplicate_and_wrong_series_lists_rejected() {
        let (s, mut e) = fixtures(83073);
        let first = e[0].clone();
        e.as_array_mut().unwrap().push(first);
        assert!(normalize(83073, &s, &e).is_err());
        let (s, mut e) = fixtures(83073);
        e[0].as_object_mut().unwrap().remove("airtime");
        assert!(normalize(83073, &s, &e).is_err());
        let (s, mut e) = fixtures(83073);
        e[0]["_links"]["show"]["href"] = json!("https://api.tvmaze.com/shows/2003");
        assert!(normalize(83073, &s, &e).is_err());
        assert!(normalize(83073, &s, &json!({"episodes":[]})).is_err());
    }
    #[test]
    fn retry_after_and_failed_request_do_not_poison_next_request() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        assert_eq!(retry_delay(Some("12"), 0), 12);
        assert_eq!(retry_delay(None, 0), 2);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            for status in ["429 Too Many Requests", "200 OK"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut req = [0; 1024];
                let _ = stream.read(&mut req);
                let body = "{\"id\":83073}";
                write!(stream,"HTTP/1.1 {status}\r\nRetry-After: 120\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let mut api = Api {
            base: format!("http://{addr}"),
            pacing: Duration::ZERO,
            client: Client::builder().no_proxy().build().unwrap(),
            last: None,
        };
        let error = api.get("/failure").unwrap_err();
        assert!(error.message.contains("429"));
        assert!(error.retry_seconds >= 120);
        assert_eq!(api.get("/healthy").unwrap()["id"], 83073);
        thread.join().unwrap();
    }
}
