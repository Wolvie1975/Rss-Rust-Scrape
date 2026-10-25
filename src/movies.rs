//! Movie calendar discovery, parsing, and stable release identities.
use chrono::{Datelike, Duration, NaiveDate};
use scraper::{ElementRef, Html, Selector};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}
fn text(e: ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    NaiveDate::parse_from_str(s, "%B %e, %Y")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}
fn heading_date(s: &str) -> Option<NaiveDate> {
    let s = s
        .split('(')
        .next()?
        .trim()
        .strip_prefix("Week of ")
        .unwrap_or(s.split('(').next()?.trim());
    date(s).or_else(|| s.split_once(' ').and_then(|(_, rest)| date(rest)))
}
/// Whole Monday-Sunday weeks of release dates, starting at `start`.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub start: NaiveDate,
    pub weeks: u32,
}
impl Window {
    pub fn week(start: NaiveDate) -> Self {
        Self { start, weeks: 1 }
    }
    pub fn end(self) -> NaiveDate {
        self.start + Duration::weeks(self.weeks.into())
    }
    fn days(self) -> impl Iterator<Item = NaiveDate> {
        (0..i64::from(self.weeks) * 7).map(move |n| self.start + Duration::days(n))
    }
    fn contains(self, d: NaiveDate) -> bool {
        d >= self.start && d < self.end()
    }
}
/// A calendar with dated headings but nothing in the window is simply empty;
/// a calendar with no recognizable headings indicates changed markup.
fn has_date_headings(html: &str) -> bool {
    let doc = Html::parse_document(html);
    doc.select(&sel(".reldate"))
        .any(|e| heading_date(&text(e)).is_some())
        || doc
            .select(&sel(".entry-content h2"))
            .any(|e| date(&text(e)).is_some())
}
fn tv_title(s: &str) -> bool {
    let s = s.to_lowercase();
    ["complete series", "series collection", "tv series"]
        .iter()
        .any(|p| s.contains(p))
        || s.split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| {
                w[0].trim_matches(|c: char| !c.is_alphabetic()) == "season"
                    && (w[1].chars().next().is_some_and(|c| c.is_ascii_digit())
                        || [
                            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
                            "ten", "eleven", "twelve", "final", "first", "second", "third",
                        ]
                        .contains(&w[1]))
            })
}
fn absolute(base: &str, href: &str) -> Option<String> {
    let base = url::Url::parse(base).ok()?;
    let target = base.join(href).ok()?;
    (target.scheme() == "https" && target.host_str() == base.host_str()).then(|| target.to_string())
}

pub fn dvd_links(html: &str, base: &str, window: Window, theatrical: bool) -> Vec<String> {
    let doc = Html::parse_document(html);
    let mut current = None;
    let mut links = BTreeSet::new();
    for e in doc.select(&sel(".reldate, .dvdcell")) {
        if e.value().classes().any(|c| c == "reldate") {
            current = heading_date(&text(e));
            continue;
        }
        let Some(d) = current else {
            continue;
        };
        // A theatrical heading labels a week, not each movie's exact date.
        let eligible = if theatrical {
            d >= window.start - Duration::days(6) && d < window.end() + Duration::days(6)
        } else {
            window.contains(d)
        };
        if !eligible {
            continue;
        }
        for a in e.select(&sel("a[href^='/movies/']")) {
            if let Some(u) = a.value().attr("href").and_then(|h| absolute(base, h)) {
                links.insert(u);
            }
        }
    }
    links.into_iter().collect()
}

pub fn streaming_links(html: &str, base: &str, window: Window) -> Vec<String> {
    let doc = Html::parse_document(html);
    let mut current = None;
    let mut links = BTreeSet::new();
    for e in doc.select(&sel(".entry-content h2, .entry-content figure a")) {
        if e.value().name() == "h2" {
            current = date(&text(e));
            continue;
        }
        if current.is_some_and(|d| window.contains(d))
            && let Some(u) = e.value().attr("href").and_then(|h| absolute(base, h))
        {
            links.insert(u);
        }
    }
    links.into_iter().collect()
}

/// Apply the same Central-day boundary as retention before persistence/poster jobs.
pub fn retain_current_releases(report: &mut Value, today: NaiveDate) -> usize {
    let Some(rows) = report["releases"].as_array_mut() else {
        return 0;
    };
    let before = rows.len();
    rows.retain(|r| {
        r["release_date"]
            .as_str()
            .and_then(date)
            .is_none_or(|d| d >= today)
    });
    let skipped = before - rows.len();
    let mut counts = std::collections::BTreeMap::new();
    for r in rows.iter() {
        if let Some(kind) = r["release_type"].as_str() {
            *counts.entry(kind.to_owned()).or_insert(0usize) += 1;
        }
    }
    report["counts"] = json!(counts);
    report["past_releases_skipped"] = json!(skipped);
    skipped
}

/// Movie Insider calendars are discovery only: card dates may describe several
/// channels. Import dates exclusively from the detail page's digital section.
pub fn movieinsider_calendar(
    html: &str,
    base: &str,
    window: Window,
) -> Result<(Vec<String>, Option<String>), String> {
    let doc = Html::parse_document(html);
    let cards = doc.select(&sel(".browse-movie")).collect::<Vec<_>>();
    if cards.is_empty() {
        return Err("Movie Insider calendar cards missing".into());
    }
    let mut links = BTreeSet::new();
    for card in cards {
        let anchor = card
            .select(&sel(".calendar-scope a.date"))
            .find(|a| {
                a.value()
                    .attr("href")
                    .is_some_and(|h| h.starts_with("/movies/"))
            })
            .ok_or("Movie Insider card release date missing")?;
        let path = anchor
            .value()
            .attr("href")
            .unwrap()
            .trim_matches('/')
            .split('/')
            .collect::<Vec<_>>();
        if path.len() != 4 {
            return Err("Movie Insider card date is not an exact day".into());
        }
        let d = date(&format!("{} {}, {}", path[1], path[3], path[2]))
            .ok_or("unrecognized Movie Insider calendar date")?;
        if window.contains(d) {
            let a = card
                .select(&sel("h2 a"))
                .next()
                .ok_or("Movie Insider movie link missing")?;
            let u = a
                .value()
                .attr("href")
                .and_then(|h| absolute(base, h))
                .ok_or("invalid Movie Insider movie URL")?;
            movieinsider_id(&u)?;
            links.insert(u);
        }
    }
    let next = doc
        .select(&sel("a.pager-ux__btn--next"))
        .next()
        .map(|a| {
            a.value()
                .attr("href")
                .and_then(|h| absolute(base, h))
                .ok_or("invalid Movie Insider pagination URL")
        })
        .transpose()?;
    if let Some(next) = &next {
        let b = url::Url::parse(base).map_err(|e| e.to_string())?;
        let u = url::Url::parse(next).map_err(|e| e.to_string())?;
        if u.path() != b.path()
            || !u
                .query_pairs()
                .any(|(k, v)| k == "page_offset" && v.parse::<u32>().is_ok())
        {
            return Err("unexpected Movie Insider pagination target".into());
        }
    }
    Ok((links.into_iter().collect(), next))
}
fn movieinsider_id(raw: &str) -> Result<String, String> {
    let u = url::Url::parse(raw).map_err(|e| e.to_string())?;
    if u.scheme() != "https" || u.host_str() != Some("www.movieinsider.com") {
        return Err("invalid Movie Insider host".into());
    }
    let id = u
        .path_segments()
        .and_then(|mut p| p.next())
        .and_then(|s| s.strip_prefix('m'))
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .ok_or("invalid Movie Insider movie ID")?;
    Ok(id.to_owned())
}
pub fn movieinsider_details(html: &str, url: &str) -> Result<Vec<Value>, String> {
    movieinsider_id(url)?;
    let doc = Html::parse_document(html);
    let title = doc
        .select(&sel(
            "main[itemtype='http://schema.org/Movie'] meta[itemprop='name']",
        ))
        .next()
        .and_then(|m| m.value().attr("content"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("Movie Insider movie title missing")?;
    if tv_title(title) {
        return Err(format!("excluded apparent TV title: {title}"));
    }
    let paragraph = if let Some(heading) = doc.select(&sel("#digital-release-date")).next() {
        heading
            .next_siblings()
            .filter_map(ElementRef::wrap)
            .next()
            .filter(|e| e.value().name() == "p")
            .ok_or("Movie Insider digital release paragraph missing")?
    } else {
        // Digital-only and simultaneous releases use the general release intro.
        // Require the digital channel in the dated sentence, not a later link.
        if doc.select(&sel("p.release-intro")).any(|p| {
            let description = text(p).to_lowercase();
            description.contains("will open in select theaters")
                && !description.contains("digital")
                && !description.contains("vod")
                && p.select(&sel("time[datetime]")).count() == 1
        }) {
            return Ok(vec![]);
        }
        doc.select(&sel("p.release-intro"))
            .find(|p| {
                let description = text(*p).to_lowercase();
                let sentence = description.split('.').next().unwrap_or("");
                sentence.contains("vod / digital on")
            })
            .ok_or("Movie Insider explicit digital release section missing")?
    };
    let description = text(paragraph).to_lowercase();
    let title_lower = title.to_lowercase();
    let description = description
        .strip_prefix(&title_lower)
        .unwrap_or(&description)
        .trim_start();
    if !(description.contains("digital and vod on")
        || description
            .split('.')
            .next()
            .is_some_and(|s| s.contains("vod / digital on")))
        || description.contains("expected")
        || description.contains("estimated")
        || description.contains("rumored")
    {
        return Err("Movie Insider digital date is not explicitly announced".into());
    }
    let times = paragraph.select(&sel("time[datetime]")).collect::<Vec<_>>();
    if times.len() != 1 {
        return Err("Movie Insider digital date missing or ambiguous".into());
    }
    let d = times[0]
        .value()
        .attr("datetime")
        .and_then(date)
        .ok_or("Movie Insider digital day unrecognized")?;
    let mut release = row(
        title,
        "movieinsider",
        "digital",
        d,
        url,
        Some("VOD / Digital"),
        None,
    );
    if let Ok(metadata) = crate::movie_metadata::parse("movieinsider", html, url) {
        release["poster_metadata"] = json!({"title":metadata.identity.title,"year":metadata.identity.year,"imdb_id":metadata.identity.imdb_id,"tmdb_id":metadata.identity.tmdb_id,"posters":metadata.posters});
    }
    Ok(vec![release])
}
fn discover_movieinsider(
    client: &reqwest::blocking::Client,
    window: Window,
) -> Result<(Vec<String>, Vec<Value>), crate::Error> {
    let years = window.days().map(|d| d.year()).collect::<BTreeSet<_>>();
    let mut found = BTreeSet::new();
    let mut coverage = vec![];
    for year in years {
        let mut next = Some(format!(
            "https://www.movieinsider.com/releases/vod-digital/{year}"
        ));
        let mut visited = BTreeSet::new();
        while let Some(url) = next {
            if visited.len() >= 20 || !visited.insert(url.clone()) {
                return Err("Movie Insider pagination exceeded limit or repeated a page".into());
            }
            eprintln!("calendar: digital: {url}");
            let html = crate::http::get_text(client, &url)?;
            let (links, following) = movieinsider_calendar(&html, &url, window)?;
            coverage.push(json!({"url":url,"category":"digital","candidate_count":links.len()}));
            found.extend(links);
            // The provider sometimes has out-of-order cards. Scan every page.
            next = following;
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    Ok((found.into_iter().collect(), coverage))
}

fn row(
    title: &str,
    source: &str,
    kind: &str,
    d: NaiveDate,
    url: &str,
    format: Option<&str>,
    platform: Option<&str>,
) -> Value {
    json!({"title":title,"source":source,"release_type":kind,"release_date":d.to_string(),"country_code":"US","date_status":"announced","source_url":url,"format":format,"platform":platform})
}

pub fn dvd_details(html: &str, url: &str) -> Result<Vec<Value>, String> {
    let doc = Html::parse_document(html);
    let title = doc
        .select(&sel("h1 [itemprop='name']"))
        .next()
        .map(text)
        .filter(|t| !t.is_empty())
        .ok_or("missing movie title")?;
    if tv_title(&title) {
        return Err(format!("excluded apparent TV title: {title}"));
    }
    if doc
        .select(&sel(".disccellinfo b"))
        .any(|e| tv_title(&text(e)))
    {
        return Err(format!(
            "excluded apparent TV title/season edition: {title}"
        ));
    }
    let mut rows = Vec::new();
    let theater_date = doc
        .select(&sel("[itemprop='datePublished']"))
        .next()
        .and_then(|e| {
            e.value()
                .attr("content")
                .and_then(date)
                .or_else(|| date(&text(e)))
        })
        .or_else(|| {
            doc.select(&sel("span.medlargeboldtext"))
                .find(|e| text(*e).eq_ignore_ascii_case("Theater date"))
                .and_then(|e| e.next_siblings().filter_map(ElementRef::wrap).next())
                .and_then(|e| date(&text(e)))
        });
    if let Some(d) = theater_date {
        rows.push(row(
            &title,
            "dvdsreleasedates",
            "theatrical",
            d,
            url,
            None,
            None,
        ));
    }
    for e in doc.select(&sel(".disccellinfo")) {
        let Some(format) = e.select(&sel("b")).next().map(text) else {
            continue;
        };
        let all = text(e);
        // No invented date for estimates, months-only dates, or TBA.
        if all.to_lowercase().contains("estimated") {
            continue;
        }
        let Some(d) = e.select(&sel("span.bold")).find_map(|s| date(&text(s))) else {
            continue;
        };
        let kind = if format.to_lowercase().starts_with("digital") {
            "digital"
        } else {
            "disc"
        };
        if kind == "disc"
            && !["dvd", "blu-ray", "4k", "uhd"]
                .iter()
                .any(|s| format.to_lowercase().contains(s))
        {
            continue;
        }
        let mut r = row(
            &title,
            "dvdsreleasedates",
            kind,
            d,
            url,
            Some(&format),
            None,
        );
        if let Some(upc) = all
            .split("UPC:")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
        {
            r["upc"] = json!(upc);
        }
        rows.push(r);
    }
    let imdb = doc
        .select(&sel("a[itemprop='sameAs']"))
        .filter_map(|e| e.value().attr("href"))
        .find_map(|u| u.split("/title/").nth(1).and_then(|s| s.split('/').next()));
    for r in &mut rows {
        r["imdb_id"] = json!(imdb);
        if let Ok(metadata) = crate::movie_metadata::parse("dvdsreleasedates", html, url) {
            r["poster_metadata"] = json!({"title":metadata.identity.title,"year":metadata.identity.year,"imdb_id":metadata.identity.imdb_id,"tmdb_id":metadata.identity.tmdb_id,"posters":metadata.posters});
        }
    }
    Ok(rows)
}

pub fn streaming_details(html: &str, url: &str) -> Result<Vec<Value>, String> {
    let doc = Html::parse_document(html);
    let title = doc
        .select(&sel("h1"))
        .next()
        .map(text)
        .filter(|t| !t.is_empty())
        .ok_or("missing movie title")?;
    if tv_title(&title) {
        return Err(format!("excluded apparent TV title: {title}"));
    }
    let mut rows = Vec::new();
    for p in doc.select(&sel(".entry-content p")) {
        let s = text(p);
        if let Some(rest) = s.strip_prefix("VOD Release Date") {
            let rest = rest.trim_start_matches([' ', ':']);
            if ["tba", "tbd", "n/a", "not announced"].contains(&rest.to_lowercase().as_str()) {
                continue;
            }
            let raw = rest.split('(').next().unwrap_or(rest).trim();
            let d = date(raw)
                .ok_or_else(|| format!("VOD date is unconfirmed or unrecognized: {rest}"))?;
            rows.push(row(
                &title,
                "whentostream",
                "digital",
                d,
                url,
                Some("VOD / Digital"),
                None,
            ));
            continue;
        }
        let Some(rest) = s.strip_prefix("SVOD Release Date") else {
            continue;
        };
        let rest = rest.trim_start_matches([' ', ':']);
        if ["tba", "tbd", "n/a", "not announced"].contains(&rest.to_lowercase().as_str()) {
            continue;
        }
        let Some((d, service)) = rest.split_once('(') else {
            return Err(format!("subscription date has no explicit service: {rest}"));
        };
        let Some(d) = date(d) else {
            return Err(format!(
                "subscription date is unconfirmed or unrecognized: {rest}"
            ));
        };
        let service = service
            .trim()
            .strip_suffix(')')
            .unwrap_or(service.trim())
            .trim();
        if service.is_empty() {
            return Err("subscription service is empty".into());
        }
        for platform in service.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            rows.push(row(
                &title,
                "whentostream",
                "subscription",
                d,
                url,
                None,
                Some(platform),
            ));
        }
    }
    if let Ok(metadata) = crate::movie_metadata::parse("whentostream", html, url) {
        for r in &mut rows {
            r["poster_metadata"] = json!({"title":metadata.identity.title,"year":metadata.identity.year,"imdb_id":metadata.identity.imdb_id,"tmdb_id":metadata.identity.tmdb_id,"posters":metadata.posters});
        }
    }
    Ok(rows)
}

pub fn preview(window: Window) -> Result<Value, crate::Error> {
    collect(
        window,
        &[],
        &[
            "dvdsreleasedates".into(),
            "whentostream".into(),
            "movieinsider".into(),
        ],
        &[],
    )
}

/// Refresh tracked movies as well as discovering new releases in the window.
pub fn ingest(
    window: Window,
    tracked: &[String],
    enabled: &[String],
) -> Result<Value, crate::Error> {
    collect(window, tracked, enabled, &[])
}

/// Provider-wide enablement and physical-disc eligibility are independent.
pub fn ingest_with_disc_policy(
    window: Window,
    tracked: &[String],
    enabled: &[String],
    disc_disabled: &[String],
) -> Result<Value, crate::Error> {
    collect(window, tracked, enabled, disc_disabled)
}

fn release_allowed(row: &Value, disc_disabled: &[String]) -> bool {
    row["release_type"] != "disc" || !disc_disabled.iter().any(|s| row["source"] == s.as_str())
}

fn collect(
    window: Window,
    tracked: &[String],
    enabled: &[String],
    disc_disabled: &[String],
) -> Result<Value, crate::Error> {
    let client = crate::http::client_builder()
        .user_agent("Mozilla/5.0 (compatible; web_scraper/0.1)")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let mut calendars = BTreeMap::new();
    for d in window.days() {
        let y = d.year();
        let m = d.month();
        let month = d.format("%B").to_string().to_lowercase();
        calendars.insert(
            format!(
                "https://www.dvdsreleasedates.com/releases/{y}/{m}/new-dvd-releases-{month}-{y}"
            ),
            "disc",
        );
        calendars.insert(format!("https://www.dvdsreleasedates.com/digital-releases/{y}/{m}/digital-hd-releases-{month}-{y}"), "digital");
        calendars.insert(
            format!("https://www.dvdsreleasedates.com/new-movies-{y}/"),
            "theatrical",
        );
        calendars.insert(
            format!("https://whentostream.com/streaming-{month}-{y}/"),
            "subscription",
        );
    }
    let mut candidates: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    let mut issues = Vec::new();
    let mut coverage = Vec::new();
    if enabled.iter().any(|s| s == "movieinsider") {
        match discover_movieinsider(&client, window) {
            Ok((links, pages)) => {
                coverage.extend(pages);
                for link in links {
                    candidates.entry(link).or_default().insert("digital");
                }
            }
            Err(e) => issues.push(json!({"source":"movieinsider","reason":e.to_string()})),
        }
    }
    for (url, category) in calendars {
        if category == "disc" && disc_disabled.iter().any(|s| s == "dvdsreleasedates") {
            continue;
        }
        let source = if category == "subscription" {
            "whentostream"
        } else {
            "dvdsreleasedates"
        };
        if !enabled.iter().any(|s| s == source) {
            continue;
        }
        eprintln!("calendar: {category}: {url}");
        let result = client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| -> crate::Error { Box::new(e) })
            .and_then(crate::http::response_text);
        match result {
            Ok(html) => {
                let links = if category == "subscription" {
                    streaming_links(&html, &url, window)
                } else {
                    dvd_links(&html, &url, window, category == "theatrical")
                };
                coverage.push(json!({"url":url,"category":category,"candidate_count":links.len()}));
                if links.is_empty() && !has_date_headings(&html) {
                    issues.push(json!({"url":url,"reason":"no candidates found; empty week or changed page markup requires review"}));
                }
                for link in links {
                    let kinds = candidates.entry(link).or_default();
                    kinds.insert(category);
                    if source == "whentostream" {
                        kinds.insert("digital");
                    }
                }
            }
            Err(e) => issues.push(json!({"url":url,"reason":e.to_string()})),
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    for url in tracked {
        let category = if url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .as_deref()
            == Some("whentostream.com")
        {
            "subscription"
        } else {
            "disc"
        };
        candidates.entry(url.clone()).or_default().insert(category);
    }
    let candidate_count = candidates.len();
    let mut releases = Vec::new();
    let mut excluded_tv = Vec::new();
    let mut no_matching_release = Vec::new();
    for (index, (url, categories)) in candidates.into_iter().enumerate() {
        if let Err(e) = crate::http::public_url(&url) {
            issues.push(json!({"url":url,"reason":e.to_string()}));
            continue;
        }
        eprintln!("detail {}/{}: {}", index + 1, candidate_count, url);
        let result = client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| -> crate::Error { Box::new(e) })
            .and_then(crate::http::response_text);
        match result {
            Ok(html) => {
                let parsed = if url::Url::parse(&url)
                    .ok()
                    .is_some_and(|u| u.host_str() == Some("www.movieinsider.com"))
                {
                    movieinsider_details(&html, &url)
                } else if categories.contains("subscription") {
                    streaming_details(&html, &url)
                } else {
                    dvd_details(&html, &url)
                };
                match parsed {
                    Ok(rows) => {
                        // Keep optional retailer candidates from this movie's source
                        // page. They still require retailer-side identity verification.
                        let retailer_candidates = Html::parse_document(&html)
                            .select(&sel("a[href]"))
                            .filter_map(|a| {
                                url::Url::parse(&url)
                                    .ok()?
                                    .join(a.value().attr("href")?)
                                    .ok()
                            })
                            .filter(|u| {
                                matches!(
                                    u.host_str(),
                                    Some(
                                        "www.fandango.com" | "fandango.com" | "athome.fandango.com"
                                    )
                                )
                            })
                            .map(|u| u.to_string())
                            .collect::<BTreeSet<_>>();
                        let found: Vec<_> = rows
                            .into_iter()
                            .map(|mut r| {
                                r["retailer_candidates"] = json!(retailer_candidates);
                                r
                            })
                            .filter(|r| {
                                if !release_allowed(r, disc_disabled) {
                                    return false;
                                }
                                if tracked.contains(&url) {
                                    return true;
                                }
                                r["release_date"]
                                    .as_str()
                                    .and_then(date)
                                    .is_some_and(|d| window.contains(d))
                                    && r["release_type"]
                                        .as_str()
                                        .is_some_and(|t| categories.contains(t))
                            })
                            .collect();
                        if found.is_empty() {
                            no_matching_release.push(json!({"url":url,"categories":categories,"reason":"no announced detail date in the selected window/category"}));
                        }
                        releases.extend(found);
                    }
                    Err(reason) if reason.starts_with("excluded apparent TV") => {
                        excluded_tv.push(json!({"url":url,"reason":reason}))
                    }
                    Err(reason) => issues.push(json!({"url":url,"reason":reason})),
                }
            }
            Err(e) => issues.push(json!({"url":url,"reason":e.to_string()})),
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    releases.sort_by_key(|r| {
        (
            r["release_date"].as_str().unwrap_or("").to_owned(),
            r["title"].as_str().unwrap_or("").to_owned(),
            r["release_type"].as_str().unwrap_or("").to_owned(),
        )
    });
    let mut counts = BTreeMap::new();
    for r in &releases {
        *counts
            .entry(r["release_type"].as_str().unwrap_or("").to_owned())
            .or_insert(0) += 1;
    }
    Ok(
        json!({"week_start":window.start.to_string(),"week_end":(window.end()-Duration::days(1)).to_string(),"weeks":window.weeks,"database_written":false,"candidate_count":candidate_count,"counts":counts,"calendars":coverage,"releases":releases,"issues":issues,"excluded_tv":excluded_tv,"no_matching_release":no_matching_release}),
    )
}

/// Persist only identities that are unambiguous within this report.
#[derive(Debug, Clone)]
pub struct MovieRelease {
    pub source: String,
    pub external_id: String,
    pub title: String,
    pub imdb_id: Option<String>,
    pub key: String,
    pub kind: String,
    pub date: NaiveDate,
    pub url: String,
    pub format: Option<String>,
    pub platform: Option<String>,
}

pub fn release_rows(report: &Value) -> Result<Vec<MovieRelease>, String> {
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for r in report["releases"].as_array().ok_or("missing releases")? {
        let field = |name: &str| {
            r[name]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("missing {name}"))
        };
        let source = field("source")?;
        let url = field("source_url")?;
        let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
        let external_id = if source == "dvdsreleasedates" {
            parsed
                .path_segments()
                .and_then(|mut p| {
                    (p.next()? == "movies")
                        .then(|| p.next().map(str::to_owned))
                        .flatten()
                })
                .ok_or("missing DVD movie ID")?
        } else if source == "movieinsider" {
            movieinsider_id(&url)?
        } else if source == "whentostream" {
            parsed.path().trim_matches('/').to_owned()
        } else {
            return Err(format!("unknown movie source: {source}"));
        };
        let kind = field("release_type")?;
        let platform = r["platform"].as_str().map(str::to_owned);
        let format = r["format"].as_str().map(str::to_owned);
        let key = match kind.as_str() {
            "theatrical" => "theatrical:first".to_owned(),
            "digital" => "digital:first".to_owned(),
            "subscription" => format!(
                "subscription:{}",
                platform.as_deref().ok_or("missing service")?.to_lowercase()
            ),
            "disc" => {
                if let Some(upc) = r["upc"]
                    .as_str()
                    .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
                {
                    format!("disc:upc:{upc}")
                } else {
                    format!(
                        "disc:format:{}",
                        format.as_deref().ok_or("missing format")?.to_lowercase()
                    )
                }
            }
            _ => return Err(format!("unknown release type: {kind}")),
        };
        if external_id.is_empty()
            || external_id.encode_utf16().count() > 200
            || key.encode_utf16().count() > 200
        {
            return Err("movie identity exceeds schema limits".into());
        }
        if !seen.insert((source.clone(), external_id.clone(), key.clone())) {
            return Err(format!(
                "ambiguous release identity: {source}/{external_id}/{key}"
            ));
        }
        rows.push(MovieRelease {
            source,
            external_id,
            title: field("title")?,
            imdb_id: r["imdb_id"]
                .as_str()
                .filter(|s| s.starts_with("tt") && s[2..].chars().all(|c| c.is_ascii_digit()))
                .map(str::to_owned),
            key,
            kind,
            date: date(&field("release_date")?).ok_or("invalid release date")?,
            url,
            format,
            platform,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_formats_are_blocked_without_blocking_other_channels() {
        let mut html = String::from(
            "<h1><span itemprop='name'>Film</span></h1><span itemprop='datePublished' content='2026-10-23'></span>",
        );
        for format in [
            "DVD",
            "Blu-ray",
            "4K UHD",
            "4K UHD + Blu-ray + Digital",
            "Digital HD",
        ] {
            html.push_str(&format!("<div class='disccellinfo'><b>{format}</b><span class='bold'>October 23, 2026</span></div>"));
        }
        let rows = dvd_details(&html, "https://www.dvdsreleasedates.com/movies/1/film").unwrap();
        assert_eq!(
            rows.iter().filter(|r| r["release_type"] == "disc").count(),
            4
        );
        let disabled = vec!["dvdsreleasedates".to_owned()];
        let kept: Vec<_> = rows
            .iter()
            .filter(|r| release_allowed(r, &disabled))
            .collect();
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0]["release_type"], "theatrical");
        assert_eq!(kept[1]["release_type"], "digital");
        assert!(rows.iter().all(|r| release_allowed(r, &[])));
        for kind in [
            "digital",
            "digital_purchase",
            "digital_rental",
            "subscription",
        ] {
            assert!(release_allowed(
                &json!({"source":"whentostream","release_type":kind}),
                &disabled
            ));
        }
    }

    #[test]
    fn movie_titles_with_season_are_kept() {
        assert!(!tv_title("Season of the Witch"));
        assert!(!tv_title("Happiest Season (2020)"));
        assert!(tv_title("Show: Season One"));
        assert!(tv_title("Show Season 12 Blu-ray"));
    }
    #[test]
    fn release_identity_survives_date_and_title_changes() {
        let mut report = json!({"releases":[row("Film", "dvdsreleasedates", "disc", start(), "https://www.dvdsreleasedates.com/movies/123/film", Some("DVD"), None)]});
        report["releases"][0]["upc"] = json!("123456");
        let before = release_rows(&report).unwrap();
        report["releases"][0]["release_date"] = json!("2026-10-13");
        report["releases"][0]["title"] = json!("Renamed");
        let after = release_rows(&report).unwrap();
        assert_eq!(before[0].key, after[0].key);
        assert_eq!(before[0].external_id, after[0].external_id);
        let duplicate = report["releases"][0].clone();
        report["releases"].as_array_mut().unwrap().push(duplicate);
        assert!(release_rows(&report).is_err());
    }
    fn start() -> NaiveDate {
        date("2026-10-05").unwrap()
    }
    #[test]
    fn calendar_filters_week_and_resolves_links() {
        let h = "<table><tr><td class='reldate'>Tuesday October 6, 2026<div>(this week)</div></td></tr><tr><td class='dvdcell'><a href='/movies/1/test'>Test</a></td></tr><tr><td class='reldate'>Tuesday October 13, 2026</td></tr><tr><td class='dvdcell'><a href='/movies/2/next'>Next</a></td></tr></table>";
        assert_eq!(
            dvd_links(
                h,
                "https://www.dvdsreleasedates.com/",
                Window::week(start()),
                false
            ),
            vec!["https://www.dvdsreleasedates.com/movies/1/test"]
        );
    }
    #[test]
    fn multi_week_window_discovers_later_weeks_only_within_range() {
        let h = "<table><tr><td class='reldate'>Tuesday October 6, 2026</td></tr><tr><td class='dvdcell'><a href='/movies/1/a'>A</a></td></tr><tr><td class='reldate'>Tuesday October 27, 2026</td></tr><tr><td class='dvdcell'><a href='/movies/2/b'>B</a></td></tr><tr><td class='reldate'>Tuesday November 3, 2026</td></tr><tr><td class='dvdcell'><a href='/movies/3/c'>C</a></td></tr></table>";
        let window = Window {
            start: start(),
            weeks: 4,
        };
        assert_eq!(window.end(), date("2026-11-02").unwrap());
        assert_eq!(
            dvd_links(h, "https://www.dvdsreleasedates.com/", window, false),
            vec![
                "https://www.dvdsreleasedates.com/movies/1/a",
                "https://www.dvdsreleasedates.com/movies/2/b"
            ]
        );
    }
    #[test]
    fn empty_window_is_not_mistaken_for_changed_markup() {
        assert!(has_date_headings(
            "<table><tr><td class='reldate'>Tuesday November 3, 2026</td></tr></table>"
        ));
        assert!(has_date_headings(
            "<div class='entry-content'><h2>November 3, 2026</h2></div>"
        ));
        assert!(!has_date_headings("<html>Access denied</html>"));
    }
    #[test]
    fn disc_bundle_does_not_become_a_digital_release() {
        let h = "<h1><span itemprop='name'>Example</span></h1><span itemprop='datePublished' content='2026-07-10'></span><div class='disccellinfo'><b>4K UHD + Blu-ray + Digital</b>UPC: 123 <span class='bold'>October 6, 2026</span></div><div class='disccellinfo'><b>Digital HD</b><span class='bold'>October 13, 2026</span></div>";
        let rows = dvd_details(h, "https://example.test").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1]["release_type"], "disc");
        assert_eq!(rows[1]["upc"], "123");
        assert_eq!(rows[2]["release_type"], "digital");
    }
    #[test]
    fn subscription_uses_its_own_date_and_service() {
        let h = "<h1>Example (2026)</h1><div class='entry-content'><p><strong>VOD Release Date</strong>: January 30, 2026</p><p><strong>SVOD Release Date</strong>: October 9, 2026 (Paramount+)</p></div>";
        let rows = streaming_details(h, "https://example.test").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["release_type"], "digital");
        assert_eq!(rows[0]["release_date"], "2026-01-30");
        assert_eq!(rows[1]["release_date"], "2026-10-09");
        assert_eq!(rows[1]["platform"], "Paramount+");
        assert!(
            streaming_details(&h.replace(" (Paramount+)", ""), "https://example.test").is_err()
        );
    }
    #[test]
    fn tv_and_estimates_are_not_movie_releases() {
        assert!(
            dvd_details(
                "<h1><span itemprop='name'>Show: Season One</span></h1>",
                "x"
            )
            .is_err()
        );
        assert!(dvd_details("<h1><span itemprop='name'>Film</span></h1><div class='disccellinfo'><b>DVD</b>estimated <span class='bold'>October 6, 2026</span></div>","x").unwrap().is_empty());
    }
    #[test]
    fn future_theatrical_date_does_not_require_schema_attribute() {
        let h = "<h1><span itemprop='name'>Film</span></h1><span class='medlargeboldtext'>Theater date<br/></span><span class='mediumtext'>October 9, 2026</span>";
        let rows = dvd_details(h, "x").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["release_date"], "2026-10-09");
    }
    #[test]
    fn season_editions_exclude_series_even_when_title_omits_season() {
        let h = "<h1><span itemprop='name'>Show</span></h1><div class='disccellinfo'><b>Show: Season One Blu-ray Release Date</b><span class='bold'>October 6, 2026</span></div>";
        assert!(
            dvd_details(h, "x")
                .unwrap_err()
                .starts_with("excluded apparent TV")
        );
    }
    #[test]
    fn tba_is_excluded_and_multiple_services_are_separate_releases() {
        let h =
            "<h1>Film (2026)</h1><div class='entry-content'><p>SVOD Release Date: TBA</p></div>";
        assert!(streaming_details(h, "x").unwrap().is_empty());
        let rows =
            streaming_details(&h.replace("TBA", "October 9, 2026 (Disney+, Hulu)"), "x").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["platform"], "Disney+");
        assert_eq!(rows[1]["platform"], "Hulu");
    }
    #[test]
    fn streaming_discovery_uses_links_not_image_titles() {
        let h = "<div class='entry-content'><h2>October 9, 2026</h2><figure><a href='/movie-2026/'><img data-image-title='Screenshot'></a></figure><h2>October 16, 2026</h2><figure><a href='/next-2026/'>x</a></figure></div>";
        assert_eq!(
            streaming_links(
                h,
                "https://whentostream.com/streaming-october-2026/",
                Window::week(start())
            ),
            vec!["https://whentostream.com/movie-2026/"]
        );
    }
}

#[cfg(test)]
mod digital_coverage_tests {
    use super::*;
    #[test]
    fn movieinsider_digital_section_never_uses_theatrical_or_estimated_dates() {
        let html = "<main itemtype='http://schema.org/Movie'><meta itemprop='name' content='Example'><time datetime='2026-07-17'>Theatrical</time><h3 id='digital-release-date'>Digital</h3><p>Example was released on Digital and VOD on <time datetime='2026-10-09'>October 9, 2026</time>.</p></main>";
        let rows = movieinsider_details(html, "https://www.movieinsider.com/m123/example").unwrap();
        assert_eq!(rows[0]["release_date"], "2026-10-09");
        assert_eq!(rows[0]["release_type"], "digital");
        assert!(
            movieinsider_details(
                &html.replace("Example", "Unexpected"),
                "https://www.movieinsider.com/m123/unexpected"
            )
            .is_ok()
        );
        assert!(
            movieinsider_details(
                &html.replace("was released", "is expected to be released"),
                "https://www.movieinsider.com/m123/example"
            )
            .is_err()
        );
        assert!(
            movieinsider_details(
                &html.replace("id='digital-release-date'", "id='theatrical'"),
                "https://www.movieinsider.com/m123/example"
            )
            .is_err()
        );
        let report = json!({"releases":rows});
        let a = release_rows(&report).unwrap();
        let changed = report
            .to_string()
            .replace("/m123/example", "/m123/renamed")
            .replace("2026-10-09", "2026-10-16");
        let b = release_rows(&serde_json::from_str(&changed).unwrap()).unwrap();
        assert_eq!(
            (a[0].external_id.as_str(), a[0].key.as_str()),
            (b[0].external_id.as_str(), b[0].key.as_str())
        );
        assert_ne!(a[0].date, b[0].date);
    }
    #[test]
    fn movieinsider_release_intro_requires_explicit_digital_date() {
        let html = "<main itemtype='http://schema.org/Movie'><meta itemprop='name' content='Devour'><p class='release-intro'>Devour will be released exclusively on VOD / Digital on <time datetime='2026-10-13'>Tuesday, October 13, 2026</time>. Devour will be available to stream on VOD / Digital.</p></main>";
        let url = "https://www.movieinsider.com/m25724/devour";
        assert_eq!(
            movieinsider_details(html, url).unwrap()[0]["release_date"],
            "2026-10-13"
        );
        let simultaneous = html.replace("exclusively on", "in theaters and on");
        assert!(movieinsider_details(&simultaneous, url).is_ok());
        let theatrical = html.replace("exclusively on VOD / Digital on", "in theaters on");
        assert!(movieinsider_details(&theatrical, url).is_err());
        let theatrical_only = "<main itemtype='http://schema.org/Movie'><meta itemprop='name' content='Clean Hands'><p class='release-intro'>The official Clean Hands release date is <time datetime='2026-10-16'>October 16, 2026</time>. Clean Hands will open in select theaters.</p></main>";
        assert!(
            movieinsider_details(theatrical_only, url)
                .unwrap()
                .is_empty()
        );
        let estimated = html.replace("will be released", "is expected to be released");
        assert!(movieinsider_details(&estimated, url).is_err());
        let ambiguous = html.replace("</p>", "<time datetime='2026-10-16'>Other date</time></p>");
        assert!(movieinsider_details(&ambiguous, url).is_err());
    }
    #[test]
    fn movieinsider_calendar_paginates_and_limits_discovery_to_the_week() {
        let html = "<div class='browse-movie'><h2><a href='/m123/film'>Film</a></h2><div class='calendar-scope'><a class='date' href='/movies/october/2026/09'>October 9</a></div></div><div class='browse-movie'><h2><a href='/m124/old'>Old</a></h2><div class='calendar-scope'><a class='date' href='/movies/october/2026/02'>October 2</a></div></div><a class='pager-ux__btn--next' href='/releases/vod-digital/2026?page_offset=20'>Next</a>";
        let (links, next) = movieinsider_calendar(
            html,
            "https://www.movieinsider.com/releases/vod-digital/2026",
            Window::week(NaiveDate::from_ymd_opt(2026, 10, 5).unwrap()),
        )
        .unwrap();
        assert_eq!(links, vec!["https://www.movieinsider.com/m123/film"]);
        assert!(next.unwrap().ends_with("page_offset=20"));
        assert!(
            movieinsider_calendar(
                "<html>Access denied</html>",
                "https://www.movieinsider.com/releases/vod-digital/2026",
                Window::week(NaiveDate::from_ymd_opt(2026, 10, 5).unwrap())
            )
            .is_err()
        );
        assert!(
            movieinsider_calendar(
                &html.replace(
                    "/releases/vod-digital/2026?page_offset=20",
                    "https://evil.example/page_offset=20"
                ),
                "https://www.movieinsider.com/releases/vod-digital/2026",
                Window::week(NaiveDate::from_ymd_opt(2026, 10, 5).unwrap())
            )
            .is_err()
        );
    }
    #[test]
    fn vod_and_subscription_dates_stay_separate_and_unknown_vod_is_not_invented() {
        let html = "<h1>Example (2026)</h1><div class='entry-content'><p>VOD Release Date: October 6, 2026</p><p>SVOD Release Date: October 9, 2026 (Hulu)</p></div>";
        let rows = streaming_details(html, "https://whentostream.com/example-2026/").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["release_type"], "digital");
        assert_eq!(rows[0]["release_date"], "2026-10-06");
        assert!(rows[0]["platform"].is_null());
        assert_eq!(rows[1]["release_type"], "subscription");
        assert_eq!(rows[1]["release_date"], "2026-10-09");
        let rows = streaming_details(
            &html.replace("VOD Release Date: October 6, 2026", "VOD Release Date: TBA"),
            "https://whentostream.com/example-2026/",
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
    }
}

#[cfg(test)]
mod movieinsider_poster_tests {
    #[test]
    fn owned_poster_is_bound_to_the_movie_and_unrelated_images_are_excluded() {
        let html = "<h1>Example Movie</h1><main itemtype='http://schema.org/Movie'><meta itemprop='name' content='Example'><img class='poster' itemprop='image' src='https://s.movieinsider.com/images/example/p/150/poster.jpg' alt='Example Movie Poster'><img class='poster' itemprop='image' src='https://s.movieinsider.com/images/other/p/150/poster.jpg' alt='Other Movie Poster'></main>";
        let parsed = crate::movie_metadata::parse(
            "movieinsider",
            html,
            "https://www.movieinsider.com/m123/example",
        )
        .unwrap();
        assert_eq!(parsed.identity.title, "Example");
        assert!(parsed.identity.year.is_none());
        assert_eq!(
            parsed.posters,
            vec!["https://s.movieinsider.com/images/example/p/150/poster.jpg"]
        );
    }
}

#[cfg(test)]
mod retention_filter_tests {
    use super::*;
    #[test]
    fn movie_retention_keeps_today_future_and_undated() {
        let mut report = json!({"releases":[{"release_date":"2026-10-12"},{"release_date":"2026-10-13"},{"release_date":"2026-10-14"},{"release_date":null}],"issues":["unchanged"]});
        assert_eq!(
            retain_current_releases(&mut report, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()),
            1
        );
        assert_eq!(report["releases"].as_array().unwrap().len(), 3);
        assert_eq!(report["issues"], json!(["unchanged"]));
    }
}
