use chrono::{DateTime, NaiveDate, Timelike, Utc};
use reqwest::blocking::Client;
use url::Url;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One game from a Sidearm-style calendar RSS feed (custom `ev:` / `s:` fields).
pub struct Event {
    pub url: String,
    pub game_id: Option<i32>,
    pub title: String,
    pub sport: Option<String>,
    pub opponent: Option<String>,
    pub is_away: Option<bool>,
    pub location: Option<String>,
    /// Calendar date in the host school's time zone.
    pub event_date: NaiveDate,
    /// `None` when the start time is still to be announced.
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub time_tbd: bool,
    pub tv: Option<String>,
    pub stream_url: Option<String>,
    pub live_stats_url: Option<String>,
    pub team_logo_url: Option<String>,
    pub opponent_logo_url: Option<String>,
}

pub fn fetch_events(client: &Client, feed_url: &str) -> Result<Vec<Event>> {
    let body = client.get(feed_url).send()?.error_for_status()?.text()?;
    parse_events(&body, feed_url)
}

/// Text of the first child element with this local name (namespace ignored).
fn child_text(node: roxmltree::Node, name: &str) -> Option<String> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Splits "Volleyball Kansas at  Colorado" into ("Volleyball Kansas", is_away, "Colorado").
fn split_matchup(line: &str) -> Option<(String, bool, String)> {
    [(" vs  ", false), (" at  ", true)].into_iter().find_map(|(sep, away)| {
        let (left, right) = line.split_once(sep)?;
        Some((left.trim().to_string(), away, right.trim().to_string()))
    })
}

fn http_url(raw: &str, base: &Url) -> Option<String> {
    let u = base.join(raw.trim()).ok()?;
    matches!(u.scheme(), "http" | "https").then(|| u.to_string())
}

/// The feed doesn't separate sport from school ("Women's Volleyball Kansas"), so the school is
/// taken to be the trailing words every matchup shares. Needs at least two different sports;
/// with one (or none) the split is ambiguous and every sport stays `None`.
fn school_word_count(lefts: &[&str]) -> usize {
    let mut distinct: Vec<&str> = lefts.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() < 2 {
        return 0;
    }
    let words: Vec<Vec<&str>> = distinct.iter().map(|l| l.split_whitespace().collect()).collect();
    let shortest = words.iter().map(Vec::len).min().unwrap_or(0);
    (1..shortest)
        .take_while(|k| {
            let tail = &words[0][words[0].len() - k..];
            words.iter().all(|w| w[w.len() - k..] == *tail)
        })
        .last()
        .unwrap_or(0)
}

fn parse_events(xml: &str, feed_url: &str) -> Result<Vec<Event>> {
    let doc = roxmltree::Document::parse(xml.trim_start_matches('\u{feff}'))?;
    let base = Url::parse(feed_url)?;

    // Each event is kept with the "Sport School" text left of vs/at, to split afterwards.
    let mut rows: Vec<(Event, Option<String>)> = Vec::new();
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let Some(url) = child_text(item, "link").filter(|u| u.len() <= 2048) else {
            continue;
        };
        let start_raw = child_text(item, "startdate");
        let date_raw = child_text(item, "localstartdate").or_else(|| start_raw.clone());
        let Some(event_date) = date_raw
            .as_deref()
            .and_then(|d| d.get(..10))
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        else {
            continue;
        };
        // Date-only start values (e.g. football) mean the time isn't set yet.
        // The feed marks some midnight-UTC times with a stray 100 ns (".0000001Z"); drop sub-seconds.
        let parse_utc = |s: &str| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .and_then(|d| d.with_timezone(&Utc).with_nanosecond(0))
        };
        let starts_at = start_raw.as_deref().filter(|s| s.contains('T')).and_then(parse_utc);
        let ends_at = starts_at
            .and_then(|_| child_text(item, "enddate"))
            .as_deref()
            .and_then(parse_utc);

        let description = child_text(item, "description").unwrap_or_default();
        let matchup = description.lines().next().and_then(split_matchup);
        let line_value = |prefix: &str| {
            description
                .lines()
                .find_map(|l| l.trim().strip_prefix(prefix))
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let logo = |name: &str| child_text(item, name).and_then(|l| http_url(&l, &base));
        let live_stats_url = item
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "livestats")
            .and_then(|n| n.text())
            .and_then(|t| http_url(t, &base));

        let event = Event {
            title: child_text(item, "title").unwrap_or_else(|| url.clone()),
            game_id: child_text(item, "gameid").and_then(|g| g.parse().ok()),
            opponent: matchup.as_ref().map(|(_, _, opp)| opp.clone()),
            is_away: matchup.as_ref().map(|(_, away, _)| *away),
            location: child_text(item, "location"),
            event_date,
            time_tbd: starts_at.is_none(),
            starts_at,
            ends_at,
            tv: line_value("TV:"),
            stream_url: line_value("Streaming Video:").and_then(|u| http_url(&u, &base)),
            live_stats_url,
            team_logo_url: logo("teamlogo"),
            opponent_logo_url: logo("opponentlogo"),
            sport: None,
            url,
        };
        rows.push((event, matchup.map(|(left, _, _)| left)));
    }

    let lefts: Vec<&str> = rows.iter().filter_map(|(_, l)| l.as_deref()).collect();
    let school_words = school_word_count(&lefts);
    Ok(rows
        .into_iter()
        .map(|(mut event, left)| {
            if school_words > 0
                && let Some(left) = left
            {
                let words: Vec<&str> = left.split_whitespace().collect();
                let sport = words[..words.len().saturating_sub(school_words)].join(" ");
                event.sport = (!sport.is_empty()).then_some(sport);
            }
            event
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:ev="https://purl.org/rss/1.0/modules/event/" xmlns:s="https://sidearmsports.com/schemas/cal_rss/1.0">
<channel><title>x</title>
<item><title>9/20 2:00 PM Volleyball Kansas vs  Grand Canyon</title>
<description>Volleyball Kansas vs  Grand Canyon
TV: ESPN+
Streaming Video: https://www.espn.com/watch/x#a=1&amp;b=2
 http://site.test/calendar.aspx?id=1</description>
<link>http://site.test/calendar.aspx?id=1</link>
<ev:location>Lawrence, Kan.</ev:location>
<ev:startdate>2026-09-20T19:00:00.0000000Z</ev:startdate>
<ev:enddate>2026-09-20T22:00:00.0000000Z</ev:enddate>
<s:localstartdate>2026-09-20T14:00:00.0000000</s:localstartdate>
<s:teamlogo>http://site.test/images/site.png</s:teamlogo>
<s:opponentlogo></s:opponentlogo>
<s:gameid>1</s:gameid>
<s:links><s:livestats>https://stats.test/?id=9</s:livestats></s:links></item>
<item><title>10/10 Football Kansas at  Kansas State</title>
<description>Football Kansas at  Kansas State
 http://site.test/calendar.aspx?id=2</description>
<link>http://site.test/calendar.aspx?id=2</link>
<ev:location>Manhattan, Kan. </ev:location>
<ev:startdate>2026-10-10</ev:startdate>
<ev:enddate>2026-10-10T08:00:00.0000000Z</ev:enddate>
<s:localstartdate>2026-10-10</s:localstartdate>
<s:opponentlogo>/images/logos/ksu.png</s:opponentlogo>
<s:gameid>2</s:gameid></item>
<item><title>11/12 8:00 PM Volleyball Kansas at  Colorado</title>
<description>Volleyball Kansas at  Colorado</description>
<link>http://site.test/calendar.aspx?id=3</link>
<ev:startdate>2026-11-13T00:00:00.0000001Z</ev:startdate>
<s:localstartdate>2026-11-12T18:00:00.0000000</s:localstartdate>
<s:gameid>3</s:gameid></item>
</channel></rss>"#;

    fn events() -> Vec<Event> {
        parse_events(FEED, "http://site.test/services/calendar.rss").unwrap()
    }

    #[test]
    fn parses_a_timed_home_game() {
        let e = &events()[0];
        assert_eq!(e.game_id, Some(1));
        assert_eq!(e.sport.as_deref(), Some("Volleyball"));
        assert_eq!(e.opponent.as_deref(), Some("Grand Canyon"));
        assert_eq!(e.is_away, Some(false));
        assert_eq!(e.location.as_deref(), Some("Lawrence, Kan."));
        assert_eq!(e.event_date.to_string(), "2026-09-20");
        assert_eq!(e.starts_at.unwrap().to_rfc3339(), "2026-09-20T19:00:00+00:00");
        assert_eq!(e.ends_at.unwrap().to_rfc3339(), "2026-09-20T22:00:00+00:00");
        assert!(!e.time_tbd);
        assert_eq!(e.tv.as_deref(), Some("ESPN+"));
        assert_eq!(e.stream_url.as_deref(), Some("https://www.espn.com/watch/x#a=1&b=2"));
        assert_eq!(e.live_stats_url.as_deref(), Some("https://stats.test/?id=9"));
        assert_eq!(e.team_logo_url.as_deref(), Some("http://site.test/images/site.png"));
        assert_eq!(e.opponent_logo_url, None);
    }

    #[test]
    fn football_without_a_time_is_flagged_tbd() {
        let e = &events()[1];
        assert_eq!(e.sport.as_deref(), Some("Football"));
        assert_eq!(e.opponent.as_deref(), Some("Kansas State"));
        assert_eq!(e.is_away, Some(true));
        assert!(e.time_tbd);
        assert_eq!(e.starts_at, None);
        assert_eq!(e.ends_at, None, "the feed's placeholder end time must not be kept");
        assert_eq!(e.event_date.to_string(), "2026-10-10");
        assert_eq!(e.location.as_deref(), Some("Manhattan, Kan."));
        assert_eq!(e.opponent_logo_url.as_deref(), Some("http://site.test/images/logos/ksu.png"));
    }

    #[test]
    fn event_date_is_the_local_date_not_the_utc_date() {
        // 8 PM local is already the next day in UTC.
        let e = &events()[2];
        assert_eq!(e.event_date.to_string(), "2026-11-12");
        assert_eq!(e.starts_at.unwrap().date_naive().to_string(), "2026-11-13");
        // ...and the feed's stray 100 ns on midnight-UTC times is dropped.
        assert_eq!(e.starts_at.unwrap().nanosecond(), 0);
    }

    #[test]
    fn sport_is_left_empty_when_it_cannot_be_told_from_the_school() {
        assert_eq!(school_word_count(&["Volleyball Kansas", "Volleyball Kansas"]), 0);
        assert_eq!(school_word_count(&["Volleyball Kansas", "Football Kansas"]), 1);
        assert_eq!(school_word_count(&["Women's Soccer Kansas State", "Football Kansas State"]), 2);
    }
}
