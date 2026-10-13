# Movie and Big 12 parser fixes — production deployment

Completed October 12, 2026, approximately 10:52 p.m. Central.

Image: `web-scraper:parser-fixes-20261012` (also `latest`).
Image ID: `sha256:2ea1258c52f6371966190fdd7e597c2849bcae5ec44f6b77a02e42c4673366d0`.

Movie Insider now reads explicitly dated VOD / Digital release-intro paragraphs
as well as the dedicated digital section. A theatrical-only detail intro remains
an unmatched candidate. Estimated and ambiguous digital dates remain rejected.

Big 12 JSON date_utc values at midnight Central are recognized as date-only
placeholders only when JSON's local kickoff exactly matches the RSS instant in
America/Chicago on the same date. Other timing disagreements still warn; RSS
remains authoritative. UTC values are parsed rather than compared as text.

Verification:

- 96 binary tests passed; 14 database-dependent tests skipped. Clippy with warnings
  denied and formatting checks passed.
- Runtime health check exited successfully. Hourly scheduler retained, next run
  at 11:00 p.m. Central. My-Newsfeed remained healthy.
- Targeted production import of the Arizona State feed and enabled movie sources
  exited successfully. ScraperRuns ID 53 succeeded with no error summary.
- Arizona State feed ID 3184 has LastIssueCount=0 and LastError=NULL.
- Game 179594 has no warning and retains 2026-10-17T19:30:00Z (2:30 p.m. Central).
- Movie Insider source ID 2011 refreshed successfully and LastError is NULL.

The host checkout received src/big12.rs, src/movies.rs and the Big 12 guide.
No schema, credential, permission or SQL Server changes were needed.

Rollback image: `web-scraper:before-parser-fixes-20261012`.
Previous container retained stopped as `web-scraper-rollback-20261012`.
Host source backup:
`/home/jcarpio/Projects/Rss-Rust-Scrape/.deployment-backups/20261012-parser-fixes/source.tar`.
No commits or pushes performed.
