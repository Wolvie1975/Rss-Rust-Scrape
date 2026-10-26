# Frontend handoff: YouTube channel categories

Backend work applied to production **WebScraper** on October 26, 2026. This handoff
is for the separate **My-Newsfeed** frontend repository. No frontend files were
changed by the backend work.

## What changed

Every YouTube channel can now belong to one category. Categories live in a new
lookup table, **separate from news `SourceCategories`**. A YouTube category such as
Tech has no news sources behind it, so do not merge the two lists.

The change only adds things. Existing YouTube tables, columns and queries keep working
unchanged, and videos are still collected hourly with the newest 10 kept per channel.

## Schema

**`dbo.YoutubeCategories`** (new)

| Column | Type | Notes |
| --- | --- | --- |
| `ID` | `INT` | Primary key |
| `Category_Name` | `NVARCHAR(100)` | Unique display name |
| `DateAdded` | `DATETIME2` | UTC |

**`dbo.YoutubeVideoFeed`** (one row per channel) gains:

| Column | Type | Notes |
| --- | --- | --- |
| `YoutubeCategoryId` | `INT NULL` | FK → `YoutubeCategories.ID`; NULL = uncategorized |

Videos get their category through their channel:

```
YouTubeVideos.YoutubeVideoFeedId → YoutubeVideoFeed.ID
YoutubeVideoFeed.YoutubeCategoryId → YoutubeCategories.ID
```

## Current categories

Counts as of October 26, 2026. Look categories up by name, or list them dynamically.
Don't hardcode the IDs.

| ID | Category | Channels | Videos |
| --- | --- | --- | --- |
| 1 | Sports | 10 | 100 |
| 2 | Video Games | 8 | 80 |
| 3 | Tech | 13 | 130 |
| 4 | News & Politics | 1 | 10 |

- **Sports:** Field of 68, BYU Sports Nation, Locked On Big 12, Locked on BYU, Big 12 Sports, CBS - Eye on College Basketball, Open For Business, Sleepers Media, Awful Announcing, Robin Lundberg
- **Video Games:** Destin, Indie Games Hub, Xbox Nation, ID@Xbox, Xbox, MrMatty Plays, Colton Eastwood, CultOfMush
- **Tech:** WaveForm Podcast, The Verge, Android, Marques Brownlee, iJustine, MrMobile, ShortCircuit, C. Scott Brown, Shane Craig, Auto Focus, CodingBite, The Linux Experiment, Paul J Lipsky
- **News & Politics:** Bulwark

## Queries

Both queries below were tested against production.

**Category navigation.** List only categories that have at least one channel:

```sql
SELECT c.ID, c.Category_Name, COUNT(DISTINCT f.ID) AS Channels
FROM dbo.YoutubeCategories c
JOIN dbo.YoutubeVideoFeed f ON f.YoutubeCategoryId = c.ID
GROUP BY c.ID, c.Category_Name
ORDER BY c.Category_Name;
```

**Videos in one category**, newest first. `@category` is the category name and
`@take` is the page size:

```sql
SELECT TOP (@take)
       v.VideoId, v.Title, v.Url, v.ThumbnailUrl, v.Description,
       v.PublishedAt, v.ViewCount,
       COALESCE(f.ChannelName, v.ChannelName) AS ChannelName,
       c.Category_Name
FROM dbo.YouTubeVideos v
JOIN dbo.YoutubeVideoFeed f ON f.ID = v.YoutubeVideoFeedId
LEFT JOIN dbo.YoutubeCategories c ON c.ID = f.YoutubeCategoryId
WHERE c.Category_Name = @category
ORDER BY v.PublishedAt DESC, v.ID DESC;
```

To include the category in an existing all-videos query, add the `JOIN` and
`LEFT JOIN` above and select `c.Category_Name`.

To list uncategorized videos, use `WHERE f.YoutubeCategoryId IS NULL`.

## Behavior to implement

1. **Category filter or tabs** on the YouTube view, built from the navigation
   query. Keep an "All" option showing every video, as the view does today.
2. **Show the category on each video**, for example as a badge, if the design
   has room.
3. **Handle uncategorized channels.** None exist today. A channel the scraper
   adds later starts with `YoutubeCategoryId = NULL` until someone assigns it.
   These channels must still appear under "All". Either hide them from the
   category tabs or group them under an "Other" tab. Never drop them silently.
4. **Hide empty categories.** The navigation query already does this. Never show
   a tab that would be empty.
5. **Keep everything read-only.** The frontend must not write to these tables.
   Categories are assigned by backend migration
   ([`sql/youtube_categories_seed.sql`](../sql/youtube_categories_seed.sql)).

## Notes

- **Database permissions.** The scraper's restricted role was granted `SELECT` on
  `YoutubeCategories`. If My-Newsfeed reads with a different restricted login, that
  login also needs `SELECT ON dbo.YoutubeCategories`. If it already reads the other
  YouTube tables, nothing else changes.
- **Retention.** Retention is unchanged at 10 videos per channel, so a category holds
  about 10 × its channel count. Paging deeper than that returns nothing.
- **Exact column names.** `Category_Name` has an underscore, matching
  `SourceCategories`. The table names differ in capitalization: the channel table is
  `YoutubeVideoFeed`, the videos table is `YouTubeVideos`.

## Acceptance

- The YouTube view lists Sports, Tech, Video Games and News & Politics, and each
  tab shows only that category's channels.
- "All" shows the same videos as before this change.
- Setting a channel's `YoutubeCategoryId` to NULL in a test database removes it
  from its category tab, but it still appears under "All" (or "Other").
- A category with no channels does not appear in the navigation.

Backend reference: [operations guide, YouTube channel categories](../docs/operations.md#youtube-channel-categories).
