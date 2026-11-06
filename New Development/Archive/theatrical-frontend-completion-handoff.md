# Frontend handoff: theatrical releases without physical discs

Prepared October 22, 2026 (America/Chicago).

## Outcome and remaining frontend scope

The scraper fix is deployed. The live frontend already displays the restored
theatrical data correctly: six films on Movies and five on Home, its existing cap.
No frontend schema change or deployment is required merely to restore coverage.
Finish by reviewing provider controls/help text, preserving the query rules below,
and adding the relevant frontend regression coverage. Do not assume another
backend migration or provider enablement step remains outstanding.

The user wants theatrical, digital rental/purchase and subscription releases,
without DVD, Blu-ray or other physical-disc releases. A provider's name does not
identify the release channels it supplies.

## What was deployed

Scraper image: `web-scraper:theatrical-no-disc-20261022`.
Frontend image at verification: `mydailyfeed-web:theatrical-window-20261022`;
this scraper task did not change the frontend image or files.

- Existing `dvdsreleasedates` provider (source ID 5) is enabled again.
- New `MovieReleaseSources.CollectDisc` BIT is 0 for that provider. Its default
  is 1 for compatibility; unrelated provider settings were preserved.
- The collector skips physical-disc calendars, excludes physical releases from
  discovered/tracked detail pages, and avoids tracking disc-only movies.
- The SQL upsert independently rejects physical-disc writes for that provider.
- Twenty-four retained scheduled disc rows were marked `withdrawn`. They were
  not relabelled as digital or theatrical releases.
- Existing source/movie/link/release IDs, dates and provenance were preserved.
- When To Stream and Movie Insider remain enabled; their digital/subscription
  behavior was preserved. The hourly command needs no new flag.

Physical bundles such as “4K UHD + Blu-ray + Digital” are `disc`, not digital
rentals. Standalone Digital HD remains `digital`. Public non-disc types are
`theatrical`, `digital`, `digital_rental`, `digital_purchase`, and `subscription`.

## Reader contract — keep these checks

Join:

`MovieReleases → MovieSourceLinks → Movies / MovieReleaseSources`

Public results must continue requiring:

- `MovieReleaseSources.Enabled = 1`.
- `MovieReleases.CountryCode = 'US'`.
- `MovieReleases.ReleaseStatus = 'scheduled'`.
- The selected release type and the view's existing date window.

`CollectDisc=0` means **do not collect physical discs**. It does not mean the
provider is disabled. Do not add a provider-wide filter requiring CollectDisc=1:
that would hide the theatrical dates again. The frontend does not need to read
CollectDisc to render releases correctly; its existing scheduled-status filter
excludes withdrawn discs, including in All releases.

ReleaseDate is a SQL DATE: render the calendar date directly. Do not convert it
from UTC, which can move a Friday release to Thursday. Resolve “today” and Friday
using the configured display zone (Central in production).

### Home: Friday theater releases

Preserve the existing behavior:

- Thursday–Sunday: show releases for that week's Friday.
- Saturday/Sunday retain Friday rather than advancing to next week.
- Monday–Wednesday: hide the subsection.
- Deduplicate movies before applying the five-film display limit.
- Keep “View all” linking to the theatrical Movies view.

The full Movies view and Home intentionally have different date windows. Do not
apply Movies' `ReleaseDate >= today` rule to Home on Saturday/Sunday; it would
hide the Friday releases prematurely.

### Movies: `/movies?type=theatrical`

Show scheduled US theatrical releases from today through month end, using the
existing provider/status checks. Do not inherit Home's five-film cap. All releases
must also exclude withdrawn rows. Keep subscription and digital views' existing
provider selection and date behavior.

## Provider controls and wording

Inspect the current settings/admin UI before deciding whether any code change is
needed. Disabling “DVDs Release Dates” disables theatrical and digital coverage
from that provider too. If the UI currently suggests this is a DVD-only switch,
clarify its label or help text, for example:

> Supplies theater and digital release dates. Physical-disc releases are excluded.

Do not wire a “hide DVDs” preference to the provider-wide Enabled flag. The
backend disc policy is already configured. If an admin UI will edit CollectDisc
later, coordinate that as a separate supported write flow: changing the flag alone
does not withdraw retained scheduled disc rows. The deployed scraper-side script
handles both operations. Do not grant the public frontend SQL account additional
write/migration privileges to finish this task.

## Verified data — dated acceptance baseline

The current provider detail pages confirmed these US theatrical dates; they are
provider claims, not a promise of availability at every local theater.

| Title | Date |
| --- | --- |
| A Talent for Murder | October 23, 2026 |
| Ali G: Who Iz I? | October 23, 2026 |
| Clayface | October 23, 2026 |
| Klara and the Sun | October 23, 2026 |
| Wildwood | October 23, 2026 |
| You Can See Everything | October 23, 2026 |

At October 22, 8:58:38 AM CDT, all three movie providers had NULL LastError.
Two deployed imports each saved 17 non-disc releases with zero issues. Final
scheduled totals were six theatrical, six digital and five subscription releases,
with 24 withdrawn disc rows and zero scheduled discs. All 40 pre-change release
IDs/dates were preserved; one new digital release was added. Counts are dated
observations: future imports and normal retention will change them.

Playwright Chromium verified Home, theatrical Movies and All releases at
1440×1000 and 390×844: HTTP 200, six theatrical titles on Movies, five on Home,
no physical-disc badges and no horizontal overflow. Some films still have the
existing unmatched-poster placeholder. Keep a useful title/placeholder fallback;
do not invent a poster match. Ticket checkout and saved-item persistence were
not exercised by the scraper task.

## Frontend completion checks

Review the existing frontend files:

- `src/MyDailyFeed.Web/Data/MoviesQuery.cs`
- `src/MyDailyFeed.Web/Data/DigitalReleasesQuery.cs`
- `src/MyDailyFeed.Web/Data/Models/Movie.cs`
- `src/MyDailyFeed.Web/Components/Pages/Movies.razor`
- `src/MyDailyFeed.Web/Components/Pages/Home.razor`
- `docs/entertainment.md`

Use isolated fixtures and a fixed Central-time clock for regression checks:

1. An enabled provider with CollectDisc=0 still returns scheduled theatrical and
   digital records. Its withdrawn physical discs never appear in All releases.
2. A disabled provider contributes no public releases of any type.
3. Home chooses the same Friday Thursday through Sunday, hides Monday through
   Wednesday, and applies its five-film cap after movie deduplication.
4. The theatrical Movies view shows all six fixture movies without Home's cap;
   its own today-through-month-end rule remains intact.
5. Digital/subscription provider choices and results remain unchanged.
6. Missing posters, ticket links, keyboard navigation and phone/desktop layouts
   remain usable. Check saved-item behavior if those controls are changed.

Recheck live data at completion instead of asserting the six October 23 films
will always be present. Report actual frontend changes and deployment status;
if the current code already satisfies these checks, document verification rather
than introducing an unnecessary reader change.

## Read-only SQL diagnostics

Resolve providers by SourceKey rather than assuming environment-specific IDs:

```sql
SELECT s.SourceKey, s.Enabled, s.CollectDisc, s.LastScrapedAt, s.LastError,
       r.ReleaseType, r.ReleaseStatus, COUNT(*) AS ReleaseCount
FROM dbo.MovieReleaseSources s
JOIN dbo.MovieSourceLinks l ON l.MovieReleaseSourceId = s.ID
JOIN dbo.MovieReleases r ON r.MovieSourceLinkId = l.ID
GROUP BY s.SourceKey, s.Enabled, s.CollectDisc, s.LastScrapedAt, s.LastError,
         r.ReleaseType, r.ReleaseStatus
ORDER BY s.SourceKey, r.ReleaseType, r.ReleaseStatus;

DECLARE @Today DATE = CONVERT(date,
    SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time');
SELECT m.ID AS MovieId, m.Title, r.ID AS ReleaseId, r.ReleaseDate, s.SourceKey
FROM dbo.MovieReleases r
JOIN dbo.MovieSourceLinks l ON l.ID = r.MovieSourceLinkId
JOIN dbo.Movies m ON m.ID = l.MovieId
JOIN dbo.MovieReleaseSources s ON s.ID = l.MovieReleaseSourceId
WHERE s.Enabled = 1
  AND r.CountryCode = 'US'
  AND r.ReleaseStatus = 'scheduled'
  AND r.ReleaseType = 'theatrical'
  AND r.ReleaseDate >= @Today
  AND r.ReleaseDate < DATEADD(day, 1, EOMONTH(@Today))
ORDER BY r.ReleaseDate, m.Title, r.ID;
```

## Backend references and rollback

- [Configuration and frontend contract](../docs/theatrical-without-discs.md)
- [Deployment, preservation checks and rollback](../reports/theatrical-no-disc-20261022.md)
- [Explicit disc policy](../sql/movie_disable_discs.sql)
- [Original frontend-to-scraper investigation](scraper-theatrical-handoff.md)

Before rolling the scraper back to an image that ignores CollectDisc, disable
`dvdsreleasedates` again. Keep the additive schema and withdrawn rows; do not
restore the shared database. Frontend rendering does not require this rollback.

This handoff is in the locally ignored `New Development/` folder for transfer to
the frontend session. The canonical backend documentation above is tracked.
