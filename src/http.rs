//! Public HTTP boundaries shared by collectors. No proxy bypass of DNS checks.
use reqwest::blocking::{Client, ClientBuilder, Response};
use std::{
    io::Read,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use url::Url;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static TEXT_RESPONSES: AtomicU64 = AtomicU64::new(0);
static TEXT_BYTES: AtomicU64 = AtomicU64::new(0);
pub fn text_metrics() -> (u64, u64) {
    (
        TEXT_RESPONSES.load(Ordering::Relaxed),
        TEXT_BYTES.load(Ordering::Relaxed),
    )
}
pub const TEXT_LIMIT: u64 = 8 * 1024 * 1024;
pub const JSON_LIMIT: u64 = 32 * 1024 * 1024;
pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let [a, b, _, _] = v.octets();
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_multicast()
                || v.is_broadcast()
                || v.is_documentation()
                || a == 0
                || a >= 240
                || (a == 100 && (64..=127).contains(&b))
                || (a == 198 && (b == 18 || b == 19)))
        }
        IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4() {
                return public_ip(IpAddr::V4(v4));
            }
            let s = v.segments();
            !(v.is_loopback()
                || v.is_unspecified()
                || v.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                || (s[0] == 0x2001 && s[1] == 0xdb8))
        }
    }
}
pub fn public_url(raw: &str) -> Result<Url> {
    let u = Url::parse(raw)?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("URL must be HTTP(S) without credentials".into());
    }
    let host = u.host_str().ok_or("URL has no host")?;
    match u.host() {
        Some(url::Host::Ipv4(ip)) if !public_ip(ip.into()) => {
            return Err("private/reserved URL destination rejected".into());
        }
        Some(url::Host::Ipv6(ip)) if !public_ip(ip.into()) => {
            return Err("private/reserved URL destination rejected".into());
        }
        Some(url::Host::Domain(_))
            if !host.contains('.')
                || host.ends_with(".local")
                || host.ends_with(".internal")
                || host.ends_with(".localhost") =>
        {
            return Err("internal URL destination rejected".into());
        }
        _ => {}
    }
    Ok(u)
}
struct PublicResolver;
impl reqwest::dns::Resolve for PublicResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addresses =
                tokio::time::timeout(Duration::from_secs(5), tokio::net::lookup_host((host, 0)))
                    .await??
                    .collect::<Vec<SocketAddr>>();
            if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
                return Err("DNS returned a private/reserved destination".into());
            }
            Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
        })
    }
}
pub fn client_builder() -> ClientBuilder {
    Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .dns_resolver(Arc::new(PublicResolver))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("redirect limit exceeded")
            } else if public_url(attempt.url().as_str()).is_err() {
                attempt.error("unsafe redirect rejected")
            } else if attempt
                .previous()
                .last()
                .is_some_and(|u| u.scheme() == "https")
                && attempt.url().scheme() != "https"
            {
                attempt.error("HTTPS downgrade rejected")
            } else {
                attempt.follow()
            }
        }))
}
pub fn response_text(response: Response) -> Result<String> {
    response_text_limit(response, TEXT_LIMIT)
}
pub fn response_text_limit(mut response: Response, limit: u64) -> Result<String> {
    TEXT_RESPONSES.fetch_add(1, Ordering::Relaxed);
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("HTTP response exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    response.by_ref().take(limit + 1).read_to_end(&mut bytes)?;
    TEXT_BYTES.fetch_add(bytes.len() as u64, Ordering::Relaxed);
    if bytes.len() as u64 > limit {
        return Err("HTTP response exceeds size limit".into());
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
pub fn get_text(client: &Client, raw: &str) -> Result<String> {
    let url = public_url(raw)?;
    response_text(client.get(url).send()?.error_for_status()?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_unsafe_targets() {
        for u in [
            "javascript:alert(1)",
            "http://127.1/a",
            "http://[::1]/a",
            "http://[::ffff:127.0.0.1]/a",
            "http://10.0.0.2/a",
            "http://169.254.169.254/a",
            "https://user:pass@example.com/",
            "http://sql2025/a",
            "http://foo.internal/a",
        ] {
            assert!(public_url(u).is_err(), "{u}")
        }
        assert!(public_url("https://www.espn.com/mlb/").is_ok());
    }
    #[test]
    fn reject_oversized_body_without_content_length() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let a = l.local_addr().unwrap();
        let t = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut b = [0; 1024];
            assert!(s.read(&mut b).unwrap() > 0);
            s.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n123456789")
                .unwrap();
        });
        let r = Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{a}"))
            .send()
            .unwrap();
        assert!(response_text_limit(r, 8).is_err());
        t.join().unwrap();
    }
}
