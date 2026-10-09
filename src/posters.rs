//! Source-first poster enrichment is independent of movie/release upserts.
use crate::{
    db::Db,
    movie_metadata::{self as meta, MovieIdentity, SourceMovie},
};
use chrono::{NaiveDateTime, Utc};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone)]
pub struct PosterJob {
    pub movie_id: i32,
    pub link_id: i32,
    pub source: String,
    pub url: String,
    pub identity: MovieIdentity,
    pub existing: Option<String>,
    pub poster_status: Option<String>,
    pub checked: Option<NaiveDateTime>,
    pub retry: Option<NaiveDateTime>,
    pub cache_identity: Option<String>,
    pub cache_metadata: Option<String>,
    pub cache_poster: Option<String>,
    pub cache_retry: Option<NaiveDateTime>,
    pub cache_error: Option<String>,
}
pub fn signature(i: &MovieIdentity) -> String {
    if let Some(id) = &i.imdb_id {
        format!("imdb:{id}")
    } else if let Some(id) = i.tmdb_id {
        format!("tmdb:{id}")
    } else {
        format!(
            "title:{}|year:{:?}",
            meta::normalized_title(&i.title),
            i.year
        )
    }
}
pub fn encode(m: &SourceMovie) -> Value {
    json!({"title":m.identity.title,"year":m.identity.year,"imdb_id":m.identity.imdb_id,"tmdb_id":m.identity.tmdb_id,"posters":m.posters})
}
pub fn decode(v: &Value) -> Option<SourceMovie> {
    Some(SourceMovie {
        identity: MovieIdentity {
            title: v["title"].as_str()?.to_owned(),
            year: v["year"].as_i64().and_then(|y| i16::try_from(y).ok()),
            imdb_id: v["imdb_id"].as_str().map(str::to_owned),
            tmdb_id: v["tmdb_id"].as_i64().and_then(|id| i32::try_from(id).ok()),
        },
        posters: v["posters"]
            .as_array()?
            .iter()
            .filter_map(|u| u.as_str().map(str::to_owned))
            .collect(),
    })
}
fn direct_search_result(raw: &str) -> bool {
    url::Url::parse(raw).ok().is_some_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some("www.dvdsreleasedates.com")
            && u.path().starts_with("/movies/")
    })
}

pub fn search_links(html: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("#leftcolumn .fieldtable-light a[href^='/movies/']").unwrap();
    doc.select(&sel)
        .filter_map(|a| a.value().attr("href"))
        .filter_map(|h| {
            url::Url::parse("https://www.dvdsreleasedates.com/")
                .ok()?
                .join(h)
                .ok()
                .map(|u| u.to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn source_page(
    client: &Client,
    source: &str,
    url: &str,
) -> std::result::Result<SourceMovie, String> {
    let u = url::Url::parse(url).map_err(|e| e.to_string())?;
    let valid = match source {
        "dvdsreleasedates" => {
            matches!(
                u.host_str(),
                Some("www.dvdsreleasedates.com" | "dvdsreleasedates.com")
            ) && u.path().starts_with("/movies/")
        }
        "whentostream" => matches!(
            u.host_str(),
            Some("whentostream.com" | "www.whentostream.com")
        ),
        _ => false,
    };
    if !valid {
        return Err("source URL is not a supported movie detail page".into());
    }
    let html = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| e.to_string())?;
    meta::parse(source, &html, url)
}
fn check_image(client: &Client, raw: &str) -> std::result::Result<String, String> {
    let url = meta::image_url(raw, raw).ok_or("not an approved HTTPS poster URL")?;
    let mut r = client
        .get(&url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    if r.url().scheme() != "https" {
        return Err("image redirected away from HTTPS".into());
    }
    if !r
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t.starts_with("image/jpeg") || t.starts_with("image/png"))
    {
        return Err("poster response is not JPEG/PNG".into());
    }
    let mut bytes = Vec::new();
    r.by_ref()
        .take(5_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 5_000_000 {
        return Err("poster exceeds image size limit".into());
    }
    let (w, h) = meta::dimensions(&bytes).ok_or("image dimensions could not be validated")?;
    if !meta::portrait(w, h) {
        return Err(format!("not a portrait poster ({w}x{h})"));
    }
    Ok(url)
}
fn validate_candidates(
    client: &Client,
    m: &SourceMovie,
    memo: &mut BTreeMap<String, std::result::Result<String, String>>,
) -> (Option<String>, Option<String>) {
    let mut error = None;
    for url in &m.posters {
        let check = memo
            .entry(url.clone())
            .or_insert_with(|| check_image(client, url));
        match check {
            Ok(url) => return (Some(url.clone()), None),
            Err(e) => error = Some(e.clone()),
        }
    }
    (None, error)
}
fn search(
    client: &Client,
    target: &MovieIdentity,
    memo: &mut BTreeMap<String, std::result::Result<String, String>>,
) -> std::result::Result<Option<(SourceMovie, String)>, String> {
    if target.year.is_none() && target.imdb_id.is_none() && target.tmdb_id.is_none() {
        return Err("original year unavailable; title-only lookup rejected".into());
    }
    let response = client
        .post("https://www.dvdsreleasedates.com/search/")
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("searchStr", &target.title)
                .finish(),
        )
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let final_url = response.url().to_string();
    let html = response.text().map_err(|e| e.to_string())?;
    if direct_search_result(&final_url) {
        let mut m = meta::parse("dvdsreleasedates", &html, &final_url)?;
        if !meta::same_identity(target, &m.identity) {
            return Ok(None);
        }
        let (poster, _) = validate_candidates(client, &m, memo);
        m.posters = poster.into_iter().collect();
        return Ok((!m.posters.is_empty()).then_some((m, final_url)));
    }
    let links = search_links(&html);
    if links.len() > 20 {
        return Err("too many search results for safe matching".into());
    }
    let mut candidates = Vec::new();
    let mut urls = Vec::new();
    for url in links {
        let mut m = source_page(client, "dvdsreleasedates", &url)?;
        if meta::same_identity(target, &m.identity) {
            let (poster, _) = validate_candidates(client, &m, memo);
            m.posters = poster.into_iter().collect();
            urls.push(url);
            candidates.push(m);
        }
    }
    let Some(m) = meta::unique_match(target, &candidates) else {
        return Ok(None);
    };
    let index = candidates.iter().position(|c| std::ptr::eq(c, m)).unwrap();
    Ok(Some((m.clone(), urls[index].clone())))
}

pub fn enrich(db: &mut Db, inputs: &BTreeMap<String, SourceMovie>) -> Result<Value> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; web_scraper/0.1)")
        .build()?;
    let jobs = db.poster_jobs()?;
    let now = Utc::now().naive_utc();
    let mut memo = BTreeMap::new();
    let mut pool = Vec::new();
    let mut details = BTreeMap::new();
    let mut preserved = BTreeSet::new();
    let mut checked_existing = BTreeMap::new();
    let mut existing_errors = BTreeMap::new();
    let mut source_requests = 0;
    let mut cache_hits = 0;
    for job in &jobs {
        let target_sig = signature(&job.identity);
        let cached = job.cache_identity.as_deref() == Some(&target_sig)
            && job.cache_retry.is_some_and(|d| d > now);
        let cached_m = job
            .cache_metadata
            .as_deref()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| decode(&v));
        let fresh = inputs.get(&job.url).cloned();
        let (m, poster, error) = if cached
            && (fresh.is_none()
                || fresh
                    .as_ref()
                    .zip(cached_m.as_ref())
                    .is_some_and(|(a, b)| encode(a) == encode(b)))
        {
            cache_hits += 1;
            let m = cached_m.filter(|m| meta::bound_identity(&job.identity, &m.identity));
            let p = if m.is_some() {
                job.cache_poster.clone()
            } else {
                None
            };
            (m, p, job.cache_error.clone())
        } else {
            let found = if let Some(m) = fresh {
                Ok(m)
            } else {
                source_requests += 1;
                source_page(&client, &job.source, &job.url)
            };
            match found {
                Ok(m) if meta::bound_identity(&job.identity, &m.identity) => {
                    let (p, e) = validate_candidates(&client, &m, &mut memo);
                    db.cache_poster_source(
                        job.link_id,
                        &target_sig,
                        &encode(&m),
                        p.as_deref(),
                        e.as_deref(),
                    )?;
                    (Some(m), p, e)
                }
                Ok(m) => {
                    let e = "linked source identity disagrees with stored movie".to_owned();
                    db.cache_poster_source(job.link_id, &target_sig, &encode(&m), None, Some(&e))?;
                    (None, None, Some(e))
                }
                Err(e) => {
                    let empty = json!({"title":job.identity.title,"year":job.identity.year,"imdb_id":job.identity.imdb_id,"tmdb_id":job.identity.tmdb_id,"posters":[]});
                    db.cache_poster_source(job.link_id, &target_sig, &empty, None, Some(&e))?;
                    (None, None, Some(e))
                }
            }
        };
        if let Some(mut candidate) = m.clone() {
            candidate.posters = poster.clone().into_iter().collect();
            pool.push((job.movie_id, candidate, job.url.clone()));
        }
        details.insert(job.link_id, (m, poster, error));
        if !checked_existing.contains_key(&job.movie_id) {
            let existing = if let Some(raw) = &job.existing {
                if job
                    .checked
                    .is_some_and(|d| now - d < chrono::Duration::days(30))
                    && matches!(job.poster_status.as_deref(), Some("valid" | "matched"))
                    && meta::image_url(raw, raw).as_deref() == Some(raw)
                {
                    Some(raw.clone())
                } else if job.poster_status.as_deref() == Some("error")
                    && job.retry.is_some_and(|d| d > now)
                {
                    existing_errors.insert(
                        job.movie_id,
                        "cached transient image validation failure".to_owned(),
                    );
                    Some(raw.clone())
                } else {
                    match check_image(&client, raw) {
                        Ok(u) => Some(u),
                        Err(e)
                            if meta::image_url(raw, raw).as_deref() == Some(raw)
                                && [
                                    "sending request",
                                    "timed out",
                                    "connection",
                                    "500",
                                    "502",
                                    "503",
                                    "504",
                                    "429",
                                ]
                                .iter()
                                .any(|s| e.contains(s)) =>
                        {
                            existing_errors.insert(job.movie_id, e);
                            Some(raw.clone())
                        }
                        Err(_) => None,
                    }
                }
            } else {
                None
            };
            if existing.is_some() {
                preserved.insert(job.movie_id);
            }
            checked_existing.insert(job.movie_id, existing);
        }
    }
    let mut processed = BTreeSet::new();
    let mut enriched = Vec::new();
    let mut unmatched = Vec::new();
    for job in &jobs {
        if !processed.insert(job.movie_id) {
            continue;
        }
        let siblings: Vec<_> = jobs.iter().filter(|j| j.movie_id == job.movie_id).collect();
        let original_year = siblings
            .iter()
            .filter_map(|j| {
                details
                    .get(&j.link_id)
                    .and_then(|(m, _, _)| m.as_ref())
                    .filter(|m| meta::bound_identity(&job.identity, &m.identity))
                    .and_then(|m| m.identity.year)
            })
            .collect::<BTreeSet<_>>();
        let year = job
            .identity
            .year
            .or_else(|| (original_year.len() == 1).then(|| *original_year.iter().next().unwrap()));
        if let Some(url) = checked_existing.get(&job.movie_id).and_then(|u| u.as_ref()) {
            if let Some(error) = existing_errors.get(&job.movie_id) {
                if job.retry.is_none_or(|d| d <= now) {
                    db.set_movie_poster(
                        job.movie_id,
                        job.existing.as_deref(),
                        Some(url),
                        year,
                        "error",
                        None,
                        Some(error),
                        0,
                    )?;
                }
            } else if job
                .checked
                .is_none_or(|d| now - d >= chrono::Duration::days(30))
                || job.existing.as_ref() != Some(url)
            {
                db.set_movie_poster(
                    job.movie_id,
                    job.existing.as_deref(),
                    Some(url),
                    year,
                    "valid",
                    None,
                    None,
                    30,
                )?;
            }
            continue;
        }
        let mut identity = job.identity.clone();
        identity.year = year;
        let own = siblings.iter().find_map(|j| {
            details
                .get(&j.link_id)
                .and_then(|(_, p, _)| p.as_ref())
                .map(|p| (p.clone(), j.url.clone()))
        });
        let candidates: Vec<_> = pool
            .iter()
            .filter(|(id, _, _)| *id != job.movie_id)
            .map(|(_, m, _)| m.clone())
            .collect();
        let cross = meta::unique_match(&identity, &candidates)
            .and_then(|m| m.posters.first())
            .and_then(|p| {
                pool.iter()
                    .find(|(_, m, _)| {
                        meta::same_identity(&identity, &m.identity) && m.posters.first() == Some(p)
                    })
                    .map(|(_, _, u)| (p.clone(), u.clone()))
            });
        let mut picked = own.or(cross);
        let mut failure = siblings
            .iter()
            .filter_map(|j| details.get(&j.link_id).and_then(|(_, _, e)| e.clone()))
            .next();
        let due = job.retry.is_none_or(|d| d <= now);
        if picked.is_none() && due {
            source_requests += 1;
            match search(&client, &identity, &mut memo) {
                Ok(Some((m, url))) => picked = m.posters.first().cloned().map(|p| (p, url)),
                Ok(None) => {}
                Err(e) => failure = Some(e),
            }
        }
        if let Some((url, source)) = picked {
            let written = db.set_movie_poster(
                job.movie_id,
                job.existing.as_deref(),
                Some(&url),
                year,
                "matched",
                Some(&source),
                None,
                30,
            )?;
            if written {
                enriched.push(json!({"id":job.movie_id,"title":job.identity.title,"year":year,"poster_url":url,"source_url":source}));
            }
        } else {
            let reason = failure.unwrap_or_else(|| {
                "no unambiguous source poster for this title/original year".into()
            });
            if due || job.existing.is_some() {
                db.set_movie_poster(
                    job.movie_id,
                    job.existing.as_deref(),
                    None,
                    year,
                    "unmatched",
                    None,
                    Some(&reason),
                    7,
                )?;
            }
            unmatched.push(json!({"id":job.movie_id,"title":job.identity.title,"reason":reason}));
        }
    }
    Ok(
        json!({"enriched_count":enriched.len(),"preserved_count":preserved.len(),"source_requests":source_requests,"cache_hits":cache_hits,"enriched":enriched,"unmatched":unmatched}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_search_result_redirect_is_a_movie_detail_not_an_empty_list() {
        assert!(direct_search_result(
            "https://www.dvdsreleasedates.com/movies/12747/hokum"
        ));
        assert!(!direct_search_result(
            "https://www.dvdsreleasedates.com/search/"
        ));
        assert!(!direct_search_result(
            "https://unrelated.test/movies/12747/hokum"
        ));
    }
    #[test]
    fn search_sidebars_and_recommendations_are_excluded() {
        let html = "<div id='leftcolumn'><table class='fieldtable-light'><tr><td><a href='/movies/1/film'>Film (2026)</a></td></tr></table></div><div id='rightcolumn'><a href='/movies/2/wrong'>Recommended</a></div>";
        assert_eq!(
            search_links(html),
            vec!["https://www.dvdsreleasedates.com/movies/1/film"]
        );
    }
    #[test]
    fn cache_identity_changes_for_remakes_not_rescheduled_dates() {
        let a = MovieIdentity {
            title: "Film".into(),
            year: Some(1947),
            imdb_id: Some("tt0039152".into()),
            tmdb_id: None,
        };
        let mut b = a.clone();
        b.year = Some(2026);
        assert_eq!(signature(&a), signature(&b));
        b.imdb_id = Some("tt39850240".into());
        assert_ne!(signature(&a), signature(&b));
    }
}
