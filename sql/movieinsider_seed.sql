-- Register the independently configurable digital provider; preserve settings.
IF NOT EXISTS(SELECT 1 FROM dbo.MovieReleaseSources WHERE SourceKey='movieinsider')
 INSERT dbo.MovieReleaseSources(SourceKey,Name,Url,Enabled)
 VALUES('movieinsider',N'Movie Insider',N'https://www.movieinsider.com/releases/vod-digital/',0);
