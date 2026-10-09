//! Movie calendar discovery, parsing, and stable release identities.
use chrono::{Datelike, Duration, NaiveDate};
use reqwest::blocking::Client;
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
fn in_week(d: NaiveDate, start: NaiveDate) -> bool {
    d >= start && d < start + Duration::days(7)
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

pub fn dvd_links(html: &str, base: &str, start: NaiveDate, theatrical: bool) -> Vec<String> {
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
            d >= start - Duration::days(6) && d < start + Duration::days(13)
        } else {
            in_week(d, start)
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

pub fn streaming_links(html: &str, base: &str, start: NaiveDate) -> Vec<String> {
    let doc = Html::parse_document(html);
    let mut current = None;
    let mut links = BTreeSet::new();
    for e in doc.select(&sel(".entry-content h2, .entry-content figure a")) {
        if e.value().name() == "h2" {
            current = date(&text(e));
            continue;
        }
        if current.is_some_and(|d| in_week(d, start)) {
            if let Some(u) = e.value().attr("href").and_then(|h| absolute(base, h)) {
                links.insert(u);
            }
        }
    }
    links.into_iter().collect()
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
        if let Ok(metadata)=crate::movie_metadata::parse("dvdsreleasedates",html,url) {
            r["poster_metadata"]=json!({"title":metadata.identity.title,"year":metadata.identity.year,"imdb_id":metadata.identity.imdb_id,"tmdb_id":metadata.identity.tmdb_id,"posters":metadata.posters});
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
    if let Ok(metadata)=crate::movie_metadata::parse("whentostream",html,url) {
        for r in &mut rows {r["poster_metadata"]=json!({"title":metadata.identity.title,"year":metadata.identity.year,"imdb_id":metadata.identity.imdb_id,"tmdb_id":metadata.identity.tmdb_id,"posters":metadata.posters});}
    }
    Ok(rows)
}

#[allow(dead_code)] // Used by the standalone read-only example.
pub fn preview(start: NaiveDate) -> Result<Value, Box<dyn std::error::Error>> {
    collect(start, &[])
}

/// Refresh tracked movies as well as discovering this week's new releases.
pub fn ingest(start: NaiveDate, tracked: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    collect(start, tracked)
}

fn collect(start: NaiveDate, tracked: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; web_scraper/0.1)")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let mut calendars = BTreeMap::new();
    for offset in 0..7 {
        let d = start + Duration::days(offset);
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
    for (url, category) in calendars {
        eprintln!("calendar: {category}: {url}");
        let result = client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.text());
        match result {
            Ok(html) => {
                let links = if category == "subscription" {
                    streaming_links(&html, &url, start)
                } else {
                    dvd_links(&html, &url, start, category == "theatrical")
                };
                coverage.push(json!({"url":url,"category":category,"candidate_count":links.len()}));
                if links.is_empty() {
                    issues.push(json!({"url":url,"reason":"no candidates found; empty week or changed page markup requires review"}));
                }
                for link in links {
                    candidates.entry(link).or_default().insert(category);
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
        eprintln!("detail {}/{}: {}", index + 1, candidate_count, url);
        let result = client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.text());
        match result {
            Ok(html) => {
                let parsed = if categories.contains("subscription") {
                    streaming_details(&html, &url)
                } else {
                    dvd_details(&html, &url)
                };
                match parsed {
                    Ok(rows) => {
                        let found: Vec<_> = rows
                            .into_iter()
                            .filter(|r| {
                                if tracked.contains(&url) {
                                    return true;
                                }
                                r["release_date"]
                                    .as_str()
                                    .and_then(date)
                                    .is_some_and(|d| in_week(d, start))
                                    && r["release_type"]
                                        .as_str()
                                        .is_some_and(|t| categories.contains(t))
                            })
                            .collect();
                        if found.is_empty() {
                            no_matching_release.push(json!({"url":url,"categories":categories,"reason":"no announced detail date in the selected week/category"}));
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
        json!({"week_start":start.to_string(),"week_end":(start+Duration::days(6)).to_string(),"database_written":false,"candidate_count":candidate_count,"counts":counts,"calendars":coverage,"releases":releases,"issues":issues,"excluded_tv":excluded_tv,"no_matching_release":no_matching_release}),
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
            dvd_links(h, "https://www.dvdsreleasedates.com/", start(), false),
            vec!["https://www.dvdsreleasedates.com/movies/1/test"]
        );
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
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["release_date"], "2026-10-09");
        assert_eq!(rows[0]["platform"], "Paramount+");
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
                start()
            ),
            vec!["https://whentostream.com/movie-2026/"]
        );
    }
}
