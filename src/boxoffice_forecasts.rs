//! Boxoffice Pro domestic forecasts: numeric facts and attribution, no article copy.
use crate::Result;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use scraper::{ElementRef, Html, Selector};
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub const INDEX: &str = "https://www.boxofficepro.com/category/forecasts-tracking/";
fn sel(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
fn text(el: ElementRef<'_>) -> String {
    el.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn date(s: &str) -> Result<NaiveDate> {
    let s = s.replace('.', "");
    for format in ["%B %e, %Y", "%b %e, %Y", "%m/%d/%y", "%m/%d/%Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s.trim(), format) {
            return Ok(d);
        }
    }
    Err(format!("unrecognized forecast date: {s}").into())
}
fn range(s: &str) -> Result<(i64, i64)> {
    let normalized = s.replace(['–', '—'], "-");
    let (a, b) = normalized
        .trim()
        .split_once('-')
        .ok_or("forecast range missing")?;
    let amount = |v: &str| -> Result<i64> {
        let v = v
            .trim()
            .strip_prefix('$')
            .ok_or("forecast is not USD")?
            .replace(',', "");
        let (number, unit) = if let Some(n) = v.strip_suffix('M') {
            (n, 1_000_000.0)
        } else if let Some(n) = v.strip_suffix('K') {
            (n, 1_000.0)
        } else {
            (v.as_str(), 1.0)
        };
        let n = number.trim().parse::<f64>()? * unit;
        if !n.is_finite() || !(0.0..=10_000_000_000.0).contains(&n) {
            return Err("invalid forecast amount".into());
        }
        Ok(n.round() as i64)
    };
    let low = amount(a)?;
    let high = amount(b)?;
    if low > high {
        return Err("reversed forecast range".into());
    }
    Ok((low, high))
}
pub fn links(body: &str) -> Result<Vec<(String, String)>> {
    let doc = Html::parse_document(body);
    let mut found = BTreeMap::new();
    for card in doc.select(&sel("article.single-item")) {
        let Some(heading) = card.select(&sel(".single-item__heading")).next() else {
            continue;
        };
        let title = text(heading);
        let kind = if title.starts_with("Weekend Preview:") {
            "weekend-preview"
        } else if title.starts_with("Long Range Forecast:") {
            "long-range"
        } else {
            continue;
        };
        let Some(link) = card
            .select(&sel("a.single-item__link"))
            .next()
            .and_then(|a| a.value().attr("href"))
        else {
            continue;
        };
        let url = url::Url::parse(INDEX)?.join(link)?;
        if url.host_str() != Some("www.boxofficepro.com") {
            return Err("unexpected forecast host".into());
        }
        let stamp = card
            .select(&sel("time"))
            .next()
            .and_then(|t| t.value().attr("datetime"))
            .ok_or("forecast publication time missing")?
            .parse::<i64>()?;
        if found
            .get(kind)
            .is_none_or(|(old, _): &(i64, String)| stamp > *old)
        {
            found.insert(kind.to_string(), (stamp, url.to_string()));
        }
    }
    if found.len() != 2 {
        return Err("domestic preview/long-range links missing from category".into());
    }
    Ok(found
        .into_iter()
        .map(|(kind, (_, url))| (kind, url))
        .collect())
}
pub struct Snapshot {
    pub kind: String,
    pub published: String,
    pub rows: Vec<Value>,
}
pub fn parse(body: &str, kind: &str, source: &str, today: NaiveDate) -> Result<Snapshot> {
    let doc = Html::parse_document(body);
    let heading = doc
        .select(&sel("h1.entry-title"))
        .next()
        .ok_or("forecast title missing")?;
    let prefix = match kind {
        "long-range" => "Long Range Forecast:",
        "weekend-preview" => "Weekend Preview:",
        _ => return Err("unknown forecast kind".into()),
    };
    if !text(heading).starts_with(prefix) {
        return Err("unexpected forecast article".into());
    }
    let published = doc
        .select(&sel("meta[property='article:published_time']"))
        .next()
        .and_then(|x| x.value().attr("content"))
        .ok_or("publication date missing")?;
    let published = DateTime::parse_from_rfc3339(published)?.with_timezone(&Utc);
    let modified = doc
        .select(&sel("meta[property='article:modified_time']"))
        .next()
        .and_then(|x| x.value().attr("content"))
        .map(DateTime::parse_from_rfc3339)
        .transpose()?
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(published);
    if published.date_naive() > today.succ_opt().unwrap()
        || (today - published.date_naive()).num_days() > 60
    {
        return Err("stale/future forecast article".into());
    }
    let content = doc
        .select(&sel(".entry-content"))
        .next()
        .ok_or("forecast content missing")?;
    let mut rows = BTreeMap::new();
    let mut add = |title: String,
                   distributor: String,
                   target: NaiveDate,
                   end: NaiveDate,
                   rank: Option<i32>,
                   value: &str,
                   as_of: NaiveDate|
     -> Result<()> {
        let (low, high) = range(value)?;
        if title.is_empty()
            || title.chars().count() > 300
            || distributor.chars().count() > 300
            || as_of > today.succ_opt().unwrap()
        {
            return Err("invalid forecast row".into());
        }
        if end < today {
            return Ok(());
        }
        if (target - today).num_days() > 200 {
            return Err("forecast date too far in future".into());
        }
        let key = (target, title.to_lowercase());
        rows.insert(key,json!({"title":title,"distributor":distributor,"targetDate":target.to_string(),"endDate":end.to_string(),"rank":rank,"low":low,"high":high,"asOf":as_of.to_string(),"sourceUrl":source,"publishedAt":published.format("%Y-%m-%dT%H:%M:%S").to_string(),"updatedAt":modified.format("%Y-%m-%dT%H:%M:%S").to_string()}));
        Ok(())
    };
    if kind == "weekend-preview" {
        let intro = content
            .select(&sel("h2,h3"))
            .map(text)
            .find(|s| s.starts_with("Forecasting the Top ") && s.contains("Domestic Box Office"))
            .ok_or("domestic preview dates missing")?;
        let period = intro
            .split_once('|')
            .ok_or("preview period missing")?
            .1
            .trim()
            .replace(['–', '—'], "-");
        let (start, tail) = period.split_once('-').ok_or("preview end missing")?;
        let (last_day, year) = tail.trim().split_once(',').ok_or("preview year missing")?;
        let mut start = date(&format!("{},{}", start.trim(), year))?;
        let end = if let Ok(day) = last_day.trim().parse::<u32>() {
            NaiveDate::from_ymd_opt(start.year(), start.month(), day)
                .ok_or("preview end invalid")?
        } else {
            date(&format!("{},{}", last_day.trim(), year))?
        };
        if start > end && start.month() == 12 && end.month() == 1 {
            start = NaiveDate::from_ymd_opt(start.year() - 1, start.month(), start.day())
                .ok_or("preview start invalid")?;
        }
        let expected = intro
            .strip_prefix("Forecasting the Top ")
            .unwrap()
            .split_whitespace()
            .next()
            .ok_or("preview count missing")?
            .parse::<i32>()?;
        if !(1..=10).contains(&expected) {
            return Err("invalid preview count".into());
        }
        if !(2..=4).contains(&(end - start).num_days()) {
            return Err("unexpected preview duration".into());
        }
        let mut count = 0;
        for el in content.select(&sel("h2,h3")) {
            let t = text(el);
            if !t.contains("Weekend Range:") {
                continue;
            }
            let title = el
                .select(&sel("em"))
                .next()
                .map(text)
                .ok_or("preview movie title missing")?;
            let rank = t
                .split_once('.')
                .ok_or("preview rank missing")?
                .0
                .trim()
                .parse::<i32>()?;
            count += 1;
            if rank != count {
                return Err("preview ranks inconsistent".into());
            }
            let value = t.split_once("Weekend Range:").unwrap().1;
            let distributor = t
                .split_once('|')
                .ok_or("preview distributor missing")?
                .0
                .split_once(&title)
                .ok_or("preview distributor missing")?
                .1
                .trim()
                .to_string();
            add(
                title,
                distributor,
                start,
                end,
                Some(rank),
                value,
                published.date_naive(),
            )?;
        }
        if count != expected {
            return Err("incomplete preview article".into());
        }
    } else {
        let headings = content.select(&sel("h2,h3")).collect::<Vec<_>>();
        let anchor = headings
            .iter()
            .map(|h| text(*h))
            .find_map(|s| {
                s.strip_prefix("Long Range Forecast")
                    .map(|d| d.trim_start_matches([' ', '–', '—', '-']).to_string())
            })
            .ok_or("long-range release date missing")?;
        let anchor = date(&anchor)?;
        // Tracking rows include holdovers from prior weeks. Resolve missing years around the explicit article target date.
        for table in content.select(&sel("table")) {
            let table_rows = table.select(&sel("tr")).collect::<Vec<_>>();
            let Some(header) = table_rows.first() else {
                continue;
            };
            if !text(*header).contains("Predicted Opening Range") {
                continue;
            }
            let as_of = content
                .select(&sel("p"))
                .map(text)
                .find_map(|s| {
                    s.strip_prefix("Tracking Updates [As Of ")
                        .and_then(|v| v.strip_suffix(']'))
                        .map(str::to_string)
                })
                .ok_or("tracking as-of date missing")?;
            let as_of = date(&as_of)?;
            for row in table_rows.into_iter().skip(1) {
                let c = row.select(&sel("td")).map(text).collect::<Vec<_>>();
                if c.len() != 4 {
                    return Err("tracking columns changed".into());
                }
                let target = if c[0].split('/').count() == 2 {
                    let (m, d) = c[0].split_once('/').ok_or("release date invalid")?;
                    let m = m.parse()?;
                    let d = d.parse()?;
                    (anchor.year() - 1..=anchor.year() + 1)
                        .filter_map(|y| NaiveDate::from_ymd_opt(y, m, d))
                        .min_by_key(|x| (*x - anchor).num_days().abs())
                        .ok_or("release date invalid")?
                } else {
                    date(&c[0])?
                };
                add(
                    c[1].clone(),
                    c[3].clone(),
                    target,
                    target,
                    None,
                    &c[2],
                    as_of,
                )?;
            }
        }
        // Newly featured films take precedence over the older tracking table in the same article.
        let mut target = anchor;
        let mut film = None;
        for el in headings {
            let t = text(el);
            if let Some(d) = t.strip_prefix("Long Range Forecast") {
                target = date(d.trim_start_matches([' ', '–', '—', '-']))?;
                film = None;
            } else if let Some(v) = t.strip_prefix("Domestic Opening Weekend Range:") {
                let (title, distributor) = film.take().ok_or("forecast movie heading missing")?;
                add(
                    title,
                    distributor,
                    target,
                    target,
                    None,
                    v,
                    published.date_naive(),
                )?;
            } else if let Some((title, distributor)) = t.split_once('|') {
                film = Some((title.trim().to_string(), distributor.trim().to_string()));
            }
        }
        if !content
            .select(&sel("h2,h3"))
            .any(|h| text(h).starts_with("Domestic Opening Weekend Range:"))
        {
            return Err("no long-range forecasts found".into());
        }
    }
    Ok(Snapshot {
        kind: kind.to_string(),
        published: published.format("%Y-%m-%dT%H:%M:%S").to_string(),
        rows: rows.into_values().collect(),
    })
}
pub fn fetch(client: &reqwest::blocking::Client) -> Result<Vec<Snapshot>> {
    let today = Utc::now()
        .with_timezone(&chrono_tz::America::Chicago)
        .date_naive();
    let body = crate::http::get_text(client, INDEX)?;
    links(&body)?
        .into_iter()
        .map(|(kind, url)| parse(&crate::http::get_text(client, &url)?, &kind, &url, today))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn amounts_reject_non_usd_and_reversed() {
        assert_eq!(range("$1.5M – $2M").unwrap(), (1_500_000, 2_000_000));
        assert!(range("£1M – £2M").is_err());
        assert!(range("$2M - $1M").is_err());
        assert!(range("$NaNM - $2M").is_err());
    }
    fn page(content: &str, title: &str) -> String {
        format!(
            "<h1 class='entry-title'>{title}</h1><meta property='article:published_time' content='2026-10-09T12:23:00Z'><div class='entry-content'>{content}</div>"
        )
    }
    #[test]
    fn expired_preview_is_valid_but_empty() {
        let body = page(
            "<h3>Forecasting the Top 1 Movies at the Domestic Box Office | October 9 – 11, 2026</h3><h3>1. <em>Film</em><br>Studio | NEW<br>Weekend Range: $1M – $2M</h3>",
            "Weekend Preview: Film",
        );
        assert!(
            parse(
                &body,
                "weekend-preview",
                INDEX,
                date("October 13, 2026").unwrap()
            )
            .unwrap()
            .rows
            .is_empty()
        );
        let rows = parse(
            &body,
            "weekend-preview",
            INDEX,
            date("October 10, 2026").unwrap(),
        )
        .unwrap()
        .rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["rank"], 1);
    }
    #[test]
    fn future_movies_and_latest_heading_override_tracking() {
        let body = page(
            "<h2>Long Range Forecast – Nov. 6, 2026</h2><h2><em>Film</em> | Studio</h2><h2>Domestic Opening Weekend Range: $2M – $3M</h2><p>Tracking Updates [As Of 10/2/26]</p><table><tr><td>Release Date</td><td>Title</td><td>Predicted Opening Range</td><td>Distributor</td></tr><tr><td>11/6</td><td>Film</td><td>$1M – $2M</td><td>Studio</td></tr><tr><td>10/9</td><td>Past</td><td>$1M – $2M</td><td>Studio</td></tr></table>",
            "Long Range Forecast: Film",
        );
        let s = parse(
            &body,
            "long-range",
            INDEX,
            date("October 13, 2026").unwrap(),
        )
        .unwrap();
        assert_eq!(s.rows.len(), 1);
        assert_eq!(s.rows[0]["low"], 2_000_000);
        assert_eq!(s.rows[0]["asOf"], "2026-10-09");
    }
    #[test]
    fn malformed_page_is_not_empty_success() {
        assert!(
            parse(
                &page("nothing", "Long Range Forecast: Film"),
                "long-range",
                INDEX,
                date("October 13, 2026").unwrap()
            )
            .is_err()
        );
    }
}
