//! Shared poster enrichment for box-office results and domestic forecasts.
use crate::{
    db::Db,
    movie_metadata::{self as meta, MovieIdentity},
    posters,
};
use chrono::{NaiveDateTime, Utc};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct Job {
    pub section: String,
    pub title: String,
    pub year: Option<i16>,
    pub url: Option<String>,
}
pub struct Cache {
    pub poster: Option<String>,
    pub status: String,
    pub retry: NaiveDateTime,
}
pub fn url_year(raw: &str) -> Option<i16> {
    let u = url::Url::parse(raw).ok()?;
    if u.host_str() != Some("www.the-numbers.com") {
        return None;
    }
    let tail = u.path().rsplit_once('(')?.1;
    let year = tail.strip_suffix(')')?.parse::<i16>().ok()?;
    (1800..=9999).contains(&year).then_some(year)
}
fn number_movie(
    body: &str,
    url: &str,
    target: &MovieIdentity,
) -> std::result::Result<meta::SourceMovie, String> {
    let doc = scraper::Html::parse_document(body);
    let heading = doc
        .select(&scraper::Selector::parse("h1").unwrap())
        .next()
        .ok_or("movie heading missing")?
        .text()
        .collect::<String>();
    let (title, year) = meta::title_year(&heading);
    let identity = MovieIdentity {
        title,
        year,
        ..Default::default()
    };
    if !meta::same_identity(target, &identity) {
        return Err("movie title/year mismatch".into());
    }
    let posters = doc
        .select(&scraper::Selector::parse("img[src*='/movie-posters/']").unwrap())
        .filter_map(|i| meta::image_url(i.value().attr("src")?, url))
        .collect();
    Ok(meta::SourceMovie { identity, posters })
}
pub(crate) fn number_lookup(
    client: &reqwest::blocking::Client,
    target: &MovieIdentity,
    memo: &mut BTreeMap<String, std::result::Result<String, String>>,
) -> std::result::Result<Option<(meta::SourceMovie, String)>, String> {
    let Some(year) = target.year else {
        return Ok(None);
    };
    let slug = meta::normalized_title(&target.title).replace(' ', "-");
    let mut url = url::Url::parse("https://www.the-numbers.com/").unwrap();
    url.set_path(&format!("/movie/{slug}-({year})"));
    let body = crate::http::get_text(client, url.as_str()).map_err(|e| e.to_string())?;
    let mut movie = number_movie(&body, url.as_str(), target)?;
    let (poster, _) = posters::validate_candidates(client, &movie, memo);
    movie.posters = poster.into_iter().collect();
    Ok((!movie.posters.is_empty()).then_some((movie, url.to_string())))
}
pub fn enrich(db: &mut Db) -> Result<Value> {
    let client = crate::http::client_builder()
        .user_agent("My-Newsfeed/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let jobs = db.cinema_poster_jobs()?;
    let pool = db.cinema_release_posters()?;
    let mut caches = db.cinema_poster_caches()?;
    let mut memo = BTreeMap::new();
    let mut enriched = 0;
    let mut cached = 0;
    let mut missing = 0;
    let mut errors = 0;
    for job in jobs {
        let identity = MovieIdentity {
            title: job.title.clone(),
            year: job.year,
            ..Default::default()
        };
        let key = posters::signature(&identity);
        if caches
            .get(&key)
            .is_some_and(|c| c.retry > Utc::now().naive_utc())
        {
            let cache = &caches[&key];
            db.attach_cinema_poster(&job, &key, cache.poster.as_deref(), &cache.status)?;
            cached += 1;
            continue;
        }
        let existing = caches.get(&key).and_then(|c| c.poster.clone());
        let mut source = None;
        let mut error = None;
        let candidates = pool
            .iter()
            .filter(|(i, _)| meta::same_identity(&identity, i))
            .map(|(_, p)| p.clone())
            .collect::<BTreeSet<_>>();
        let mut poster = if candidates.len() == 1 {
            candidates.into_iter().next()
        } else {
            None
        };
        if poster.is_none()
            && let Some(url) = &job.url
        {
            match crate::http::get_text(&client, url)
                .map_err(|e| e.to_string())
                .and_then(|b| number_movie(&b, url, &identity))
            {
                Ok(m) => {
                    let (p, e) = posters::validate_candidates(&client, &m, &mut memo);
                    poster = p;
                    error = e;
                    source = Some(url.clone());
                }
                Err(e) => error = Some(e),
            }
        }
        if poster.is_none() {
            match posters::search(&client, &identity, &mut memo) {
                Ok(Some((m, url))) => {
                    poster = m.posters.first().cloned();
                    source = Some(url);
                    error = None;
                }
                Ok(None) => {}
                Err(e) => error = Some(e),
            }
        }
        // Transient refresh failures retain the last validated URL for the same identity.
        if poster.is_none() && error.is_some() {
            poster = existing;
        }
        let status = if poster.is_some() && error.is_none() {
            "matched"
        } else if error.is_some() {
            "error"
        } else {
            "missing"
        };
        let hours = if status == "matched" {
            720
        } else if status == "missing" {
            168
        } else {
            6
        };
        let retry = Utc::now().naive_utc() + chrono::Duration::hours(hours);
        db.save_cinema_poster(
            &key,
            &identity,
            poster.as_deref(),
            source.as_deref(),
            status,
            error.as_deref(),
            retry,
        )?;
        db.attach_cinema_poster(&job, &key, poster.as_deref(), status)?;
        if poster.is_some() {
            enriched += 1;
        } else {
            missing += 1;
        }
        if status == "error" {
            errors += 1;
        }
        caches.insert(
            key,
            Cache {
                poster,
                status: status.to_string(),
                retry,
            },
        );
    }
    Ok(json!({"enriched":enriched,"cache_hits":cached,"missing":missing,"errors":errors}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rerelease_uses_original_year() {
        assert_eq!(
            url_year("https://www.the-numbers.com/movie/Film-(1946)"),
            Some(1946)
        );
        assert_eq!(url_year("https://www.the-numbers.com/movie/Film"), None);
    }
    #[test]
    fn rejects_wrong_movie_and_landscape_editorial_image() {
        let target = MovieIdentity {
            title: "Film".into(),
            year: Some(2026),
            ..Default::default()
        };
        let m = number_movie(
            "<h1>Film (2026)</h1><img src='/images/movie-posters/film.jpg'><img src='/story.jpg'>",
            "https://www.the-numbers.com/movie/Film-(2026)",
            &target,
        )
        .unwrap();
        assert_eq!(m.posters.len(), 1);
        assert!(
            number_movie(
                "<h1>Film (1946)</h1>",
                "https://www.the-numbers.com/movie/Film-(1946)",
                &target
            )
            .is_err()
        );
    }
}
