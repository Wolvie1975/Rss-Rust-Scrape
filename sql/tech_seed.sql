-- Seed TechReleaseProjects, TechStatusServices, and HomelabServices (repeatable and preserves edits)

-- 1. TechReleaseProjects
MERGE dbo.TechReleaseProjects AS target
USING (VALUES
    (N'linux-mainline', N'Linux kernel', N'kernel', N'kernel_org', N'https://www.kernel.org/releases.json', N'mainline', N'Changelog', 0, 1, 10, 1),
    (N'linux-lts', N'Linux kernel (LTS)', N'kernel', N'kernel_org', N'https://www.kernel.org/releases.json', N'longterm', N'Changelog', 0, 1, 20, 0),
    (N'fedora', N'Fedora', N'distro', N'rss', N'https://fedoramagazine.org/tag/releases/feed/', N'Fedora (?:Linux )?(\d+) is', N'Announcement', 0, 1, 30, 1),
    (N'ubuntu', N'Ubuntu', N'distro', N'rss', N'https://ubuntu.com/blog/feed', N'Ubuntu (\d\d\.\d\d(?:\.\d)?)', N'Release notes', 0, 1, 40, 0),
    (N'kde-plasma', N'KDE Plasma', N'desktop', N'rss', N'https://kde.org/announcements/index.xml', N'Plasma (\d+\.\d+(?:\.\d+)?)', N'Announcement', 0, 1, 50, 0),
    (N'gnome', N'GNOME', N'desktop', N'gitlab_tags', N'https://gitlab.gnome.org/api/v4/projects/GNOME%2Fgnome-shell/repository/tags?order_by=updated&per_page=20', N'^(\d+(?:\.\d+)?)$', N'Release notes', 0, 1, 60, 0),
    (N'mesa', N'Mesa', N'graphics', N'gitlab_tags', N'https://gitlab.freedesktop.org/api/v4/projects/mesa%2Fmesa/repository/tags?order_by=updated&per_page=20', N'^mesa-(\d+\.\d+\.\d+)$', N'Release notes', 0, 1, 70, 1),
    (N'proton', N'Proton', N'gaming', N'github_releases', N'https://api.github.com/repos/ValveSoftware/Proton/releases', NULL, N'Changelog', 0, 1, 80, 1),
    (N'linux-mint', N'Linux Mint', N'distro', N'rss', N'https://blog.linuxmint.com/?s=released&feed=rss2', N'Linux Mint (\d+(?:\.\d+)?) .*released', N'Announcement', 0, 1, 90, 0),
    (N'dotnet', N'.NET', N'sdk', N'dotnet_releases', N'https://dotnetcli.blob.core.windows.net/dotnet/release-metadata/releases-index.json', NULL, N'Release notes', 0, 1, 100, 1),
    (N'rust', N'Rust', N'toolchain', N'github_releases', N'https://api.github.com/repos/rust-lang/rust/releases', NULL, N'Release notes', 0, 1, 110, 1),
    (N'qt', N'Qt', N'framework', N'rss', N'https://github.com/qt/qtbase/tags.atom', N'^(?:v|qtbase-)?(6\.\d+\.\d+)$', N'Changelog', 0, 1, 120, 1),
    (N'nodejs', N'Node.js', N'runtime', N'github_releases', N'https://api.github.com/repos/nodejs/node/releases', NULL, N'Changelog', 0, 1, 130, 1),
    (N'typescript', N'TypeScript', N'compiler', N'github_releases', N'https://api.github.com/repos/microsoft/TypeScript/releases', NULL, N'Release notes', 0, 1, 140, 1)
) AS source (Slug, Name, Kind, SourceKind, SourceUrl, SourceFilter, NotesLabel, IncludePrerelease, Enabled, SortOrder, InYourStack)
ON target.Slug = source.Slug
WHEN MATCHED THEN
    UPDATE SET InYourStack = source.InYourStack
WHEN NOT MATCHED THEN
    INSERT (Slug, Name, Kind, SourceKind, SourceUrl, SourceFilter, NotesLabel, IncludePrerelease, Enabled, SortOrder, InYourStack)
    VALUES (source.Slug, source.Name, source.Kind, source.SourceKind, source.SourceUrl, source.SourceFilter, source.NotesLabel, source.IncludePrerelease, source.Enabled, source.SortOrder, source.InYourStack);

-- 2. TechStatusServices
MERGE dbo.TechStatusServices AS target
USING (VALUES
    (N'github', N'GitHub', N'statuspage', N'https://www.githubstatus.com/api/v2/summary.json', N'https://www.githubstatus.com', 1, 10),
    (N'cloudflare', N'Cloudflare', N'statuspage', N'https://www.cloudflarestatus.com/api/v2/summary.json', N'https://www.cloudflarestatus.com', 1, 20),
    (N'discord', N'Discord', N'statuspage', N'https://discordstatus.com/api/v2/summary.json', N'https://discordstatus.com', 1, 30)
) AS source (Slug, Name, Kind, ApiUrl, PageUrl, Enabled, SortOrder)
ON target.Slug = source.Slug
WHEN NOT MATCHED THEN
    INSERT (Slug, Name, Kind, ApiUrl, PageUrl, Enabled, SortOrder)
    VALUES (source.Slug, source.Name, source.Kind, source.ApiUrl, source.PageUrl, source.Enabled, source.SortOrder);

-- 3. HomelabServices
MERGE dbo.HomelabServices AS target
USING (VALUES
    (N'plex', N'Plex', N'tautulli', N'http://tautulli:8181/api/v2?cmd=get_activity', NULL, N'TAUTULLI_API_KEY', 1500, N'http://host.docker.internal:32400/web', 1, 10),
    (N'jellyfin', N'Jellyfin', N'http-body', N'http://host.docker.internal:8096/health', N'Healthy', NULL, 1500, N'http://host.docker.internal:8096', 1, 20),
    (N'tautulli', N'Tautulli', N'tautulli', N'http://tautulli:8181/api/v2?cmd=get_activity', NULL, N'TAUTULLI_API_KEY', 1500, N'http://tautulli:8181', 1, 30),
    (N'ollama', N'Ollama', N'http', N'http://ai-machine.local:11434/api/version', NULL, NULL, 1500, NULL, 1, 40),
    (N'open-webui', N'Open WebUI', N'http', N'http://ai-machine.local:8080/health', NULL, NULL, 1500, N'http://ai-machine.local:8080', 1, 50),
    (N'portainer', N'Portainer', N'http', N'http://portainer:9000/api/system/status', NULL, NULL, 1500, N'http://portainer:9000', 1, 60),
    (N'scraper', N'myNewspaper scraper', N'self', NULL, NULL, NULL, 1500, NULL, 1, 70)
) AS source (Slug, Name, CheckKind, CheckUrl, ExpectBody, SecretEnv, SlowMs, LinkUrl, Enabled, SortOrder)
ON target.Slug = source.Slug
WHEN NOT MATCHED THEN
    INSERT (Slug, Name, CheckKind, CheckUrl, ExpectBody, SecretEnv, SlowMs, LinkUrl, Enabled, SortOrder)
    VALUES (source.Slug, source.Name, source.CheckKind, source.CheckUrl, source.ExpectBody, source.SecretEnv, source.SlowMs, source.LinkUrl, source.Enabled, source.SortOrder);

