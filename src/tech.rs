//! Collectors for the Tech section: Release Tracker, Dev Reading, and Service Status.

use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;

use crate::Result;

pub const USER_AGENT: &str = "myNewspaper-scraper/1.0 (+https://mynewspaper.local)";

// ==========================================
// 1. Release Tracker
// ==========================================

#[derive(Debug, Clone)]
pub struct ReleaseProject {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub source_kind: String,
    pub source_url: String,
    pub source_filter: Option<String>,
    pub notes_label: String,
    pub include_prerelease: bool,
    pub in_your_stack: bool,
}

#[derive(Debug, Clone)]
pub struct ReleaseItem {
    pub project_id: i32,
    pub version: String,
    pub released_at: NaiveDateTime,
    pub notes_url: Option<String>,
    pub title: Option<String>,
    pub is_prerelease: bool,
}

pub fn strip_version_prefix(raw: &str) -> String {
    let s = raw.trim();
    if let Some(stripped) = s.strip_prefix('v').or_else(|| s.strip_prefix('V')) {
        stripped.trim().to_string()
    } else {
        s.to_string()
    }
}

pub fn collect_kernel_org(
    _client: &Client,
    body: &str,
    projects: &[ReleaseProject],
) -> Result<Vec<ReleaseItem>> {
    let json: Value = serde_json::from_str(body)?;
    let releases = json["releases"]
        .as_array()
        .ok_or("kernel.org missing releases array")?;

    let mut out = Vec::new();
    let now = Utc::now().naive_utc();

    for p in projects {
        if p.source_kind != "kernel_org" {
            continue;
        }
        let moniker_filter = p.source_filter.as_deref().unwrap_or("mainline");

        let mut matching: Vec<&Value> = releases
            .iter()
            .filter(|r| r["moniker"].as_str() == Some(moniker_filter))
            .collect();

        if matching.is_empty() {
            continue;
        }

        if moniker_filter == "longterm" {
            // Take the highest version
            matching.sort_by(|a, b| {
                let va = a["version"].as_str().unwrap_or("");
                let vb = b["version"].as_str().unwrap_or("");
                compare_semver(va, vb)
            });
        }

        // Take matching items
        for r in matching {
            let ver_raw = match r["version"].as_str() {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };
            let ver = strip_version_prefix(ver_raw);

            let released_at = if let Some(isodate) = r["released"]["isodate"].as_str() {
                chrono::NaiveDate::parse_from_str(isodate, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .unwrap_or(now)
            } else {
                now
            };

            let notes_url = r["changelog"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| r["gitweb"].as_str())
                .or_else(|| r["source"].as_str())
                .map(String::from);

            out.push(ReleaseItem {
                project_id: p.id,
                version: ver,
                released_at,
                notes_url,
                title: Some(format!("Linux {}", ver_raw)),
                is_prerelease: false,
            });

            if moniker_filter == "longterm" || moniker_filter == "mainline" {
                // If only highest wanted per project row:
                break;
            }
        }
    }

    Ok(out)
}

fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let parse_nums = |s: &str| -> Vec<u64> {
        s.split(['.', '-', '_'])
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };
    parse_nums(b).cmp(&parse_nums(a))
}

pub fn collect_github_releases(
    _client: &Client,
    body: &str,
    project: &ReleaseProject,
) -> Result<Vec<ReleaseItem>> {
    let now = Utc::now().naive_utc();
    let mut out = Vec::new();

    // Check if body is JSON (e.g. GitHub Releases API)
    if let Ok(json) = serde_json::from_str::<Value>(body)
        && let Some(arr) = json.as_array()
    {
        for r in arr {
            let is_pre = r["prerelease"].as_bool().unwrap_or(false);
            let is_draft = r["draft"].as_bool().unwrap_or(false);
            if is_draft || (is_pre && !project.include_prerelease) {
                continue;
            }

            let tag_raw = r["tag_name"].as_str().unwrap_or("");
            let name_raw = r["name"].as_str().unwrap_or("");
            let ver_str = if !tag_raw.is_empty() {
                tag_raw
            } else {
                name_raw
            };

            let tag_lower = ver_str.to_ascii_lowercase();
            let is_pre_name = tag_lower.contains("rc")
                || tag_lower.contains("beta")
                || tag_lower.contains("alpha")
                || tag_lower.contains("pre")
                || tag_lower.contains("bleeding-edge")
                || tag_lower.contains("experimental");

            if is_pre_name && !project.include_prerelease {
                continue;
            }

            let mut ver = ver_str.to_string();
            if let Some(stripped) = ver.strip_prefix(&project.name) {
                ver = stripped.trim_start_matches(['-', '_', ' ']).to_string();
            }
            if let Some(stripped) = ver.strip_prefix(&project.slug) {
                ver = stripped.trim_start_matches(['-', '_', ' ']).to_string();
            }
            ver = strip_version_prefix(&ver);

            if ver.is_empty() {
                continue;
            }

            let released_at = r["published_at"]
                .as_str()
                .or_else(|| r["created_at"].as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.naive_utc())
                .unwrap_or(now);

            let notes_url = r["html_url"].as_str().map(String::from);
            let title = if name_raw.is_empty() {
                None
            } else {
                Some(name_raw.to_string())
            };

            out.push(ReleaseItem {
                project_id: project.id,
                version: ver,
                released_at,
                notes_url,
                title,
                is_prerelease: is_pre || is_pre_name,
            });
        }
        return Ok(out);
    }

    let doc = roxmltree::Document::parse(body)?;

    for entry in doc.descendants().filter(|n| n.has_tag_name("entry")) {
        let title = entry
            .descendants()
            .find(|n| n.has_tag_name("title"))
            .and_then(|n| n.text())
            .unwrap_or("");

        let updated_str = entry
            .descendants()
            .find(|n| n.has_tag_name("updated"))
            .and_then(|n| n.text());

        let href = entry
            .descendants()
            .find(|n| n.has_tag_name("link"))
            .and_then(|n| n.attribute("href"))
            .map(String::from);

        let id_str = entry
            .descendants()
            .find(|n| n.has_tag_name("id"))
            .and_then(|n| n.text())
            .unwrap_or("");

        // Determine tag or version text
        let tag = if let Some(idx) = id_str.rfind('/') {
            &id_str[idx + 1..]
        } else {
            title
        };

        let tag_lower = tag.to_ascii_lowercase();
        let is_pre = tag_lower.contains("rc")
            || tag_lower.contains("beta")
            || tag_lower.contains("alpha")
            || tag_lower.contains("pre")
            || tag_lower.contains("bleeding-edge")
            || tag_lower.contains("experimental");

        if is_pre && !project.include_prerelease {
            continue;
        }

        // Strip leading 'v' and any project-name prefix, e.g. "proton-11.0-2" -> "11.0-2"
        let mut ver = tag.to_string();
        if let Some(stripped) = ver.strip_prefix(&project.name) {
            ver = stripped.trim_start_matches(['-', '_', ' ']).to_string();
        }
        if let Some(stripped) = ver.strip_prefix(&project.slug) {
            ver = stripped.trim_start_matches(['-', '_', ' ']).to_string();
        }
        ver = strip_version_prefix(&ver);

        if ver.is_empty() {
            continue;
        }

        let released_at = updated_str
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.naive_utc())
            .unwrap_or(now);

        out.push(ReleaseItem {
            project_id: project.id,
            version: ver,
            released_at,
            notes_url: href,
            title: if title.is_empty() {
                None
            } else {
                Some(title.to_string())
            },
            is_prerelease: is_pre,
        });
    }

    Ok(out)
}

pub fn collect_gitlab_tags(
    _client: &Client,
    body: &str,
    project: &ReleaseProject,
) -> Result<Vec<ReleaseItem>> {
    let json: Value = serde_json::from_str(body)?;
    let tags = json
        .as_array()
        .ok_or("gitlab tags response is not an array")?;
    let mut out = Vec::new();
    let now = Utc::now().naive_utc();

    let re = if let Some(filter) = &project.source_filter {
        Some(Regex::new(filter)?)
    } else {
        None
    };

    for t in tags {
        let name = match t["name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let name_lower = name.to_ascii_lowercase();
        let is_pre = name_lower.contains("rc")
            || name_lower.contains("beta")
            || name_lower.contains("alpha")
            || name_lower.contains("pre");

        if is_pre && !project.include_prerelease {
            continue;
        }

        let ver = if let Some(ref r) = re {
            if let Some(caps) = r.captures(name) {
                if let Some(m) = caps.get(1) {
                    m.as_str().to_string()
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            strip_version_prefix(name)
        };

        let created_at_str = t["commit"]["created_at"]
            .as_str()
            .or_else(|| t["created_at"].as_str());

        let released_at = created_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.naive_utc())
            .unwrap_or(now);

        let notes_url = if let Some(web_url) = t["web_url"].as_str() {
            Some(web_url.to_string())
        } else if let Ok(u) = Url::parse(&project.source_url) {
            let host = u.host_str().unwrap_or("gitlab.com");
            Some(format!("https://{host}/tags/{name}"))
        } else {
            None
        };

        out.push(ReleaseItem {
            project_id: project.id,
            version: ver,
            released_at,
            notes_url,
            title: Some(name.to_string()),
            is_prerelease: is_pre,
        });
    }

    Ok(out)
}

pub fn collect_rss_releases(
    _client: &Client,
    body: &str,
    project: &ReleaseProject,
) -> Result<Vec<ReleaseItem>> {
    let feed = feed_rs::parser::Builder::new()
        .build()
        .parse(body.as_bytes())?;
    let mut out = Vec::new();
    let now = Utc::now().naive_utc();

    let re = if let Some(filter) = &project.source_filter {
        Some(Regex::new(filter)?)
    } else {
        None
    };

    for entry in feed.entries {
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.as_str())
            .unwrap_or("");
        let ver = if let Some(ref r) = re {
            if let Some(caps) = r.captures(title) {
                if let Some(m) = caps.get(1) {
                    m.as_str().to_string()
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            strip_version_prefix(title)
        };

        let link = entry
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("alternate"))
            .or_else(|| entry.links.first())
            .map(|l| l.href.clone());

        let released_at = entry
            .published
            .or(entry.updated)
            .map(|dt| dt.naive_utc())
            .unwrap_or(now);

        let title_lower = title.to_ascii_lowercase();
        let is_pre = title_lower.contains("rc")
            || title_lower.contains("beta")
            || title_lower.contains("alpha")
            || title_lower.contains("preview");

        if is_pre && !project.include_prerelease {
            continue;
        }

        out.push(ReleaseItem {
            project_id: project.id,
            version: ver,
            released_at,
            notes_url: link,
            title: if title.is_empty() {
                None
            } else {
                Some(title.to_string())
            },
            is_prerelease: is_pre,
        });
    }

    Ok(out)
}

pub fn collect_dotnet_releases(
    _client: &Client,
    body: &str,
    project: &ReleaseProject,
) -> Result<Vec<ReleaseItem>> {
    let json: Value = serde_json::from_str(body)?;
    let releases_idx = json["releases-index"]
        .as_array()
        .ok_or("dotnet releases-index is not an array")?;

    let now = Utc::now().naive_utc();
    let mut out = Vec::new();

    for ch in releases_idx {
        let phase = ch["support-phase"].as_str().unwrap_or("");
        let latest_rel = match ch["latest-release"].as_str() {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };

        let ver = strip_version_prefix(latest_rel);
        if ver.is_empty() {
            continue;
        }

        let ver_lower = ver.to_ascii_lowercase();
        let is_pre = phase.eq_ignore_ascii_case("preview")
            || phase.eq_ignore_ascii_case("go-live")
            || ver_lower.contains("preview")
            || ver_lower.contains("rc")
            || ver_lower.contains("alpha")
            || ver_lower.contains("beta");

        if is_pre && !project.include_prerelease {
            continue;
        }

        let released_at = ch["latest-release-date"]
            .as_str()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap_or(now);

        let notes_url = Some(format!(
            "https://github.com/dotnet/core/releases/tag/v{ver}"
        ));

        out.push(ReleaseItem {
            project_id: project.id,
            version: ver.clone(),
            released_at,
            notes_url,
            title: Some(format!(".NET {ver}")),
            is_prerelease: is_pre,
        });
    }

    Ok(out)
}

// ==========================================
// 2. Dev Reading
// ==========================================

#[derive(Debug, Clone)]
pub struct DevReadingItem {
    pub site: String,
    pub rank: i32,
    pub external_id: String,
    pub title: String,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub points: i32,
    pub comments: i32,
    pub discussion_url: String,
    pub posted_at: Option<NaiveDateTime>,
    pub snapshot_at: NaiveDateTime,
}

pub fn extract_domain(raw_url: &str) -> Option<String> {
    let u = Url::parse(raw_url).ok()?;
    u.host_str()
        .map(|h| h.trim_start_matches("www.").to_string())
}

pub fn collect_hn_top10(client: &Client) -> Result<Vec<DevReadingItem>> {
    let top_url = "https://hacker-news.firebaseio.com/v0/topstories.json";
    let body = client
        .get(top_url)
        .header("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(15))
        .send()?
        .error_for_status()?
        .text()?;

    let ids: Vec<i64> = serde_json::from_str(&body)?;
    let snapshot_at = Utc::now().naive_utc();
    let mut items = Vec::new();

    for id in ids.into_iter().take(25) {
        if items.len() >= 10 {
            break;
        }
        let item_url = format!("https://hacker-news.firebaseio.com/v0/item/{id}.json");
        let resp = match client
            .get(&item_url)
            .header("User-Agent", USER_AGENT)
            .timeout(Duration::from_secs(5))
            .send()
        {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let body = match resp.text() {
            Ok(b) => b,
            _ => continue,
        };

        let val: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            _ => continue,
        };

        if val["dead"].as_bool().unwrap_or(false) || val["deleted"].as_bool().unwrap_or(false) {
            continue;
        }

        let title = val["title"].as_str().unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }

        let link_url = val["url"].as_str().map(|u| u.trim().to_string());
        let domain = link_url.as_deref().and_then(extract_domain);
        let points = val["score"].as_i64().unwrap_or(0) as i32;
        let comments = val["descendants"].as_i64().unwrap_or(0) as i32;
        let discussion_url = format!("https://news.ycombinator.com/item?id={id}");

        let posted_at = val["time"]
            .as_i64()
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.naive_utc());

        let rank = (items.len() + 1) as i32;
        items.push(DevReadingItem {
            site: "hn".to_string(),
            rank,
            external_id: id.to_string(),
            title,
            url: link_url,
            domain,
            points,
            comments,
            discussion_url,
            posted_at,
            snapshot_at,
        });
    }

    if items.is_empty() {
        return Err("no valid HN items collected".into());
    }

    Ok(items)
}

pub fn collect_lobsters_top10(client: &Client) -> Result<Vec<DevReadingItem>> {
    let url = "https://lobste.rs/hottest.json";
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(15))
        .send()?
        .error_for_status()?;

    let body = resp.text()?;
    let val: Value = serde_json::from_str(&body)?;
    let arr = val.as_array().ok_or("lobsters response not an array")?;

    let snapshot_at = Utc::now().naive_utc();
    let mut items = Vec::new();

    for row in arr.iter().take(10) {
        let short_id = row["short_id"].as_str().unwrap_or("").to_string();
        let title = row["title"].as_str().unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }

        let link_url = row["url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string());
        let domain = link_url.as_deref().and_then(extract_domain);
        let points = row["score"].as_i64().unwrap_or(0) as i32;
        let comments = row["comment_count"].as_i64().unwrap_or(0) as i32;
        let discussion_url = row["comments_url"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        let posted_at = row["created_at"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.naive_utc());

        let rank = (items.len() + 1) as i32;
        items.push(DevReadingItem {
            site: "lobsters".to_string(),
            rank,
            external_id: short_id,
            title,
            url: link_url,
            domain,
            points,
            comments,
            discussion_url,
            posted_at,
            snapshot_at,
        });
    }

    if items.is_empty() {
        return Err("no valid Lobsters items collected".into());
    }

    Ok(items)
}

// ==========================================
// 3. Service Status
// ==========================================

#[derive(Debug, Clone)]
pub struct StatusService {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub api_url: String,
    pub page_url: String,
}

#[derive(Debug, Clone)]
pub struct ServiceStatusUpdate {
    pub service_id: i32,
    pub indicator: String,
    pub description: Option<String>,
    pub incident_name: Option<String>,
    pub incident_component: Option<String>,
    pub incident_started_at: Option<NaiveDateTime>,
    pub incident_url: Option<String>,
    pub checked_at: NaiveDateTime,
    pub last_error: Option<String>,
}

pub fn parse_statuspage_summary(
    service_id: i32,
    body: &str,
    checked_at: NaiveDateTime,
) -> Result<ServiceStatusUpdate> {
    let json: Value = serde_json::from_str(body)?;

    let status = &json["status"];
    let mut indicator = status["indicator"].as_str().unwrap_or("none").to_string();
    let description = status["description"].as_str().map(|s| s.to_string());

    // Check if any component is under_maintenance
    if indicator == "none"
        && let Some(comps) = json["components"].as_array()
        && comps
            .iter()
            .any(|c| c["status"].as_str() == Some("under_maintenance"))
    {
        indicator = "maintenance".to_string();
    }

    let mut inc_name = None;
    let mut inc_component = None;
    let mut inc_started_at = None;
    let mut inc_url = None;

    if let Some(incidents) = json["incidents"].as_array() {
        for inc in incidents {
            let st = inc["status"].as_str().unwrap_or("");
            if st != "resolved" && st != "postmortem" && !st.is_empty() {
                inc_name = inc["name"].as_str().map(String::from);
                inc_started_at = inc["created_at"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.naive_utc());
                inc_url = inc["shortlink"].as_str().map(String::from);

                if let Some(comps) = inc["components"].as_array() {
                    inc_component = comps
                        .first()
                        .and_then(|c| c["name"].as_str())
                        .map(String::from);
                }
                break;
            }
        }
    }

    Ok(ServiceStatusUpdate {
        service_id,
        indicator,
        description,
        incident_name: inc_name,
        incident_component: inc_component,
        incident_started_at: inc_started_at,
        incident_url: inc_url,
        checked_at,
        last_error: None,
    })
}

// ==========================================
// 4. Homelab Services
// ==========================================

#[derive(Debug, Clone)]
pub struct HomelabService {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub check_kind: String, // http | http-body | tautulli | self
    pub check_url: Option<String>,
    pub expect_body: Option<String>,
    pub secret_env: Option<String>,
    pub slow_ms: i32,
    pub link_url: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct HomelabServiceStatusUpdate {
    pub service_id: i32,
    pub state: String, // ok | slow | down | unknown
    pub latency_ms: Option<i32>,
    pub detail: Option<String>,
    pub state_since: NaiveDateTime,
    pub checked_at: NaiveDateTime,
    pub fail_count: i32,
}

#[derive(Debug, Clone, Default)]
pub struct TautulliActivity {
    pub is_success: bool,
    pub latency_ms: i32,
    pub stream_count: Option<String>,
}

pub struct HomelabCheckContext<'a> {
    pub prev_state: Option<&'a str>,
    pub prev_state_since: Option<NaiveDateTime>,
    pub prev_fail_count: i32,
    pub now: NaiveDateTime,
    pub last_feed_run_utc: Option<NaiveDateTime>,
    pub shared_tautulli: Option<&'a TautulliActivity>,
}

pub fn check_homelab_service(
    client: &Client,
    service: &HomelabService,
    cx: &HomelabCheckContext<'_>,
) -> HomelabServiceStatusUpdate {
    let now = cx.now;
    if service.check_kind == "self" {
        let (state, detail) = if let Some(last_run) = cx.last_feed_run_utc {
            let diff_mins = (now - last_run).num_minutes().max(0);
            let detail_str = format!("last run {diff_mins} min ago");
            if diff_mins <= 30 {
                ("ok".to_string(), Some(detail_str))
            } else {
                ("slow".to_string(), Some(detail_str))
            }
        } else {
            ("ok".to_string(), Some("no runs recorded".to_string()))
        };
        let state_since = if cx.prev_state == Some(state.as_str()) {
            cx.prev_state_since.unwrap_or(now)
        } else {
            now
        };
        return HomelabServiceStatusUpdate {
            service_id: service.id,
            state,
            latency_ms: None,
            detail,
            state_since,
            checked_at: now,
            fail_count: 0,
        };
    }

    if service.check_kind == "tautulli" && service.slug == "plex" {
        if let Some(t) = cx.shared_tautulli
            && t.is_success
        {
            let detail = t.stream_count.as_ref().map(|c| format!("{c} streaming"));
            let state = if t.latency_ms > service.slow_ms {
                "slow"
            } else {
                "ok"
            };
            let state_since = if cx.prev_state == Some(state) {
                cx.prev_state_since.unwrap_or(now)
            } else {
                now
            };
            return HomelabServiceStatusUpdate {
                service_id: service.id,
                state: state.to_string(),
                latency_ms: Some(t.latency_ms),
                detail,
                state_since,
                checked_at: now,
                fail_count: 0,
            };
        }
        let plex_identity_url = service
            .link_url
            .as_deref()
            .and_then(|u| u.strip_suffix("/web"))
            .map(|u| format!("{u}/identity"))
            .unwrap_or_else(|| "http://host.docker.internal:32400/identity".to_string());
        return perform_http_check(client, service, &plex_identity_url, None, cx);
    }

    if service.check_kind == "tautulli" {
        let mut url = match &service.check_url {
            Some(u) => u.clone(),
            None => {
                return make_failed_update(
                    service,
                    cx.prev_state,
                    cx.prev_state_since,
                    cx.prev_fail_count,
                    now,
                    Some("missing URL".to_string()),
                );
            }
        };
        if let Some(ref sec_var) = service.secret_env {
            if let Ok(val) = std::env::var(sec_var) {
                if !val.trim().is_empty() {
                    let sep = if url.contains('?') { "&" } else { "?" };
                    url.push_str(&format!("{sep}apikey={}", val.trim()));
                }
            } else {
                return make_failed_update(
                    service,
                    cx.prev_state,
                    cx.prev_state_since,
                    cx.prev_fail_count,
                    now,
                    Some("unconfigured".to_string()),
                );
            }
        }
        let start = std::time::Instant::now();
        match client.get(&url).send() {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as i32;
                if resp.status().is_success() {
                    let body = resp.text().unwrap_or_default();
                    if let Ok(v) = serde_json::from_str::<Value>(&body)
                        && v["response"]["result"].as_str() == Some("success")
                    {
                        let state = if latency > service.slow_ms {
                            "slow"
                        } else {
                            "ok"
                        };
                        let state_since = if cx.prev_state == Some(state) {
                            cx.prev_state_since.unwrap_or(now)
                        } else {
                            now
                        };
                        return HomelabServiceStatusUpdate {
                            service_id: service.id,
                            state: state.to_string(),
                            latency_ms: Some(latency),
                            detail: None,
                            state_since,
                            checked_at: now,
                            fail_count: 0,
                        };
                    }
                    make_failed_update(
                        service,
                        cx.prev_state,
                        cx.prev_state_since,
                        cx.prev_fail_count,
                        now,
                        Some("invalid response".to_string()),
                    )
                } else {
                    let code = resp.status().as_u16();
                    make_failed_update(
                        service,
                        cx.prev_state,
                        cx.prev_state_since,
                        cx.prev_fail_count,
                        now,
                        Some(format!("HTTP {code}")),
                    )
                }
            }
            Err(e) => {
                let detail = if e.is_timeout() {
                    "timeout"
                } else if e.is_connect() {
                    "connection refused"
                } else {
                    "network error"
                };
                make_failed_update(
                    service,
                    cx.prev_state,
                    cx.prev_state_since,
                    cx.prev_fail_count,
                    now,
                    Some(detail.to_string()),
                )
            }
        }
    } else {
        let url = match &service.check_url {
            Some(u) => u.as_str(),
            None => {
                return make_failed_update(
                    service,
                    cx.prev_state,
                    cx.prev_state_since,
                    cx.prev_fail_count,
                    now,
                    Some("missing URL".to_string()),
                );
            }
        };
        perform_http_check(client, service, url, service.expect_body.as_deref(), cx)
    }
}

fn perform_http_check(
    client: &Client,
    service: &HomelabService,
    url: &str,
    expect_body: Option<&str>,
    cx: &HomelabCheckContext<'_>,
) -> HomelabServiceStatusUpdate {
    let now = cx.now;
    let start = std::time::Instant::now();
    match client.get(url).send() {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as i32;
            if resp.status().is_success() {
                if let Some(exp) = expect_body {
                    let body = resp.text().unwrap_or_default();
                    if !body.contains(exp) {
                        return make_failed_update(
                            service,
                            cx.prev_state,
                            cx.prev_state_since,
                            cx.prev_fail_count,
                            now,
                            Some("unexpected body".to_string()),
                        );
                    }
                }
                let state = if latency > service.slow_ms {
                    "slow"
                } else {
                    "ok"
                };
                let state_since = if cx.prev_state == Some(state) {
                    cx.prev_state_since.unwrap_or(now)
                } else {
                    now
                };
                HomelabServiceStatusUpdate {
                    service_id: service.id,
                    state: state.to_string(),
                    latency_ms: Some(latency),
                    detail: None,
                    state_since,
                    checked_at: now,
                    fail_count: 0,
                }
            } else {
                let code = resp.status().as_u16();
                make_failed_update(
                    service,
                    cx.prev_state,
                    cx.prev_state_since,
                    cx.prev_fail_count,
                    now,
                    Some(format!("HTTP {code}")),
                )
            }
        }
        Err(e) => {
            let detail = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "connection refused"
            } else {
                "network error"
            };
            make_failed_update(
                service,
                cx.prev_state,
                cx.prev_state_since,
                cx.prev_fail_count,
                now,
                Some(detail.to_string()),
            )
        }
    }
}

fn make_failed_update(
    service: &HomelabService,
    prev_state: Option<&str>,
    prev_state_since: Option<NaiveDateTime>,
    prev_fail_count: i32,
    now: NaiveDateTime,
    detail: Option<String>,
) -> HomelabServiceStatusUpdate {
    let fail_count = prev_fail_count + 1;
    let state = if fail_count >= 2 {
        "down".to_string()
    } else {
        prev_state.unwrap_or("unknown").to_string()
    };
    let state_since = if prev_state == Some(state.as_str()) {
        prev_state_since.unwrap_or(now)
    } else {
        now
    };
    HomelabServiceStatusUpdate {
        service_id: service.id,
        state,
        latency_ms: None,
        detail,
        state_since,
        checked_at: now,
        fail_count,
    }
}

// ==========================================
// 5. Ollama Library
// ==========================================

#[derive(Debug, Clone)]
pub struct OllamaScrapedModel {
    pub name: String,
    pub description: Option<String>,
    pub updated_text: Option<String>,
    pub library_url: String,
    pub card_tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OllamaScrapedTag {
    pub tag: String,
    pub size_gb: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct OllamaModelSummary {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub change: String,
    pub changed_at: NaiveDateTime,
    pub updated_text: Option<String>,
    pub library_url: String,
    pub first_seen_at: NaiveDateTime,
    pub snapshot_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct OllamaModelUpsert<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub change: &'a str,
    pub changed_at: NaiveDateTime,
    pub updated_text: Option<&'a str>,
    pub library_url: &'a str,
    pub now: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct OllamaTag {
    pub id: Option<i32>,
    pub model_id: i32,
    pub tag: String,
    pub size_gb: Option<f64>,
    pub fits: bool,
    pub installed: bool,
    pub sort_order: i32,
}

pub fn parse_ollama_search_page(html_body: &str) -> Result<Vec<OllamaScrapedModel>> {
    let doc = scraper::Html::parse_document(html_body);
    let a_sel = scraper::Selector::parse("a[href^='/library/']").map_err(|e| format!("{e:?}"))?;
    let p_sel = scraper::Selector::parse("p").map_err(|e| format!("{e:?}"))?;
    let span_sel = scraper::Selector::parse("span").map_err(|e| format!("{e:?}"))?;

    let tag_re = Regex::new(r"(?i)^(\d+(?:\.\d+)?)[bmt]$")?;
    let mut out = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for a in doc.select(&a_sel) {
        let href = match a.value().attr("href") {
            Some(h) if h.starts_with("/library/") => h,
            _ => continue,
        };
        let raw_name = href.trim_start_matches("/library/");
        if raw_name.is_empty() || raw_name.contains(':') || !seen_names.insert(raw_name.to_string())
        {
            continue;
        }

        let name = raw_name.to_string();
        let library_url = format!("https://ollama.com/library/{name}");

        let description = a
            .select(&p_sel)
            .next()
            .map(|p| p.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .filter(|s| !s.is_empty());

        let mut updated_text = None;
        let mut card_tags = Vec::new();

        for span in a.select(&span_sel) {
            if let Some(title) = span.value().attr("title")
                && title.starts_with("Updated ")
            {
                updated_text = Some(title.to_string());
            }
            let txt = span.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if updated_text.is_none()
                && span.value().classes().any(|c| c == "text-muted")
                && !txt.is_empty()
            {
                updated_text = Some(txt.clone());
            }
            if tag_re.is_match(&txt) {
                card_tags.push(txt.to_ascii_lowercase());
            }
        }

        out.push(OllamaScrapedModel {
            name,
            description,
            updated_text,
            library_url,
            card_tags,
        });
    }

    Ok(out)
}

pub fn parse_ollama_tags_page(model_name: &str, html_body: &str) -> Result<Vec<OllamaScrapedTag>> {
    let doc = scraper::Html::parse_document(html_body);
    let prefix = format!("/library/{model_name}:");
    let a_sel = scraper::Selector::parse("a[href]").map_err(|e| format!("{e:?}"))?;

    let size_re = Regex::new(r"(?i)([0-9]+(?:\.[0-9]+)?)\s*(GB|MB|KB)")?;
    let mut out = Vec::new();
    let mut seen_tags = std::collections::HashSet::new();

    for a in doc.select(&a_sel) {
        let href = match a.value().attr("href") {
            Some(h) if h.starts_with(&prefix) => h,
            _ => continue,
        };
        let tag = href.strip_prefix(&prefix).unwrap_or("").trim();
        if tag.is_empty() || !seen_tags.insert(tag.to_string()) {
            continue;
        }

        let mut size_gb = None;
        if let Some(parent) = a.parent().and_then(|p| p.parent()) {
            let container_html = scraper::ElementRef::wrap(parent)
                .map(|e| e.html())
                .unwrap_or_default();

            if let Some(caps) = size_re.captures(&container_html)
                && let (Some(num_m), Some(unit_m)) = (caps.get(1), caps.get(2))
                && let Ok(num) = num_m.as_str().parse::<f64>()
            {
                let unit = unit_m.as_str().to_ascii_uppercase();
                let in_gb = match unit.as_str() {
                    "GB" => num,
                    "MB" => num / 1024.0,
                    "KB" => num / (1024.0 * 1024.0),
                    _ => num,
                };
                size_gb = Some(((in_gb * 10.0).round()) / 10.0);
            }
        }

        out.push(OllamaScrapedTag {
            tag: tag.to_string(),
            size_gb,
        });
    }

    Ok(out)
}

pub fn calculate_ollama_fits(size_gb: Option<f64>, tag: &str, memory_gb: f64) -> bool {
    let usable_gb = (memory_gb - 1.5).max(0.0);
    if usable_gb <= 0.0 {
        return false;
    }
    if let Some(s) = size_gb {
        return s <= usable_gb;
    }
    let tag_lower = tag.to_ascii_lowercase();
    if let Some(b_pos) = tag_lower.find('b') {
        let num_part = &tag_lower[..b_pos];
        if let Ok(billions) = num_part.parse::<f64>() {
            let est_gb = billions * 0.6 + 1.0;
            return est_gb <= usable_gb;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://www.github.com/torvalds/linux"),
            Some("github.com".into())
        );
        assert_eq!(
            extract_domain("http://example.com/item?id=1"),
            Some("example.com".into())
        );
        assert_eq!(extract_domain("not a url"), None);
    }

    #[test]
    fn test_compare_semver() {
        assert_eq!(
            compare_semver("6.18.60", "6.12.115"),
            std::cmp::Ordering::Less
        );
        assert_eq!(compare_semver("6.1.1", "6.1.1"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_statuspage_parser() {
        let fixture = r#"{
            "status": { "indicator": "none", "description": "All Systems Operational" },
            "components": [
                { "name": "API", "status": "operational" },
                { "name": "Dallas", "status": "under_maintenance" }
            ],
            "incidents": [
                {
                    "name": "Degraded Performance",
                    "status": "investigating",
                    "created_at": "2026-10-31T08:26:29Z",
                    "shortlink": "https://stspg.io/xyz",
                    "components": [{ "name": "Actions" }]
                }
            ]
        }"#;

        let res = parse_statuspage_summary(1, fixture, Utc::now().naive_utc()).unwrap();
        assert_eq!(res.incident_name.as_deref(), Some("Degraded Performance"));
        assert_eq!(res.incident_component.as_deref(), Some("Actions"));
        assert_eq!(res.incident_url.as_deref(), Some("https://stspg.io/xyz"));
    }

    #[test]
    fn test_dotnet_releases_parser() {
        let fixture = r#"{
            "releases-index": [
                {
                    "channel-version": "10.0",
                    "latest-release": "10.0.14",
                    "latest-release-date": "2026-10-25",
                    "support-phase": "active"
                },
                {
                    "channel-version": "11.0",
                    "latest-release": "11.0.0-rc.3",
                    "latest-release-date": "2026-10-25",
                    "support-phase": "preview"
                }
            ]
        }"#;

        let proj = ReleaseProject {
            id: 10,
            slug: "dotnet".into(),
            name: ".NET".into(),
            kind: "sdk".into(),
            source_kind: "dotnet_releases".into(),
            source_url: "https://example.com/releases-index.json".into(),
            source_filter: None,
            notes_label: "Release notes".into(),
            include_prerelease: true,
            in_your_stack: true,
        };

        let client = Client::new();
        let items = collect_dotnet_releases(&client, fixture, &proj).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].version, "10.0.14");
        assert!(!items[0].is_prerelease);
        assert_eq!(items[1].version, "11.0.0-rc.3");
        assert!(items[1].is_prerelease);
    }

    #[test]
    fn test_ollama_fits_calculation() {
        // 16 GB memory: usable is 16 - 1.5 = 14.5 GB
        assert!(calculate_ollama_fits(Some(1.1), "1.7b", 16.0));
        assert!(calculate_ollama_fits(Some(14.0), "14b", 16.0));
        assert!(!calculate_ollama_fits(Some(15.0), "20b", 16.0));

        // Unknown size, estimated by tag: "8b" -> 8 * 0.6 + 1.0 = 5.8 GB -> fits
        assert!(calculate_ollama_fits(None, "8b", 16.0));
        // "70b" -> 70 * 0.6 + 1.0 = 43.0 GB -> does not fit in 14.5 GB
        assert!(!calculate_ollama_fits(None, "70b", 16.0));
        // "70b" fits in 64 GB
        assert!(calculate_ollama_fits(None, "70b", 64.0));
        // non-b tag with no size does not fit
        assert!(!calculate_ollama_fits(None, "latest", 16.0));
    }

    #[test]
    fn test_homelab_self_check() {
        let client = Client::new();
        let svc = HomelabService {
            id: 7,
            slug: "scraper".into(),
            name: "myNewspaper scraper".into(),
            check_kind: "self".into(),
            check_url: None,
            expect_body: None,
            secret_env: None,
            slow_ms: 1500,
            link_url: None,
            enabled: true,
            sort_order: 70,
        };
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let ten_min_ago = now - chrono::Duration::minutes(10);
        let cx1 = HomelabCheckContext {
            prev_state: None,
            prev_state_since: None,
            prev_fail_count: 0,
            now,
            last_feed_run_utc: Some(ten_min_ago),
            shared_tautulli: None,
        };
        let res = check_homelab_service(&client, &svc, &cx1);
        assert_eq!(res.state, "ok");
        assert_eq!(res.detail.as_deref(), Some("last run 10 min ago"));

        let forty_min_ago = now - chrono::Duration::minutes(40);
        let cx2 = HomelabCheckContext {
            prev_state: Some("ok"),
            prev_state_since: Some(now),
            prev_fail_count: 0,
            now,
            last_feed_run_utc: Some(forty_min_ago),
            shared_tautulli: None,
        };
        let res_slow = check_homelab_service(&client, &svc, &cx2);
        assert_eq!(res_slow.state, "slow");
        assert_eq!(res_slow.detail.as_deref(), Some("last run 40 min ago"));
    }
}
