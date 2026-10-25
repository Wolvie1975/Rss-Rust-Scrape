//! MLB league schedules, including postseason and announced future fixtures.
use crate::Result;
use crate::espn_schedule::{self, Fixture, League};
use reqwest::blocking::Client;
pub fn is_feed(raw: &str) -> bool {
    espn_schedule::is_feed(raw, League::Mlb)
}
pub fn fetch(client: &Client, raw: &str) -> Result<Vec<Fixture>> {
    espn_schedule::fetch(client, raw, League::Mlb)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::Value;
    pub fn sample() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/mlb/october-2026.json")).unwrap()
    }
    #[test]
    fn live_mlb_fixtures_map_home_away_venue_and_game_links() {
        let fs = espn_schedule::parse(&sample(), League::Mlb, None).unwrap();
        assert_eq!(fs.len(), 3);
        let f = &fs[0];
        assert_eq!(f.team_name, "Cleveland Guardians");
        assert_eq!(f.event.opponent.as_deref(), Some("Chicago White Sox"));
        assert_eq!(f.team_id, 5);
        assert_eq!(f.opponent_id, 4);
        assert_eq!(f.event.is_away, Some(false));
        assert_eq!(f.event.sport.as_deref(), Some("Baseball"));
        assert_eq!(f.event.location.as_deref(), Some("Progressive Field"));
        assert_eq!(f.event.event_date.to_string(), "2026-10-10");
        assert_eq!(
            f.event.starts_at.unwrap().to_rfc3339(),
            "2026-10-11T00:00:00+00:00"
        );
        assert_eq!(
            f.event.url,
            "https://www.espn.com/mlb/game/_/gameId/401907994"
        );
        assert!(
            f.event
                .live_stats_url
                .as_ref()
                .unwrap()
                .contains("/mlb/game/")
        );
        assert_eq!(f.league.canonical_key(401907994), "espn:mlb:401907994");
        assert_eq!(fs[1].event.opponent.as_deref(), Some("CLE/CHW"));
        assert_eq!(fs[2].team_name, "TBD");
        assert_eq!(fs[2].event.opponent.as_deref(), Some("TBD"));
    }
    #[test]
    fn doubleheaders_keep_two_ids_even_with_same_teams_and_date() {
        let mut x = sample();
        x["events"] = serde_json::json!([x["events"][0].clone(), x["events"][0].clone()]);
        x["events"][1]["id"] = "401907995".into();
        let fs = espn_schedule::parse(&x, League::Mlb, None).unwrap();
        assert_eq!(fs[0].event.event_date, fs[1].event.event_date);
        assert_ne!(fs[0].event.url, fs[1].event.url);
        assert_ne!(
            fs[0].league.canonical_key(fs[0].event.game_id.unwrap()),
            fs[1].league.canonical_key(fs[1].event.game_id.unwrap())
        );
    }
    #[test]
    fn rescheduled_playoff_participants_update_without_changing_identity() {
        let mut x = sample();
        let before = espn_schedule::parse(&x, League::Mlb, None).unwrap();
        let c = &mut x["events"][1]["competitions"][0];
        c["date"] = "2026-10-15T04:00Z".into();
        c["timeValid"] = false.into();
        c["status"]["type"]["name"] = "STATUS_POSTPONED".into();
        let away = c["competitors"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["homeAway"] == "away")
            .unwrap();
        away["team"]["id"] = "5".into();
        away["team"]["displayName"] = "Cleveland Guardians".into();
        let after = espn_schedule::parse(&x, League::Mlb, None).unwrap();
        assert_eq!(before[1].event.url, after[1].event.url);
        assert_eq!(after[1].opponent_id, 5);
        assert_eq!(after[1].event.event_date.to_string(), "2026-10-15");
        assert!(after[1].event.starts_at.is_none());
        assert!(after[1].event.time_tbd);
        assert_eq!(
            espn_schedule::parse(&sample(), League::Mlb, Some("4,5"))
                .unwrap()
                .len(),
            1
        );
        assert_ne!(
            League::Mlb.canonical_key(42),
            League::Wnba.canonical_key(42)
        );
    }
}
