-- Upcoming ranked games this week (Monday-Sunday Central), excluding Big 12 games and cancelled games.
CREATE OR ALTER VIEW dbo.SportsScoreboardTop25Week AS
WITH CurrentWeek AS (
    SELECT
        -- Central calendar date today
        CONVERT(DATE, SYSUTCDATETIME() AT TIME ZONE 'UTC' AT TIME ZONE 'Central Standard Time') AS CentralToday
),
WeekBounds AS (
    SELECT
        -- Monday of current week: DATEADD(day, 1 - DATEPART(weekday, CentralToday), ...) adjusted for SET DATEFIRST
        -- Using standard DATEADD based on known Monday anchor (1900-01-01 was a Monday)
        DATEADD(DAY, (DATEDIFF(DAY, '1900-01-01', CentralToday) / 7) * 7, '1900-01-01') AS WeekMonday,
        DATEADD(DAY, (DATEDIFF(DAY, '1900-01-01', CentralToday) / 7) * 7 + 6, '1900-01-01') AS WeekSunday
    FROM CurrentWeek
),
Big12Teams AS (
    SELECT ft.TeamId, f.League
    FROM dbo.ResultFollows f
    JOIN dbo.ResultFollowTeams ft ON ft.FollowId = f.ID
    WHERE f.Enabled = 1
      AND f.Kind = 'conference'
      AND f.League IN ('college-football', 'mens-college-basketball')
)
SELECT s.*
FROM dbo.SportsScoreboard s
CROSS JOIN WeekBounds wb
WHERE s.League IN ('college-football', 'mens-college-basketball')
  AND (s.HomeRank IS NOT NULL OR s.AwayRank IS NOT NULL)
  AND s.ScoreboardDate BETWEEN wb.WeekMonday AND wb.WeekSunday
  AND s.Status <> 'cancelled'
  AND NOT EXISTS (
      SELECT 1 FROM Big12Teams b12
      WHERE b12.League = s.League
        AND (b12.TeamId = s.HomeTeamId OR b12.TeamId = s.AwayTeamId)
  );

