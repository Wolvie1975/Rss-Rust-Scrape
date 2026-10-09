# Kansas team logo correction — October 9, 2026

Corrected **101 existing Kansas event rows** in WebScraper.dbo.SportsEvents.

- Existing rows before/after: 101 / 101.
- Current RSS: 91 events updated through the existing URL-keyed MERGE.
- Retained older events: 10 corrected by a TeamLogoUrl-only update scoped to their single-school feed type.
- Event IDs, event URLs, FirstSeenAt and OpponentLogoUrl were unchanged on all 101 rows. No duplicate event URLs.
- Saved school logo: [https://big12sports.com/images/logos/jhwk4C_RF_OL%20(3).png](https://big12sports.com/images/logos/jhwk4C_RF_OL%20(3).png). HTTP 200, image/png, 23269 bytes; visually confirmed to show the Kansas Jayhawk.
- Cause: every RSS `<s:teamlogo>` points to `/images/logos/site/site.png`, the conference logo.
- Fix: resolve the feed's school_id against the calendar page members component and use that school's image.url. Missing school metadata/image yields NULL. Opponent extraction is unchanged.
- Parser regression fixtures: tests/fixtures/big12-kansas-calendar.xml and big12-calendar-members.html.
- Tests: cargo test --locked --all-targets; rollback-only live SQL test school_logo_refresh_preserves_identity_opponents_and_other_feeds passed.
- Standalone Docker service rebuilt and restarted with the fix; existing hourly schedule, flags, network, credentials and persistent data volume retained.
