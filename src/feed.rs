use chrono::Utc;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use url::Url;

use crate::PageMeta;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Locations tried, relative to the site root, when a page doesn't advertise its feed.
const COMMON_FEED_PATHS: &[&str] = &["/feed", "/rss", "/feed.xml", "/rss.xml", "/atom.xml", "/index.xml"];

const MAX_DESCRIPTION_CHARS: usize = 500;

fn get_text(client: &Client, url: &str) -> Result<String> {
    Ok(client.get(url).send()?.error_for_status()?.text()?)
}

/// Feed URLs advertised by the page via `<link rel="alternate" type="application/rss+xml|atom+xml">`.
fn discover_links(html: &str, base: &Url) -> Vec<String> {
    let Ok(sel) = Selector::parse(r#"link[rel="alternate"]"#) else {
        return Vec::new();
    };
    Html::parse_document(html)
        .select(&sel)
        .filter(|el| {
            el.value()
                .attr("type")
                .is_some_and(|t| t.contains("rss") || t.contains("atom"))
        })
        .filter_map(|el| base.join(el.value().attr("href")?).ok())
        .map(String::from)
        .collect()
}

fn http_url(raw: &str, base: &str) -> Option<String> {
    let u = Url::parse(base).ok()?.join(raw.trim()).ok()?;
    matches!(u.scheme(), "http" | "https").then(|| u.to_string())
}

fn first_img_src(html: &str, base: &str) -> Option<String> {
    let sel = Selector::parse("img").ok()?;
    let doc = Html::parse_fragment(html);
    let src = doc.select(&sel).find_map(|el| el.value().attr("src"))?;
    http_url(src, base)
}

fn plain_text(html: &str) -> Option<String> {
    let text: String = Html::parse_fragment(html).root_element().text().collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return None;
    }
    if text.chars().count() > MAX_DESCRIPTION_CHARS {
        let cut: String = text.chars().take(MAX_DESCRIPTION_CHARS).collect();
        return Some(format!("{}…", cut.trim_end()));
    }
    Some(text)
}

fn parse_entries(body: &str, feed_url: &str, source: &str) -> Result<Vec<PageMeta>> {
    let feed = feed_rs::parser::Builder::new()
        .base_uri(Some(feed_url))
        .build()
        .parse(body.as_bytes())?;
    Ok(feed
        .entries
        .into_iter()
        .filter_map(|e| {
            let link = e
                .links
                .iter()
                .find(|l| l.rel.as_deref() == Some("alternate"))
                .or(e.links.first())?
                .href
                .clone();
            if link.len() > 2048 {
                return None;
            }
            let title = e
                .title
                .as_ref()
                .map(|t| t.content.trim().to_string())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| link.clone());
            let html = e
                .summary
                .as_ref()
                .map(|t| t.content.clone())
                .or_else(|| e.content.as_ref().and_then(|c| c.body.clone()));

            let media_image = e.media.iter().find_map(|m| {
                m.thumbnails
                    .first()
                    .map(|t| t.image.uri.clone())
                    .or_else(|| {
                        m.content
                            .iter()
                            .find(|c| c.content_type.as_ref().is_some_and(|t| t.to_string().starts_with("image/")))
                            .and_then(|c| c.url.as_ref().map(|u| u.to_string()))
                    })
            });
            let image_url = media_image
                .and_then(|u: String| http_url(&u, &link))
                .or_else(|| html.as_deref().and_then(|h| first_img_src(h, &link)));

            Some(PageMeta {
                source: source.to_string(),
                url: link,
                title,
                description: html.as_deref().and_then(plain_text),
                published: e.published.or(e.updated).map(|d| d.with_timezone(&Utc)),
                image_url,
            })
        })
        .collect())
}

/// Finds the source's RSS/Atom feed and returns one `PageMeta` per entry.
/// Tries, in order: the source URL itself, feeds advertised in its HTML, then common feed paths.
pub fn fetch_entries(client: &Client, source: &str) -> Result<Vec<PageMeta>> {
    let base = Url::parse(source)?;
    let mut candidates: Vec<String> = Vec::new();

    if let Ok(body) = get_text(client, source) {
        if let Ok(entries) = parse_entries(&body, source, source)
            && !entries.is_empty()
        {
            return Ok(entries);
        }
        candidates.extend(discover_links(&body, &base));
    }
    candidates.extend(COMMON_FEED_PATHS.iter().filter_map(|p| base.join(p).ok()).map(String::from));

    let mut seen = std::collections::HashSet::new();
    for candidate in candidates.into_iter().filter(|c| seen.insert(c.clone())) {
        if let Ok(body) = get_text(client, &candidate)
            && let Ok(entries) = parse_entries(&body, &candidate, source)
            && !entries.is_empty()
        {
            return Ok(entries);
        }
    }
    Err("no RSS/Atom feed found".into())
}
