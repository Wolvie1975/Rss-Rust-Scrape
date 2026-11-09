//! Page metadata scraping and verified article identity enrichment.
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use scraper::{Html, Selector};

use crate::{Result, article_identity, db, feed, http};

/// Metadata scraped from a single page.
#[derive(Clone)]
pub struct PageMeta {
    /// The URL that was requested (the row in `Sources` this page came from).
    pub source: String,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub image_url: Option<String>,
    /// Verified source-scoped redirect/canonical aliases, never inferred from content.
    pub identity_urls: Vec<String>,
}

/// Returns the `content` of the first `<meta>` tag matching `selector`.
fn meta_content(doc: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Best-effort lookup of a preview image, resolved to an absolute http(s) URL.
fn image_url(doc: &Html, page_url: &str) -> Option<String> {
    let raw = meta_content(doc, r#"meta[property="og:image"]"#)
        .or_else(|| meta_content(doc, r#"meta[property="og:image:secure_url"]"#))
        .or_else(|| meta_content(doc, r#"meta[name="twitter:image"]"#))
        .or_else(|| {
            let sel = Selector::parse(r#"link[rel="image_src"]"#).ok()?;
            let href = doc.select(&sel).next()?.value().attr("href")?.trim();
            (!href.is_empty()).then(|| href.to_string())
        })?;
    let resolved = url::Url::parse(page_url).ok()?.join(&raw).ok()?;
    http::public_url(resolved.as_str())
        .is_ok()
        .then(|| resolved.to_string())
}

fn scrape(client: &Client, url: &str) -> Result<PageMeta> {
    let body = http::get_text(client, url)?;
    let doc = Html::parse_document(&body);

    let title = meta_content(&doc, r#"meta[property="og:title"]"#)
        .or_else(|| {
            let sel = Selector::parse("title").ok()?;
            let t: String = doc.select(&sel).next()?.text().collect();
            Some(t.trim().to_string())
        })
        .unwrap_or_else(|| url.to_string());

    let description = meta_content(&doc, r#"meta[property="og:description"]"#)
        .or_else(|| meta_content(&doc, r#"meta[name="description"]"#));

    let published = meta_content(&doc, r#"meta[property="article:published_time"]"#)
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));

    let image_url = image_url(&doc, url);

    Ok(PageMeta {
        source: url.to_string(),
        identity_urls: vec![],
        url: url.to_owned(),
        title,
        description,
        published,
        image_url,
    })
}

/// Resolve at most 20 recent/incoming URLs per source, within a run-wide budget.
/// Old URLs are revisited even if the feed already switched to a newer URL.
pub fn enrich_article_identities(
    client: &Client,
    found: &mut [PageMeta],
    recent: Vec<PageMeta>,
    started: Instant,
    mut db: Option<&mut db::Db>,
    dry: bool,
) -> Result<()> {
    let source = found
        .first()
        .map(|p| p.source.clone())
        .or_else(|| recent.first().map(|p| p.source.clone()));
    let Some(source) = source else { return Ok(()) };
    let unknown = found
        .iter()
        .any(|p| !recent.iter().any(|old| old.url == p.url));
    let mut jobs = recent
        .iter()
        .take(if unknown { 10 } else { 2 })
        .cloned()
        .collect::<Vec<_>>();
    for page in found.iter() {
        if !recent.iter().any(|old| old.url == page.url) && !jobs.iter().any(|p| p.url == page.url)
        {
            jobs.push(page.clone());
        }
    }
    jobs.truncate(20);
    eprintln!(
        "article identity: plan source={source} jobs={} recent={} run_elapsed_ms={}",
        jobs.len(),
        recent.len(),
        started.elapsed().as_millis()
    );
    let source_started = Instant::now();
    let mut evidence = Vec::new();
    for chunk in jobs.chunks(4) {
        if started.elapsed() > Duration::from_secs(90)
            || source_started.elapsed() > Duration::from_secs(12)
        {
            break;
        }
        let results = std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|p| {
                    scope.spawn(move || {
                        article_identity::fetch(client, &p.url).map_err(|e| e.to_string())
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("article identity worker panicked".into()))
                })
                .collect::<Vec<_>>()
        });
        for (page, result) in chunk.iter().zip(results) {
            if !dry && let Some(d) = db.as_deref_mut() {
                d.article_checked(&source, &page.url)?;
            }
            match result {
                Ok(mut e) => {
                    if e.target != e.requested {
                        eprintln!(
                            "article identity: verified {} -> {} aliases={:?}",
                            e.requested, e.target, e.aliases
                        );
                    }
                    if !e.aliases.contains(&page.url) {
                        e.aliases.push(page.url.clone());
                    }
                    evidence.push((page.clone(), e));
                }
                Err(error) => eprintln!(
                    "article identity: skipped source={source} url={} reason={error}",
                    page.url
                ),
            }
        }
    }
    // Reconcile retained URLs first. They never become extra feed/RSS entries.
    for (old, e) in &evidence {
        if recent.iter().any(|p| p.url == old.url) && (e.target != old.url || e.aliases.len() > 1) {
            let mut update = found
                .iter()
                .find(|p| e.aliases.contains(&p.url))
                .cloned()
                .unwrap_or_else(|| old.clone());
            update.url = e.target.clone();
            update.identity_urls = e.aliases.clone();
            if let Some(d) = db.as_deref_mut() {
                d.save_article(&update, dry)?;
            } else {
                eprintln!(
                    "article identity preview: {} -> {} aliases={:?}",
                    old.url, e.target, e.aliases
                );
            }
        }
    }
    for page in found {
        let mut aliases = BTreeSet::from([page.url.clone()]);
        let mut target = page.url.clone();
        for _ in 0..evidence.len() + 1 {
            let before = aliases.len();
            for (_, e) in &evidence {
                if aliases.iter().any(|u| e.aliases.contains(u)) {
                    aliases.extend(e.aliases.iter().cloned());
                    target = e.target.clone();
                    if page.image_url.is_none() {
                        page.image_url = image_url(&Html::parse_document(&e.body), &e.target);
                    }
                }
            }
            if before == aliases.len() {
                break;
            }
        }
        if evidence
            .iter()
            .any(|(_, e)| aliases.iter().any(|u| e.aliases.contains(u)))
        {
            page.url = target;
            page.identity_urls = aliases.into_iter().collect();
        }
    }
    Ok(())
}

/// For feed entries that came without an image, tries each article page's own `og:image`.
/// An image several articles share is a site-wide logo, not the article's, so it is dropped.
pub fn fill_missing_images(client: &Client, pages: &mut [PageMeta]) {
    let started = Instant::now();
    let mut found: Vec<(usize, String)> = Vec::new();
    for (i, p) in pages
        .iter()
        .enumerate()
        .filter(|(_, p)| p.image_url.is_none())
    {
        if started.elapsed() > Duration::from_secs(30) {
            break;
        }
        let Ok(body) = http::get_text(client, &p.url) else {
            continue;
        };
        if let Some(img) = image_url(&Html::parse_document(&body), &p.url) {
            found.push((i, img));
        }
    }
    for (i, img) in &found {
        if found.iter().filter(|(_, other)| other == img).count() == 1 {
            pages[*i].image_url = Some(img.clone());
        }
    }
}

/// Scrapes one source: each entry of its RSS/Atom feed if it has one, otherwise the
/// page's own metadata (a single entry). Returns the pages and which method produced them.
pub fn scrape_source(client: &Client, url: &str) -> Result<(Vec<PageMeta>, &'static str)> {
    let feed_err = match feed::fetch_entries(client, url) {
        Ok(entries) => return Ok((entries, "feed")),
        Err(e) => e,
    };
    match scrape(client, url) {
        Ok(page) => Ok((vec![page], "page metadata")),
        Err(e) => Err(format!("{feed_err}; page scrape failed: {e}").into()),
    }
}
