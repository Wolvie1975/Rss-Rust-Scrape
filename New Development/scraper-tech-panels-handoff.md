# Scraper handoff: Tech front panels (Your stack, Your servers, New in Ollama)

Prepared November 1, 2026 (America/Chicago) for the separate **Rss-Rust-Scrape** project. The web side is
`docs/tech/tech-05-front-panels.md` in the myNewspaper repo (`develop` at `75d6e5f`), which reads everything below
read-only and hides each panel until its tables exist.

**Save as:** `docs/scraper-tech-panels-handoff.md` (myNewspaper repo, for reference) and hand to the scraper session.

## What the scraper adds

| Panel | Scraper work | New tables / columns |
|---|---|---|
| **Your stack** | Seed a few more release projects; mark which ones are "your stack" | `TechReleaseProjects.InYourStack` |
| **Your servers** | A health-check job every minute against self-hosted services, plus a heartbeat for the scraper itself | `HomelabServices`, `HomelabServiceStatus` |
| **New in Ollama** | Poll the Ollama library for new/updated models; ask the AI machine what's installed; work out which sizes fit | `OllamaLibraryModels`, `OllamaLibraryTags` |
| What developers are reading | **Nothing**: `TechDevReadingItems` already exists | none |

The scraper **owns** these tables (as with the `tech-04` tables): it creates them, the web never writes to them, and
the web user gets `SELECT` only.

> This handoff describes **what** to store and the rules. It doesn't assume the scraper's internal module names,
> since that code isn't in this repo. Fit the jobs into its existing scheduler and source-kind patterns.

---

## 1. Your stack

### 1.1 Schema

```sql
IF COL_LENGTH(N'dbo.TechReleaseProjects', N'InYourStack') IS NULL
    ALTER TABLE dbo.TechReleaseProjects ADD InYourStack bit NOT NULL
        CONSTRAINT DF_TechReleaseProjects_InYourStack DEFAULT (0);
```

A project can be in the Linux desk's tracker **and** your stack (the kernel is), which is why this is a flag rather
than a second table.

### 1.2 Projects

Use the scraper's existing source kinds (kernel.org, GitHub releases, feeds, GitLab) wherever they fit. **If a
project is already tracked, just set `InYourStack = 1`** rather than adding a duplicate.

| Project | Kind (display) | Source | Notes |
|---|---|---|---|
| Linux kernel | Kernel | existing kernel.org row | flag only; `-rc` releases are `IsPrerelease = 1` |
| Fedora | Distro | existing distro row, if any | flag only; skip if not tracked |
| Mesa | Graphics | existing row, if any; else GitLab tags `gitlab.freedesktop.org/mesa/mesa` | `-rc` = prerelease |
| Proton | Gaming | GitHub releases `ValveSoftware/Proton` | `experimental-…` and `-rc` = prerelease |
| .NET | SDK | `https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/releases-index.json` | official JSON; take each channel's `latest-release` and `latest-release-date`; preview channels = prerelease. Needs a small new source kind if the scraper has no JSON kind |
| Rust | Toolchain | GitHub releases `rust-lang/rust` | |
| Qt | Framework | GitHub tags feed `https://github.com/qt/qtbase/tags.atom` | Qt doesn't publish GitHub releases; tags `v6.x.y`; `-alpha`, `-beta`, `-rc` = prerelease |
| Node.js | Runtime | GitHub releases `nodejs/node` | |
| TypeScript | Compiler | GitHub releases `microsoft/TypeScript` | `-beta` and `-rc` = prerelease |

`SortOrder` follows the table. `NotesUrl` is each release's page (GitHub release URL, the kernel.org changelog, the
.NET release notes link from the JSON).

> **Check every source URL** before seeding. The table above is what these projects published as of mid-2026 and may
> have moved. Record failures in `LastError` the way the tracker already does.

---

## 2. Your servers

### 2.1 Schema

```sql
CREATE TABLE dbo.HomelabServices (
    ID          int IDENTITY PRIMARY KEY,
    Slug        nvarchar(40)  NOT NULL UNIQUE,
    Name        nvarchar(80)  NOT NULL,
    CheckKind   nvarchar(20)  NOT NULL,          -- http | http-body | tautulli | self
    CheckUrl    nvarchar(400) NULL,              -- internal address; never contains a secret
    ExpectBody  nvarchar(80)  NULL,              -- for http-body, e.g. 'Healthy'
    SecretEnv   nvarchar(60)  NULL,              -- name of an env var holding an API key, e.g. 'TAUTULLI_API_KEY'
    SlowMs      int           NOT NULL DEFAULT (1500),
    LinkUrl     nvarchar(400) NULL,              -- where the web dot links (the service's own UI)
    Enabled     bit           NOT NULL DEFAULT (1),
    SortOrder   int           NOT NULL DEFAULT (0)
);

CREATE TABLE dbo.HomelabServiceStatus (
    ServiceId   int           NOT NULL PRIMARY KEY REFERENCES dbo.HomelabServices(ID),
    State       nvarchar(10)  NOT NULL,          -- ok | slow | down | unknown
    LatencyMs   int           NULL,
    Detail      nvarchar(120) NULL,
    StateSince  datetime2     NOT NULL,          -- UTC, when State last changed
    CheckedAt   datetime2     NOT NULL,          -- UTC, last check
    FailCount   int           NOT NULL DEFAULT (0)
);
```

**Never store an API key in `CheckUrl`.** The web user can read this table. Keys live in the scraper's environment
and are named by `SecretEnv`.

### 2.2 Services to seed

Fill in the real hosts and ports. The addresses below are the usual defaults:

| Slug | Name | CheckKind | CheckUrl (example) | Expect / Detail |
|---|---|---|---|---|
| plex | Plex | tautulli **or** http | via Tautulli below, else `http://<plex-host>:32400/identity` | 200. Detail: "N streaming" when known |
| jellyfin | Jellyfin | http-body | `http://<host>:8096/health` | body `Healthy` |
| tautulli | Tautulli | tautulli | `http://<tautulli-host>:8181/api/v2?cmd=get_activity` (key appended from `TAUTULLI_API_KEY`) | `response.result = success`; `stream_count` also feeds Plex's Detail |
| ollama | Ollama | http | `http://<ai-machine>:11434/api/version` | 200 |
| open-webui | Open WebUI | http | `http://<ai-machine>:<port>/health` | 200 |
| portainer | Portainer | http | `http://<host>:9000/api/system/status` | 200 (use the HTTP port; skip TLS checks rather than disabling verification globally) |
| scraper | myNewspaper scraper | self | none | written every cycle; Detail "last run N min ago" from the last successful feed run |

Nextcloud gets a row (`/status.php`, JSON `installed: true`) when it exists.

### 2.3 The job

- **Every 60 seconds**, each enabled service, concurrently, **5 s timeout**, no redirects followed beyond one.
- **Result**:
  - Response as expected, latency ≤ `SlowMs`: `ok`.
  - Response as expected, latency > `SlowMs`: `slow`.
  - Timeout, connection error, unexpected status or body: a failure. Only after **2 consecutive failures**
    (`FailCount`) does the state become `down`, so one dropped request doesn't flash red.
- **`StateSince`** changes only when `State` changes. **`CheckedAt`** is set on every check.
- **`Detail`**: short and human ("2 streaming", "HTTP 502", "timeout"). No stack traces or internal hostnames.
- **Tautulli**: one `get_activity` call serves both rows: Tautulli's own state, and Plex's Detail (`stream_count`).
  If Tautulli is down, check Plex with its own `/identity` URL instead.
- **Heartbeat (`self`)**: upsert the scraper row at the end of each scheduler cycle with `ok` and "last run N min ago"
  from its newest successful feed fetch, or `slow` if that's over 30 minutes old. If the scraper dies, this row
  stops updating, and the web marks everything as stale after 5 minutes; that's how an outage of the scraper itself
  shows up.

### 2.4 Reaching the services

| Service runs on | How the scraper container reaches it |
|---|---|
| The same Docker network | Container name (`http://tautulli:8181`) |
| The host with **host networking** (Jellyfin) | `host.docker.internal`; add `extra_hosts: ["host.docker.internal:host-gateway"]` to the scraper's compose service |
| **Another machine** (the AI machine: Ollama, Open WebUI) | That machine's **LAN address** if both are on the same network (simplest). Tailscale runs as a container on the prod box, so other containers **aren't on the tailnet by default**: reaching a tailnet address needs either the host itself on Tailscale, a subnet route, or the scraper sharing the Tailscale container's network. **Decide which before seeding those two rows** |

---

## 3. New in Ollama

### 3.1 Configuration (scraper environment)

| Variable | Example | Meaning |
|---|---|---|
| `OLLAMA_URL` | `http://192.168.1.50:11434` | The AI machine's Ollama API (see §2.4 for reachability) |
| `OLLAMA_MEMORY_GB` | `16` | Memory available to models on that machine: **GPU VRAM** if it runs models on the GPU, otherwise usable RAM |

### 3.2 Schema

```sql
CREATE TABLE dbo.OllamaLibraryModels (
    ID           int IDENTITY PRIMARY KEY,
    Name         nvarchar(80)  NOT NULL UNIQUE,
    Description  nvarchar(300) NULL,
    Change       nvarchar(10)  NOT NULL,         -- new | updated
    ChangedAt    datetime2     NOT NULL,         -- UTC
    UpdatedText  nvarchar(40)  NULL,             -- the library's own "Updated 2 days ago", kept to detect updates
    LibraryUrl   nvarchar(300) NOT NULL,
    FirstSeenAt  datetime2     NOT NULL,
    SnapshotAt   datetime2     NOT NULL
);

CREATE TABLE dbo.OllamaLibraryTags (
    ID         int IDENTITY PRIMARY KEY,
    ModelId    int          NOT NULL REFERENCES dbo.OllamaLibraryModels(ID) ON DELETE CASCADE,
    Tag        nvarchar(40) NOT NULL,            -- '8b', '70b', 'e4b'
    SizeGb     decimal(6,1) NULL,
    Fits       bit          NOT NULL DEFAULT (0),
    Installed  bit          NOT NULL DEFAULT (0),
    SortOrder  int          NOT NULL DEFAULT (0),
    CONSTRAINT UQ_OllamaLibraryTags UNIQUE (ModelId, Tag)
);
```

### 3.3 Collecting the library

Ollama has **no public API for its library**. The list comes from the website:

- **List:** `https://ollama.com/search?o=newest`, the first page. Each card has the model name, description, size
  tags (`8b`, `70b` …) and "Updated N days ago".
- **Sizes in GB:** only for models that are new or changed, fetch `https://ollama.com/library/<name>/tags`, which
  lists each tag's download size.
- **Every 6 hours**, a normal `User-Agent` naming the project, no parallel page fetches.
- Parse defensively. If the page shape changes, keep the last good rows, record the error (an `OllamaLibrarySource`
  row or the scraper's existing error log), and **don't delete** anything.

### 3.4 New vs updated

- **First run:** store everything with `Change = 'new'` and `ChangedAt` set to **2000-01-01**, so the first sync is
  a baseline and doesn't show 20 "new" models.
- **A name not seen before:** `new`, `ChangedAt = now`.
- **A known name whose `UpdatedText` now reads newer** (for example "Updated 2 days ago" after "Updated 3 weeks
  ago"), or whose tag list changed: `updated`, `ChangedAt = now`, and refresh its tags.

The web shows rows with `ChangedAt` in the last 7 days.

### 3.5 Installed and Fits (refresh each run, and every hour on their own)

- **Installed:** `GET {OLLAMA_URL}/api/tags` returns installed models as `name:tag` (e.g. `qwen3:8b`). A tag row is
  installed when `<Name>:<Tag>` matches. A bare `<Name>:latest` matches the tag the library marks as the default; if
  that isn't knowable, match `latest` to the smallest size tag. If the AI machine is unreachable, **leave the flags
  as they were** rather than clearing them.
- **Fits:**

  | What's known | Rule |
  |---|---|
  | `SizeGb` | fits when `SizeGb ≤ OLLAMA_MEMORY_GB − 1.5` (headroom for context) |
  | only the tag, like `8b` | estimate `billions × 0.6 + 1` GB (a 4-bit quantised download) and apply the same rule |
  | neither (e.g. `latest`) | `Fits = 0` |

---

## 4. Grants and checks

```sql
GRANT SELECT ON dbo.HomelabServices        TO [<web user>];
GRANT SELECT ON dbo.HomelabServiceStatus   TO [<web user>];
GRANT SELECT ON dbo.OllamaLibraryModels    TO [<web user>];
GRANT SELECT ON dbo.OllamaLibraryTags      TO [<web user>];
-- TechReleaseProjects already has SELECT; the new column comes with it.
```

| Check | Expected |
|---|---|
| `SELECT Name, InYourStack FROM dbo.TechReleaseProjects ORDER BY SortOrder` | The nine projects; `InYourStack = 1` on each; no duplicates of tracker rows |
| Newest `TechReleases` per stack project | Matches each project's real latest version; kernel shows today's 7.4-rc1 as a prerelease |
| `HomelabServiceStatus` after 2 minutes | One row per service, `CheckedAt` within the last minute |
| Stop Jellyfin's container | Two checks later: `down`, `StateSince` = when it first failed; start it: `ok` again |
| Stop the scraper | Its row stops updating (the web turns everything grey after 5 minutes) |
| `OllamaLibraryModels` after the first run | Populated, all with `ChangedAt` 2000-01-01 (baseline) |
| A later run with a genuinely new library model | One row with `Change = 'new'` and today's `ChangedAt` |
| Pull a model on the AI machine | Within an hour, its tag shows `Installed = 1` |
| No secret anywhere in the four tables | `SELECT * FROM dbo.HomelabServices` shows no keys |
