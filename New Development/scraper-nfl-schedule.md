# Scraper spec: NFL upcoming schedule (`espn-nfl`)

Requested November 6, 2026 (America/Chicago). Target: the Rust scraper repository.

## Goal

Collect the league-wide NFL schedule from ESPN into `dbo.SportsEvents` so that myNewspaper's Competitions page
(`/events`) can list upcoming NFL games. NFL standings, recent scores and box scores already work. This spec adds
**only the upcoming schedule**.

Follow the existing **NHL** schedule source (`espn-nhl`, `SportsEventsType` ID 5637 in production) as the template.
It is league-wide, stored from the home team's perspective, has no `SportsEventSources` rows, and the web app
already shows rows of that shape. NFL rows that match it need only a small change on the web side; see the last
section.

## Current state (live database, November 6, 2026)

- `dbo.SportsEvents` has no `espn-nfl` rows.
- `dbo.SportsEventsType` has no `espn-nfl` row. The ESPN sources configured are NWSL, WNBA, MLB, EFL Championship,
  Premier League, MLS and NHL.

## 1. Configuration: `--configure-nfl`

Add `--configure-nfl` alongside `--configure-nhl`, `--configure-mls` and the others. Like them, it should:

- Register **one** league-wide `dbo.SportsEventsType` row, **disabled**:

  | Column | Value |
  | --- | --- |
  | `RssUrl` | `https://site.api.espn.com/apis/site/v2/sports/football/nfl/scoreboard` |
  | `EventsTypeName` | `NFL` |
  | `ProviderKey` | `espn-nfl` |
  | `ProviderSchoolId` | `0` (league-wide, not a participant ID) |
  | `SchoolName` | `NULL` |
  | `Enabled` | `0` until it's enabled in Admin |

- Be safe to run again: running it twice keeps an edited URL or settings and doesn't add a second row.
- Not create per-team or per-season rows.

After it's enabled, the existing hourly `--events-from-db` run should pick NFL up automatically, with no new scheduler
argument (as with MLS and NHL).

## 2. Fetching

- Use the ESPN scoreboard endpoint above. The other ESPN leagues fetch with monthly `dates=YYYYMM` requests from the
  current month through the end of the season. Use the same approach if the NFL scoreboard returns a whole month for
  `dates=YYYYMM`. **Check this first:** the NFL scoreboard is organized by week, so if a monthly request returns only
  one week, fetch with `seasontype=<2|3>&week=<n>` instead. Cover regular season weeks 1–18 (`seasontype=2`) and
  postseason weeks 1–5 (`seasontype=3`).
- The NFL season crosses the new year: it starts in September and the Super Bowl is in February. ESPN's `season` is
  the year the season starts (the 2026 season runs into February 2027). Don't stop collecting at December 31.
- Preseason (`seasontype=1`) is **out of scope**; don't store it.
- Use the same HTTP client settings, user agent, timeouts and retry behavior as the other ESPN sources.
- If a fetch or validation fails, keep the last good schedule and record the error on the `SportsEventsType` row
  (`LastAttemptAt`, `LastError`). Update `LastSuccessAt`, `LastEventCount` and `LastIssueCount` after a successful
  import.

## 3. Row mapping (`dbo.SportsEvents`)

Store every game from the **home team's** perspective, exactly like `espn-nhl`. Live NHL example for comparison:
`Buffalo Sabres vs Columbus Blue Jackets`, `SchoolName = Buffalo Sabres`, `Opponent = Columbus Blue Jackets`,
`IsAway = 0`, `CanonicalKey = espn:nhl:401892659`.

| Column | NFL value |
| --- | --- |
| `ProviderKey` | `espn-nfl` |
| `SportsEventsTypeId` | ID of the `espn-nfl` row |
| `Sport` | `Football`. The web app's team records already map `nfl` to `("espn-nfl", "Football")` |
| `GameId` | ESPN event ID (int) |
| `CanonicalKey` | `espn:nfl:<event ID>` |
| `Url` | `https://www.espn.com/nfl/game/_/gameId/<event ID>` (its hash goes in `UrlHash`, as for the other leagues) |
| `LiveStatsUrl` | ESPN's game link from the event, if one is given |
| `Title` | `<Home team> vs <Away team>`, using ESPN display names |
| `SchoolId` / `SchoolName` / `TeamLogoUrl` | Home team: ESPN team ID, display name, logo |
| `OpponentSchoolId` / `Opponent` / `OpponentLogoUrl` | Away team: ESPN team ID, display name, logo |
| `IsAway` | `0` (home perspective) |
| `IsNeutral` | From ESPN's `neutralSite` (for example international games and the Super Bowl) |
| `Location` | Venue name from ESPN, or `NULL` |
| `EventDate` | Kickoff date in America/Chicago |
| `StartsAtUtc` | Confirmed kickoff in UTC |
| `TimeTbd` | `1` when ESPN marks the time TBD (common for late-season and flexed games) |
| `CalendarTimeZone` | `America/Chicago` |
| `Tv` | Broadcast names from ESPN joined with `, ` (as NHL does), or `NULL` |
| `StreamUrl`, `Radio`, `AudioUrl` | Only when ESPN supplies them; otherwise `NULL` |
| `ScheduleStatus` | ESPN status name, for example `STATUS_SCHEDULED`, `STATUS_POSTPONED` or `STATUS_FINAL` |
| `FirstSeenAt` / `LastSeenAt` | UTC; keep `FirstSeenAt` the same across re-imports |
| `EndsAtUtc` | `NULL`. Don't guess end times |

Don't write `SportsEventSources` rows. NHL and the other league-wide sources don't use them, and the web app only
shows rows with no source rows when their `SportsEventsType` is enabled.

### Rules carried over from the other ESPN leagues
- IDs and `FirstSeenAt` survive re-imports and rescheduling. When a status or time changes, update the same row.
- A game missing from a fetch doesn't mean it was canceled. Only an explicit status changes a game's state.
- For a kickoff that isn't known, keep the provider's date and set `TimeTbd = 1`. Don't convert a placeholder
  midnight to another time zone.
- Don't invent venues, times, logos or broadcasts.
- Use ESPN team IDs scoped to `espn-nfl` only. Never match teams across providers by name.

### NFL-specific cases
- **Postseason placeholders:** before matchups are decided, ESPN lists playoff games with "TBD" or seed
  placeholders. **Skip any event that doesn't have two real teams with ESPN team IDs.** Store it on a later run,
  once the teams are known.
- **Bye weeks:** these aren't events; nothing to store.
- **Flexed games:** the kickoff time and network can change late. Update the existing row by event ID.
- **Pro Bowl:** skip it, as it isn't a competitive game. If ESPN lists it in `seasontype=3`, it's the event whose
  teams are AFC and NFC.

## 4. Retention

Use the same rules as the other schedules: remove past dates and canceled or unnecessary fixtures. The web page
already hides `STATUS_FINAL*`, `STATUS_CANCELED`/`STATUS_CANCELLED`, `STATUS_UNNECESSARY` and `STATUS_FULL_TIME`.

## 5. Tests (scraper repository)

Provider fixture tests, in the style of the NHL and MLS ones:
- A regular-season week: home and away mapping, the `espn:nfl:` canonical key, UTC start and Central `EventDate`
  (including a Sunday night game, which is the next day in UTC), and the TV list.
- A TBD kickoff gives `TimeTbd = 1` and keeps the provider's date.
- A neutral-site game (international or Super Bowl) gives `IsNeutral = 1`.
- A postseason event with placeholder teams is skipped, and is stored once the teams are set.
- The Pro Bowl is skipped.
- Re-import after a time change keeps `ID` and `FirstSeenAt`.
- A failed fetch keeps the last good rows and sets `LastError`.
- A disabled `espn-nfl` row means nothing is collected.
- The season crossing the new year: January and February postseason games are collected.

SQL validation goes against a named staging database only, never production.

## 6. Rollout

1. Run `--configure-nfl` (it registers the source disabled).
2. Enable the NFL source in myNewspaper Admin.
3. Preview the import: `cargo run --locked -- --events-from-db --dry-run -o /tmp/schedule-preview.xml`.
4. Import: `cargo run --locked -- --events-from-db -o /tmp/schedule-preview.xml`.
5. Check the result:

```sql
SELECT COUNT(*) AS games, MIN(EventDate) AS first_game, MAX(EventDate) AS last_game,
       SUM(CASE WHEN TimeTbd = 1 THEN 1 ELSE 0 END) AS tbd, MAX(LastSeenAt) AS seen
FROM dbo.SportsEvents
WHERE ProviderKey = 'espn-nfl';
```

Expect about 16 games a week for the rest of the regular season, and no rows with fewer than two teams.

## 7. Web follow-up (myNewspaper repository; for reference, not part of the scraper work)

Once NFL rows exist, myNewspaper needs:
- `EventsQuery.GetScheduleChoicesAsync`: add `"espn-nfl" => "NFL"`. Otherwise the dropdown choice is labelled just
  "Football", next to "Big 12 Football".
- `GameGroups`: add the `NFL` league heading.
- `ScheduleCoverage.Map`: add `"espn-nfl" => ("football", "nfl", null)`.
- A check that `EventsQuery.Visible()` shows the rows. If they follow the NHL shape above, it should.
- Tests, updates to `docs/sports.md`, and mobile checks of `/events?schedule=espn-nfl|Football`.
