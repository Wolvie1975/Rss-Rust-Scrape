//! Optional retailer links. Release dates never prove retailer availability.
use crate::{db::Db, movie_metadata::MovieIdentity};
use scraper::{Html, Selector};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use url::Url;

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    FandangoTickets,
    Fandango,
}
impl Provider {
    pub fn key(self) -> &'static str {
        match self {
            Self::FandangoTickets => "fandango",
            Self::Fandango => "fandango-at-home",
        }
    }
    pub fn from_key(s: &str) -> Option<Self> {
        match s {
            "fandango" => Some(Self::FandangoTickets),
            "fandango-at-home" => Some(Self::Fandango),
            _ => None,
        }
    }
    pub fn search(self, title: &str) -> String {
        let (base, key, query) = match self {
            Self::FandangoTickets => ("https://www.fandango.com/search", "q", title.to_owned()),
            Self::Fandango => (
                "https://athome.fandango.com/content/browse/search",
                "searchString",
                title.to_owned(),
            ),
        };
        let mut u = Url::parse(base).unwrap();
        u.query_pairs_mut().append_pair(key, &query);
        u.to_string()
    }
    pub fn direct(self, raw: &str) -> Option<String> {
        let mut u = Url::parse(raw).ok()?;
        if u.scheme() != "https"
            || !u.username().is_empty()
            || u.password().is_some()
            || u.port().is_some()
        {
            return None;
        }
        let p = u.path_segments()?.collect::<Vec<_>>();
        let valid = match self {
            Self::FandangoTickets => {
                matches!(u.host_str(), Some("www.fandango.com" | "fandango.com"))
                    && p.len() == 2
                    && p[1] == "movie-overview"
                    && p[0].rsplit_once('-').is_some_and(|(slug, id)| {
                        !slug.is_empty() && !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
                    })
            }
            Self::Fandango => {
                u.host_str() == Some("athome.fandango.com")
                    && p.len() == 5
                    && p[..3] == ["content", "browse", "details"]
                    && !p[3].is_empty()
                    && !p[4].is_empty()
                    && p[4].chars().all(|c| c.is_ascii_digit())
            }
        };
        if !valid {
            return None;
        }
        u.set_fragment(None);
        u.set_query(None);
        Some(u.to_string())
    }
}

#[derive(Debug)]
pub struct Job {
    pub movie_id: i32,
    pub identity: MovieIdentity,
    pub provider: Provider,
    pub source_urls: Vec<String>,
    pub due: bool,
    pub cached_direct: Option<String>,
    pub verified_until: Option<chrono::NaiveDateTime>,
}
#[derive(Debug, Default)]
pub struct Outcome {
    pub direct_url: Option<String>,
    pub purchase: bool,
    pub error: Option<String>,
    pub reviewed_until: Option<chrono::NaiveDateTime>,
}

/// Explicitly reviewed retailer identities can survive a collector-side HTTP 403.
/// This registry never asserts purchase availability and cannot renew its review.
fn reviewed_mapping(raw: &str, job: &Job, now: chrono::DateTime<chrono::Utc>) -> Option<Outcome> {
    let rows: Vec<Value> = serde_json::from_str(raw).ok()?;
    let mut matches = Vec::new();
    for row in rows {
        if row["provider"].as_str() != Some(job.provider.key())
            || row["title"]
                .as_str()
                .map(crate::movie_metadata::normalized_title)
                != Some(crate::movie_metadata::normalized_title(&job.identity.title))
            || job.identity.year.map(i64::from) != row["original_year"].as_i64()
            || job.identity.year.is_none()
        {
            continue;
        }
        if let Some(id) = row["imdb_id"].as_str()
            && job.identity.imdb_id.as_deref() != Some(id)
        {
            continue;
        }
        let Some(url) = row["url"].as_str().and_then(|u| job.provider.direct(u)) else {
            continue;
        };
        let Some(reviewed) = row["reviewed_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        else {
            continue;
        };
        let Some(expires) = row["expires_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        else {
            continue;
        };
        if reviewed > now
            || expires <= now
            || expires <= reviewed
            || expires - reviewed > chrono::Duration::days(30)
            || row["evidence"].as_str().is_none_or(|s| s.trim().is_empty())
        {
            continue;
        }
        matches.push(Outcome {
            direct_url: Some(url),
            reviewed_until: Some(expires.naive_utc()),
            ..Default::default()
        });
    }
    (matches.len() == 1).then(|| matches.remove(0))
}

/// Rank exact title/year anchors before related films, bundles and other search hits.
fn matching_candidates(
    html: &str,
    base: &str,
    provider: Provider,
    identity: &MovieIdentity,
) -> Vec<String> {
    let Ok(base) = Url::parse(base) else {
        return vec![];
    };
    let expected = crate::movie_metadata::normalized_title(&identity.title);
    let mut matches = BTreeSet::new();
    for a in Html::parse_document(html).select(&Selector::parse("a[href]").unwrap()) {
        let Some(url) = a
            .value()
            .attr("href")
            .and_then(|h| base.join(h).ok())
            .and_then(|u| provider.direct(u.as_str()))
        else {
            continue;
        };
        let title = a
            .value()
            .attr("title")
            .map(str::to_owned)
            .unwrap_or_else(|| a.text().collect::<Vec<_>>().join(" "));
        let (title, year) = crate::movie_metadata::title_year(&title);
        if crate::movie_metadata::normalized_title(&title) == expected
            && (year.is_none() || identity.year.is_none() || year == identity.year)
        {
            matches.insert(url);
        }
    }
    matches.into_iter().collect()
}

pub fn candidates(html: &str, base: &str, provider: Provider) -> Vec<String> {
    let Ok(base) = Url::parse(base) else {
        return vec![];
    };
    Html::parse_document(html)
        .select(&Selector::parse("a[href]").unwrap())
        .filter_map(|a| base.join(a.value().attr("href")?).ok())
        .filter_map(|u| provider.direct(u.as_str()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn objects<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Array(a) => {
            for v in a {
                objects(v, out);
            }
        }
        Value::Object(m) => {
            out.push(v);
            for v in m.values() {
                objects(v, out);
            }
        }
        _ => {}
    }
}
/// Require movie-specific structured identity; a URL slug or search result is not evidence.
pub fn verified(html: &str, identity: &MovieIdentity) -> Option<bool> {
    let doc = Html::parse_document(html);
    let mut matches = Vec::new();
    let mut movie_nodes = 0;
    for script in doc.select(&Selector::parse("script[type='application/ld+json']").unwrap()) {
        let Ok(root) = serde_json::from_str::<Value>(&script.inner_html()) else {
            continue;
        };
        let mut nodes = vec![];
        objects(&root, &mut nodes);
        for m in nodes {
            if !(m["@type"] == "Movie"
                || m["@type"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|t| t == "Movie")))
            {
                continue;
            }
            movie_nodes += 1;
            let (movie_name, name_year) =
                crate::movie_metadata::title_year(m["name"].as_str().unwrap_or(""));
            let same_title = crate::movie_metadata::normalized_title(&movie_name)
                == crate::movie_metadata::normalized_title(&identity.title);
            let year = m["datePublished"]
                .as_str()
                .and_then(|s| s.get(..4))
                .and_then(|s| s.parse::<i16>().ok())
                .or(name_year);
            if name_year.is_some() && identity.year.is_some() && name_year != identity.year {
                continue;
            }
            let mut ids = vec![];
            for key in ["sameAs", "url"] {
                if let Some(s) = m[key].as_str() {
                    ids.push(s);
                }
                if let Some(a) = m[key].as_array() {
                    ids.extend(a.iter().filter_map(Value::as_str));
                }
            }
            let same_id = identity.imdb_id.as_ref().is_some_and(|id| {
                ids.iter()
                    .any(|s| crate::movie_metadata::imdb_id(s).as_ref() == Some(id))
            });
            let id_conflict = identity.imdb_id.as_ref().is_some_and(|id| {
                ids.iter()
                    .filter_map(|s| crate::movie_metadata::imdb_id(s))
                    .any(|other| &other != id)
            });
            if !same_title
                || id_conflict
                || (identity.year.is_some() && year.is_some() && identity.year != year)
                || !(same_id || identity.year.is_some() && identity.year == year)
            {
                continue;
            }
            let mut offers = vec![];
            objects(&m["offers"], &mut offers);
            let purchase = offers.iter().any(|o| {
                o["businessFunction"].as_str().is_some_and(|s| {
                    s == "http://purl.org/goodrelations/v1#Sell"
                        || s == "https://purl.org/goodrelations/v1#Sell"
                }) && o["availability"].as_str().is_some_and(|s| {
                    s == "https://schema.org/InStock" || s == "http://schema.org/InStock"
                }) && o["price"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| o["price"].as_f64())
                    .is_some_and(|n| n.is_finite() && n >= 0.0)
            });
            matches.push(purchase);
        }
    }
    if matches.len() == 1 {
        return Some(matches[0]);
    }
    if movie_nodes > 0 {
        return None;
    }
    // Public Fandango HTML can expose matching SEO title and H1 without
    // JSON-LD. Require the original year in BOTH, never infer it from the URL.
    let expected = identity.year.map(|year| {
        crate::movie_metadata::normalized_title(&format!("{} {year}", identity.title))
    })?;
    let heading = doc
        .select(&Selector::parse("h1").unwrap())
        .map(|e| e.text().collect::<String>())
        .collect::<Vec<_>>();
    let title = doc
        .select(&Selector::parse("title").unwrap())
        .next()?
        .text()
        .collect::<String>();
    (heading.len() == 1
        && crate::movie_metadata::normalized_title(&heading[0]) == expected
        && crate::movie_metadata::normalized_title(title.split('|').next().unwrap_or(""))
            == expected)
        .then_some(false)
}

fn lookup(
    client: &reqwest::blocking::Client,
    job: &Job,
    supplied: &[String],
    refreshed_sources: &BTreeSet<String>,
    deadline: Instant,
) -> Outcome {
    let mut links = supplied
        .iter()
        .filter_map(|u| job.provider.direct(u))
        .collect::<BTreeSet<_>>();
    let mut errors = Vec::new();
    // Sources already identify this exact movie; extract their retailer links.
    // Never search by title and silently select the first result.
    for source in job
        .source_urls
        .iter()
        .filter(|u| !refreshed_sources.contains(*u))
        .take(2)
    {
        if Instant::now() >= deadline {
            break;
        }
        match crate::http::get_text(client, source) {
            Ok(html) => links.extend(candidates(&html, source, job.provider)),
            Err(e) => errors.push(e.to_string()),
        }
    }
    if links.is_empty() && Instant::now() < deadline {
        // Discovery only; the retailer page must separately verify identity.
        let search = job.provider.search(&job.identity.title);
        match crate::http::get_text(client, &search) {
            Ok(html) => {
                links.extend(matching_candidates(
                    &html,
                    &search,
                    job.provider,
                    &job.identity,
                ));
            }
            Err(e) => errors.push(e.to_string()),
        }
    }
    let mut found = BTreeSet::new();
    let mut buy = false;
    for link in links.into_iter().take(3) {
        if Instant::now() >= deadline {
            break;
        }
        let result = (|| -> Result<Option<(String, bool)>> {
            let response = client.get(&link).send()?.error_for_status()?;
            let final_url = job
                .provider
                .direct(response.url().as_str())
                .ok_or("retailer redirected away from movie page")?;
            let html = crate::http::response_text(response)?;
            Ok(verified(&html, &job.identity).map(|purchase| (final_url, purchase)))
        })();
        match result {
            Ok(Some((url, purchase))) => {
                found.insert(url);
                buy |= purchase;
            }
            Ok(None) => {}
            Err(e) => errors.push(e.to_string()),
        }
    }
    if found.len() == 1 {
        Outcome {
            direct_url: found.into_iter().next(),
            purchase: buy,
            error: None,
            reviewed_until: None,
        }
    } else {
        Outcome {
            error: (!errors.is_empty()).then(|| errors.join("; ").chars().take(800).collect()),
            ..Outcome::default()
        }
    }
}

pub fn enrich(db: &mut Db, report: &Value) -> Result<usize> {
    let jobs = db.movie_commerce_jobs()?;
    let client = crate::http::client_builder()
        .user_agent("web_scraper/0.1")
        .timeout(Duration::from_secs(5))
        .build()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let refreshed_sources = report["releases"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["retailer_candidates"].is_array())
        .filter_map(|r| r["source_url"].as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    let mut checked = 0;
    for job in jobs {
        db.seed_movie_commerce(&job)?; // Every eligible movie gets a fallback immediately.
        if let Some(outcome) = reviewed_mapping(
            include_str!("movie_commerce_mappings.json"),
            &job,
            chrono::Utc::now(),
        ) {
            if job.due
                || job.cached_direct != outcome.direct_url
                || job.verified_until.is_none()
                || job.verified_until > outcome.reviewed_until
            {
                db.save_movie_commerce(&job, &outcome)?;
                eprintln!(
                    "movie retailer links: reviewed mapping movie_id={} provider={}",
                    job.movie_id,
                    job.provider.key()
                );
            }
            continue;
        }
        if !job.due || checked >= 12 || Instant::now() >= deadline {
            continue;
        }
        let supplied = report["releases"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| {
                r["source_url"]
                    .as_str()
                    .is_some_and(|u| job.source_urls.iter().any(|s| s == u))
            })
            .flat_map(|r| {
                r["retailer_candidates"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        let outcome = lookup(&client, &job, &supplied, &refreshed_sources, deadline);
        db.save_movie_commerce(&job, &outcome)?;
        checked += 1;
    }
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mapping_job() -> Job {
        Job {
            movie_id: 2185,
            identity: MovieIdentity {
                title: "Street Fighter".into(),
                year: Some(2026),
                ..Default::default()
            },
            provider: Provider::FandangoTickets,
            source_urls: vec![],
            due: false,
            cached_direct: None,
            verified_until: None,
        }
    }
    #[test]
    fn reviewed_mapping_handles_blocked_http_without_guessing_or_claiming_stock() {
        let raw = include_str!("movie_commerce_mappings.json");
        let entries: Value = serde_json::from_str(raw).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339(entries[0]["reviewed_at"].as_str().unwrap())
            .unwrap()
            .with_timezone(&chrono::Utc)
            + chrono::Duration::seconds(1);
        let job = mapping_job();
        let result = reviewed_mapping(raw, &job, now).unwrap();
        assert_eq!(
            result.direct_url.as_deref(),
            Some("https://www.fandango.com/street-fighter-2026-246472/movie-overview")
        );
        assert!(!result.purchase);
        assert!(result.reviewed_until.is_some());
        for job in [
            Job {
                identity: MovieIdentity {
                    year: Some(1994),
                    ..job.identity.clone()
                },
                ..mapping_job()
            },
            Job {
                identity: MovieIdentity {
                    year: None,
                    ..job.identity.clone()
                },
                ..mapping_job()
            },
            Job {
                provider: Provider::Fandango,
                ..mapping_job()
            },
        ] {
            assert!(reviewed_mapping(raw, &job, now).is_none());
        }
        assert!(reviewed_mapping(raw, &job, now + chrono::Duration::days(31)).is_none());
        assert!(reviewed_mapping(raw, &job, now - chrono::Duration::days(1)).is_none());
        let mut duplicate = entries.as_array().unwrap().clone();
        duplicate.push(duplicate[0].clone());
        assert!(reviewed_mapping(&serde_json::to_string(&duplicate).unwrap(), &job, now).is_none());
        let unsafe_url = raw.replace(
            "https://www.fandango.com/street-fighter-2026-246472/movie-overview",
            "https://evil.com/street-fighter-2026-246472/movie-overview",
        );
        assert!(reviewed_mapping(&unsafe_url, &job, now).is_none());
        let mut conflicting = entries.clone();
        conflicting[0]["imdb_id"] = serde_json::json!("tt9999999");
        assert!(reviewed_mapping(&conflicting.to_string(), &job, now).is_none());
    }
    #[test]
    fn discovery_does_not_spend_candidate_budget_on_unrelated_sidebar_links() {
        let html = r#"<a href="/other-123/movie-overview">Other (2026)</a><a href="/street-fighter-1994-456/movie-overview">Street Fighter (1994)</a><a href="/street-fighter-2026-246472/movie-overview">Street Fighter (2026)</a><a href="https://evil.com/street-fighter-2026-246472/movie-overview">Street Fighter (2026)</a>"#;
        assert_eq!(
            matching_candidates(
                html,
                "https://www.fandango.com/search?q=Street+Fighter",
                Provider::FandangoTickets,
                &mapping_job().identity
            ),
            vec!["https://www.fandango.com/street-fighter-2026-246472/movie-overview"]
        );
    }
    #[test]
    fn structured_movie_name_can_include_year() {
        let html = r#"<script type="application/ld+json">{"@type":"Movie","name":"Street Fighter (2026)","datePublished":"2026-10-16"}</script>"#;
        assert_eq!(verified(html, &mapping_job().identity), Some(false));
        assert_eq!(
            verified(
                &html.replace("Street Fighter (2026)", "Street Fighter (1994)"),
                &mapping_job().identity
            ),
            None
        );
    }
    #[test]
    fn search_encodes_titles_and_avoids_extra_parameters() {
        for p in [Provider::FandangoTickets, Provider::Fandango] {
            let u = Url::parse(&p.search("A & B #2? + café")).unwrap();
            assert_eq!(u.query_pairs().count(), 1);
            assert!(
                u.query_pairs()
                    .next()
                    .unwrap()
                    .1
                    .contains("A & B #2? + café")
            );
        }
    }
    #[test]
    fn retailer_url_boundaries() {
        assert!(
            Provider::FandangoTickets
                .direct("https://www.fandango.com/moana-2026-243429/movie-overview")
                .is_some()
        );
        assert!(
            Provider::Fandango
                .direct("https://athome.fandango.com/content/browse/details/Moana/5100835")
                .is_some()
        );
        for s in [
            "https://www.fandango.com.evil.com/moana-243429/movie-overview",
            "https://u@www.fandango.com/moana-243429/movie-overview",
            "http://www.fandango.com/moana-243429/movie-overview",
            "https://www.fandango.com/moana-243429/videos",
            "https://www.fandango.com/",
        ] {
            assert!(Provider::FandangoTickets.direct(s).is_none(), "{s}");
        }
        assert!(
            Provider::Fandango
                .direct("https://athome.fandango.com/content/browse/details/Bundle/not-an-id")
                .is_none()
        );
    }
    #[test]
    fn identity_is_verified_not_guessed_from_title() {
        let id = MovieIdentity {
            title: "Moana".into(),
            year: Some(2026),
            ..Default::default()
        };
        let html = r#"<script type="application/ld+json">{"@type":"Movie","name":"Moana","datePublished":"2026-07-10"}</script>"#;
        assert_eq!(verified(html, &id), Some(false));
        assert_eq!(verified(&html.replace("2026", "2016"), &id), None);
        assert_eq!(
            verified(
                html,
                &MovieIdentity {
                    year: None,
                    ..id.clone()
                }
            ),
            None
        );
        assert_eq!(verified(&format!("{html}{html}"), &id), None);
        assert_eq!(verified("<h1>Moana (2026)</h1>", &id), None);
    }
    #[test]
    fn rental_offer_does_not_prove_purchase() {
        let identity = MovieIdentity {
            title: "Film".into(),
            year: Some(2026),
            ..Default::default()
        };
        let html = r#"<script type="application/ld+json">{"@type":"Movie","name":"Film","datePublished":"2026-01-01","offers":{"businessFunction":"http://purl.org/goodrelations/v1#Sell","availability":"https://schema.org/InStock","price":"12.99"}}</script>"#;
        assert_eq!(verified(html, &identity), Some(true));
        assert_eq!(
            verified(&html.replace("#Sell", "#LeaseOut"), &identity),
            Some(false)
        );
        assert_eq!(
            verified(&html.replace("InStock", "OutOfStock"), &identity),
            Some(false)
        );
    }
    #[test]
    fn public_seo_metadata_requires_original_year_in_heading_and_title() {
        let identity = MovieIdentity {
            title: "Moana".into(),
            year: Some(2026),
            ..Default::default()
        };
        assert_eq!(
            verified(
                "<title>Moana 2026 | Stream on Fandango (Vudu)</title><h1>Moana 2026</h1>",
                &identity
            ),
            Some(false)
        );
        assert_eq!(
            verified(
                "<title>Moana 2016 | Stream on Fandango (Vudu)</title><h1>Moana 2026</h1>",
                &identity
            ),
            None
        );
        assert_eq!(
            verified(
                "<title>Moana | Stream on Fandango (Vudu)</title><h1>Moana 2026</h1>",
                &identity
            ),
            None
        );
    }
    #[test]
    fn source_links_are_only_candidates() {
        let html = r#"<a href="https://www.fandango.com/film-123/movie-overview">Tickets</a><a href="https://evil.com/movies/film-123">Buy</a>"#;
        assert_eq!(
            candidates(
                html,
                "https://www.movieinsider.com/m123/film",
                Provider::FandangoTickets
            )
            .len(),
            1
        );
        assert_eq!(verified(html, &MovieIdentity::default()), None);
    }
}
