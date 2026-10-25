//! Latest three-day domestic weekend chart. Never replace a good chart with partial data.
use crate::Result;
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde_json::{Value, json};
pub const URL: &str = "https://www.the-numbers.com/weekend-box-office-chart";
pub struct Chart {
    pub weekend: NaiveDate,
    pub rows: Vec<Value>,
}
pub fn parse(body: &str, today: NaiveDate) -> Result<Chart> {
    let doc = Html::parse_document(body);
    let sel = |s| Selector::parse(s).unwrap();
    let heading = doc
        .select(&sel("h1"))
        .next()
        .ok_or("chart heading missing")?
        .text()
        .collect::<String>();
    let date = heading
        .trim()
        .strip_prefix("Weekend Domestic Box Office ")
        .ok_or("unexpected chart heading")?;
    let weekend = NaiveDate::parse_from_str(date, "%B %e, %Y")?;
    let age = (today - weekend).num_days();
    if !(0..=10).contains(&age) || weekend.format("%u").to_string() != "5" {
        return Err("weekend chart is stale or has an unexpected date".into());
    }
    let table = doc
        .select(&sel("table.chart-desktop"))
        .next()
        .ok_or("weekend table missing")?;
    let headers = table
        .select(&sel("th"))
        .map(|x| x.text().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");
    if !headers.contains("Gross") || !headers.contains("Theaters") {
        return Err("unexpected chart columns".into());
    }
    let mut rows = Vec::new();
    let money = |s: &str| -> Result<i64> {
        if !s.starts_with('$') {
            return Err("missing dollar amount".into());
        }
        let n: i64 = s[1..].replace(',', "").parse()?;
        if n < 0 {
            return Err("negative gross".into());
        }
        Ok(n)
    };
    for tr in table.select(&sel("tr")) {
        let cells = tr.select(&sel("td")).collect::<Vec<_>>();
        if cells.len() != 9 || cells[2].select(&sel("a")).next().is_none() {
            continue;
        }
        let values = cells
            .iter()
            .map(|c| c.text().collect::<String>().trim().to_string())
            .collect::<Vec<_>>();
        let rank = rows.len() + 1;
        // Some provider rows omit their rank. Preserve gross ordering, but reject contradictory explicit ranks.
        if values[0] != "-" && values[0].parse::<usize>()? != rank {
            return Err("inconsistent chart rank".into());
        }
        let gross = money(&values[3])?;
        if rows
            .last()
            .is_some_and(|r: &Value| r["weekendGross"].as_i64().unwrap() < gross)
        {
            return Err("chart is not sorted by gross".into());
        }
        let title = &values[2];
        if title.is_empty() || title.chars().count() > 300 {
            return Err("invalid movie title".into());
        }
        let change = if values[4].is_empty() {
            None
        } else {
            Some(
                values[4]
                    .strip_suffix('%')
                    .ok_or("invalid percent")?
                    .parse::<f64>()?,
            )
        };
        let link = cells[2]
            .select(&sel("a"))
            .next()
            .unwrap()
            .value()
            .attr("href")
            .ok_or("movie URL missing")?;
        let movie_url = url::Url::parse(URL)?.join(link)?;
        if movie_url.host_str() != Some("www.the-numbers.com") {
            return Err("unexpected movie URL host".into());
        }
        // Status applies only to the weekend gross. An unmarked number is not evidence of actuals.
        let gross_cell = cells[3];
        let estimated = gross_cell.value().classes().any(|c| c == "chart_estimate");
        let label = gross_cell
            .value()
            .attr("title")
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let actual = matches!(label.as_str(), "actual" | "actuals");
        let status = match (estimated, actual) {
            (true, false) => "estimated",
            (false, true) => "final",
            _ => "unknown",
        };
        rows.push(json!({"resultStatus":status,"statusSourceUrl":null,"statusAsOfUtc":null,"rank":rank,"title":title,"weekendGross":gross,"totalDomesticGross":money(&values[7])?,"weeklyChangePercent":change,"theaters":values[5].replace(',', "").parse::<i32>()?,"movieUrl":movie_url.as_str()}));
        if rows.len() == 10 {
            break;
        }
    }
    if rows.len() != 10 {
        return Err("weekend chart must contain ten movies".into());
    }
    Ok(Chart { weekend, rows })
}
pub fn fetch(client: &reqwest::blocking::Client) -> Result<Chart> {
    parse(
        &crate::http::get_text(client, URL)?,
        chrono::Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive(),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(count: usize) -> String {
        let mut s = "<h1>Weekend Domestic Box Office October 9, 2026</h1><table class='chart-desktop'><tr><th>Gross</th><th>Theaters</th></tr>".to_string();
        for n in 1..=count {
            s.push_str(&format!("<tr><td>{n}</td><td>new</td><td><a href='/movie/test-{n}'>Movie {n}</a></td><td>$100</td><td></td><td>10</td><td>$10</td><td>$200</td><td>3</td></tr>"));
        }
        s.push_str("</table>");
        s
    }
    #[test]
    fn complete_chart_and_missing_change() {
        let c = parse(&fixture(10), NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()).unwrap();
        assert_eq!(c.rows.len(), 10);
        assert!(c.rows[0]["weeklyChangePercent"].is_null());
    }
    #[test]
    fn partial_or_stale_chart_rejected() {
        assert!(parse(&fixture(9), NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()).is_err());
        assert!(parse(&fixture(10), NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()).is_err());
    }
    #[test]
    fn live_estimate_marker_is_row_scoped() {
        let c = parse(
            include_str!("../tests/fixtures/boxoffice/provider-20261022.html"),
            NaiveDate::from_ymd_opt(2026, 10, 22).unwrap(),
        )
        .unwrap();
        assert_eq!(c.rows[8]["resultStatus"], "estimated");
        assert_eq!(c.rows[0]["resultStatus"], "unknown");
        assert!(c.rows.iter().all(|r| r["statusAsOfUtc"].is_null()));
    }
    #[test]
    fn explicit_actual_and_conflicting_signals() {
        // Simulated explicit labels: these have not been observed on the live provider.
        let base = fixture(10);
        let actual = base.replacen("<td>$100</td>", "<td title='Actuals'>$100</td>", 1);
        let c = parse(&actual, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()).unwrap();
        assert_eq!(c.rows[0]["resultStatus"], "final");
        assert_eq!(c.rows[1]["resultStatus"], "unknown");
        let conflict = actual.replace("title='Actuals'", "title='Actuals' class='chart_estimate'");
        assert_eq!(
            parse(&conflict, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap())
                .unwrap()
                .rows[0]["resultStatus"],
            "unknown"
        );
    }
}
