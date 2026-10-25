//! SQL Server integration and fixture tests for `Db`. Live tests are `#[ignore]`d opt-ins.

mod movie_tests {
    use crate::db::*;

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn movie_disc_policy_blocks_writes_and_preserves_other_channels() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, DB_NAME)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            let source = "disc-policy-fixture";
            db.rt.block_on(db.client.execute("INSERT dbo.MovieReleaseSources(SourceKey,Name,Url,Enabled,CollectDisc) VALUES(@P1,N'Disc policy fixture',N'https://example.invalid/',1,0)", &[&source]))?;
            let report = serde_json::json!({"issues":[]});
            let mut row = crate::movies::MovieRelease {
                source: source.into(),
                external_id: "fixture".into(),
                title: "Policy fixture".into(),
                imdb_id: None,
                key: "disc:format:dvd".into(),
                kind: "disc".into(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
                url: "https://example.invalid/film".into(),
                format: Some("DVD".into()),
                platform: None,
            };
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET CollectDisc=1 WHERE SourceKey=@P1",
                &[&source],
            ))?;
            assert_eq!(
                db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                1
            );
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET CollectDisc=0 WHERE SourceKey=@P1",
                &[&source],
            ))?;
            assert!(!db.tracked_movie_urls()?.contains(&row.url));
            db.rt.block_on(db.client.execute("UPDATE r SET ReleaseStatus='withdrawn' FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.SourceKey=@P1 AND r.ReleaseType='disc'", &[&source]))?;
            assert_eq!(
                db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                0
            );
            for kind in [
                "theatrical",
                "digital",
                "digital_purchase",
                "digital_rental",
                "subscription",
            ] {
                row.kind = kind.into();
                row.key = format!("{kind}:first");
                row.platform = (kind == "subscription").then(|| "Fixture service".into());
                assert_eq!(
                    db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                    1
                );
                assert_eq!(
                    db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                    1
                );
            }
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey=@P1",
                &[&source],
            ))?;
            assert_eq!(
                db.save_movie_releases(std::slice::from_ref(&row), &report)?,
                0
            );
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn movie_upserts_are_repeatable_and_reschedule_in_place() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, DB_NAME)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let report = serde_json::json!({"issues": []});
            db.save_movie_releases(&[], &report)?;
            db.rt.block_on(db.client.execute(
                "UPDATE dbo.MovieReleaseSources SET Enabled=1,CollectDisc=1 WHERE SourceKey='dvdsreleasedates'",
                &[],
            ))?;
            let mut row = crate::movies::MovieRelease {
                source: "dvdsreleasedates".into(),
                external_id: "integration-fixture-movie".into(),
                title: "Integration fixture".into(),
                imdb_id: None,
                key: "disc:upc:fixture".into(),
                kind: "disc".into(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(),
                url: "https://example.invalid/integration-fixture".into(),
                format: Some("DVD".into()),
                platform: None,
            };
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            row.date = chrono::NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
            assert_eq!(db.save_movie_releases(&[row.clone()], &report)?, 1);
            db.rt.block_on(async {
                let rows = db.client.query("SELECT r.ReleaseDate FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId WHERE l.ExternalMovieId='integration-fixture-movie'", &[]).await?.into_first_result().await?;
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].get::<chrono::NaiveDate,_>(0), Some(row.date));
                db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='dvdsreleasedates'", &[]).await?;
                Ok::<_, crate::Error>(())
            })?;
            assert_eq!(db.save_movie_releases(&[row], &report)?, 0);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

mod school_logo_tests {
    use crate::db::*;

    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn school_logo_refresh_preserves_identity_opponents_and_other_feeds() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let kansas = db.ensure_sports_events_type(
                "https://example.invalid/logo-test-kansas",
                "Logo test Kansas",
            )?;
            let other = db.ensure_sports_events_type(
                "https://example.invalid/logo-test-other",
                "Logo test other",
            )?;
            let fixture = |id| Event {
                url: format!("https://example.invalid/logo-test-event-{id}"),
                game_id: Some(id),
                title: "Logo test event".into(),
                sport: None,
                opponent: Some("Opponent".into()),
                is_away: Some(false),
                location: None,
                event_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
                starts_at: None,
                ends_at: None,
                time_tbd: true,
                tv: None,
                stream_url: None,
                live_stats_url: None,
                team_logo_url: Some("http://big12sports.com/images/logos/site/site.png".into()),
                opponent_logo_url: Some("https://example.invalid/opponent.png".into()),
            };
            let mut current = fixture(1);
            db.save_events(kansas, &[fixture(1), fixture(2)])?;
            db.save_events(other, &[fixture(3)])?;
            let before = db.rt.block_on(async {
                db.client
                    .query(
                        "SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE Url=@P1",
                        &[&current.url],
                    )
                    .await?
                    .into_first_result()
                    .await
            })?;
            let old_id = before[0].get::<i32, _>(0);
            let old_seen = before[0].get::<chrono::NaiveDateTime, _>(1);
            current.team_logo_url = Some("https://example.invalid/jayhawk.png".into());
            db.save_events(kansas, &[current])?;
            assert_eq!(
                db.refresh_school_team_logo(kansas, Some("https://example.invalid/jayhawk.png"))?,
                1
            );
            assert_eq!(
                db.refresh_school_team_logo(kansas, Some("https://example.invalid/jayhawk.png"))?,
                0
            );
            db.rt.block_on(async {
                let rows = db.client.query("SELECT ID,Url,FirstSeenAt,TeamLogoUrl,OpponentLogoUrl FROM dbo.SportsEvents WHERE SportsEventsTypeId IN (@P1,@P2) ORDER BY ID", &[&kansas,&other]).await?.into_first_result().await?;
                assert_eq!(rows.len(),3);
                assert_eq!(rows[0].get::<i32,_>("ID"),old_id);
                assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>("FirstSeenAt"),old_seen);
                for row in &rows {
                    assert_eq!(row.get::<&str,_>("OpponentLogoUrl"),Some("https://example.invalid/opponent.png"));
                }
                assert_eq!(rows[0].get::<&str,_>("TeamLogoUrl"),Some("https://example.invalid/jayhawk.png"));
                assert_eq!(rows[1].get::<&str,_>("TeamLogoUrl"),Some("https://example.invalid/jayhawk.png"));
                assert_eq!(rows[2].get::<&str,_>("TeamLogoUrl"),Some("http://big12sports.com/images/logos/site/site.png"));
                Ok::<_,crate::Error>(())
            })?;
            assert_eq!(db.refresh_school_team_logo(kansas, None)?, 2);
            assert_eq!(db.refresh_school_team_logo(kansas, None)?, 0);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

mod big12_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires the named Big 12 staging database; fixtures roll back"]
    fn mirrored_imports_reschedule_without_duplicates_or_perspective_changes() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open_named(
            &std::env::var("MSSQL_CONNECTION_STRING")?,
            "WebScraper_Big12_Validation_20261009",
        )?;
        let feeds = db.sports_event_feeds()?;
        let kansas = feeds
            .iter()
            .find(|f| crate::big12::school_id(&f.url) == Some(3))
            .ok_or("Kansas staging feed missing")?
            .id;
        let utah = feeds
            .iter()
            .find(|f| crate::big12::school_id(&f.url) == Some(36))
            .ok_or("Utah staging feed missing")?
            .id;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let before=db.rt.block_on(async{db.client.query("SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE ProviderKey='big12' AND GameId=179871",&[]).await?.into_first_result().await})?;
            let original_id = before[0].get::<i32, _>(0).unwrap();
            let original_seen = before[0].get::<chrono::NaiveDateTime, _>(1);
            let mut a = crate::big12::tests::fixture(3);
            let b = crate::big12::tests::fixture(36);
            for _ in 0..2 {
                assert_eq!(db.save_big12_observation(kansas, &a)?, "saved");
                assert_eq!(db.save_big12_observation(utah, &b)?, "saved");
            }
            a["event"]["event_date"] = serde_json::json!("2026-10-17");
            assert_eq!(db.save_big12_observation(kansas, &a)?, "saved");
            assert_eq!(db.save_big12_observation(utah, &b)?, "saved"); // Secondary stale date cannot revert the primary schedule.
            db.rt.block_on(async {
    let rows=db.client.query("SELECT ID,FirstSeenAt,SchoolId,Opponent,CanonicalKey,EventDate FROM dbo.SportsEvents WHERE CanonicalKey='big12:pair:179871:179872'",&[]).await?.into_first_result().await?;
    assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<i32,_>("ID"),Some(original_id));assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>("FirstSeenAt"),original_seen);
    assert_eq!(rows[0].get::<i32,_>("SchoolId"),Some(3));assert_eq!(rows[0].get::<&str,_>("Opponent"),Some("Utah"));assert_eq!(rows[0].get::<chrono::NaiveDate,_>("EventDate"),chrono::NaiveDate::from_ymd_opt(2026,10,17));
    let sources=db.client.query("SELECT ProviderGameId FROM dbo.SportsEventSources WHERE SportsEventId=@P1 AND ProviderGameId IN(179871,179872)",&[&original_id]).await?.into_first_result().await?;assert_eq!(sources.len(),2);
    Ok::<_,crate::Error>(())
   })?;
            db.event_result(utah, true, 1, 0, None)?;
            db.event_attempt(utah)?;
            db.event_result(utah, false, 0, 1, Some("fixture feed failure"))?;
            db.rt.block_on(async {
    let rows=db.client.query("SELECT LastAttemptAt,LastSuccessAt,LastError FROM dbo.SportsEventsType WHERE ID=@P1",&[&utah]).await?.into_first_result().await?;
    assert!(rows[0].get::<chrono::NaiveDateTime,_>(0).is_some());assert!(rows[0].get::<chrono::NaiveDateTime,_>(1).is_some());assert_eq!(rows[0].get::<&str,_>(2),Some("fixture feed failure"));
    Ok::<_,crate::Error>(())
   })?;
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                        &[&utah],
                    )
                    .await?;
                Ok::<_, crate::Error>(())
            })?;
            assert_eq!(db.save_big12_observation(utah, &b)?, "disabled");
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ID=@P1",
                        &[&utah],
                    )
                    .await?;
                Ok::<_, crate::Error>(())
            })?;
            let mut bad = b;
            bad["issue"] = serde_json::json!("identity unavailable");
            bad["event"]["url"] = serde_json::json!("https://example.invalid/broken");
            assert_eq!(db.save_big12_observation(utah, &bad)?, "quarantined");
            db.rt.block_on(async {
    let rows=db.client.query("SELECT Payload,SourceUrl FROM dbo.SportsEventSources WHERE SportsEventsTypeId=@P1 AND ProviderGameId=179872",&[&utah]).await?.into_first_result().await?;
    let payload:serde_json::Value=serde_json::from_str(rows[0].get::<&str,_>(0).unwrap())?;assert!(payload["issue"].is_null());assert_eq!(rows[0].get::<&str,_>(1),Some("http://big12sports.com/calendar.aspx?id=179872"));
    Ok::<_,crate::Error>(())
   })?;
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

mod poster_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING; fixtures roll back"]
    fn poster_backfill_preserves_rows_and_concurrent_valid_posters() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open(&std::env::var("MSSQL_CONNECTION_STRING")?)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let result = (|| -> Result<()> {
            let report = serde_json::json!({"issues":[]});
            db.save_movie_releases(&[], &report)?;
            db.rt.block_on(async{db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='dvdsreleasedates'",&[]).await?;Ok::<_,crate::Error>(())})?;
            let row = crate::movies::MovieRelease {
                source: "dvdsreleasedates".into(),
                external_id: "poster-regression-fixture".into(),
                title: "Poster regression fixture".into(),
                imdb_id: None,
                key: "theatrical:first".into(),
                kind: "theatrical".into(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
                url: "https://example.invalid/poster-fixture".into(),
                format: None,
                platform: None,
            };
            db.save_movie_releases(std::slice::from_ref(&row), &report)?;
            let job = db
                .poster_jobs()?
                .into_iter()
                .find(|j| j.identity.title == row.title)
                .unwrap();
            let poster = "https://image.tmdb.org/t/p/w500/fixture.jpg";
            assert!(db.set_movie_poster(
                job.movie_id,
                None,
                Some(poster),
                Some(1947),
                "matched",
                Some(&job.url),
                None,
                30
            )?);
            assert!(!db.set_movie_poster(
                job.movie_id,
                None,
                Some("https://image.tmdb.org/t/p/w500/wrong-remake.jpg"),
                Some(2026),
                "matched",
                Some(&job.url),
                None,
                30
            )?);
            let metadata = serde_json::json!({"title":row.title,"year":1947,"imdb_id":null,"tmdb_id":null,"posters":[poster]});
            db.cache_poster_source(job.link_id, "fixture", &metadata, Some(poster), None)?;
            db.cache_poster_source(job.link_id, "fixture", &metadata, Some(poster), None)?;
            db.save_movie_releases(&[row], &report)?;
            db.rt.block_on(async {
                let rows=db.client.query("SELECT PosterUrl,OriginalYear FROM dbo.Movies WHERE ID=@P1",&[&job.movie_id]).await?.into_first_result().await?;
                assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<&str,_>(0),Some(poster));assert_eq!(rows[0].get::<i16,_>(1),Some(1947));
                let rows=db.client.query("SELECT COUNT(*) AS Total FROM dbo.MovieReleases WHERE MovieSourceLinkId=@P1",&[&job.link_id]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(1));
                let rows=db.client.query("SELECT COUNT(*) FROM dbo.MoviePosterCache WHERE SourceLinkId=@P1",&[&job.link_id]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(1));
                Ok::<_,crate::Error>(())
            })?;
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        result
    }
}

mod tvmaze_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires named TVmaze staging database; test failures/status changes isolated to staging"]
    fn tv_snapshots_reschedule_preserve_history_and_disabled_settings() -> Result<()> {
        let _ = dotenvy::dotenv();
        let mut db = Db::open_named(
            &std::env::var("MSSQL_CONNECTION_STRING")?,
            "WebScraper_Tvmaze_Validation_20261009",
        )?;
        db.setup_tv(true)?;
        let before = db.rt.block_on(async {
            db.client
                .query(
                    "SELECT ID,FirstSeenAt FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?;
        let original_id = before[0].get::<i32, _>(0);
        let original_seen = before[0].get::<chrono::NaiveDateTime, _>(1);
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        let phase = (|| -> Result<()> {
            let (s, mut e) = crate::tvmaze::tests::fixtures(83073);
            e[0]["airdate"] = serde_json::json!("2026-10-11");
            let (p, _) = crate::tvmaze::normalize(83073, &s, &e)?;
            for _ in 0..2 {
                let lease = db.claim_tv_series(83073, true)?.unwrap();
                assert_eq!(db.save_tv_snapshot(83073, &lease, &p)?, "saved");
            }
            db.rt.block_on(async {
    let rows=db.client.query("SELECT ID,FirstSeenAt,Airdate,StartsAtUtc,IsDateOnly FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await?;
    assert_eq!(rows.len(),1);assert_eq!(rows[0].get::<i32,_>(0),original_id);assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>(1),original_seen);assert_eq!(rows[0].get::<chrono::NaiveDate,_>(2),chrono::NaiveDate::from_ymd_opt(2026,10,11));assert!(rows[0].get::<chrono::NaiveDateTime,_>(3).is_none());assert_eq!(rows[0].get::<bool,_>(4),Some(true));
    db.client.execute("UPDATE dbo.TvTrackedSeries SET Enabled=0 WHERE TvmazeShowId=83073",&[]).await?;Ok::<_,crate::Error>(())
   })?;
            db.setup_tv(true)?;
            assert!(!db.tv_jobs()?.iter().any(|(id, _)| *id == 83073));
            assert!(db.claim_tv_series(83073, true)?.is_none());
            assert_eq!(
                db.save_tv_snapshot(83073, "00000000-0000-0000-0000-000000000000", &p)?,
                "disabled_or_superseded"
            );
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await
        })?;
        phase?;
        let (s, mut e) = crate::tvmaze::tests::fixtures(83073);
        e.as_array_mut().unwrap().pop();
        let (p, _) = crate::tvmaze::normalize(83073, &s, &e)?;
        let lease = db.claim_tv_series(83073, true)?.unwrap();
        assert!(db.save_tv_snapshot(83073, &lease, &p).is_err());
        db.fail_tv_series(83073, &lease, "fixture incomplete response", 900)?;
        let lease = db.claim_tv_series(64950, true)?.unwrap();
        db.fail_tv_series(64950, &lease, "fixture HTTP 503", 900)?;
        let (s, e) = crate::tvmaze::tests::fixtures(45039);
        let (p, _) = crate::tvmaze::normalize(45039, &s, &e)?;
        let lease = db.claim_tv_series(45039, true)?.unwrap();
        assert_eq!(db.save_tv_snapshot(45039, &lease, &p)?, "saved");
        db.rt.block_on(async {
   let rows=db.client.query("SELECT COUNT(*) FROM dbo.TvEpisodes",&[]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),Some(90));
   let rows=db.client.query("SELECT LastSuccessAt,EpisodeCount,LastError FROM dbo.TvTrackedSeries WHERE TvmazeShowId=83073",&[]).await?.into_first_result().await?;assert!(rows[0].get::<chrono::NaiveDateTime,_>(0).is_some());assert_eq!(rows[0].get::<i32,_>(1),Some(13));assert_eq!(rows[0].get::<&str,_>(2),Some("fixture incomplete response"));
   let rows=db.client.query("SELECT ID,FirstSeenAt,Airdate FROM dbo.TvEpisodes WHERE TvmazeEpisodeId=3682283",&[]).await?.into_first_result().await?;assert_eq!(rows[0].get::<i32,_>(0),original_id);assert_eq!(rows[0].get::<chrono::NaiveDateTime,_>(1),original_seen);assert_eq!(rows[0].get::<chrono::NaiveDate,_>(2),chrono::NaiveDate::from_ymd_opt(2026,10,9));
   Ok::<_,crate::Error>(())
  })?;
        Ok(())
    }
}

mod nwsl_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING and NWSL_TEST_DATABASE (isolated staging); fixture writes roll back"]
    fn schedule_reimports_reschedules_and_cancellation_preserve_rows() -> Result<()> {
        if let Ok(profile) = std::env::var("NWSL_TEST_PROFILE") {
            dotenvy::from_filename(profile)?;
        }
        let _ = dotenvy::dotenv();
        let name = std::env::var("NWSL_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Nwsl_Validation") {
            return Err("use an isolated WebScraper_Nwsl_Validation database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            let legacy=db.ensure_sports_events_type("https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard?team=20907","Kansas City Current")?;
            db.rt.block_on(async {db.client.execute("UPDATE dbo.SportsEventsType SET ProviderKey='espn-nwsl',ProviderSchoolId=20907,Enabled=0 WHERE ID=@P1",&[&legacy]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_nwsl()?;
            let id = db.ensure_sports_events_type(
                "https://site.api.espn.com/apis/site/v2/sports/soccer/usa.nwsl/scoreboard",
                "Kansas City Current",
            )?;
            assert_eq!(id, legacy);
            let mut fixtures = crate::nwsl::parse(&crate::nwsl::tests::sample(), "20907")?;
            for f in &mut fixtures {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 0); // Disabled configuration stays disabled.
            db.rt.block_on(async {
                db.client
                    .execute(
                        "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ID=@P1",
                        &[&id],
                    )
                    .await?;
                Ok::<_, crate::Error>(())
            })?;
            db.configure_nwsl()?; // Reconfiguration must preserve Enabled.
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 3);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,GameId,ScheduleStatus,EventDate,StartsAtUtc FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&id]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].event.event_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
            fixtures[0].event.starts_at = Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-08T01:00:00Z")?.with_timezone(&Utc),
            );
            db.save_espn_schedule(id, &fixtures)?;
            let rescheduled = read(&mut db)?;
            let game = rescheduled
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(
                game.get::<chrono::NaiveDateTime, _>("StartsAtUtc"),
                fixtures[0].event.starts_at.map(|t| t.naive_utc())
            );
            fixtures[0].event.starts_at = None;
            fixtures[0].event.time_tbd = true;
            fixtures[0].status = "STATUS_CANCELED".into();
            db.save_espn_schedule(id, &fixtures)?;
            db.save_espn_schedule(id, &[])?; // Missing observations never delete games.
            let after = read(&mut db)?;
            assert_eq!(before.len(), 3);
            assert_eq!(after.len(), 3);
            for (a, b) in before.iter().zip(&after) {
                assert_eq!(a.get::<i32, _>("ID"), b.get::<i32, _>("ID"));
                assert_eq!(
                    a.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    b.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
            }
            let canceled = after
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(
                canceled.get::<&str, _>("ScheduleStatus"),
                Some("STATUS_CANCELED")
            );
            assert!(
                canceled
                    .get::<chrono::NaiveDateTime, _>("StartsAtUtc")
                    .is_none()
            );
            assert_eq!(
                canceled.get::<chrono::NaiveDate, _>("EventDate"),
                Some(fixtures[0].event.event_date)
            );
            let root: serde_json::Value = serde_json::from_str(include_str!(
                "../../tests/fixtures/nwsl/league-october-2026.json"
            ))?;
            let mut all =
                crate::espn_schedule::parse(&root, crate::espn_schedule::League::Nwsl, None)?;
            for f in &mut all {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            db.save_espn_schedule(id, &all)?;
            let expanded = read(&mut db)?;
            assert_eq!(expanded.len(), 24);
            for old in &before {
                let now = expanded
                    .iter()
                    .find(|r| r.get::<i32, _>("GameId") == old.get::<i32, _>("GameId"))
                    .unwrap();
                assert_eq!(now.get::<i32, _>("ID"), old.get::<i32, _>("ID"));
                assert_eq!(
                    now.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    old.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
            }
            db.save_espn_schedule(id, &all)?;
            assert_eq!(read(&mut db)?.len(), 24);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

mod wnba_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING and WNBA_TEST_DATABASE (isolated staging); fixture writes roll back"]
    fn wnba_configuration_and_schedule_updates_keep_identity_and_other_leagues() -> Result<()> {
        let _ = dotenvy::dotenv();
        let name = std::env::var("WNBA_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Wnba_Validation") {
            return Err("use an isolated WebScraper_Wnba_Validation database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            db.configure_wnba()?;
            let id = db.ensure_sports_events_type(
                "https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard",
                "WNBA",
            )?;
            let mut fixtures = crate::espn_schedule::parse(
                &crate::wnba::tests::sample(),
                crate::espn_schedule::League::Wnba,
                None,
            )?;
            for f in &mut fixtures {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 0);
            db.rt.block_on(async {db.client.execute("UPDATE dbo.SportsEventsType SET Enabled=1,RssUrl=RssUrl+'?season=2026' WHERE ID=@P1",&[&id]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_wnba()?;
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .filter(|f| f.id == id)
                    .count(),
                1
            );
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .find(|f| f.id == id)
                    .unwrap()
                    .url,
                "https://site.api.espn.com/apis/site/v2/sports/basketball/wnba/scoreboard?season=2026"
            );
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 2);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,GameId,ProviderKey,CanonicalKey,ScheduleStatus,EventDate,StartsAtUtc,SchoolName,Opponent FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&id]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].event.event_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
            fixtures[0].event.starts_at = Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-05T01:00:00Z")?.with_timezone(&Utc),
            );
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].status = "STATUS_POSTPONED".into();
            fixtures[0].event.starts_at = None;
            fixtures[0].event.time_tbd = true;
            db.save_espn_schedule(id, &fixtures)?;
            db.save_espn_schedule(id, &[])?;
            let after = read(&mut db)?;
            assert_eq!(after.len(), 2);
            for (a, b) in before.iter().zip(&after) {
                assert_eq!(a.get::<i32, _>("ID"), b.get::<i32, _>("ID"));
                assert_eq!(
                    a.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    b.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
                assert_eq!(b.get::<&str, _>("ProviderKey"), Some("espn-wnba"));
            }
            let f = after
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(f.get::<&str, _>("SchoolName"), Some("Atlanta Dream"));
            assert_eq!(f.get::<&str, _>("Opponent"), Some("Connecticut Sun"));
            assert_eq!(f.get::<&str, _>("ScheduleStatus"), Some("STATUS_POSTPONED"));
            assert!(f.get::<chrono::NaiveDateTime, _>("StartsAtUtc").is_none());
            assert_eq!(
                f.get::<chrono::NaiveDate, _>("EventDate"),
                Some(fixtures[0].event.event_date)
            );
            // Equal numeric IDs in another league remain independent public rows.
            let soccer_id = db.ensure_sports_events_type(
                "https://example.invalid/wnba-test-nwsl",
                "Other league",
            )?;
            let mut soccer = crate::nwsl::parse(&crate::nwsl::tests::sample(), "20907")?.remove(0);
            soccer.completed = false;
            soccer.status = "STATUS_SCHEDULED".into();
            soccer.event.event_date = Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive()
                + chrono::Duration::days(1);
            soccer.event.game_id = fixtures[0].event.game_id;
            soccer.event.url = format!(
                "https://www.espn.com/soccer/match/_/gameId/{}",
                soccer.event.game_id.unwrap()
            );
            assert_eq!(db.save_espn_schedule(soccer_id, &[soccer])?, 1);
            assert_eq!(read(&mut db)?.len(), 2);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

mod mlb_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires MSSQL_CONNECTION_STRING and MLB_TEST_DATABASE (isolated staging); fixture writes roll back"]
    fn mlb_configuration_and_schedule_updates_keep_identity_and_other_leagues() -> Result<()> {
        let _ = dotenvy::dotenv();
        let name = std::env::var("MLB_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Mlb_Validation") {
            return Err("use an isolated WebScraper_Mlb_Validation database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("BEGIN TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let result = (|| -> Result<()> {
            db.configure_mlb()?;
            let id = db.ensure_sports_events_type(
                "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard",
                "MLB",
            )?;
            let mut fixtures = crate::espn_schedule::parse(
                &crate::mlb::tests::sample(),
                crate::espn_schedule::League::Mlb,
                None,
            )?;
            let mut doubleheader = crate::espn_schedule::parse(
                &crate::mlb::tests::sample(),
                crate::espn_schedule::League::Mlb,
                None,
            )?
            .remove(0);
            doubleheader.event.game_id = Some(401907995);
            doubleheader.event.url = "https://www.espn.com/mlb/game/_/gameId/401907995".into();
            fixtures.push(doubleheader);
            for f in &mut fixtures {
                f.completed = false;
                f.status = "STATUS_SCHEDULED".into();
                f.event.event_date = Utc::now()
                    .with_timezone(&chrono_tz::America::Chicago)
                    .date_naive()
                    + chrono::Duration::days(1);
            }
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 0);
            db.rt.block_on(async {db.client.execute("UPDATE dbo.SportsEventsType SET Enabled=1,RssUrl=RssUrl+'?season=2026' WHERE ID=@P1",&[&id]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_mlb()?;
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .filter(|f| f.id == id)
                    .count(),
                1
            );
            assert_eq!(
                db.sports_event_feeds()?
                    .iter()
                    .find(|f| f.id == id)
                    .unwrap()
                    .url,
                "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard?season=2026"
            );
            assert_eq!(db.save_espn_schedule(id, &fixtures)?, 4);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,GameId,ProviderKey,CanonicalKey,ScheduleStatus,EventDate,StartsAtUtc,SchoolName,Opponent FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&id]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].event.event_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
            fixtures[0].event.starts_at = Some(
                chrono::DateTime::parse_from_rfc3339("2026-11-05T01:00:00Z")?.with_timezone(&Utc),
            );
            db.save_espn_schedule(id, &fixtures)?;
            fixtures[0].status = "STATUS_POSTPONED".into();
            fixtures[0].event.starts_at = None;
            fixtures[0].event.time_tbd = true;
            db.save_espn_schedule(id, &fixtures)?;
            db.save_espn_schedule(id, &[])?;
            let after = read(&mut db)?;
            assert_eq!(after.len(), 4);
            for (a, b) in before.iter().zip(&after) {
                assert_eq!(a.get::<i32, _>("ID"), b.get::<i32, _>("ID"));
                assert_eq!(
                    a.get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
                    b.get::<chrono::NaiveDateTime, _>("FirstSeenAt")
                );
                assert_eq!(b.get::<&str, _>("ProviderKey"), Some("espn-mlb"));
            }
            let f = after
                .iter()
                .find(|r| r.get::<i32, _>("GameId") == fixtures[0].event.game_id)
                .unwrap();
            assert_eq!(f.get::<&str, _>("SchoolName"), Some("Cleveland Guardians"));
            assert_eq!(f.get::<&str, _>("Opponent"), Some("Chicago White Sox"));
            assert_eq!(f.get::<&str, _>("ScheduleStatus"), Some("STATUS_POSTPONED"));
            assert!(f.get::<chrono::NaiveDateTime, _>("StartsAtUtc").is_none());
            assert_eq!(
                f.get::<chrono::NaiveDate, _>("EventDate"),
                Some(fixtures[0].event.event_date)
            );
            // Equal numeric IDs in another league remain independent public rows.
            let soccer_id = db.ensure_sports_events_type(
                "https://example.invalid/mlb-test-nwsl",
                "Other league",
            )?;
            let mut soccer = crate::nwsl::parse(&crate::nwsl::tests::sample(), "20907")?.remove(0);
            soccer.completed = false;
            soccer.status = "STATUS_SCHEDULED".into();
            soccer.event.event_date = Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive()
                + chrono::Duration::days(1);
            soccer.event.game_id = fixtures[0].event.game_id;
            soccer.event.url = format!(
                "https://www.espn.com/soccer/match/_/gameId/{}",
                soccer.event.game_id.unwrap()
            );
            assert_eq!(db.save_espn_schedule(soccer_id, &[soccer])?, 1);
            assert_eq!(read(&mut db)?.len(), 4);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK TRANSACTION")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

mod boundary_tests {
    use crate::db::*;
    #[test]
    fn checked_retention_and_unicode_limits() {
        assert!(checked_keep(2147483648).is_err());
        assert!(checked_keep(0).is_err());
        assert_eq!(utf16_prefix("😀😀x", 4), "😀😀");
        assert_eq!(utf16_prefix("😀x", 1), "");
    }
}

mod hardening_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires isolated HARDENING_TEST_DATABASE; never runs against production"]
    fn final_status_reconciliation_and_failed_snapshot_are_safe() -> Result<()> {
        let _ = dotenvy::dotenv();
        let name = std::env::var("HARDENING_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Hardening_Validation_") {
            return Err("isolated hardening database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.setup_tv(false)?;
        let feed = db.ensure_sports_events_type(
            "https://site.api.espn.com/apis/site/v2/sports/baseball/mlb/scoreboard?season=2026",
            "Hardening fixture",
        )?;
        db.rt.block_on(async {db.client.execute("DELETE dbo.SportsEvents WHERE ProviderKey='espn-mlb' AND GameId IN(90000001,90000002,90000003)",&[]).await?;Ok::<_,crate::Error>(())})?;
        let today = Utc::now()
            .with_timezone(&chrono_tz::America::Chicago)
            .date_naive();
        let mut f = crate::espn_schedule::parse(
            &crate::mlb::tests::sample(),
            crate::espn_schedule::League::Mlb,
            None,
        )?
        .remove(0);
        f.event.game_id = Some(90000001);
        f.event.url = "https://www.espn.com/mlb/game/_/gameId/90000001".into();
        f.event.event_date = today;
        f.completed = false;
        f.status = "STATUS_SCHEDULED".into();
        assert_eq!(db.save_espn_schedule(feed, std::slice::from_ref(&f))?, 1);
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,Title,ScheduleStatus FROM dbo.SportsEvents WHERE ProviderKey='espn-mlb' AND GameId=90000001",&[]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        f.status = "STATUS_IN_PROGRESS".into();
        db.save_espn_schedule(feed, std::slice::from_ref(&f))?;
        f.status = "STATUS_FINAL".into();
        f.completed = true;
        db.save_espn_schedule(feed, std::slice::from_ref(&f))?;
        let after = read(&mut db)?;
        assert_eq!(
            after[0].get::<&str, _>("ScheduleStatus"),
            Some("STATUS_FINAL")
        );
        assert_eq!(before[0].get::<i32, _>("ID"), after[0].get::<i32, _>("ID"));
        assert_eq!(
            before[0].get::<chrono::NaiveDateTime, _>("FirstSeenAt"),
            after[0].get::<chrono::NaiveDateTime, _>("FirstSeenAt")
        );
        f.event.game_id = Some(90000003);
        f.event.url = "https://www.espn.com/mlb/game/_/gameId/90000003".into();
        assert_eq!(db.save_espn_schedule(feed, std::slice::from_ref(&f))?, 0); // New completed games are never imported.
        f.event.game_id = Some(90000002);
        f.event.url = "https://www.espn.com/mlb/game/_/gameId/90000002".into();
        f.completed = false;
        f.status = "STATUS_SCHEDULED".into();
        db.save_espn_schedule(feed, std::slice::from_ref(&f))?;
        let mut first = crate::espn_schedule::parse(
            &crate::mlb::tests::sample(),
            crate::espn_schedule::League::Mlb,
            None,
        )?
        .remove(0);
        first.event.game_id = Some(90000001);
        first.event.url = "https://www.espn.com/mlb/game/_/gameId/90000001".into();
        first.event.event_date = today;
        first.event.title = "must roll back".into();
        f.event.url = first.event.url.clone(); // This second source observation conflicts with two distinct canonical rows.
        assert!(db.save_espn_schedule(feed, &[first, f]).is_err());
        let after_error = read(&mut db)?;
        assert_eq!(
            after_error[0].get::<&str, _>("Title"),
            after[0].get::<&str, _>("Title")
        );
        assert_eq!(
            after_error[0].get::<&str, _>("ScheduleStatus"),
            Some("STATUS_FINAL")
        );
        db.rt.block_on(async {db.client.execute("DELETE dbo.SportsEvents WHERE ProviderKey='espn-mlb' AND GameId IN(90000001,90000002,90000003)",&[]).await?;Ok::<_,crate::Error>(())})?;
        let id = db.start_run()?;
        db.run_heartbeat(id)?;
        db.finish_run(id, "succeeded", None)?;
        db.check_health()?;
        Ok(())
    }
}

mod standings_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires migration credentials and isolated STANDINGS_TEST_DATABASE"]
    fn standings_upserts_rollover_and_failed_refresh_preserve_last_good() -> Result<()> {
        let name = std::env::var("STANDINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Standings_Validation_") {
            return Err("isolated standings database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.setup_tv(false)?;
        db.configure_standings()?;
        db.rt.block_on(async {
            db.client.simple_query("DELETE dbo.SportsStandings; UPDATE dbo.SportsStandingsSources SET Enabled=1; DROP TRIGGER IF EXISTS dbo.StandingsValidationFailure;").await?.into_results().await?;
            Ok::<_,crate::Error>(())
        })?;
        db.configure_standings()?;
        let sources = db.standings_sources()?;
        assert_eq!(sources.len(), 12);
        let source = sources.iter().find(|s| s.provider == "espn-nwsl").unwrap();
        let mut snapshot = crate::standings::tests::professional("nwsl");
        db.standings_attempt(source.id)?;
        db.save_standings(source, &snapshot)?;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,TeamId,Season,IsCurrent,StatsJson FROM dbo.SportsStandings WHERE SourceId=@P1 ORDER BY ID",&[&source.id]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        snapshot.title.push_str(" (updated display label)");
        snapshot.entries[0].position = 16;
        snapshot.entries[0].stats[0]["displayValue"] = serde_json::json!("28");
        db.save_standings(source, &snapshot)?;
        let after = read(&mut db)?;
        assert_eq!(after.len(), 16);
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.get::<i64, _>(0), b.get::<i64, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
        }
        let updated = after
            .iter()
            .find(|r| r.get::<&str, _>("TeamId") == Some(snapshot.entries[0].team_id.as_str()))
            .unwrap();
        let stats: serde_json::Value =
            serde_json::from_str(updated.get::<&str, _>("StatsJson").unwrap())?;
        assert_eq!(stats[0]["displayValue"], serde_json::json!("28"));
        let mut incomplete = snapshot.clone();
        incomplete.entries.pop();
        assert!(db.save_standings(source, &incomplete).is_err());
        db.standings_error(source.id, "fixture HTTP failure")?;
        assert!(
            read(&mut db)?
                .iter()
                .all(|r| r.get::<bool, _>("IsCurrent") == Some(true))
        );
        db.rt.block_on(async {db.client.simple_query("CREATE TRIGGER dbo.StandingsValidationFailure ON dbo.SportsStandings AFTER INSERT AS THROW 50000,'fixture write failure',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        snapshot.season = "2027".into();
        let failed = db.save_standings(source, &snapshot);
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.StandingsValidationFailure")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert!(failed.is_err());
        let rows = read(&mut db)?;
        assert_eq!(rows.len(), 16);
        assert!(
            rows.iter()
                .all(|r| r.get::<bool, _>("IsCurrent") == Some(true))
        );
        db.save_standings(source, &snapshot)?;
        let rows = read(&mut db)?;
        assert_eq!(rows.len(), 32);
        assert_eq!(
            rows.iter()
                .filter(|r| r.get::<bool, _>("IsCurrent") == Some(true))
                .count(),
            16
        );
        let epl_source = sources.iter().find(|s| s.provider == "espn-epl").unwrap();
        let mut epl_snapshot = crate::standings::tests::professional("epl");
        db.save_standings(epl_source, &epl_snapshot)?;
        epl_snapshot.season = "2027".into();
        db.save_standings(epl_source, &epl_snapshot)?;
        let epl_rows=db.rt.block_on(async {db.client.query("SELECT COUNT(*),SUM(CASE WHEN IsCurrent=0 THEN 1 ELSE 0 END) FROM dbo.SportsStandings WHERE SourceId=@P1",&[&epl_source.id]).await?.into_first_result().await})?;
        assert_eq!(epl_rows[0].get::<i32, _>(0), Some(20));
        assert_eq!(epl_rows[0].get::<i32, _>(1), Some(0));
        db.rt.block_on(async {
            let r=db.client.query("SELECT LatestSeason,LastTeamCount,LastError FROM dbo.SportsStandingsSources WHERE ID=@P1",&[&source.id]).await?.into_first_result().await?;
            assert_eq!(r[0].get::<&str,_>(0),Some("2027"));assert_eq!(r[0].get::<i32,_>(1),Some(16));assert!(r[0].get::<&str,_>(2).is_none());
            db.client.simple_query(include_str!("../../sql/runtime_permissions.sql")).await?.into_results().await?;
            let p=db.client.query("EXECUTE AS USER='webscraper_ingest'; SELECT HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','SELECT'),HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','INSERT'),HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','UPDATE'),HAS_PERMS_BY_NAME('dbo.SportsStandings','OBJECT','DELETE'),HAS_PERMS_BY_NAME('dbo.SportsStandingsSources','OBJECT','UPDATE','Url','COLUMN'),HAS_PERMS_BY_NAME(DB_NAME(),'DATABASE','CREATE TABLE'); REVERT;",&[]).await?.into_first_result().await?;
            for i in 0..4 {assert_eq!(p[0].get::<i32,_>(i),Some(1));}
            for i in 4..6 {assert_eq!(p[0].get::<i32,_>(i),Some(0));}
            Ok::<_,crate::Error>(())
        })?;
        Ok(())
    }
}

mod standings_live_tests {
    use crate::db::*;
    #[test]
    #[ignore = "live network; isolated STANDINGS_TEST_DATABASE and runtime credentials required"]
    fn live_standings_all_sources_with_restricted_account() -> Result<()> {
        let name = std::env::var("STANDINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Standings_Validation_") {
            return Err("isolated standings database required".into());
        }
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        let sources = db.standings_sources()?;
        assert_eq!(sources.len(), 12);
        let client = crate::http::client_builder()
            .user_agent("My-Newsfeed/1.0")
            .timeout(Duration::from_secs(30))
            .build()?;
        let mut snapshots = vec![];
        for source in &sources {
            db.standings_attempt(source.id)?;
            let snapshot = crate::standings::fetch(&client, source)?;
            db.save_standings(source, &snapshot)?;
            eprintln!(
                "{} {}: {} teams, {}",
                source.provider,
                source.sport,
                snapshot.entries.len(),
                snapshot.season
            );
            snapshots.push(snapshot);
        }
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {
            db.client.query("SELECT ID,FirstSeenAt,SourceId,TeamId FROM dbo.SportsStandings WHERE IsCurrent=1 ORDER BY ID",&[]).await?.into_first_result().await
        })?)
        };
        let before = read(&mut db)?;
        assert_eq!(
            before.len(),
            snapshots.iter().map(|s| s.entries.len()).sum::<usize>()
        );
        for (source, snapshot) in sources.iter().zip(&snapshots) {
            db.save_standings(source, snapshot)?;
        }
        let after = read(&mut db)?;
        assert_eq!(before.len(), after.len());
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.get::<i64, _>(0), b.get::<i64, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
        }
        Ok(())
    }
}

mod digital_movie_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires migration credentials and isolated DIGITAL_TEST_DATABASE"]
    fn digital_sources_preserve_identity_and_disable_settings() -> Result<()> {
        let name = std::env::var("DIGITAL_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Digital_Validation_") {
            return Err("isolated digital validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_movieinsider()?;
        db.rt.block_on(async {db.client.simple_query("BEGIN TRANSACTION; UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='movieinsider';").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        let result = (|| -> Result<()> {
            db.configure_movieinsider()?;
            assert!(
                !db.enabled_movie_sources()?
                    .iter()
                    .any(|s| s == "movieinsider")
            );
            let report = serde_json::json!({"releases":[{"title":"Digital Fixture","source":"movieinsider","release_type":"digital","release_date":"2026-10-09","source_url":"https://www.movieinsider.com/m900001/digital-fixture","format":"VOD / Digital"}],"issues":[]});
            let mut rows = crate::movies::release_rows(&report)?;
            assert_eq!(db.save_movie_releases(&rows, &report)?, 0);
            db.rt.block_on(async {db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=1 WHERE SourceKey='movieinsider'",&[]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_movieinsider()?;
            assert!(
                db.enabled_movie_sources()?
                    .iter()
                    .any(|s| s == "movieinsider")
            );
            assert_eq!(db.save_movie_releases(&rows, &report)?, 1);
            let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
                Ok(db.rt.block_on(async {db.client.query("SELECT r.ID,r.FirstSeenAt,r.ReleaseDate,l.ExternalMovieId,r.ReleaseKey FROM dbo.MovieReleases r JOIN dbo.MovieSourceLinks l ON l.ID=r.MovieSourceLinkId JOIN dbo.MovieReleaseSources s ON s.ID=l.MovieReleaseSourceId WHERE s.SourceKey='movieinsider' AND l.ExternalMovieId='900001'",&[]).await?.into_first_result().await})?)
            };
            let before = read(&mut db)?;
            rows[0].date = chrono::NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
            rows[0].url = "https://www.movieinsider.com/m900001/changed-slug".into();
            assert_eq!(db.save_movie_releases(&rows, &report)?, 1);
            assert_eq!(db.save_movie_releases(&rows, &report)?, 1);
            let after = read(&mut db)?;
            assert_eq!(after.len(), 1);
            assert_eq!(before[0].get::<i32, _>(0), after[0].get::<i32, _>(0));
            assert_eq!(
                before[0].get::<chrono::NaiveDateTime, _>(1),
                after[0].get::<chrono::NaiveDateTime, _>(1)
            );
            assert_eq!(after[0].get::<chrono::NaiveDate, _>(2), Some(rows[0].date));
            assert_eq!(after[0].get::<&str, _>(3), Some("900001"));
            assert_eq!(after[0].get::<&str, _>(4), Some("digital:first"));
            db.rt.block_on(async {db.client.execute("UPDATE dbo.MovieReleaseSources SET Enabled=0 WHERE SourceKey='movieinsider'",&[]).await?;Ok::<_,crate::Error>(())})?;
            db.configure_movieinsider()?;
            assert_eq!(db.save_movie_releases(&rows, &report)?, 0);
            assert_eq!(read(&mut db)?.len(), 1);
            Ok(())
        })();
        db.rt.block_on(async {
            db.client
                .simple_query("IF @@TRANCOUNT>0 ROLLBACK")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        result
    }
}

mod ranking_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires migration credentials and isolated RANKINGS_TEST_DATABASE"]
    fn national_rankings_repeat_correct_advance_and_rollback_safely() -> Result<()> {
        let name = std::env::var("RANKINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Rankings_Validation_") {
            return Err("isolated rankings database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.setup_tv(false)?;
        db.configure_rankings()?;
        db.rt.block_on(async {db.client.simple_query("DELETE dbo.SportsRankingTeams; DELETE dbo.SportsRankingPolls; UPDATE dbo.SportsRankingSources SET Enabled=1; DROP TRIGGER IF EXISTS dbo.RankingValidationFailure;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        db.configure_rankings()?;
        let sources = db.ranking_sources()?;
        assert_eq!(sources.len(), 5);
        let source = sources.iter().find(|s| s.sport == "Football").unwrap();
        let (data, _) = crate::rankings::tests::fixture("football");
        let mut p = crate::rankings::parse(data, source, &crate::rankings::tests::members())?;
        db.ranking_attempt(source.id)?;
        db.save_ranking(source, &p)?;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT t.ID,t.FirstSeenAt,t.TeamId,t.Rank,t.IsBig12,p.ID AS PollId,p.PublishedAtUtc FROM dbo.SportsRankingTeams t JOIN dbo.SportsRankingPolls p ON p.ID=t.PollRowId WHERE p.SourceId=@P1 AND p.IsCurrent=1 AND t.IsCurrent=1 ORDER BY t.ID",&[&source.id]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        assert_eq!(before.len(), 25);
        p.teams.swap(0, 1);
        p.teams[0].rank = 1;
        p.teams[1].rank = 2;
        p.updated = p.updated.map(|t| t + chrono::Duration::minutes(1));
        db.save_ranking(source, &p)?;
        db.save_ranking(source, &p)?;
        let after = read(&mut db)?;
        let corrected = after
            .iter()
            .find(|r| r.get::<&str, _>("TeamId") == Some(p.teams[0].id.as_str()))
            .unwrap();
        assert_eq!(corrected.get::<i32, _>("Rank"), Some(1));
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(a.get::<i64, _>(0), b.get::<i64, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
            assert_eq!(a.get::<i64, _>("PollId"), b.get::<i64, _>("PollId"));
        }
        assert!(
            after
                .iter()
                .any(|r| r.get::<bool, _>("IsBig12") == Some(true))
        );
        assert!(
            after
                .iter()
                .any(|r| r.get::<bool, _>("IsBig12") == Some(false))
        );
        let mut incomplete = p.clone();
        incomplete.teams.pop();
        assert!(db.save_ranking(source, &incomplete).is_err());
        let mut older = p.clone();
        older.updated = older.updated.map(|t| t - chrono::Duration::minutes(2));
        assert!(db.save_ranking(source, &older).is_err());
        assert_eq!(read(&mut db)?.len(), 25);
        db.rt.block_on(async {db.client.simple_query("CREATE TRIGGER dbo.RankingValidationFailure ON dbo.SportsRankingTeams AFTER INSERT AS THROW 50000,'fixture insert failure',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        p.published += chrono::Duration::days(7);
        p.updated = p.updated.map(|t| t + chrono::Duration::days(7));
        let failure = db.save_ranking(source, &p);
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.RankingValidationFailure")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert!(failure.is_err());
        let unchanged = read(&mut db)?;
        assert_eq!(unchanged.len(), 25);
        assert_eq!(unchanged[0].get::<i64, _>(0), before[0].get::<i64, _>(0));
        db.save_ranking(source, &p)?;
        assert_ne!(
            read(&mut db)?[0].get::<i64, _>("PollId"),
            before[0].get::<i64, _>("PollId")
        );
        let obsolete = db.rt.block_on(async {
            db.client.query("SELECT COUNT(*) FROM dbo.SportsRankingPolls WHERE SourceId=@P1 AND IsCurrent=0", &[&source.id]).await?.into_first_result().await
        })?;
        assert_eq!(obsolete[0].get::<i32, _>(0), Some(0));
        let mut old = p.clone();
        old.published -= chrono::Duration::days(7);
        assert!(db.save_ranking(source, &old).is_err());
        db.rt.block_on(async {
            db.client
                .simple_query(include_str!("../../sql/runtime_permissions.sql"))
                .await?
                .into_results()
                .await?;
            db.client
                .simple_query("DELETE dbo.SportsRankingTeams; DELETE dbo.SportsRankingPolls;")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }
    #[test]
    #[ignore = "live network; isolated RANKINGS_TEST_DATABASE with runtime credentials"]
    fn all_five_live_polls_import_with_restricted_account() -> Result<()> {
        let name = std::env::var("RANKINGS_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Rankings_Validation_") {
            return Err("isolated rankings database required".into());
        }
        let mut db = Db::connect_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        let client = crate::http::client_builder()
            .user_agent("My-Newsfeed/1.0")
            .timeout(Duration::from_secs(30))
            .build()?;
        let members = crate::rankings::current_members(&client)?;
        // Run the isolated SQL setup test first; it clears its artificial polls.
        for source in db.ranking_sources()? {
            let p = crate::rankings::fetch(&client, &source, &members)?;
            db.ranking_attempt(source.id)?;
            db.save_ranking(&source, &p)?;
            eprintln!(
                "{}: {}, poll date {}, {} teams / {} Big 12",
                source.sport,
                p.season_label,
                p.published.date_naive(),
                p.teams.len(),
                p.teams
                    .iter()
                    .filter(|t| t.big12_school_id.is_some())
                    .count()
            );
        }
        Ok(())
    }
}

mod championship_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires isolated CHAMPIONSHIP_TEST_DATABASE and migration credentials"]
    fn championship_results_repeat_correct_and_disable_safely() -> Result<()> {
        let name = std::env::var("CHAMPIONSHIP_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Championship_Validation_") {
            return Err("isolated Championship validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_championship()?;
        let source=db.rt.block_on(async {
            db.client.execute("UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-championship'",&[]).await?;
            db.client.query("SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-championship'",&[]).await?.into_first_result().await
        })?[0].get::<i32,_>(0).unwrap();
        let root = serde_json::from_str(include_str!(
            "../../tests/fixtures/championship/october-2026.json"
        ))?;
        let (fixtures, results) = crate::championship::parse(&root, 2026)?;
        let mut batch = crate::championship::Batch {
            season: 2026,
            fixtures,
            results,
        };
        let id = batch.results[0]["gameId"].as_i64().unwrap() as i32;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt,HomeScore FROM dbo.SportsMatchResults WHERE GameId=@P1",&[&id]).await?.into_first_result().await})?)
        };
        db.save_championship_results(source, &batch)?;
        let before = read(&mut db)?;
        db.save_championship_results(source, &batch)?;
        batch.results[0]["homeScore"] = serde_json::json!(7);
        db.save_championship_results(source, &batch)?;
        let after = read(&mut db)?;
        assert_eq!(before[0].get::<i64, _>(0), after[0].get::<i64, _>(0));
        assert_eq!(
            before[0].get::<chrono::NaiveDateTime, _>(1),
            after[0].get::<chrono::NaiveDateTime, _>(1)
        );
        assert_eq!(after[0].get::<i32, _>(2), Some(7));
        db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                    &[&source],
                )
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert!(db.save_championship_results(source, &batch).is_err());
        assert_eq!(read(&mut db)?[0].get::<i32, _>(2), Some(7));
        // The shared writer must preserve MLS shootouts without affecting Championship results.
        db.configure_mls()?;
        let mls_source = db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-mls'",
                    &[],
                )
                .await?;
            db.client
                .query(
                    "SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-mls'",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?[0]
            .get::<i32, _>(0)
            .unwrap();
        let playoff =
            serde_json::from_str(include_str!("../../tests/fixtures/mls/playoffs-2025.json"))?;
        let (_, scores) = crate::mls::parse(&playoff, 2025)?;
        let penalty = scores
            .into_iter()
            .find(|r| r["status"] == "STATUS_FINAL_PEN")
            .unwrap();
        let penalty_id = penalty["gameId"].as_i64().unwrap() as i32;
        let rows = vec![penalty.clone()];
        db.save_match_results(mls_source, crate::espn_schedule::League::Mls, 2025, &rows)?;
        let read_penalty = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {
                db.client.query("SELECT ID,FirstSeenAt,HomeScore,AwayScore,HomeShootoutScore,AwayShootoutScore FROM dbo.SportsMatchResults WHERE ProviderKey='espn-mls' AND GameId=@P1",&[&penalty_id]).await?.into_first_result().await
            })?)
        };
        let original = read_penalty(&mut db)?;
        db.save_match_results(mls_source, crate::espn_schedule::League::Mls, 2025, &rows)?;
        let repeated = read_penalty(&mut db)?;
        assert_eq!(original.len(), 1);
        assert_eq!(original[0].get::<i64, _>(0), repeated[0].get::<i64, _>(0));
        assert_eq!(
            original[0].get::<chrono::NaiveDateTime, _>(1),
            repeated[0].get::<chrono::NaiveDateTime, _>(1)
        );
        for (column, key) in [
            (2, "homeScore"),
            (3, "awayScore"),
            (4, "homeShootout"),
            (5, "awayShootout"),
        ] {
            assert_eq!(
                repeated[0].get::<i32, _>(column),
                Some(penalty[key].as_i64().unwrap() as i32)
            );
        }
        db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                    &[&mls_source],
                )
                .await
        })?;
        assert!(
            db.save_match_results(mls_source, crate::espn_schedule::League::Mls, 2025, &rows)
                .is_err()
        );
        assert_eq!(
            read_penalty(&mut db)?[0].get::<i64, _>(0),
            original[0].get::<i64, _>(0)
        );
        assert_eq!(read(&mut db)?[0].get::<i32, _>(2), Some(7));
        Ok(())
    }
}

mod schedule_batch_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires isolated WNBA_TEST_DATABASE and migration credentials"]
    fn schedule_batches_rollback_across_chunks_and_preserve_ids() -> Result<()> {
        let name = std::env::var("WNBA_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_Wnba_Validation") {
            return Err("isolated validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_wnba()?;
        let feed = db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-wnba'",
                    &[],
                )
                .await?;
            db.client
                .query(
                    "SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-wnba'",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?[0]
            .get::<i32, _>(0)
            .unwrap();
        let template = crate::espn_schedule::parse(
            &crate::wnba::tests::sample(),
            crate::espn_schedule::League::Wnba,
            None,
        )?
        .remove(0);
        let mut games = Vec::new();
        for n in 0..101 {
            let mut f = template.clone();
            let id = 90000000 + n;
            f.event.game_id = Some(id);
            f.event.url = format!("https://www.espn.com/wnba/game/_/gameId/{id}");
            f.event.event_date = Utc::now()
                .with_timezone(&chrono_tz::America::Chicago)
                .date_naive()
                + chrono::Days::new(1);
            f.completed = false;
            games.push(f);
        }
        db.rt.block_on(async {db.client.simple_query("CREATE TRIGGER dbo.BatchFailure ON dbo.SportsEvents AFTER INSERT AS IF EXISTS(SELECT 1 FROM inserted WHERE GameId=90000100) THROW 50000,'second chunk failure',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        assert!(db.save_espn_schedule(feed, &games).is_err());
        let count = db.rt.block_on(async {
            db.client
                .query(
                    "SELECT COUNT(*) FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1",
                    &[&feed],
                )
                .await?
                .into_first_result()
                .await
        })?;
        assert_eq!(count[0].get::<i32, _>(0), Some(0));
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.BatchFailure")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert_eq!(db.save_espn_schedule(feed, &games)?, 101);
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {db.client.query("SELECT ID,FirstSeenAt FROM dbo.SportsEvents WHERE SportsEventsTypeId=@P1 ORDER BY GameId",&[&feed]).await?.into_first_result().await})?)
        };
        let before = read(&mut db)?;
        assert_eq!(db.save_espn_schedule(feed, &games)?, 101);
        let after = read(&mut db)?;
        for (a, b) in before.iter().zip(after.iter()) {
            assert_eq!(a.get::<i32, _>(0), b.get::<i32, _>(0));
            assert_eq!(
                a.get::<chrono::NaiveDateTime, _>(1),
                b.get::<chrono::NaiveDateTime, _>(1)
            );
        }
        Ok(())
    }
}

mod nhl_sql_tests {
    use crate::db::*;
    #[test]
    #[ignore = "requires isolated NHL_TEST_DATABASE and migration credentials"]
    fn nhl_setup_results_and_disabled_source_are_safe() -> Result<()> {
        let name = std::env::var("NHL_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_NHL_Validation_") {
            return Err("isolated NHL validation database required".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_nhl()?;
        let source = db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=1 WHERE ProviderKey='espn-nhl'",
                    &[],
                )
                .await?;
            db.client
                .query(
                    "SELECT ID FROM dbo.SportsEventsType WHERE ProviderKey='espn-nhl'",
                    &[],
                )
                .await?
                .into_first_result()
                .await
        })?[0]
            .get::<i32, _>(0)
            .unwrap();
        db.configure_nhl()?;
        let enabled = db.rt.block_on(async {
            db.client
                .query(
                    "SELECT Enabled FROM dbo.SportsEventsType WHERE ID=@P1",
                    &[&source],
                )
                .await?
                .into_first_result()
                .await
        })?;
        assert_eq!(enabled[0].get::<bool, _>(0), Some(true));
        let root =
            serde_json::from_str(include_str!("../../tests/fixtures/nhl/october-2026.json"))?;
        let (fixtures, mut results) = crate::nhl::parse(&root, 2027)?;
        let game_id = results[0]["gameId"].as_i64().unwrap() as i32;
        db.save_espn_schedule(source, &fixtures)?;
        db.save_match_results(source, crate::espn_schedule::League::Nhl, 2027, &results)?;
        let read = |db: &mut Db| -> Result<Vec<tiberius::Row>> {
            Ok(db.rt.block_on(async {
                db.client.query("SELECT ID,FirstSeenAt,HomeScore,ScheduleStatus FROM dbo.SportsMatchResults WHERE ProviderKey='espn-nhl' AND GameId=@P1", &[&game_id]).await?.into_first_result().await
            })?)
        };
        let original = read(&mut db)?;
        results[0]["homeScore"] = serde_json::json!(7);
        db.save_match_results(source, crate::espn_schedule::League::Nhl, 2027, &results)?;
        let corrected = read(&mut db)?;
        assert_eq!(original.len(), 1);
        assert_eq!(corrected.len(), 1);
        assert_eq!(original[0].get::<i64, _>(0), corrected[0].get::<i64, _>(0));
        assert_eq!(
            original[0].get::<chrono::NaiveDateTime, _>(1),
            corrected[0].get::<chrono::NaiveDateTime, _>(1)
        );
        assert_eq!(corrected[0].get::<i32, _>(2), Some(7));
        db.rt.block_on(async {
            db.client
                .execute(
                    "DELETE dbo.SportsEvents WHERE SportsEventsTypeId=@P1",
                    &[&source],
                )
                .await?;
            db.client
                .execute(
                    "UPDATE dbo.SportsEventsType SET Enabled=0 WHERE ID=@P1",
                    &[&source],
                )
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert_eq!(read(&mut db)?.len(), 1); // Schedule removal never removes results.
        assert!(
            db.save_match_results(source, crate::espn_schedule::League::Nhl, 2027, &results)
                .is_err()
        );
        assert_eq!(read(&mut db)?[0].get::<i32, _>(2), Some(7));
        Ok(())
    }
}

mod article_identity_sql_tests {
    use crate::db::*;
    fn page(source: &str, url: &str) -> PageMeta {
        PageMeta {
            source: source.into(),
            url: url.into(),
            title: "same title".into(),
            description: Some("last-good description".into()),
            published: Some("2026-10-16T10:00:00Z".parse().unwrap()),
            image_url: Some("https://publisher.test/img/good.jpg".into()),
            identity_urls: vec![],
        }
    }
    fn scalar(db: &mut Db, sql: &str) -> Result<i32> {
        db.rt.block_on(async {
            Ok(
                db.client.query(sql, &[]).await?.into_first_result().await?[0]
                    .get::<i32, _>(0)
                    .unwrap(),
            )
        })
    }
    #[test]
    #[ignore = "requires isolated ARTICLE_IDENTITY_TEST_DATABASE and migration credentials"]
    fn identity_merge_idempotence_collision_references_and_concurrency() -> Result<()> {
        let name = std::env::var("ARTICLE_IDENTITY_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_ArticleIdentity_Validation_") {
            return Err("isolated article identity database required".into());
        }
        let ado = std::env::var("MSSQL_CONNECTION_STRING")?;
        let mut db = Db::open_named(&ado, &name)?;
        db.rt.block_on(async {db.client.simple_query("DROP TRIGGER IF EXISTS dbo.IdentityRollbackTest; DROP TABLE IF EXISTS dbo.IdentityTestSaved; DELETE dbo.ArticleIdRedirects; DELETE dbo.ArticleUrlAliases; DELETE dbo.Pages; DELETE dbo.Sources;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        let source = "https://publisher.test/";
        let old = "https://publisher.test/news/old";
        let new = "https://publisher.test/news/new";
        let old_page = page(source, old);
        db.save_pages(std::slice::from_ref(&old_page))?;
        let first = scalar(&mut db, "SELECT MIN(ID) FROM dbo.Pages")?;
        db.save_pages(std::slice::from_ref(&old_page))?;
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 1);
        // Content equality alone is never identity: same title/time/image, different URL.
        let mut incoming = page(source, new);
        db.save_pages(&[incoming.clone()])?;
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        incoming.identity_urls = vec![old.into(), new.into()];
        incoming.title = "new title".into();
        incoming.description = None;
        incoming.image_url = None;
        let preview = db.save_article(&incoming, true)?;
        assert_eq!(preview.1, Some(first));
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        let result = db.save_article(&incoming, false)?;
        assert_eq!(result.0, "merged");
        assert_eq!(result.1, Some(first));
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 1);
        assert_eq!(
            scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?,
            1
        );
        assert_eq!(
            scalar(
                &mut db,
                "SELECT COUNT(*) FROM dbo.Pages WHERE ImageUrl IS NOT NULL AND Description IS NOT NULL AND Published='2026-10-16T10:00:00'"
            )?,
            1
        );
        // A later unverified old URL must use its alias without reverting the canonical URL.
        db.save_pages(&[page(source, old)])?;
        assert_eq!(
            scalar(
                &mut db,
                "SELECT COUNT(*) FROM dbo.Pages WHERE Url='https://publisher.test/news/new'"
            )?,
            1
        );
        // Stale old-URL feed entries resolve to the survivor without changing its identity.
        let mut stale = old_page;
        stale.url = new.into();
        stale.identity_urls = vec![old.into(), new.into()];
        db.save_article(&stale, false)?;
        assert_eq!(scalar(&mut db, "SELECT MIN(ID) FROM dbo.Pages")?, first);
        db.save_article(&incoming, false)?;
        assert_eq!(
            scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?,
            1
        );
        let other = page("https://another.test/", new);
        assert!(
            db.save_article(&other, false)?
                .0
                .starts_with("skipped-cross-source")
        );
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 1);
        // Unknown foreign references block retirement rather than guessing a rewrite policy.
        let third = "https://publisher.test/news/third";
        let third_page = page(source, third);
        db.save_pages(&[third_page])?;
        db.rt.block_on(async {
            db.client
                .simple_query(
                    "CREATE TABLE dbo.IdentityTestSaved(PageId INT REFERENCES dbo.Pages(ID))",
                )
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        incoming.identity_urls.push(third.into());
        assert!(
            db.save_article(&incoming, false)?
                .0
                .starts_with("skipped-unhandled")
        );
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TABLE dbo.IdentityTestSaved")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.save_article(&incoming, false)?;
        let fourth = "https://publisher.test/news/fourth";
        db.save_pages(&[page(source, fourth)])?;
        incoming.identity_urls.push(fourth.into());
        let before = scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?;
        db.rt.block_on(async {db.client.simple_query("CREATE OR ALTER TRIGGER dbo.IdentityRollbackTest ON dbo.Pages AFTER UPDATE AS THROW 50000,'injected rollback test',1;").await?.into_results().await?;Ok::<_,crate::Error>(())})?;
        assert!(db.save_article(&incoming, false).is_err());
        assert_eq!(scalar(&mut db, "SELECT COUNT(*) FROM dbo.Pages")?, 2);
        assert_eq!(
            scalar(&mut db, "SELECT COUNT(*) FROM dbo.ArticleIdRedirects")?,
            before
        );
        db.rt.block_on(async {
            db.client
                .simple_query("DROP TRIGGER dbo.IdentityRollbackTest")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.save_article(&incoming, false)?;
        // Separate sessions racing on a single URL converge under the global hash lock.
        let mut workers = vec![];
        for _ in 0..4 {
            let ado = ado.clone();
            let name = name.clone();
            workers.push(std::thread::spawn(move || -> Result<(), String> {
                let mut db = Db::connect_named(&ado, &name).map_err(|e| e.to_string())?;
                db.save_pages(&[page(
                    "https://publisher.test/",
                    "https://publisher.test/news/race",
                )])
                .map_err(|e| e.to_string())?;
                Ok(())
            }));
        }
        for worker in workers {
            worker
                .join()
                .unwrap()
                .map_err(|e| format!("concurrent ingestion: {e}"))?;
        }
        assert_eq!(
            scalar(
                &mut db,
                "SELECT COUNT(*) FROM dbo.Pages WHERE Url='https://publisher.test/news/race'"
            )?,
            1
        );
        // Deterministic HTTP + database integration: the new row is discovered first,
        // then revisiting the old URL confirms the redirect and preserves the oldest ID.
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let base = format!("http://publisher.test:{}", address.port());
        let server = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let n = socket.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..n]);
                let response = if request.starts_with("GET /news/previous ") {
                    "HTTP/1.1 301 Moved\r\nLocation: /news/current\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
                } else {
                    let body =
                        "<title>Current article</title><meta property='og:type' content='article'>";
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        let test_source = format!("{base}/");
        let previous = format!("{base}/news/previous");
        let current = format!("{base}/news/current");
        let old = page(&test_source, &previous);
        let new = page(&test_source, &current);
        let oldest = db.save_article(&old, false)?.1;
        db.save_pages(std::slice::from_ref(&new))?;
        let mut found = vec![new];
        let recent = db.recent_articles(&test_source)?;
        // Keep just the old URL as the reconciliation job; the incoming new URL
        // supplies the other request, and its redirect adds the third request.
        let recent = recent.into_iter().filter(|p| p.url == previous).collect();
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .resolve("publisher.test", address)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()?;
        crate::pages::enrich_article_identities(
            &client,
            &mut found,
            recent,
            std::time::Instant::now(),
            Some(&mut db),
            false,
        )?;
        db.save_pages(&found)?;
        assert_eq!(db.save_article(&found[0], true)?.1, oldest);
        server.join().unwrap();
        Ok(())
    }
}

mod movie_commerce_sql_tests {
    use crate::db::*;
    use crate::movie_commerce::{Outcome, Provider};
    #[test]
    #[ignore = "requires isolated MOVIE_COMMERCE_TEST_DATABASE and migration credentials"]
    fn movie_commerce_cache_preserves_identity_and_expires() -> Result<()> {
        let name = std::env::var("MOVIE_COMMERCE_TEST_DATABASE")?;
        if !name.starts_with("WebScraper_MovieCommerce_Validation_") {
            return Err("isolated movie commerce database required".into());
        }
        let profile = std::env::var("MOVIE_COMMERCE_TEST_PROFILE")?;
        dotenvy::from_filename(profile)?;
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.rt.block_on(async {
            db.client
                .simple_query("IF DATABASE_PRINCIPAL_ID('WebScraperIngest') IS NULL CREATE ROLE WebScraperIngest; IF USER_ID('webscraper_ingest') IS NULL CREATE USER webscraper_ingest FOR LOGIN webscraper_ingest; IF NOT EXISTS(SELECT 1 FROM sys.database_role_members WHERE role_principal_id=DATABASE_PRINCIPAL_ID('WebScraperIngest') AND member_principal_id=USER_ID('webscraper_ingest')) ALTER ROLE WebScraperIngest ADD MEMBER webscraper_ingest; GRANT SELECT,INSERT,UPDATE ON dbo.MovieReleaseSources TO WebScraperIngest; GRANT SELECT,INSERT,UPDATE ON dbo.Movies TO WebScraperIngest; GRANT SELECT,INSERT,UPDATE ON dbo.MovieSourceLinks TO WebScraperIngest; GRANT SELECT,INSERT,UPDATE ON dbo.MovieReleases TO WebScraperIngest;")
                .await?
                .into_results()
                .await?;
            db.client.simple_query(include_str!("../../sql/movie_commerce_permissions.sql")).await?.into_results().await?;
            db.client
                .simple_query("EXECUTE AS USER='webscraper_ingest';")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        let report = serde_json::json!({"releases":[],"issues":[]});
        let row = crate::movies::MovieRelease {
            source: "dvdsreleasedates".into(),
            external_id: "commerce-validation".into(),
            title: "A & B: Commerce Fixture".into(),
            imdb_id: None,
            key: "theatrical:first".into(),
            kind: "theatrical".into(),
            date: chrono::Utc::now().date_naive() + chrono::Duration::days(1),
            url: "https://www.dvdsreleasedates.com/movies/999999/commerce-validation".into(),
            format: None,
            platform: None,
        };
        db.save_movie_releases(&[row], &report)?;
        let job = db
            .movie_commerce_jobs()?
            .into_iter()
            .find(|j| j.identity.title == "A & B: Commerce Fixture")
            .ok_or("missing commerce job")?;
        assert_eq!(job.provider, Provider::FandangoTickets);
        db.seed_movie_commerce(&job)?;
        db.seed_movie_commerce(&job)?;
        let read = |db: &mut Db| -> Result<(String, String)> {
            db.rt.block_on(async {
            let rows=db.client.query("SELECT LinkKind,ActionUrl FROM dbo.MovieCommerce WHERE MovieId=@P1 AND Provider='fandango'",&[&job.movie_id]).await?.into_first_result().await?;
            Ok((rows[0].get::<&str,_>(0).unwrap().into(),rows[0].get::<&str,_>(1).unwrap().into()))
        })
        };
        assert_eq!(read(&mut db)?.0, "search");
        assert_eq!(url::Url::parse(&read(&mut db)?.1)?.query_pairs().count(), 1);
        db.save_movie_commerce(
            &job,
            &Outcome {
                direct_url: Some("https://www.fandango.com/fixture-123/movie-overview".into()),
                purchase: false,
                error: None,
                ..Default::default()
            },
        )?;
        assert_eq!(read(&mut db)?.0, "direct");
        db.save_movie_commerce(
            &job,
            &Outcome {
                error: Some("temporary HTTP error".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(read(&mut db)?.0, "direct");
        let cap = chrono::Utc::now().naive_utc() + chrono::Duration::hours(2);
        db.save_movie_commerce(
            &job,
            &Outcome {
                direct_url: Some("https://www.fandango.com/fixture-123/movie-overview".into()),
                reviewed_until: Some(cap),
                ..Default::default()
            },
        )?;
        let capped=db.rt.block_on(async {db.client.query("SELECT VerifiedUntil,RetryAfter FROM dbo.MovieCommerceLinks WHERE MovieId=@P1 AND Provider='fandango'",&[&job.movie_id]).await?.into_first_result().await})?;
        assert!(capped[0].get::<chrono::NaiveDateTime, _>(0).unwrap() <= cap);
        assert!(capped[0].get::<chrono::NaiveDateTime, _>(1).unwrap() <= cap);
        db.rt.block_on(async {db.client.execute("UPDATE dbo.MovieCommerceLinks SET VerifiedUntil=DATEADD(day,-1,SYSUTCDATETIME()) WHERE MovieId=@P1",&[&job.movie_id]).await?;Ok::<_,crate::Error>(())})?;
        assert_eq!(read(&mut db)?.0, "search");
        db.rt.block_on(async {
            db.client
                .execute(
                    "UPDATE dbo.Movies SET OriginalYear=2016 WHERE ID=@P1",
                    &[&job.movie_id],
                )
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.save_movie_commerce(
            &job,
            &Outcome {
                direct_url: Some("https://www.fandango.com/fixture-123/movie-overview".into()),
                ..Default::default()
            },
        )?;
        let changed = db
            .movie_commerce_jobs()?
            .into_iter()
            .find(|j| j.movie_id == job.movie_id)
            .unwrap();
        assert!(changed.due);
        db.seed_movie_commerce(&changed)?;
        assert_eq!(read(&mut db)?.0, "search");
        db.rt.block_on(async {
            db.client
                .simple_query("REVERT;")
                .await?
                .into_results()
                .await?;
            db.client
                .simple_query(include_str!("../../tests/sql/movie_commerce.sql"))
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        Ok(())
    }
}

mod f1_sql_tests {
    use crate::db::*;
    use serde_json::Value;
    #[test]
    #[ignore = "requires migration credentials, isolated F1_TEST_DATABASE and F1_TEST_PAYLOAD"]
    fn f1_views_atomic_refresh_disable_and_regression() -> Result<()> {
        let name = std::env::var("F1_TEST_DATABASE")?;
        if !name.starts_with("F1_Test_") {
            return Err("F1 test requires isolated F1_Test_ database".into());
        }
        let mut db = Db::open_named(&std::env::var("MSSQL_CONNECTION_STRING")?, &name)?;
        db.configure_f1()?;
        let payload: Value =
            serde_json::from_str(&std::fs::read_to_string(std::env::var("F1_TEST_PAYLOAD")?)?)?;
        let rows = |key: &str| payload[key].as_array().unwrap().clone();
        let mut snapshot = crate::f1::Snapshot {
            season: 2026,
            standings_round: 17,
            races: rows("races"),
            results: rows("results"),
            sprint: rows("sprint"),
            drivers: rows("drivers"),
            constructors: rows("constructors"),
        };
        assert!(!db.f1_enabled()?);
        assert!(db.save_f1(&snapshot).is_err());
        db.rt.block_on(async {
            db.client
                .simple_query("UPDATE dbo.F1Source SET Enabled=1 WHERE ID=1")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        db.configure_f1()?; // Repeat migration preserves Enabled and existing snapshots.
        assert!(db.f1_enabled()?);
        db.save_f1(&snapshot)?;
        db.save_f1(&snapshot)?;
        fn count(db: &mut Db, table: &str) -> Result<i32> {
            Ok(db.rt.block_on(async {
                db.client
                    .query(format!("SELECT COUNT(*) FROM dbo.{table}"), &[])
                    .await?
                    .into_first_result()
                    .await
            })?[0]
                .get::<i32, _>(0)
                .unwrap())
        }
        assert_eq!(count(&mut db, "F1Schedule")?, snapshot.races.len() as i32);
        assert_eq!(
            count(&mut db, "F1Results")?,
            (snapshot.results.len() + snapshot.sprint.len()) as i32
        );
        assert_eq!(
            count(&mut db, "F1DriverStandings")?,
            snapshot.drivers.len() as i32
        );
        assert_eq!(
            count(&mut db, "F1ConstructorStandings")?,
            snapshot.constructors.len() as i32
        );
        snapshot.standings_round = 16;
        assert!(db.save_f1(&snapshot).is_err());
        snapshot.standings_round = 17;
        snapshot.races[0]["date"] = serde_json::json!("bad");
        assert!(db.save_f1(&snapshot).is_err());
        assert_eq!(count(&mut db, "F1Schedule")?, snapshot.races.len() as i32);
        db.rt.block_on(async {
            db.client
                .simple_query("UPDATE dbo.F1Source SET Enabled=0 WHERE ID=1")
                .await?
                .into_results()
                .await?;
            Ok::<_, crate::Error>(())
        })?;
        assert_eq!(count(&mut db, "F1Schedule")?, 0);
        assert_eq!(count(&mut db, "F1DriverStandings")?, 0);
        assert!(db.save_f1(&snapshot).is_err());
        assert_eq!(count(&mut db, "F1Snapshot")?, 1);
        Ok(())
    }
}
