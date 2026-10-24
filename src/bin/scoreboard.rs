//! Independently scheduled scoreboard worker. Runtime reads configuration, never changes it.
#[path = "../http.rs"]
#[allow(dead_code)]
mod http;
#[path = "../scoreboard.rs"]
mod scoreboard;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Parser;
use scoreboard::{Coverage, Result};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration as StdDuration,
};
use tiberius::{Client, Config, ToSql};
use tokio::{net::TcpStream, runtime::Runtime};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
#[derive(Parser)]
struct Args {
    /// Run continuously, checking which requests are due every minute.
    #[arg(long)]
    watch: bool,
    /// Preview coverage and due requests without writes or provider requests.
    #[arg(long)]
    dry_run: bool,
    /// Read-only database and collector progress check.
    #[arg(long)]
    health_check: bool,
    #[arg(long,default_value_t=10,value_parser=clap::value_parser!(u32).range(10..=15))]
    live_minutes: u32,
    #[arg(long,default_value_t=1,value_parser=clap::value_parser!(u32).range(1..=7))]
    lookback_days: u32,
    #[arg(long,default_value_t=7,value_parser=clap::value_parser!(u32).range(1..=14))]
    lookahead_days: u32,
    #[arg(long,default_value_t=1,value_parser=clap::value_parser!(u32).range(1..=90))]
    retention_days: u32,
}
struct Db {
    rt: Runtime,
    client: Client<Compat<TcpStream>>,
}
impl Db {
    fn open() -> Result<Self> {
        let mut cfg = Config::from_ado_string(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        cfg.database("WebScraper");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = rt.block_on(async {
            tokio::time::timeout(StdDuration::from_secs(30), async {
                let tcp = TcpStream::connect(cfg.get_addr()).await?;
                tcp.set_nodelay(true)?;
                Client::connect(cfg, tcp.compat_write()).await
            })
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
        })?;
        let mut db = Self { rt, client };
        db.query("SET XACT_ABORT ON; SET LOCK_TIMEOUT 15000;", &[])?;
        Ok(db)
    }
    fn query(&mut self, sql: &str, params: &[&dyn ToSql]) -> Result<Vec<tiberius::Row>> {
        // On timeout the run exits and drops this session before another SQL statement is attempted.
        self.rt.block_on(async {
            Ok(tokio::time::timeout(StdDuration::from_secs(90), async {
                self.client
                    .query(sql, params)
                    .await?
                    .into_first_result()
                    .await
            })
            .await??)
        })
    }
}
fn load(db: &mut Db, dry: bool) -> Result<Vec<Coverage>> {
    let mut out = vec![];
    for r in db.query(include_str!("../../sql/scoreboard_config.sql"), &[])? {
        let sport = r.get::<&str, _>("Sport").ok_or("sport missing")?.to_owned();
        let league = r
            .get::<&str, _>("League")
            .ok_or("league missing")?
            .to_owned();
        if !scoreboard::supported(&sport, &league) {
            eprintln!("unsupported coverage: {sport}/{league}");
            continue;
        }
        let kind = r.get::<&str, _>("SourceKind").unwrap();
        let origin = r.get::<i32, _>("OriginId").unwrap();
        let scope = r.get::<&str, _>("Scope").unwrap().to_owned();
        let label = r.get::<&str, _>("Label").unwrap();
        let mut teams = BTreeSet::new();
        let mut season = None;
        if let Some(t) = r.get::<&str, _>("TeamId") {
            teams.insert(t.to_owned());
        }
        if let Some(m) = r.get::<&str, _>("Members") {
            let m: Value = serde_json::from_str(m)?;
            for t in m.as_array().ok_or("member list invalid")? {
                teams.insert(t["TeamId"].as_str().ok_or("member id missing")?.to_owned());
            }
        }
        if kind == "events" {
            let url = url::Url::parse(r.get::<&str, _>("Url").ok_or("feed URL missing")?)?;
            if url.host_str() != Some("site.api.espn.com")
                || url.path() != format!("/apis/site/v2/sports/{sport}/{league}/scoreboard")
            {
                return Err("configured schedule URL differs from supported identity".into());
            }
            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "season" => season = Some(v.parse()?),
                    "team" => teams.extend(v.split(',').map(str::to_owned)),
                    _ => {
                        return Err(
                            "unsupported schedule scope parameter; review before collecting".into(),
                        );
                    }
                }
            }
        }
        if matches!(scope.as_str(), "team" | "conference") && teams.is_empty() {
            eprintln!("coverage unavailable: {sport}/{league} {label}: no validated member IDs");
            continue;
        }
        let id = if dry {
            origin
        } else {
            let r=db.query("IF NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardSources WITH(UPDLOCK,HOLDLOCK) WHERE SourceKind=@P1 AND OriginId=@P2) INSERT dbo.SportsScoreboardSources(SourceKind,OriginId,Sport,League,Label) VALUES(@P1,@P2,@P3,@P4,@P5); UPDATE dbo.SportsScoreboardSources SET Sport=@P3,League=@P4,Label=@P5 WHERE SourceKind=@P1 AND OriginId=@P2; SELECT ID FROM dbo.SportsScoreboardSources WHERE SourceKind=@P1 AND OriginId=@P2;",&[&kind,&origin,&sport,&league,&label])?;
            r[0].get::<i32, _>(0).unwrap()
        };
        println!(
            "coverage {sport}/{league} {scope} {label}: {} scoped teams",
            teams.len()
        );
        out.push(Coverage {
            id,
            sport,
            league,
            scope,
            teams,
            season,
        });
    }
    Ok(out)
}
fn run(args: &Args) -> Result<()> {
    let mut db = Db::open()?;
    if args.health_check {
        let r=db.query("SELECT CASE WHEN EXISTS(SELECT 1 FROM dbo.SportsScoreboardCoverage) AND NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardCoverage WHERE LastAttemptAt IS NULL OR LastAttemptAt<DATEADD(minute,-75,SYSUTCDATETIME())) THEN 1 ELSE 0 END Healthy;",&[])?;
        if r[0].get::<i32, _>(0) != Some(1) {
            return Err("scoreboard configuration/progress check failed".into());
        }
        println!("scoreboard database/progress healthy");
        return Ok(());
    }
    if !args.dry_run {
        let r=db.query("DECLARE @r INT; EXEC @r=sys.sp_getapplock @Resource='WebScraper.Scoreboard.Collector',@LockMode='Exclusive',@LockOwner='Session',@LockTimeout=0; SELECT @r;",&[])?;
        if r[0].get::<i32, _>(0).unwrap_or(-1) < 0 {
            println!("another scoreboard collector owns the run lock; skipping");
            return Ok(());
        }
    }
    let sources = load(&mut db, args.dry_run)?;
    let mut groups: BTreeMap<(String, String), Vec<Coverage>> = BTreeMap::new();
    for s in sources {
        groups
            .entry((s.sport.clone(), s.league.clone()))
            .or_default()
            .push(s);
    }
    let now = Utc::now();
    let today = now
        .with_timezone(&chrono_tz::America::New_York)
        .date_naive();
    let client = http::client_builder()
        .timeout(StdDuration::from_secs(30))
        .build()?;
    let mut count = 0;
    let mut failures = 0;
    for ((sport, league), coverage) in groups {
        if db.query("SELECT CASE WHEN EXISTS(SELECT 1 FROM dbo.SportsScoreboardRequests WHERE Sport=@P1 AND League=@P2 AND Availability='error' AND NextAttemptAt>SYSUTCDATETIME()) THEN 1 ELSE 0 END;",&[&sport,&league])?[0].get::<i32,_>(0)==Some(1) {
            println!("{sport}/{league}: provider failure cooldown");continue;
        }
        let mut days = (-i64::from(args.lookback_days)..=i64::from(args.lookahead_days) + 1)
            .map(|n| today + Duration::days(n))
            .collect::<BTreeSet<_>>();
        // Keep carried-over live/delayed/suspended games discoverable beyond the ordinary window.
        for r in db.query("SELECT DISTINCT CONVERT(VARCHAR(10),ProviderDate,23) Day FROM dbo.SportsScoreboardGames WHERE Sport=@P1 AND League=@P2 AND Status IN('live','delayed','suspended') AND ScoreboardDate>=DATEADD(day,-@P3,CONVERT(date,SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time'));",&[&sport,&league,&(args.retention_days as i32)])?{
            days.insert(NaiveDate::parse_from_str(r.get::<&str,_>(0).unwrap(),"%Y-%m-%d")?);
        }
        // Current day first, then nearby dates; initial backfill does not delay today's games.
        let mut days = days.into_iter().collect::<Vec<_>>();
        days.sort_by_key(|d| (*d - today).num_days().abs());
        for day in days {
            let day_dt = day.and_hms_opt(0, 0, 0).unwrap();
            let due=db.query("SELECT CASE WHEN EXISTS(SELECT 1 FROM dbo.SportsScoreboardRequests WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3 AND NextAttemptAt>SYSUTCDATETIME()) THEN 0 ELSE 1 END;",&[&sport,&league,&day_dt])?[0].get::<i32,_>(0)==Some(1);
            if !due {
                continue;
            }
            if args.dry_run {
                println!("would fetch {sport}/{league} {day}");
                continue;
            }
            let observed = Utc::now().naive_utc();
            db.query("IF NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardRequests WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3) INSERT dbo.SportsScoreboardRequests(Sport,League,ProviderDate,NextAttemptAt,Availability) VALUES(@P1,@P2,@P3,SYSUTCDATETIME(),'pending'); UPDATE dbo.SportsScoreboardRequests SET LastAttemptAt=@P4 WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3; UPDATE dbo.SportsScoreboardSources SET LastAttemptAt=@P4 WHERE ID IN(SELECT value FROM OPENJSON(@P5));",&[&sport,&league,&day_dt,&observed,&json!(coverage.iter().map(|s|s.id).collect::<Vec<_>>()).to_string()])?;
            let url = scoreboard::request_url(&sport, &league, day)?;
            if count > 0 {
                std::thread::sleep(StdDuration::from_millis(350));
            }
            count += 1;
            let mut retry_after = 0i32;
            let fetched = (|| -> Result<scoreboard::Parsed> {
                let response = client
                    .get(url)
                    .header("User-Agent", "MyDailyFeed-Scoreboard/1.0")
                    .send()?;
                if matches!(response.status().as_u16(), 429 | 503)
                    && let Some(v) = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                {
                    retry_after = v
                        .parse::<i32>()
                        .ok()
                        .map(|s| s.saturating_add(59) / 60)
                        .or_else(|| {
                            DateTime::parse_from_rfc2822(v).ok().map(|d| {
                                ((d.with_timezone(&Utc) - Utc::now()).num_seconds().max(0) / 60 + 1)
                                    as i32
                            })
                        })
                        .unwrap_or(60)
                        .clamp(1, 10080);
                }
                let body =
                    http::response_text_limit(response.error_for_status()?, http::JSON_LIMIT)?;
                scoreboard::parse(
                    &serde_json::from_str(&body)?,
                    &sport,
                    &league,
                    day,
                    &coverage,
                )
            })();
            let source_ids = json!(coverage.iter().map(|s| s.id).collect::<Vec<_>>()).to_string();
            match fetched {
                Ok(mut parsed) => {
                    let cutoff =
                        scoreboard::retention_cutoff(Utc::now(), args.retention_days).to_string();
                    parsed.rows.retain(|r| {
                        r["scoreboardDate"]
                            .as_str()
                            .is_some_and(|day| day >= cutoff.as_str())
                    });
                    let payload = json!(parsed.rows).to_string();
                    db.query(
                        include_str!("../../sql/scoreboard_upsert.sql"),
                        &[&sport, &league, &payload, &observed, &day_dt],
                    )?;
                    let partial = parsed.issues > 0;
                    let availability = if partial {
                        "partial"
                    } else if parsed.event_count == 0 {
                        "empty"
                    } else {
                        "available"
                    };
                    // Re-evaluate using newly persisted states so a first live sample gets the live cadence.
                    let minutes = db.query(
                        include_str!("../../sql/scoreboard_due.sql"),
                        &[
                            &sport,
                            &league,
                            &day_dt,
                            &observed,
                            &(args.live_minutes as i32),
                        ],
                    )?[0]
                        .get::<i32, _>(0)
                        .unwrap()
                        .min(if partial { 15 } else { 1440 });
                    db.query("UPDATE dbo.SportsScoreboardRequests SET LastSuccessAt=CASE WHEN @P5=0 THEN @P4 ELSE LastSuccessAt END,NextAttemptAt=DATEADD(minute,@P6,@P4),Availability=@P7,FailureCount=0,LastEventCount=@P8,LastIssueCount=@P5 WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3; UPDATE dbo.SportsScoreboardSources SET LastSuccessAt=CASE WHEN @P5=0 THEN @P4 ELSE LastSuccessAt END,Availability=@P7 WHERE ID IN(SELECT value FROM OPENJSON(@P9));",&[&sport,&league,&day_dt,&observed,&(parsed.issues as i32),&minutes,&availability,&(parsed.event_count as i32),&source_ids])?;
                    println!(
                        "{sport}/{league} {day}: {} scoped games, {} issues, next {minutes}m",
                        parsed.rows.len(),
                        parsed.issues
                    );
                    if partial {
                        failures += 1;
                    }
                }
                Err(e) => {
                    failures += 1;
                    eprintln!("{sport}/{league} {day}: {e}");
                    db.query("UPDATE dbo.SportsScoreboardRequests SET FailureCount=FailureCount+1,Availability='error',NextAttemptAt=DATEADD(minute,CASE WHEN @P4>CASE WHEN FailureCount<4 THEN 15*POWER(2,FailureCount) ELSE 240 END THEN @P4 ELSE CASE WHEN FailureCount<4 THEN 15*POWER(2,FailureCount) ELSE 240 END END,SYSUTCDATETIME()) WHERE Sport=@P1 AND League=@P2 AND ProviderDate=@P3; UPDATE dbo.SportsScoreboardSources SET Availability='error' WHERE ID IN(SELECT value FROM OPENJSON(@P5));",&[&sport,&league,&day_dt,&retry_after,&source_ids])?;
                    // Stop this league on failure rather than hammering every date. Other leagues continue.
                    if retry_after > 0 {
                        db.query("UPDATE dbo.SportsScoreboardRequests SET NextAttemptAt=CASE WHEN NextAttemptAt<DATEADD(minute,@P3,SYSUTCDATETIME()) THEN DATEADD(minute,@P3,SYSUTCDATETIME()) ELSE NextAttemptAt END WHERE Sport=@P1 AND League=@P2;",&[&sport,&league,&retry_after])?;
                    }
                    break;
                }
            }
        }
    }
    if !args.dry_run {
        db.query(
            include_str!("../../sql/scoreboard_retention.sql"),
            &[&(args.retention_days as i32)],
        )?;
    }
    println!("scoreboard run: {count} requests, {failures} failed/partial");
    if failures > 0 {
        return Err("some scoreboard requests failed; last-good samples retained".into());
    }
    Ok(())
}
fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    if args.retention_days < args.lookback_days {
        return Err("retention must be at least lookback".into());
    }
    loop {
        match run(&args) {
            Ok(()) => {}
            Err(e) if args.watch => eprintln!("scoreboard run failed: {e}"),
            Err(e) => return Err(e),
        }
        if !args.watch {
            break;
        }
        std::thread::sleep(StdDuration::from_secs(60));
    }
    Ok(())
}

#[cfg(test)]
mod sql_tests {
    use super::*;
    #[test]
    #[ignore = "requires explicit migration profile and additive scoreboard setup; all fixture writes roll back"]
    fn scoreboard_sql_samples_and_reader_isolation() {
        dotenvy::from_filename(".env.migrations").unwrap();
        let mut db = Db::open().unwrap();
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION;")
                .await
                .unwrap()
                .into_results()
                .await
                .unwrap();
        });
        let origin = db
            .query(
                "SELECT TOP(1) ID FROM dbo.ResultFollows WHERE Enabled=1 AND Kind='league';",
                &[],
            )
            .unwrap()[0]
            .get::<i32, _>(0)
            .unwrap();
        let sr=db.query("IF NOT EXISTS(SELECT 1 FROM dbo.SportsScoreboardSources WHERE SourceKind='follow' AND OriginId=@P1) INSERT dbo.SportsScoreboardSources(SourceKind,OriginId,Sport,League,Label) VALUES('follow',@P1,'football','nfl','Rollback test'); SELECT ID FROM dbo.SportsScoreboardSources WHERE SourceKind='follow' AND OriginId=@P1;",&[&origin]).unwrap()[0].get::<i32,_>(0).unwrap();
        let mut row = json!({"eventId":"scoreboard-rollback-fixture","providerDate":"2026-10-23","scoreboardDate":"2026-10-23","startsAtUtc":"2026-10-24T03:30:00","timeTbd":false,"homeTeamId":"test-h","awayTeamId":"test-a","homeName":"Fixture Home","awayName":"Fixture Away","homeScore":0,"awayScore":0,"status":"live","completed":false,"providerStatus":"STATUS_IN_PROGRESS","statusText":"Quarter 1","period":1,"liveDetails":{},"sources":[sr]});
        let sport = "football";
        let league = "nfl";
        let mut observed = NaiveDate::from_ymd_opt(2026, 10, 24)
            .unwrap()
            .and_hms_opt(4, 0, 0)
            .unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 10, 23)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let upsert = |db: &mut Db, row: &Value, observed: chrono::NaiveDateTime| {
            db.query(
                include_str!("../../sql/scoreboard_upsert.sql"),
                &[&sport, &league, &json!([row]).to_string(), &observed, &day],
            )
            .unwrap();
        };
        upsert(&mut db, &row, observed);
        upsert(&mut db, &row, observed);
        let r=db.query("SELECT COUNT(*) N FROM dbo.SportsScoreboard WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap();
        assert_eq!(r[0].get::<i32, _>(0), Some(1));
        db.query("UPDATE dbo.SportsScoreboardGames SET LastObservedAtUtc=DATEADD(minute,-26,SYSUTCDATETIME()) WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap();
        assert_eq!(db.query("SELECT CONVERT(int,IsStale) FROM dbo.SportsScoreboard WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap()[0].get::<i32,_>(0),Some(1));
        // Empty successful responses never touch existing event freshness or scores.
        db.query(
            include_str!("../../sql/scoreboard_upsert.sql"),
            &[&sport, &league, &"[]", &observed, &day],
        )
        .unwrap();
        assert_eq!(db.query("SELECT CONVERT(int,IsStale) FROM dbo.SportsScoreboard WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap()[0].get::<i32,_>(0),Some(1));
        // Restore the fixture's test timeline after the wall-clock stale check.
        db.query("UPDATE dbo.SportsScoreboardGames SET LastObservedAtUtc=@P1 WHERE EventId='scoreboard-rollback-fixture';",&[&observed]).unwrap();
        row["homeScore"] = json!(7);
        row["period"] = json!(2);
        observed += Duration::minutes(10);
        upsert(&mut db, &row, observed);
        let older = observed - Duration::minutes(15);
        row["homeScore"] = json!(0);
        upsert(&mut db, &row, older);
        assert_eq!(db.query("SELECT CONVERT(int,HomeScore) FROM dbo.SportsScoreboardGames WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap()[0].get::<i32,_>(0),Some(7));
        // A verified correction may lower a score and must update other live fields atomically.
        row["homeScore"] = json!(6);
        row["status"] = json!("final");
        row["completed"] = json!(true);
        row["providerStatus"] = json!("STATUS_FINAL");
        observed += Duration::minutes(10);
        upsert(&mut db, &row, observed);
        assert_eq!(db.query("SELECT COUNT(*) FROM dbo.SportsScoreboardGames WHERE EventId='scoreboard-rollback-fixture' AND HomeScore=6 AND Completed=1 AND Status='final';",&[]).unwrap()[0].get::<i32,_>(0),Some(1));
        row["providerUpdatedAtUtc"] = json!("2026-10-24T04:20:00");
        upsert(&mut db, &row, observed);
        row["providerUpdatedAtUtc"] = json!("2026-10-24T04:10:00");
        row["homeScore"] = json!(99);
        observed += Duration::minutes(10);
        upsert(&mut db, &row, observed);
        assert_eq!(db.query("SELECT CONVERT(int,HomeScore) FROM dbo.SportsScoreboardGames WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap()[0].get::<i32,_>(0),Some(6));
        row["providerUpdatedAtUtc"] = json!("2026-10-24T04:40:00");
        row["scoreboardDate"] = json!("2026-10-25");
        row["startsAtUtc"] = json!("2026-10-26T01:00:00");
        upsert(&mut db, &row, observed);
        assert_eq!(db.query("SELECT COUNT(*) FROM dbo.SportsScoreboardGames WHERE EventId='scoreboard-rollback-fixture' AND WasRescheduled=1 AND OriginalScoreboardDate='2026-10-23' AND ScoreboardDate='2026-10-25';",&[]).unwrap()[0].get::<i32,_>(0),Some(1));
        assert_eq!(db.query("SELECT CONVERT(int,HasBoxScore) FROM dbo.SportsScoreboard WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap()[0].get::<i32,_>(0),Some(0));
        db.query(
            "UPDATE dbo.SportsScoreboardSources SET OriginId=-777 WHERE ID=@P1;",
            &[&sr],
        )
        .unwrap();
        assert_eq!(db.query("SELECT COUNT(*) FROM dbo.SportsScoreboard WHERE EventId='scoreboard-rollback-fixture';",&[]).unwrap()[0].get::<i32,_>(0),Some(0));
        db.rt.block_on(async {
            db.client
                .simple_query("ROLLBACK;")
                .await
                .unwrap()
                .into_results()
                .await
                .unwrap();
        });
    }
}
