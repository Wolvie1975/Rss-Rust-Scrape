-- Seed TechReleaseProjects and TechStatusServices (repeatable and preserves edits)

-- 1. TechReleaseProjects
MERGE dbo.TechReleaseProjects AS target
USING (VALUES
    (N'linux-mainline', N'Linux kernel', N'kernel', N'kernel_org', N'https://www.kernel.org/releases.json', N'mainline', N'Changelog', 0, 1, 10),
    (N'linux-lts', N'Linux kernel (LTS)', N'kernel', N'kernel_org', N'https://www.kernel.org/releases.json', N'longterm', N'Changelog', 0, 1, 20),
    (N'fedora', N'Fedora', N'distro', N'rss', N'https://fedoramagazine.org/tag/releases/feed/', N'Fedora (?:Linux )?(\d+) is', N'Announcement', 0, 1, 30),
    (N'ubuntu', N'Ubuntu', N'distro', N'rss', N'https://ubuntu.com/blog/feed', N'Ubuntu (\d\d\.\d\d(?:\.\d)?)', N'Release notes', 0, 1, 40),
    (N'kde-plasma', N'KDE Plasma', N'desktop', N'rss', N'https://kde.org/announcements/index.xml', N'Plasma (\d+\.\d+(?:\.\d+)?)', N'Announcement', 0, 1, 50),
    (N'gnome', N'GNOME', N'desktop', N'gitlab_tags', N'https://gitlab.gnome.org/api/v4/projects/GNOME%2Fgnome-shell/repository/tags?order_by=updated&per_page=20', N'^(\d+(?:\.\d+)?)$', N'Release notes', 0, 1, 60),
    (N'mesa', N'Mesa', N'graphics', N'gitlab_tags', N'https://gitlab.freedesktop.org/api/v4/projects/mesa%2Fmesa/repository/tags?order_by=updated&per_page=20', N'^mesa-(\d+\.\d+\.\d+)$', N'Release notes', 0, 1, 70),
    (N'proton', N'Proton', N'gaming', N'github_releases', N'https://api.github.com/repos/ValveSoftware/Proton/releases', NULL, N'Changelog', 0, 1, 80),
    (N'linux-mint', N'Linux Mint', N'distro', N'rss', N'https://blog.linuxmint.com/?s=released&feed=rss2', N'Linux Mint (\d+(?:\.\d+)?) .*released', N'Announcement', 0, 1, 90)
) AS source (Slug, Name, Kind, SourceKind, SourceUrl, SourceFilter, NotesLabel, IncludePrerelease, Enabled, SortOrder)
ON target.Slug = source.Slug
WHEN NOT MATCHED THEN
    INSERT (Slug, Name, Kind, SourceKind, SourceUrl, SourceFilter, NotesLabel, IncludePrerelease, Enabled, SortOrder)
    VALUES (source.Slug, source.Name, source.Kind, source.SourceKind, source.SourceUrl, source.SourceFilter, source.NotesLabel, source.IncludePrerelease, source.Enabled, source.SortOrder);

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

