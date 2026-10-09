-- Explicit IDs only. Matched tracking rows retain Enabled and all existing settings.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 MERGE dbo.TvTrackedSeries WITH(HOLDLOCK) AS t
 USING (VALUES
  (83073,N'Avatar: Seven Havens'),
  (64950,N'VisionQuest'),
  (45039,N'Slow Horses'),
  (33352,N'The Lord of the Rings: The Rings of Power'),
  (90632,N'Line of Fire (2026, NBC)')
 ) AS s(TvmazeShowId,ExpectedTitle) ON t.TvmazeShowId=s.TvmazeShowId
 WHEN NOT MATCHED THEN INSERT(TvmazeShowId,ExpectedTitle) VALUES(s.TvmazeShowId,s.ExpectedTitle);
 COMMIT TRANSACTION;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK TRANSACTION;
 THROW;
END CATCH;
