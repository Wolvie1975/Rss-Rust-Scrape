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
    pub updated_at: Option<DateTime<Utc>>,
    pub thumbnail_url: Option<String>,
    pub description: Option<String>,
    pub views: Option<i64>,
    pub rating_count: Option<i32>,
    pub rating_average: Option<f64>,
}

/// The channel's newest `limit` videos.
pub fn fetch_latest(client: &Client, feed_url: &str, limit: usize) -> Result<Vec<Video>> {
    let body = client.get(feed_url).send()?.error_for_status()?.text()?;
    parse_videos(&body, limit)
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
}
