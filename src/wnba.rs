//! WNBA league schedule; optional team IDs filter games without duplicating them.
use crate::espn_schedule::{self, Fixture, League};
use reqwest::blocking::Client;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn is_feed(raw: &str) -> bool {
    espn_schedule::is_feed(raw, League::Wnba)
}
pub fn fetch(client: &Client, raw: &str) -> Result<Vec<Fixture>> {
    espn_schedule::fetch(client, raw, League::Wnba)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::Value;
    pub fn sample() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/wnba/september-2026.json")).unwrap()
    }
    #[test]
    fn league_games_keep_home_team_away_opponent_and_basketball_links() {
        let games = espn_schedule::parse(&sample(), League::Wnba, None).unwrap();
        assert_eq!(games.len(), 2);
        let f = &games[0];
        assert_eq!(f.team_name, "Atlanta Dream");
        assert_eq!(f.team_id, 20);
        assert_eq!(f.opponent_id, 18);
        assert_eq!(f.event.opponent.as_deref(), Some("Connecticut Sun"));
        assert_eq!(f.event.is_away, Some(false));
        assert_eq!(f.event.title, "Atlanta Dream vs Connecticut Sun");
        assert_eq!(f.event.sport.as_deref(), Some("Basketball"));
        assert_eq!(f.event.location.as_deref(), Some("Gateway Center"));
        assert_eq!(
            f.event.starts_at.unwrap().to_rfc3339(),
            "2026-09-17T23:30:00+00:00"
        );
        assert_eq!(
            f.event.url,
            "https://www.espn.com/wnba/game/_/gameId/401857190"
        );
        assert!(
            f.event
                .live_stats_url
                .as_ref()
                .unwrap()
                .contains("/wnba/game/")
        );
        assert!(f.event.tv.as_ref().unwrap().contains("WNBA League Pass"));
        assert_eq!(
            f.league.canonical_key(f.event.game_id.unwrap()),
            "espn:wnba:401857190"
        );
    }
    #[test]
    fn team_filters_do_not_duplicate_games_or_flip_home_perspective() {
        let all = espn_schedule::parse(&sample(), League::Wnba, None).unwrap();
        let away = espn_schedule::parse(&sample(), League::Wnba, Some("18")).unwrap();
        assert_eq!(away.len(), 1);
        assert_eq!(away[0].team_name, "Atlanta Dream");
        let both = espn_schedule::parse(&sample(), League::Wnba, Some("18,20")).unwrap();
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].event.url, all[0].event.url);
        assert!(
            espn_schedule::parse(&sample(), League::Wnba, Some("99999"))
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn reschedules_cancellations_and_neutral_sites_preserve_wnba_identity() {
        let mut x = sample();
        let before = espn_schedule::parse(&x, League::Wnba, None).unwrap();
        let c = &mut x["events"][0]["competitions"][0];
        c["date"] = "2026-11-05T01:00Z".into();
        c["neutralSite"] = true.into();
        c["status"]["type"]["name"] = "STATUS_CANCELED".into();
        let after = espn_schedule::parse(&x, League::Wnba, None).unwrap();
        assert_eq!(before[0].event.url, after[0].event.url);
        assert_eq!(after[0].event.event_date.to_string(), "2026-11-04");
        assert!(after[0].event.time_tbd);
        assert!(!after[0].completed); // Cancellation remains a maintenance observation even with provider post-state.
        assert!(after[0].event.starts_at.is_none());
        assert_eq!(after[0].event.is_away, None);
        assert_ne!(
            League::Wnba.canonical_key(401857190),
            League::Nwsl.canonical_key(401857190)
        );
    }
}
