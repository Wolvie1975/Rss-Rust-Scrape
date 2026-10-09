//! Kansas City Current schedule provider; shared ESPN parsing lives in espn_schedule.
pub use crate::espn_schedule::Fixture;
use crate::espn_schedule::{self, League};
use reqwest::blocking::Client;
#[cfg(test)]
use serde_json::Value;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn is_feed(raw: &str) -> bool {
    espn_schedule::is_feed(raw, League::Nwsl)
}
pub fn fetch(client: &Client, raw: &str) -> Result<Vec<Fixture>> {
    espn_schedule::fetch(client, raw, League::Nwsl)
}
#[cfg(test)]
pub fn parse(root: &Value, team: &str) -> Result<Vec<Fixture>> {
    espn_schedule::parse(root, League::Nwsl, Some(team))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono_tz::America::Chicago;
    pub fn sample() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/nwsl/october-2026.json")).unwrap()
    }
    #[test]
    fn live_fixture_maps_upcoming_home_and_away_games() {
        let fs = parse(&sample(), "20907").unwrap();
        assert_eq!(fs.len(), 3);
        let away = &fs[1].event;
        assert_eq!(away.game_id, Some(401854022));
        assert_eq!(away.opponent.as_deref(), Some("Utah Royals"));
        assert_eq!(away.is_away, Some(true));
        assert_eq!(away.location.as_deref(), Some("America First Field"));
        assert_eq!(
            away.starts_at.unwrap().to_rfc3339(),
            "2026-10-17T22:30:00+00:00"
        );
        assert!(!away.time_tbd);
        assert_eq!(fs[2].event.is_away, Some(false));
        assert_eq!(fs[2].event.location.as_deref(), Some("CPKC Stadium"));
        assert!(
            fs[2]
                .event
                .live_stats_url
                .as_ref()
                .unwrap()
                .starts_with("https://www.espn.com/")
        );
    }
    #[test]
    fn local_dates_use_central_dst_and_never_invent_tbd_times() {
        for (utc, local, hour) in [
            ("2026-07-02T01:30Z", "2026-07-01", 20),
            ("2026-12-02T01:30Z", "2026-12-01", 19),
        ] {
            let mut x = sample();
            x["events"][0]["competitions"][0]["date"] = utc.into();
            let f = parse(&x, "20907").unwrap().remove(0);
            assert_eq!(f.event.event_date.to_string(), local);
            use chrono::Timelike;
            assert_eq!(
                f.event.starts_at.unwrap().with_timezone(&Chicago).hour(),
                hour
            );
            x["events"][0]["competitions"][0]["timeValid"] = false.into();
            let f = parse(&x, "20907").unwrap().remove(0);
            assert!(f.event.time_tbd);
            assert!(f.event.starts_at.is_none());
        }
    }
    #[test]
    fn rescheduled_canceled_and_neutral_games_keep_identity() {
        let mut x = sample();
        let before = parse(&x, "20907").unwrap().remove(0).event;
        for status in ["STATUS_POSTPONED", "STATUS_CANCELED", "STATUS_SCHEDULED"] {
            let c = &mut x["events"][0]["competitions"][0];
            c["date"] = "2026-11-05T00:00Z".into();
            c["status"]["type"]["name"] = status.into();
            c["neutralSite"] = true.into();
            x["events"][0]["links"] = serde_json::json!([]);
            let f = parse(&x, "20907").unwrap().remove(0);
            assert_eq!(f.event.url, before.url);
            assert_eq!(f.event.game_id, before.game_id);
            assert_eq!(f.status, status);
            assert_eq!(f.event.is_away, None);
            assert_eq!(f.event.starts_at.is_some(), status == "STATUS_SCHEDULED");
        }
    }
    #[test]
    fn malformed_snapshots_fail_and_unrelated_games_are_excluded() {
        assert!(parse(&serde_json::json!({}), "20907").is_err());
        assert!(
            parse(&serde_json::json!({"events":[]}), "20907")
                .unwrap()
                .is_empty()
        );
        assert!(parse(&sample(), "not-a-team").unwrap().is_empty());
        for key in ["id", "competitions"] {
            let mut x = sample();
            x["events"][0][key] = Value::Null;
            assert!(parse(&x, "20907").is_err());
        }
        let mut x = sample();
        x["events"][0]["competitions"][0]["date"] = "bad-date".into();
        assert!(parse(&x, "20907").is_err());
    }
}

#[cfg(test)]
mod fetch_tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn monthly_requests_deduplicate_rescheduled_ids_and_do_not_send_team_filter() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for month in 1..=12 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buf = [0; 1024];
                while !request.windows(4).any(|b| b == b"\r\n\r\n") {
                    let n = socket.read(&mut buf).unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&buf[..n]);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.contains(&format!("dates=2026{month:02}&limit=1000")));
                assert!(!request.contains("team="));
                assert!(
                    request
                        .to_lowercase()
                        .contains("user-agent: my-newsfeed/1.0")
                );
                let mut body = tests::sample();
                body["events"][0]["competitions"][0]["status"]["type"] =
                    serde_json::json!({"name":"STATUS_SCHEDULED","state":"pre","completed":false});
                if month == 12 {
                    body["events"][0]["competitions"][0]["date"] = "2026-12-04T23:00:00Z".into();
                }
                let body = body.to_string();
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let result = espn_schedule::fetch_from(
            &client,
            &format!("http://{address}/scoreboard?team=20907&season=2026"),
            League::Nwsl,
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        )
        .unwrap();
        server.join().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(
            result
                .iter()
                .find(|f| f.event.game_id == Some(401854018))
                .unwrap()
                .event
                .event_date
                .to_string(),
            "2026-12-04"
        );
    }
}
