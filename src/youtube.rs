use chrono::{DateTime, Utc};
use reqwest::blocking::Client;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One video from a YouTube channel's Atom feed (`/feeds/videos.xml?channel_id=...`).
pub struct Video {
    pub video_id: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub title: String,
    pub url: String,
    pub published_at: DateTime<Utc>,
    /// True when `published_at` was worked out from text like "2 days ago" (the channel page),
    /// not read from the feed. Such a date is only ever used for a video not stored yet.
    pub published_is_estimate: bool,
    pub updated_at: Option<DateTime<Utc>>,
    pub thumbnail_url: Option<String>,
    pub description: Option<String>,
    pub views: Option<i64>,
    pub rating_count: Option<i32>,
    pub rating_average: Option<f64>,
}

/// The channel's newest `limit` videos, and where they came from. Reads the RSS feed (exact dates
/// and counts); if that fails, falls back to the channel's public Videos page.
pub fn fetch_latest(client: &Client, feed_url: &str, limit: usize) -> Result<(Vec<Video>, &'static str)> {
    let rss_error = match fetch_feed(client, feed_url, limit) {
        Ok(videos) if !videos.is_empty() => return Ok((videos, "rss feed")),
        Ok(_) => "the feed has no videos".to_string(),
        Err(e) => e.to_string(),
    };
    let Some(channel_id) = channel_id_from(feed_url) else {
        return Err(rss_error.into());
    };
    match fetch_from_page(client, &channel_id, limit) {
        Ok(videos) if !videos.is_empty() => Ok((videos, "channel page")),
        Ok(_) => Err(format!("{rss_error}; the channel page listed no videos").into()),
        Err(e) => Err(format!("{rss_error}; the channel page also failed: {e}").into()),
    }
}

fn fetch_feed(client: &Client, feed_url: &str, limit: usize) -> Result<Vec<Video>> {
    let body = client.get(feed_url).send()?.error_for_status()?.text()?;
    parse_videos(&body, limit)
}

fn channel_id_from(feed_url: &str) -> Option<String> {
    url::Url::parse(feed_url)
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == "channel_id")
        .map(|(_, v)| v.into_owned())
        .filter(|v| !v.is_empty())
}

fn fetch_from_page(client: &Client, channel_id: &str, limit: usize) -> Result<Vec<Video>> {
    // A browser-like request: the age text ("2 days ago") is only reliably English with this header.
    let html = client
        .get(format!("https://www.youtube.com/channel/{channel_id}/videos"))
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()?
        .error_for_status()?
        .text()?;
    parse_page(&html, channel_id, limit, Utc::now())
}

/// "9 hours ago" / "Streamed 2 days ago" / the short form "9h ago" -> the moment that long before
/// `now`. YouTube serves either form. Months and years are approximated as 30 and 365 days.
/// Text with no "... ago" gives `None`.
fn parse_age(text: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let lower = text.to_ascii_lowercase();
    let before = &lower[..lower.find(" ago")?];
    let mut words = before.split_whitespace().rev();
    let last = words.next()?;
    // "9h" is one word, "9 hours" is two.
    let (n, unit) = match last.find(|c: char| !c.is_ascii_digit()) {
        Some(0) => (words.next()?.parse::<i64>().ok()?, last),
        Some(i) => (last[..i].parse::<i64>().ok()?, &last[i..]),
        None => return None,
    };
    let age = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => chrono::Duration::seconds(n),
        "m" | "min" | "mins" | "minute" | "minutes" => chrono::Duration::minutes(n),
        "h" | "hr" | "hrs" | "hour" | "hours" => chrono::Duration::hours(n),
        "d" | "day" | "days" => chrono::Duration::days(n),
        "w" | "wk" | "wks" | "week" | "weeks" => chrono::Duration::weeks(n),
        "mo" | "mos" | "month" | "months" => chrono::Duration::days(30 * n),
        "y" | "yr" | "yrs" | "year" | "years" => chrono::Duration::days(365 * n),
        _ => return None,
    };
    Some(now - age)
}

fn collect_video_lockups<'a>(v: &'a serde_json::Value, out: &mut Vec<&'a serde_json::Value>) {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            if let Some(lockup) = map.get("lockupViewModel")
                && lockup.get("contentType").and_then(Value::as_str) == Some("LOCKUP_CONTENT_TYPE_VIDEO")
            {
                out.push(lockup);
            }
            map.values().for_each(|x| collect_video_lockups(x, out));
        }
        Value::Array(items) => items.iter().for_each(|x| collect_video_lockups(x, out)),
        _ => {}
    }
}

fn collect_text(v: &serde_json::Value, out: &mut Vec<String>) {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            if let Some(Value::String(s)) = map.get("content") {
                out.push(s.clone());
            }
            map.values().for_each(|x| collect_text(x, out));
        }
        Value::Array(items) => items.iter().for_each(|x| collect_text(x, out)),
        _ => {}
    }
}

/// The first `limit` videos on a channel's Videos page (newest first, as YouTube lists them).
/// Publish dates are estimated from the "N days ago" text; views, ratings and descriptions are
/// left empty. Entries with no age text (live or upcoming) are skipped.
fn parse_page(html: &str, channel_id: &str, limit: usize, now: DateTime<Utc>) -> Result<Vec<Video>> {
    const MARKER: &str = "var ytInitialData = ";
    let start = html.find(MARKER).ok_or("the page has no video data (YouTube may have asked for consent or changed its layout)")? + MARKER.len();
    let data: serde_json::Value = serde_json::Deserializer::from_str(&html[start..])
        .into_iter::<serde_json::Value>()
        .next()
        .ok_or("the page's video data is empty")??;

    let channel_name = data["metadata"]["channelMetadataRenderer"]["title"].as_str().map(str::to_string);
    let mut lockups = Vec::new();
    collect_video_lockups(&data, &mut lockups);

    let mut seen = std::collections::HashSet::new();
    let mut videos = Vec::new();
    for lockup in lockups {
        let Some(video_id) = lockup["contentId"].as_str().filter(|id| seen.insert(id.to_string())) else {
            continue;
        };
        let meta = &lockup["metadata"]["lockupMetadataViewModel"];
        let Some(title) = meta["title"]["content"].as_str() else {
            continue;
        };
        let mut texts = Vec::new();
        collect_text(&meta["metadata"], &mut texts);
        let Some(published_at) = texts.iter().find_map(|t| parse_age(t, now)) else {
            continue;
        };
        // The largest thumbnail, without its tracking query string.
        let thumbnail_url = lockup["contentImage"]["thumbnailViewModel"]["image"]["sources"]
            .as_array()
            .and_then(|s| s.last())
            .and_then(|s| s["url"].as_str())
            .map(|u| u.split('?').next().unwrap_or(u).to_string());
        videos.push(Video {
            video_id: video_id.to_string(),
            channel_id: channel_id.to_string(),
            channel_name: channel_name.clone(),
            title: title.to_string(),
            url: format!("https://www.youtube.com/watch?v={video_id}"),
            published_at,
            published_is_estimate: true,
            updated_at: None,
            thumbnail_url,
            description: None,
            views: None,
            rating_count: None,
            rating_average: None,
        });
        if videos.len() == limit {
            break;
        }
    }
    Ok(videos)
}

fn find<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn text(node: roxmltree::Node, name: &str) -> Option<String> {
    find(node, name)
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn attr(node: roxmltree::Node, name: &str, attr: &str) -> Option<String> {
    find(node, name)
        .and_then(|n| n.attribute(attr))
        .map(str::to_string)
}

fn time(s: Option<String>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&s?).ok().map(|d| d.with_timezone(&Utc))
}

fn parse_videos(xml: &str, limit: usize) -> Result<Vec<Video>> {
    let doc = roxmltree::Document::parse(xml.trim_start_matches('\u{feff}'))?;
    let mut videos: Vec<Video> = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "entry")
        .filter_map(|entry| {
            let video_id = text(entry, "videoId")?;
            // Links appear as `<link rel="alternate" href=...>`; fall back to the standard watch URL.
            let url = entry
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "link")
                .and_then(|n| n.attribute("href"))
                .map(str::to_string)
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
            Some(Video {
                channel_id: text(entry, "channelId")?,
                channel_name: find(entry, "author").and_then(|a| text(a, "name")),
                title: text(entry, "title")?,
                url,
                published_at: time(text(entry, "published"))?,
                published_is_estimate: false,
                updated_at: time(text(entry, "updated")),
                thumbnail_url: attr(entry, "thumbnail", "url"),
                description: find(entry, "group").and_then(|g| text(g, "description")),
                views: attr(entry, "statistics", "views").and_then(|v| v.parse().ok()),
                rating_count: attr(entry, "starRating", "count").and_then(|v| v.parse().ok()),
                rating_average: attr(entry, "starRating", "average").and_then(|v| v.parse().ok()),
                video_id,
            })
        })
        .collect();
    videos.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    videos.truncate(limit);
    Ok(videos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, published: &str, extra: &str) -> String {
        format!(
            r#"<entry><id>yt:video:{id}</id><yt:videoId>{id}</yt:videoId><yt:channelId>UC1</yt:channelId>
<title>Video {id}</title><link rel="alternate" href="https://www.youtube.com/watch?v={id}"/>
<author><name>Some Channel</name><uri>https://www.youtube.com/channel/UC1</uri></author>
<published>{published}</published><updated>2026-09-21T00:00:00+00:00</updated>
<media:group><media:title>Video {id}</media:title>
<media:thumbnail url="https://i3.ytimg.com/vi/{id}/hqdefault.jpg" width="480" height="360"/>
<media:description>About {id}
second line</media:description>{extra}</media:group></entry>"#
        )
    }

    fn feed(entries: &[String]) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015" xmlns:media="http://search.yahoo.com/mrss/" xmlns="http://www.w3.org/2005/Atom">
<yt:channelId>UC1</yt:channelId><title>Some Channel</title>{}</feed>"#,
            entries.concat()
        )
    }

    #[test]
    fn keeps_the_newest_videos_regardless_of_feed_order() {
        let xml = feed(&[
            entry("old", "2026-01-01T00:00:00+00:00", ""),
            entry("newest", "2026-09-20T17:00:00+00:00", ""),
            entry("middle", "2026-05-05T00:00:00+00:00", ""),
        ]);
        let ids: Vec<_> = parse_videos(&xml, 2).unwrap().into_iter().map(|v| v.video_id).collect();
        assert_eq!(ids, ["newest", "middle"]);
    }

    #[test]
    fn reads_all_the_fields() {
        let stats = r#"<media:community><media:starRating count="306" average="4.50" min="1" max="5"/><media:statistics views="7723"/></media:community>"#;
        let xml = feed(&[entry("abc", "2026-09-20T17:00:14+00:00", stats)]);
        let v = &parse_videos(&xml, 5).unwrap()[0];
        assert_eq!(v.video_id, "abc");
        assert_eq!(v.channel_id, "UC1");
        assert_eq!(v.channel_name.as_deref(), Some("Some Channel"));
        assert_eq!(v.title, "Video abc");
        assert_eq!(v.url, "https://www.youtube.com/watch?v=abc");
        assert_eq!(v.published_at.to_rfc3339(), "2026-09-20T17:00:14+00:00");
        assert_eq!(v.thumbnail_url.as_deref(), Some("https://i3.ytimg.com/vi/abc/hqdefault.jpg"));
        assert_eq!(v.description.as_deref(), Some("About abc\nsecond line"));
        assert_eq!((v.views, v.rating_count, v.rating_average), (Some(7723), Some(306), Some(4.5)));
    }

    #[test]
    fn missing_stats_are_left_empty() {
        let xml = feed(&[entry("abc", "2026-09-20T17:00:14+00:00", "")]);
        let v = &parse_videos(&xml, 5).unwrap()[0];
        assert_eq!((v.views, v.rating_count, v.rating_average), (None, None, None));
    }

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn relative_ages_become_dates() {
        let now = at("2026-09-21T12:00:00+00:00");
        assert_eq!(parse_age("9 hours ago", now), Some(at("2026-09-21T03:00:00+00:00")));
        assert_eq!(parse_age("1 day ago", now), Some(at("2026-09-20T12:00:00+00:00")));
        assert_eq!(parse_age("Streamed 2 weeks ago", now), Some(at("2026-09-07T12:00:00+00:00")));
        assert_eq!(parse_age("Premiered 5 minutes ago", now), Some(at("2026-09-21T11:55:00+00:00")));
        assert_eq!(parse_age("3 months ago", now), Some(at("2026-06-23T12:00:00+00:00")));
        assert_eq!(parse_age("1 year ago", now), Some(at("2025-09-21T12:00:00+00:00")));
        // The short form YouTube sometimes serves instead.
        assert_eq!(parse_age("9h ago", now), Some(at("2026-09-21T03:00:00+00:00")));
        assert_eq!(parse_age("2d ago", now), Some(at("2026-09-19T12:00:00+00:00")));
        assert_eq!(parse_age("5m ago", now), Some(at("2026-09-21T11:55:00+00:00")));
        assert_eq!(parse_age("3mo ago", now), Some(at("2026-06-23T12:00:00+00:00")));
        assert_eq!(parse_age("1w ago", now), Some(at("2026-09-14T12:00:00+00:00")));
        assert_eq!(parse_age("Streamed 4h ago", now), Some(at("2026-09-21T08:00:00+00:00")));
        assert_eq!(parse_age("37K", now), None);
        assert_eq!(parse_age("37K views", now), None);
        assert_eq!(parse_age("Scheduled for 9/25/26", now), None);
    }

    fn lockup(id: &str, title: &str, age: &str) -> String {
        format!(
            r#"{{"lockupViewModel":{{"contentType":"LOCKUP_CONTENT_TYPE_VIDEO","contentId":"{id}",
"contentImage":{{"thumbnailViewModel":{{"image":{{"sources":[{{"url":"https://i.ytimg.com/vi/{id}/small.jpg?x=1"}},{{"url":"https://i.ytimg.com/vi/{id}/hq720.jpg?sqp=abc"}}]}}}}}},
"metadata":{{"lockupMetadataViewModel":{{"title":{{"content":"{title}"}},
"metadata":{{"contentMetadataViewModel":{{"metadataRows":[{{"metadataParts":[{{"text":{{"content":"37K views"}}}},{{"text":{{"content":"{age}"}}}}]}}]}}}}}}}}}}}}"#
        )
    }

    fn page(lockups: &[String]) -> String {
        format!(
            r#"<html><script>var ytInitialData = {{"metadata":{{"channelMetadataRenderer":{{"title":"Destin","externalId":"UC1"}}}},
"contents":[{{"items":[{}]}}]}};</script><script>other()</script></html>"#,
            lockups.join(",")
        )
    }

    #[test]
    fn reads_videos_from_a_channel_page_in_page_order() {
        let now = at("2026-09-21T12:00:00+00:00");
        let html = page(&[
            lockup("newest", "First video", "9 hours ago"),
            lockup("older", "Second video", "2 days ago"),
            lockup("oldest", "Third video", "1 week ago"),
        ]);
        let videos = parse_page(&html, "UC1", 2, now).unwrap();
        assert_eq!(videos.iter().map(|v| v.video_id.as_str()).collect::<Vec<_>>(), ["newest", "older"]);
        let v = &videos[0];
        assert_eq!(v.title, "First video");
        assert_eq!(v.channel_name.as_deref(), Some("Destin"));
        assert_eq!(v.url, "https://www.youtube.com/watch?v=newest");
        assert_eq!(v.thumbnail_url.as_deref(), Some("https://i.ytimg.com/vi/newest/hq720.jpg"));
        assert_eq!(v.published_at, at("2026-09-21T03:00:00+00:00"));
        assert!(v.published_is_estimate);
        assert_eq!((v.views, v.description.as_deref()), (None, None));
    }

    #[test]
    fn page_entries_without_an_age_are_skipped() {
        let now = at("2026-09-21T12:00:00+00:00");
        let html = page(&[lockup("live", "Live now", "1.2K watching"), lockup("ok", "Normal", "3 days ago")]);
        let videos = parse_page(&html, "UC1", 5, now).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].video_id, "ok");
    }

    #[test]
    fn a_page_without_video_data_is_an_error() {
        assert!(parse_page("<html>consent required</html>", "UC1", 5, Utc::now()).is_err());
    }

    #[test]
    fn channel_id_comes_from_the_feed_url() {
        assert_eq!(channel_id_from("https://www.youtube.com/feeds/videos.xml?channel_id=UCabc").as_deref(), Some("UCabc"));
        assert_eq!(channel_id_from("https://example.com/feed"), None);
    }
}
