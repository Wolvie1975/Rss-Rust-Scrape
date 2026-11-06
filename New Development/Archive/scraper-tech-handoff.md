# Scraper handoff: three feeds for the Tech section

Prepared October 31, 2026 (America/Chicago) for the separate **Rss-Rust-Scrape** project.

**Save as:** `docs/tech/scraper-tech-handoff.md` in the web repo. Copy it into the scraper repo's docs if you keep
handoffs there.

## User intent

Jason is adding a **Tech** section to myNewspaper (web guides `docs/tech/tech-00…05`). It runs mostly on headlines
the scraper already collects. Three features need small new feeds:

1. **Release tracker:** the newest version of a short list of projects (Linux kernel, distros, desktops, Mesa,
   Proton…), read from each project's **own** release source, not guessed from headlines.
2. **What developers are reading:** the current top stories on **Hacker News** and **Lobsters**.
3. **Service status:** whether a handful of services (GitHub, Cloudflare, Discord…) report a problem right now.

He explicitly decided **not** to track game releases, security advisories, podcasts or a tech calendar. Don't add
them.

The web app only **reads** these tables. It never fetches from the internet (`DEPLOY.md`). Table creation, seeding,
collection and enable flags belong to the scraper, in its own `sql/` migrations.

This handoff is a specification, not a change log. Nothing was changed in the scraper or the database while writing
it.

## Before you build: confirm every endpoint

The endpoints below are what these services offered as of mid-2026. **Check each one is still available, unchanged
and allowed** (terms, rate limits, User-Agent requirements) before relying on it. Where one is marked **confirm**, I
wasn't certain of the exact URL or format. Find the current one, or drop that row and tell Jason. Don't invent a
substitute source without saying so.

---

## Feed 1: release tracker

### Tables

```sql
-- Which projects to track and where each one announces releases. Seeded below; editable later (Admin, or SQL).
CREATE TABLE dbo.TechReleaseProjects (
    Id             int IDENTITY(1,1) NOT NULL CONSTRAINT PK_TechReleaseProjects PRIMARY KEY,
    Slug           nvarchar(40)   NOT NULL CONSTRAINT UQ_TechReleaseProjects_Slug UNIQUE,   -- "linux-mainline"
    Name           nvarchar(80)   NOT NULL,                                                 -- "Linux kernel"
    Kind           nvarchar(16)   NOT NULL,      -- kernel | distro | desktop | graphics | gaming | app   (shown as the row's tag)
    SourceKind     nvarchar(20)   NOT NULL,      -- kernel_org | github_releases | gitlab_tags | rss
    SourceUrl      nvarchar(400)  NOT NULL,
    SourceFilter   nvarchar(200)  NULL,          -- kernel_org: moniker (mainline | stable | longterm); rss/gitlab_tags: version regex with one capture group
    NotesLabel     nvarchar(30)   NOT NULL CONSTRAINT DF_TechReleaseProjects_NotesLabel DEFAULT N'Release notes',
    IncludePrerelease bit         NOT NULL CONSTRAINT DF_TechReleaseProjects_Pre DEFAULT 0,
    Enabled        bit            NOT NULL CONSTRAINT DF_TechReleaseProjects_Enabled DEFAULT 1,
    SortOrder      int            NOT NULL CONSTRAINT DF_TechReleaseProjects_Sort DEFAULT 100,
    LastCheckedAt  datetime2      NULL,
    LastError      nvarchar(400)  NULL,
    CONSTRAINT CK_TechReleaseProjects_Kind CHECK (Kind IN (N'kernel', N'distro', N'desktop', N'graphics', N'gaming', N'app')),
    CONSTRAINT CK_TechReleaseProjects_Source CHECK (SourceKind IN (N'kernel_org', N'github_releases', N'gitlab_tags', N'rss'))
);

-- Every release seen. Small (a few rows per project per month); keep them all.
CREATE TABLE dbo.TechReleases (
    Id            int IDENTITY(1,1) NOT NULL CONSTRAINT PK_TechReleases PRIMARY KEY,
    ProjectId     int            NOT NULL CONSTRAINT FK_TechReleases_Project REFERENCES dbo.TechReleaseProjects(Id) ON DELETE CASCADE,
    Version       nvarchar(64)   NOT NULL,       -- "6.18", "44", "6.5.2" (no "v" prefix)
    ReleasedAt    datetime2      NOT NULL,       -- UTC; the source's date, else FirstSeenAt
    NotesUrl      nvarchar(400)  NULL,           -- changelog / announcement / release page
    Title         nvarchar(300)  NULL,           -- the source's own title, for reference
    IsPrerelease  bit            NOT NULL CONSTRAINT DF_TechReleases_Pre DEFAULT 0,
    FirstSeenAt   datetime2      NOT NULL CONSTRAINT DF_TechReleases_Seen DEFAULT SYSUTCDATETIME(),
    CONSTRAINT UQ_TechReleases_Project_Version UNIQUE (ProjectId, Version)
);
CREATE INDEX IX_TechReleases_ReleasedAt ON dbo.TechReleases (ReleasedAt DESC) INCLUDE (ProjectId, Version);
```

### Collectors (one per `SourceKind`)

| SourceKind | Endpoint | What to read |
|---|---|---|
| `kernel_org` | `https://www.kernel.org/releases.json` | `releases[]` entries whose `moniker` equals `SourceFilter`. For `longterm` take the **highest** version. Use `version`, `released.isodate` and `changelog` (→ NotesUrl, or the `gitweb`/`source` link if changelog is empty). One fetch serves every kernel row |
| `github_releases` | `https://github.com/{owner}/{repo}/releases.atom` | Each `<entry>`: title or tag → Version (strip a leading `v` and any project-name prefix), `<updated>` → ReleasedAt, `<link href>` → NotesUrl. Skip entries whose tag contains `rc`, `beta`, `alpha` or `pre` unless `IncludePrerelease` |
| `gitlab_tags` | `https://{host}/api/v4/projects/{url-encoded path}/repository/tags?order_by=updated&per_page=20` | Tag `name` through `SourceFilter`'s regex → Version. `commit.created_at` or `created_at` → ReleasedAt. NotesUrl = the project's release notes page if known, else the tag page. Skip pre-releases as above |
| `rss` | The project's announcement feed | Each item's title through `SourceFilter`'s regex. When it matches, the capture group → Version, `pubDate` → ReleasedAt and `link` → NotesUrl. Items that don't match are ignored (announcement feeds carry other news) |

- **Upsert** on `(ProjectId, Version)`. Never delete releases.
- **Set** `LastCheckedAt`; set `LastError` on failure, and clear it on success.
- **One failing project must not stop the others.**
- **Interval:** **hourly**. Send a clear User-Agent (`myNewspaper-scraper/1.0 (+contact)`). For GitHub/GitLab, stay
  well within anonymous rate limits. Hourly for a handful of projects is fine, but don't poll faster.

### Seed (adjust, and confirm each URL and regex against a real feed first)

| Slug | Name | Kind | SourceKind | SourceUrl | SourceFilter | NotesLabel |
|---|---|---|---|---|---|---|
| linux-mainline | Linux kernel | kernel | kernel_org | https://www.kernel.org/releases.json | `mainline` | Changelog |
| linux-lts | Linux kernel (LTS) | kernel | kernel_org | (same) | `longterm` | Changelog |
| fedora | Fedora | distro | rss | Fedora Magazine feed, **confirm** | e.g. `Fedora (?:Linux )?(\d+) is (?:now )?available` | Announcement |
| ubuntu | Ubuntu | distro | rss | Ubuntu release announcements feed, **confirm** | e.g. `Ubuntu (\d\d\.\d\d(?:\.\d)?)` | Release notes |
| kde-plasma | KDE Plasma | desktop | rss | KDE announcements feed, **confirm** | e.g. `Plasma (\d+\.\d+(?:\.\d+)?)` | Announcement |
| gnome | GNOME | desktop | gitlab_tags | gitlab.gnome.org, project `GNOME/gnome-shell`, **confirm** | `^(\d+(?:\.\d+)?)$` (skips alpha/beta/rc) | Release notes |
| mesa | Mesa | graphics | gitlab_tags | gitlab.freedesktop.org, project `mesa/mesa`, **confirm** | `^mesa-(\d+\.\d+\.\d+)$` | Release notes |
| proton | Proton | gaming | github_releases | https://github.com/ValveSoftware/Proton/releases.atom | (none) | Changelog |
| linux-mint | Linux Mint | distro | rss | Linux Mint blog feed, **confirm** | e.g. `Linux Mint (\d+(?:\.\d+)?) .*released` | Announcement |

**DistroWatch** publishes release news covering many distros at once. If its feed is allowed and stable, one `rss`
row per distro pointing at it (with a distro-specific regex) can replace the individual distro feeds. **Confirm** its
terms first.

---

## Feed 2: what developers are reading

### Table (replaced on every run)

```sql
CREATE TABLE dbo.TechDevReading (
    Site          nvarchar(12)   NOT NULL,       -- hn | lobsters
    Rank          int            NOT NULL,       -- 1 = top
    ExternalId    nvarchar(40)   NOT NULL,
    Title         nvarchar(400)  NOT NULL,
    Url           nvarchar(800)  NULL,           -- the linked article; NULL for text posts (Ask HN)
    Domain        nvarchar(120)  NULL,           -- "github.com" (from Url); NULL for text posts
    Points        int            NOT NULL,
    Comments      int            NOT NULL,
    DiscussionUrl nvarchar(400)  NOT NULL,       -- the HN item / Lobsters comments page
    PostedAt      datetime2      NULL,           -- UTC
    SnapshotAt    datetime2      NOT NULL,
    CONSTRAINT PK_TechDevReading PRIMARY KEY (Site, Rank),
    CONSTRAINT CK_TechDevReading_Site CHECK (Site IN (N'hn', N'lobsters'))
);
```

### Collectors

| Site | Endpoint | Notes |
|---|---|---|
| hn | `https://hacker-news.firebaseio.com/v0/topstories.json`, then `…/v0/item/{id}.json` for the first **10** ids | `title`, `url`, `score` → Points, `descendants` → Comments, `time` (Unix) → PostedAt. DiscussionUrl = `https://news.ycombinator.com/item?id={id}`. Skip `dead`/`deleted` items and keep 10 |
| lobsters | `https://lobste.rs/hottest.json` (**confirm**; `/rss` is the fallback) | The first **10**: `title`, `url`, `score`, `comment_count`, `comments_url`, `created_at` |

- **Each run:** fetch, then in **one transaction** delete the site's rows and insert the new top 10. If a fetch fails,
  **leave the previous rows**, so the web keeps showing the last good list (with its `SnapshotAt`).
- **Interval:** every **15 minutes**. For HN, fetch the 10 items concurrently but politely (a small concurrency
  limit).

---

## Feed 3: service status

### Tables

```sql
CREATE TABLE dbo.TechStatusServices (
    Id         int IDENTITY(1,1) NOT NULL CONSTRAINT PK_TechStatusServices PRIMARY KEY,
    Slug       nvarchar(30)  NOT NULL CONSTRAINT UQ_TechStatusServices_Slug UNIQUE,
    Name       nvarchar(60)  NOT NULL,
    Kind       nvarchar(16)  NOT NULL,          -- statuspage (Atlassian Statuspage JSON); others only if added deliberately
    ApiUrl     nvarchar(300) NOT NULL,          -- e.g. https://www.githubstatus.com/api/v2/summary.json
    PageUrl    nvarchar(300) NOT NULL,          -- the human status page, linked from the strip
    Enabled    bit           NOT NULL CONSTRAINT DF_TechStatusServices_Enabled DEFAULT 1,
    SortOrder  int           NOT NULL CONSTRAINT DF_TechStatusServices_Sort DEFAULT 100,
    CONSTRAINT CK_TechStatusServices_Kind CHECK (Kind IN (N'statuspage'))
);
CREATE TABLE dbo.TechServiceStatus (
    ServiceId         int           NOT NULL CONSTRAINT PK_TechServiceStatus PRIMARY KEY
                                    CONSTRAINT FK_TechServiceStatus_Service REFERENCES dbo.TechStatusServices(Id) ON DELETE CASCADE,
    Indicator         nvarchar(12)  NOT NULL,   -- none | minor | major | critical | maintenance | unknown
    Description       nvarchar(200) NULL,       -- "All Systems Operational" / "Partial System Outage"
    IncidentName      nvarchar(300) NULL,       -- the newest unresolved incident, if any
    IncidentComponent nvarchar(120) NULL,       -- the first affected component ("Actions")
    IncidentStartedAt datetime2     NULL,       -- UTC
    IncidentUrl       nvarchar(400) NULL,
    CheckedAt         datetime2     NOT NULL,
    LastError         nvarchar(400) NULL
);
```

### Collector

Atlassian Statuspage's public `summary.json`:

- `status.indicator` → Indicator. If any component is `under_maintenance` and nothing worse, use `maintenance`.
- `status.description` → Description.
- The newest entry in `incidents[]` with status other than `resolved`/`postmortem` → IncidentName,
  `created_at` → IncidentStartedAt, `shortlink` → IncidentUrl.
- Its first `components[].name` → IncidentComponent.

**Upsert** one row per service. On a failed fetch, set `Indicator = 'unknown'` and `LastError` only after **three**
consecutive failures, so a blip doesn't show as an outage. **Interval:** every **5 minutes**.

### Seed (confirm each still runs on Statuspage)

| Slug | Name | ApiUrl | PageUrl |
|---|---|---|---|
| github | GitHub | https://www.githubstatus.com/api/v2/summary.json | https://www.githubstatus.com |
| cloudflare | Cloudflare | https://www.cloudflarestatus.com/api/v2/summary.json | https://www.cloudflarestatus.com |
| discord | Discord | https://discordstatus.com/api/v2/summary.json | https://discordstatus.com |

The mockup also shows **Steam** and **Xbox Live**. As far as I know, neither publishes an official Statuspage-format
feed (Steam's popular status site is unofficial). Leave them out unless you find an official, allowed JSON source.
If you do, add a new `Kind` and collector deliberately, and tell Jason, rather than scraping a status web page. The
web strip just shows whichever services are seeded.

---

## Database permissions and rollout

1. Create the five tables with the scraper's usual repeatable migration style in its `sql/` folder, and seed the
   project and service rows.
2. Grant the **web app's database user `SELECT`** on `TechReleaseProjects`, `TechReleases`, `TechDevReading`,
   `TechStatusServices` and `TechServiceStatus`, as for the other scraper-owned tables the web reads
   (`DEPLOY.md`: "Feature prerequisites include SELECT access…").
3. Turn the three jobs on and let them run once. Then give Jason a short note:
   - which seed rows were confirmed, changed or dropped (with the URLs used);
   - the first run's row counts;
   - any `LastError` values.
4. The web side (`tech-04-live-data.md`) checks that the tables exist and hides each feature until they do, so the
   order of deployment doesn't matter.

## Acceptance

- **Release tracker:**
  - After one run, `TechReleases` has at least one row per enabled project, with the correct latest version against
    each project's own site.
  - Re-running adds no duplicates.
  - A broken URL on one project sets its `LastError` and the others still update.
- **What developers are reading:** 10 rows per site, matching each site's front page within 15 minutes. A forced
  fetch failure leaves the previous 10 in place.
- **Service status:** one row per service. A service with an open incident shows its name, component and link. Three
  forced failures in a row produce `unknown`, and one doesn't.
- **Logging:** each job logs a line per run (counts, duration, errors) in the scraper's existing format.
