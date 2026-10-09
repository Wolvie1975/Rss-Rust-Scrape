//! Movie-specific source metadata and conservative identity matching; no database/network access.
use scraper::{Html, Selector};
use url::Url;
#[derive(Debug, Clone, Default)]
pub struct MovieIdentity {
    pub title: String,
    pub year: Option<i16>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i32>,
}
#[derive(Debug, Clone)]
pub struct SourceMovie {
    pub identity: MovieIdentity,
    pub posters: Vec<String>,
}
fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}
fn text(e: scraper::ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn title_year(raw: &str) -> (String, Option<i16>) {
    let s = raw.trim();
    if let Some(s) = s.strip_suffix(')') {
        if let Some((title, year)) = s.rsplit_once('(') {
            if let Ok(year) = year.trim().parse::<i16>() {
                if (1800..=9999).contains(&year) {
                    return (title.trim().to_owned(), Some(year));
                }
            }
        }
    }
    (raw.trim().to_owned(), None)
}
pub fn normalized_title(raw: &str) -> String {
    raw.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn imdb_id(raw: &str) -> Option<String> {
    let u = Url::parse(raw).ok()?;
    if !matches!(u.host_str(), Some("imdb.com" | "www.imdb.com")) {
        return None;
    }
    let mut p = u.path_segments()?;
    if p.next()? != "title" {
        return None;
    }
    let id = p.next()?;
    let digits = id.strip_prefix("tt")?;
    (digits.len() >= 7 && digits.chars().all(|c| c.is_ascii_digit())).then(|| id.to_owned())
}
pub fn image_url(raw: &str, base: &str) -> Option<String> {
    let mut u = Url::parse(base).ok()?.join(raw.trim()).ok()?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return None;
    }
    u.set_scheme("https").ok()?;
    let host = u.host_str()?;
    if host == "localhost"
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || !host.contains('.')
    {
        return None;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v)
                if v.is_private() || v.is_loopback() || v.is_link_local() || v.is_unspecified() =>
            {
                return None;
            }
            std::net::IpAddr::V6(_) => return None,
            _ => {}
        }
    }
    let path = u.path().to_lowercase();
    if [
        "/logos/",
        "site-logo",
        "favicon",
        "placeholder",
        "no-poster",
        "noposter",
        "noimage",
        "no-image",
        "not-available",
        "not_available",
        "default-poster",
    ]
    .iter()
    .any(|s| path.contains(s))
    {
        return None;
    }
    Some(u.to_string())
}
pub fn parse(source: &str, html: &str, url: &str) -> Result<SourceMovie, String> {
    let doc = Html::parse_document(html);
    let heading = doc
        .select(&selector("h1"))
        .next()
        .map(text)
        .filter(|s| !s.is_empty())
        .ok_or("missing movie heading")?;
    let (mut title, year) = title_year(&heading);
    if source == "dvdsreleasedates" {
        title = doc
            .select(&selector("h1 [itemprop='name']"))
            .next()
            .map(text)
            .ok_or("missing DVD movie title")?;
    }
    let imdb = doc
        .select(&selector(
            "a[itemprop='sameAs'], .entry-content a[href*='imdb.com/title/']",
        ))
        .filter(|e| {
            source == "dvdsreleasedates"
                || normalized_title(&title_year(&text(*e)).0) == normalized_title(&title)
        })
        .filter_map(|e| e.value().attr("href"))
        .find_map(imdb_id);
    let tmdb = doc
        .select(&selector(".entry-content a[href*='themoviedb.org/movie/']"))
        .filter(|e| {
            source == "dvdsreleasedates"
                || normalized_title(&title_year(&text(*e)).0) == normalized_title(&title)
        })
        .filter_map(|e| e.value().attr("href"))
        .find_map(|s| {
            let u = Url::parse(s).ok()?;
            if !matches!(u.host_str(), Some("www.themoviedb.org" | "themoviedb.org")) {
                return None;
            }
            u.path()
                .strip_prefix("/movie/")?
                .split(['-', '/'])
                .next()?
                .parse::<i32>()
                .ok()
                .filter(|id| *id > 0)
        });
    let mut posters = Vec::new();
    let mut push = |raw: &str| {
        if let Some(u) = image_url(raw, url) {
            if !posters.contains(&u) {
                posters.push(u);
            }
        }
    };
    if source == "dvdsreleasedates" {
        // Dedicated portrait link belongs to this movie; recommended movies are excluded.
        for a in doc.select(&selector("a[data-lightbox]")) {
            if text(a).eq_ignore_ascii_case("Movie Poster")
                && a.value()
                    .attr("title")
                    .is_some_and(|t| normalized_title(t) == normalized_title(&title))
            {
                if let Some(h) = a.value().attr("href") {
                    push(h);
                }
            }
        }
        for img in doc.select(&selector("img[itemprop='image']")) {
            if let Some(alt) = img.value().attr("alt") {
                let alt = alt.strip_suffix(" DVD Release Date").unwrap_or(alt);
                let (alt_title, alt_year) = title_year(alt);
                if normalized_title(&alt_title) != normalized_title(&title)
                    || (alt_year.is_some() && year.is_some() && alt_year != year)
                {
                    continue;
                }
            } else {
                continue;
            }
            if let Some(h) = img.value().attr("src") {
                push(h);
            }
        }
    } else if source == "whentostream" {
        for img in doc.select(&selector(".entry-content img, img.wp-post-image")) {
            let label = format!(
                "{} {} {}",
                img.value().attr("alt").unwrap_or(""),
                img.value().attr("data-image-title").unwrap_or(""),
                img.value().attr("src").unwrap_or("")
            );
            if !label.to_lowercase().contains("poster") {
                continue;
            }
            if let (Some(w), Some(h)) = (
                img.value()
                    .attr("width")
                    .and_then(|s| s.parse::<u32>().ok()),
                img.value()
                    .attr("height")
                    .and_then(|s| s.parse::<u32>().ok()),
            ) {
                if !portrait(w, h) {
                    continue;
                }
            }
            if let Some(h) = img.value().attr("src") {
                push(h);
            }
        }
    } else {
        return Err("unsupported movie source".into());
    }
    Ok(SourceMovie {
        identity: MovieIdentity {
            title,
            year,
            imdb_id: imdb,
            tmdb_id: tmdb,
        },
        posters,
    })
}
pub fn same_identity(target: &MovieIdentity, candidate: &MovieIdentity) -> bool {
    if target.imdb_id.is_some()
        && candidate.imdb_id.is_some()
        && target.imdb_id != candidate.imdb_id
    {
        return false;
    }
    if target.tmdb_id.is_some()
        && candidate.tmdb_id.is_some()
        && target.tmdb_id != candidate.tmdb_id
    {
        return false;
    }
    if let Some(id) = &target.imdb_id {
        return candidate.imdb_id.as_ref() == Some(id);
    }
    if let Some(id) = target.tmdb_id {
        return candidate.tmdb_id == Some(id);
    }
    target.year.is_some()
        && target.year == candidate.year
        && normalized_title(&target.title) == normalized_title(&candidate.title)
}
/// A linked provider detail page can supply a missing original year; release dates never do.
pub fn bound_identity(target: &MovieIdentity, candidate: &MovieIdentity) -> bool {
    if target.imdb_id.is_some() || target.tmdb_id.is_some() {
        return same_identity(target, candidate);
    }
    if target.year.is_some() {
        return same_identity(target, candidate);
    }
    candidate.year.is_some()
        && normalized_title(&target.title) == normalized_title(&candidate.title)
}
pub fn unique_match<'a>(
    target: &MovieIdentity,
    candidates: &'a [SourceMovie],
) -> Option<&'a SourceMovie> {
    let matches: Vec<_> = candidates
        .iter()
        .filter(|c| same_identity(target, &c.identity))
        .collect();
    if target.imdb_id.is_some() || target.tmdb_id.is_some() {
        return matches.into_iter().find(|c| !c.posters.is_empty());
    }
    if matches.len() == 1 {
        return matches.into_iter().find(|c| !c.posters.is_empty());
    }
    let same_imdb = matches
        .first()
        .and_then(|m| m.identity.imdb_id.as_ref())
        .is_some_and(|id| {
            matches
                .iter()
                .all(|m| m.identity.imdb_id.as_ref() == Some(id))
        });
    let same_tmdb = matches
        .first()
        .and_then(|m| m.identity.tmdb_id)
        .is_some_and(|id| matches.iter().all(|m| m.identity.tmdb_id == Some(id)));
    if same_imdb || same_tmdb {
        matches.into_iter().find(|c| !c.posters.is_empty())
    } else {
        None
    }
}
pub fn portrait(w: u32, h: u32) -> bool {
    w >= 150
        && h >= 220
        && u64::from(h) * 10 >= u64::from(w) * 12
        && u64::from(h) * 10 <= u64::from(w) * 22
}
pub fn dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") && data.len() >= 24 {
        return Some((
            u32::from_be_bytes(data[16..20].try_into().ok()?),
            u32::from_be_bytes(data[20..24].try_into().ok()?),
        ));
    }
    if !data.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xff {
            return None;
        }
        while i < data.len() && data[i] == 0xff {
            i += 1;
        }
        let marker = *data.get(i)?;
        i += 1;
        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if marker == 0xd8 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let len = u16::from_be_bytes(data.get(i..i + 2)?.try_into().ok()?) as usize;
        if len < 2 || i + len > data.len() {
            return None;
        }
        if [
            0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf,
        ]
        .contains(&marker)
            && len >= 7
        {
            let h = u16::from_be_bytes(data.get(i + 3..i + 5)?.try_into().ok()?);
            let w = u16::from_be_bytes(data.get(i + 5..i + 7)?.try_into().ok()?);
            return Some((w.into(), h.into()));
        }
        i += len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity(title: &str, year: Option<i16>, imdb: Option<&str>) -> MovieIdentity {
        MovieIdentity {
            title: title.into(),
            year,
            imdb_id: imdb.map(str::to_owned),
            tmdb_id: None,
        }
    }
    fn candidate(title: &str, year: i16, imdb: Option<&str>, poster: bool) -> SourceMovie {
        SourceMovie {
            identity: identity(title, Some(year), imdb),
            posters: if poster {
                vec!["https://image.tmdb.org/t/p/w500/film.jpg".into()]
            } else {
                vec![]
            },
        }
    }
    #[test]
    fn streaming_related_movie_links_do_not_supply_this_movies_ids() {
        let html = "<h1>Actual Film (2026)</h1><div class='entry-content'><a href='https://www.imdb.com/title/tt1234567/'>Other Movie</a><a href='https://www.themoviedb.org/movie/123-other'>Other Movie</a></div>";
        let m = parse(
            "whentostream",
            html,
            "https://whentostream.com/actual-film-2026/",
        )
        .unwrap();
        assert!(m.identity.imdb_id.is_none());
        assert!(m.identity.tmdb_id.is_none());
    }
    #[test]
    fn remake_ids_and_years_do_not_cross_match() {
        let original = identity("Moana", Some(2016), Some("tt3521164"));
        let remake = identity("Moana", Some(2026), Some("tt27419466"));
        assert!(!same_identity(&original, &remake));
        assert!(!same_identity(
            &identity("Moana", Some(2016), None),
            &remake
        ));
        assert!(!same_identity(&identity("Moana", None, None), &remake));
    }
    #[test]
    fn re_release_uses_verified_id_not_current_release_year() {
        let original = identity("Angel and the Badman", Some(1947), Some("tt0039152"));
        let mut rerelease = original.clone();
        rerelease.year = Some(2026);
        assert!(same_identity(&original, &rerelease));
        assert!(!same_identity(
            &original,
            &identity("Angel and the Badman", Some(2026), Some("tt39850240"))
        ));
    }
    #[test]
    fn ambiguous_title_year_including_missing_posters_is_rejected() {
        let target = identity("Same Name", Some(2026), None);
        let c = vec![
            candidate("Same Name", 2026, Some("tt1234567"), true),
            candidate("Same Name", 2026, Some("tt7654321"), false),
        ];
        assert!(unique_match(&target, &c).is_none());
        let target = identity("Same Name", Some(2026), Some("tt1234567"));
        assert!(unique_match(&target, &c).is_some());
    }
    #[test]
    fn tmdb_id_matching_rejects_conflicting_ids() {
        let mut a = identity("Film", Some(2026), None);
        a.tmdb_id = Some(123);
        let mut b = a.clone();
        b.tmdb_id = Some(456);
        assert!(!same_identity(&a, &b));
        assert!(same_identity(&a, &a));
    }
    #[test]
    fn original_year_comes_from_heading_not_disc_date() {
        let html = "<h1><span itemprop='name'>Old Film</span> (1947)</h1><span>DVD October 6, 2026</span><a itemprop='sameAs' href='https://www.imdb.com/title/tt0039152/'></a><img itemprop='image' src='/posters/300/O/Old-Film-1947.jpg' alt='Old Film (1947) DVD Release Date'><img class='movieimg' src='/posters/300/O/Old-Film-2026.jpg' alt='Old Film (2026) DVD Release Date'>";
        let m = parse(
            "dvdsreleasedates",
            html,
            "https://www.dvdsreleasedates.com/movies/123/old-film",
        )
        .unwrap();
        assert_eq!(m.identity.year, Some(1947));
        assert_eq!(m.identity.imdb_id.as_deref(), Some("tt0039152"));
        assert_eq!(m.posters.len(), 1);
        assert!(m.posters[0].contains("1947"));
    }
    #[test]
    fn generic_and_landscape_images_are_not_posters() {
        assert!(
            image_url(
                "https://example.com/images/logos/site/site.png",
                "https://example.com"
            )
            .is_none()
        );
        assert!(
            image_url(
                "/posters/no-poster.jpg",
                "https://www.dvdsreleasedates.com/"
            )
            .is_none()
        );
        assert!(image_url("data:image/png;base64,AAAA", "https://example.com").is_none());
        let html = "<h1>Film (2026)</h1><meta property='og:image' content='https://whentostream.com/site-logo.png'><img class='wp-post-image' width='1370' height='770' src='https://whentostream.com/Film-Poster-Horizontal.jpeg'>";
        assert!(
            parse("whentostream", html, "https://whentostream.com/film-2026/")
                .unwrap()
                .posters
                .is_empty()
        );
        assert!(!portrait(1370, 770));
        assert!(!portrait(150, 150));
        assert!(portrait(800, 1200));
    }
    #[test]
    fn unrelated_image_alt_and_imdb_domains_are_rejected() {
        let html = "<h1><span itemprop='name'>Film</span> (2026)</h1><img itemprop='image' src='/posters/300/O/Other.jpg' alt='Other Film (2026) DVD Release Date'>";
        assert!(
            parse(
                "dvdsreleasedates",
                html,
                "https://www.dvdsreleasedates.com/movies/1/film"
            )
            .unwrap()
            .posters
            .is_empty()
        );
        assert!(imdb_id("https://evil.test/title/tt1234567/").is_none());
        assert_eq!(
            normalized_title("Don’t Look Back"),
            normalized_title("Don't Look Back")
        );
        assert_ne!(normalized_title("The Film"), normalized_title("Film"));
    }
    #[test]
    fn png_and_jpeg_dimensions_are_read_safely() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[16..20].copy_from_slice(&300u32.to_be_bytes());
        png[20..24].copy_from_slice(&450u32.to_be_bytes());
        assert_eq!(dimensions(&png), Some((300, 450)));
        let jpeg = [0xff, 0xd8, 0xff, 0xc0, 0, 7, 8, 1, 194, 1, 44];
        assert_eq!(dimensions(&jpeg), Some((300, 450)));
        assert_eq!(dimensions(&jpeg[..5]), None);
    }
}
