//! Conservative, source-scoped article URL evidence. No content-similarity matching.
use crate::Result;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::{collections::BTreeSet, time::Duration};
use url::Url;
#[derive(Clone, Debug)]
pub struct Evidence {
    pub requested: String,
    pub target: String,
    pub aliases: Vec<String>,
    pub body: String,
}
pub fn client() -> Result<Client> {
    Ok(crate::http::client_builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .user_agent("My-Newsfeed/1.0")
        .build()?)
}
pub fn normalize(raw: &str) -> Result<String> {
    let mut url = crate::http::public_url(raw)?;
    url.set_fragment(None);
    let text = url.to_string();
    if text.encode_utf16().count() > 2048 {
        return Err("article URL exceeds SQL limit".into());
    }
    Ok(text)
}
fn same_publisher(a: &Url, b: &Url) -> bool {
    if a.host_str() == b.host_str() {
        return true;
    }
    // Explicit publisher aliases only; no suffix/eTLD guesswork.
    const ALIASES: &[&[&str]] = &[&["purexbox.com", "www.purexbox.com"]];
    ALIASES.iter().any(|hosts| {
        hosts.contains(&a.host_str().unwrap_or("")) && hosts.contains(&b.host_str().unwrap_or(""))
    })
}
fn article_target(original: &Url, target: &Url) -> bool {
    let parts: Vec<_> = target.path().split('/').filter(|p| !p.is_empty()).collect();
    same_publisher(original, target)
        && (original.port_or_known_default() == target.port_or_known_default()
            || (original.scheme() == "http"
                && target.scheme() == "https"
                && original.port_or_known_default() == Some(80)
                && target.port_or_known_default() == Some(443)))
        && !(original.scheme() == "https" && target.scheme() != "https")
        && (!parts.is_empty()
            || target.query_pairs().any(|(k, v)| {
                matches!(k.as_ref(), "id" | "p" | "article" | "story") && !v.is_empty()
            }))
        && !(parts.len() == 1
            && matches!(
                parts[0].to_ascii_lowercase().as_str(),
                "news" | "articles" | "home" | "index" | "index.html" | "index.php"
            ))
        && !parts.iter().any(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "login"
                    | "signin"
                    | "sign-in"
                    | "auth"
                    | "challenge"
                    | "captcha"
                    | "error"
                    | "404"
                    | "not-found"
                    | "search"
                    | "category"
                    | "tag"
                    | "account"
                    | "consent"
                    | "subscribe"
                    | "cdn-cgi"
                    | "challenge-platform"
                    | "access-denied"
                    | "page-not-found"
                    | "server-error"
                    | "temporarily-unavailable"
            )
        })
}
fn attr(document: &Html, selector: &str, attribute: &str) -> Option<String> {
    document
        .select(&Selector::parse(selector).ok()?)
        .next()?
        .value()
        .attr(attribute)
        .map(str::to_owned)
}
pub fn from_document(original: &str, chain: Vec<String>, body: String) -> Result<Evidence> {
    let original = Url::parse(&normalize(original)?)?;
    let final_url = Url::parse(chain.last().ok_or("article redirect chain empty")?)?;
    let document = Html::parse_document(&body);
    let article = attr(&document, "meta[property='og:type']", "content")
        .is_some_and(|s| s.eq_ignore_ascii_case("article"))
        || attr(
            &document,
            "meta[property='article:published_time']",
            "content",
        )
        .is_some();
    let title: String = document
        .select(&Selector::parse("title").unwrap())
        .next()
        .map(|e| e.text().collect::<String>().to_ascii_lowercase())
        .unwrap_or_default();
    if !article
        || matches!(
            title.trim(),
            "error" | "oops" | "home" | "welcome" | "login" | "404"
        )
        || [
            "just a moment",
            "access denied",
            "captcha",
            "sign in",
            "log in",
            "not found",
            "something went wrong",
            "service unavailable",
            "temporarily unavailable",
            "server error",
            "request blocked",
        ]
        .iter()
        .any(|s| title.contains(s))
    {
        return Err(
            "article identity rejected: missing article metadata or challenge/error page".into(),
        );
    }
    if !chain
        .iter()
        .all(|s| Url::parse(s).is_ok_and(|u| article_target(&original, &u)))
    {
        return Err("article identity rejected: unrelated or generic redirect target".into());
    }
    if chain.windows(2).any(|pair| {
        Url::parse(&pair[0]).is_ok_and(|a| a.scheme() == "https")
            && Url::parse(&pair[1]).is_ok_and(|b| b.scheme() != "https")
    }) {
        return Err("article HTTPS downgrade rejected".into());
    }
    let mut aliases: BTreeSet<String> = chain.into_iter().collect();
    let mut target = normalize(final_url.as_str())?;
    let canonical_selector = Selector::parse("link[rel~='canonical']").unwrap();
    let canonicals: Vec<_> = document
        .select(&canonical_selector)
        .filter_map(|el| el.value().attr("href"))
        .collect();
    if canonicals.len() == 1
        && !canonicals[0].trim().is_empty()
        && let Ok(url) = final_url.join(canonicals[0])
        && let Ok(normalized) = normalize(url.as_str())
        && let Ok(url) = Url::parse(&normalized)
        && article_target(&final_url, &url)
    {
        target = normalized.clone();
        aliases.insert(normalized);
    }
    Ok(Evidence {
        requested: original.to_string(),
        target,
        aliases: aliases.into_iter().collect(),
        body,
    })
}
pub fn fetch(client: &Client, raw: &str) -> Result<Evidence> {
    fetch_inner(client, raw, 1)
}
fn fetch_inner(client: &Client, raw: &str, canonical_depth: usize) -> Result<Evidence> {
    let requested = normalize(raw)?;
    let origin = Url::parse(&requested)?;
    let mut current = requested.clone();
    let mut chain = Vec::new();
    let started = std::time::Instant::now();
    for _ in 0..6 {
        if started.elapsed() > Duration::from_secs(10) {
            return Err("article identity deadline exceeded".into());
        }
        if chain.contains(&current) {
            return Err("article redirect loop".into());
        }
        chain.push(current.clone());
        let response = client.get(crate::http::public_url(&current)?).send()?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or("redirect Location missing")?
                .to_str()?;
            let current_url = Url::parse(&current)?;
            let next = current_url.join(location)?;
            if current_url.scheme() == "https" && next.scheme() != "https" {
                return Err("article HTTPS downgrade rejected".into());
            }
            current = normalize(next.as_str())?;
            if !article_target(&origin, &Url::parse(&current)?) {
                return Err("article redirect destination rejected".into());
            }
            continue;
        }
        let response = response.error_for_status()?;
        if !response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .is_some_and(|s| s.to_ascii_lowercase().contains("text/html"))
        {
            return Err("article identity requires an HTML response".into());
        }
        let body = crate::http::response_text(response)?;
        let final_url = current.clone();
        let mut evidence = from_document(&requested, chain, body)?;
        if evidence.target != final_url {
            let canonical = evidence.target.clone();
            let verified = if canonical_depth > 0 {
                fetch_inner(client, &canonical, canonical_depth - 1).ok()
            } else {
                None
            };
            if let Some(verified) = verified {
                evidence.aliases.extend(verified.aliases);
                evidence.target = verified.target;
                evidence.aliases.sort();
                evidence.aliases.dedup();
            } else {
                evidence.aliases.retain(|u| u != &canonical);
                evidence.target = final_url;
            }
        }
        return Ok(evidence);
    }
    Err("article redirect limit exceeded".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn article(canonical: &str) -> String {
        format!(
            "<html><head><title>Article</title><meta property='og:type' content='article'><link rel='canonical' href='{canonical}'></head><body>Article</body></html>"
        )
    }
    #[test]
    fn normalization_keeps_article_query_and_path_case() {
        assert_eq!(
            normalize("https://example.com/News/A?id=One&utm_source=x#fragment").unwrap(),
            "https://example.com/News/A?id=One&utm_source=x"
        );
        assert_ne!(
            normalize("https://example.com/News/A?id=One").unwrap(),
            normalize("https://example.com/news/a?id=one").unwrap()
        );
        assert!(normalize("https://user:pass@example.com/news/a").is_err());
    }
    #[test]
    fn canonicals_are_article_and_publisher_scoped() {
        let original = "https://www.purexbox.com/news/old";
        let e = from_document(
            original,
            vec![original.into()],
            article("https://purexbox.com/news/new"),
        )
        .unwrap();
        assert_eq!(e.target, "https://purexbox.com/news/new");
        for invalid in [
            "",
            "https://other.com/news/new",
            "/",
            "/account/login",
            "/news/error",
            "javascript:alert(1)",
            "http://127.0.0.1/x",
            "http://www.purexbox.com/news/new",
        ] {
            let e = from_document(original, vec![original.into()], article(invalid)).unwrap();
            assert_eq!(e.target, original, "{invalid}");
        }
        assert!(
            from_document(
                original,
                vec![original.into()],
                "<title>Just a moment</title>".into()
            )
            .is_err()
        );
        assert!(
            from_document(
                original,
                vec!["https://unrelated.com/news/x".into()],
                article("/news/new")
            )
            .is_err()
        );
    }
    fn server(
        responses: Vec<(&'static str, String)>,
    ) -> (Client, String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            for (expected, response) in responses {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = [0; 4096];
                let n = socket.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                assert!(
                    request.starts_with(&format!("GET {expected} ")),
                    "{request}"
                );
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve("publisher.test", address)
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        (
            client,
            format!("http://publisher.test:{}", address.port()),
            handle,
        )
    }
    fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }
    fn redirect(path: &str) -> String {
        format!(
            "HTTP/1.1 301 Moved\r\nLocation: {path}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
    }
    #[test]
    fn redirect_chain_and_canonical_are_verified_over_http() {
        let (client, base, handle) = server(vec![
            ("/news/old", redirect("/news/mid")),
            ("/news/mid", redirect("/news/new")),
            ("/news/new", ok(&article("/news/canonical"))),
            ("/news/canonical", ok(&article("/news/canonical"))),
        ]);
        let e = fetch(&client, &format!("{base}/news/old")).unwrap();
        assert_eq!(e.target, format!("{base}/news/canonical"));
        assert_eq!(e.aliases.len(), 4);
        handle.join().unwrap();
    }
    #[test]
    fn failed_canonical_does_not_authorize_merge() {
        let (client, base, handle) = server(vec![
            ("/news/valid", ok(&article("/news/canonical"))),
            (
                "/news/canonical",
                "HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
            ),
        ]);
        let e = fetch(&client, &format!("{base}/news/valid")).unwrap();
        assert_eq!(e.target, format!("{base}/news/valid"));
        assert_eq!(e.aliases.len(), 1);
        handle.join().unwrap();
    }
    #[test]
    fn loops_rate_limits_errors_and_generic_redirects_fail_closed() {
        for (path, response) in [
            ("/news/old", redirect("/news/old")),
            ("/news/old", redirect("/account/login")),
            ("/news/old", redirect("http://127.0.0.1/private")),
            (
                "/news/old",
                "HTTP/1.1 429 Limited\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
            ),
            ("/news/old", ok("<title>Sign in</title>")),
        ] {
            let (client, base, handle) = server(vec![(path, response)]);
            assert!(fetch(&client, &format!("{base}/news/old")).is_err());
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod timeout_test {
    #[test]
    fn stalled_response_cannot_produce_identity_evidence() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = socket.read(&mut request);
            std::thread::sleep(std::time::Duration::from_millis(100));
        });
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .resolve("publisher.test", address)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_millis(20))
            .build()
            .unwrap();
        assert!(
            super::fetch(
                &client,
                &format!("http://publisher.test:{}/news/stall", address.port())
            )
            .is_err()
        );
        thread.join().unwrap();
    }
}

#[cfg(test)]
mod downgrade_tests {
    #[test]
    fn upgrade_then_downgrade_is_not_evidence() {
        let body = "<title>Article</title><meta property='og:type' content='article'>".to_owned();
        assert!(
            super::from_document(
                "http://example.com/news/a",
                vec![
                    "http://example.com/news/a".into(),
                    "https://example.com/news/a".into(),
                    "http://example.com/news/b".into()
                ],
                body
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod single_segment_tests {
    #[test]
    fn article_metadata_allows_single_slugs_but_not_generic_destinations() {
        let old = "https://publisher.test/old-article";
        let body="<title>Updated story</title><meta property='og:type' content='article'><link rel='canonical' href='/new-article'>".to_owned();
        let e = super::from_document(old, vec![old.into()], body).unwrap();
        assert_eq!(e.target, "https://publisher.test/new-article");
        for path in [
            "/",
            "/news",
            "/home",
            "/login",
            "/challenge-platform",
            "/temporarily-unavailable",
        ] {
            let body = format!(
                "<title>Article</title><meta property='og:type' content='article'><link rel='canonical' href='{path}'>"
            );
            assert_eq!(
                super::from_document(old, vec![old.into()], body)
                    .unwrap()
                    .target,
                old
            );
        }
        let body = "<title>Oops</title><meta property='og:type' content='article'>".into();
        assert!(
            super::from_document(old, vec!["https://publisher.test/error-page".into()], body)
                .is_err()
        );
    }
}
