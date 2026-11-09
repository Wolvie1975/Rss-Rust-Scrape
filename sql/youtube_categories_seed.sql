-- Creates the YouTube channel categories and assigns the current channels by ChannelId.
-- Repeatable; channels not listed here (including ones added later) keep their category.
SET XACT_ABORT ON;
BEGIN TRY
 BEGIN TRANSACTION;
 INSERT dbo.YoutubeCategories(Category_Name)
 SELECT c.Name FROM (VALUES(N'Sports'),(N'Video Games'),(N'Tech'),(N'News & Politics')) c(Name)
 WHERE NOT EXISTS(SELECT 1 FROM dbo.YoutubeCategories WITH(UPDLOCK,HOLDLOCK) WHERE Category_Name=c.Name);
 UPDATE f SET YoutubeCategoryId=c.ID
 FROM dbo.YoutubeVideoFeed f
 JOIN (VALUES
  (N'UC9by2xjmM_ldmvIwYrARCDg',N'Sports'),       -- Field of 68
  (N'UCPGJ6U6tzIKHubr5zFWFZLw',N'Sports'),       -- BYU Sports Nation
  (N'UCIaiqufEekxiTnnLc129t2g',N'Sports'),       -- Locked On Big 12
  (N'UC4DgNb05EO8oom2NZsmAQXg',N'Sports'),       -- Locked on BYU
  (N'UCLnfOCTbfqMy_3ah8OmTHEQ',N'Sports'),       -- Big 12 Sports
  (N'UCeFb_xyBgOekQPZYC7Ijilw',N'Sports'),       -- CBS - Eye on College Basketball
  (N'UCYKMXVCgo3h9fK5BtxDIorg',N'Sports'),       -- Open For Business: College Football
  (N'UCaqPH-Ckzu_pSoO3AKcatNw',N'Sports'),       -- Sleepers Media
  (N'UCJ3oBwEVD3IpRAyn9FqNA1w',N'Sports'),       -- Awful Announcing
  (N'UCUcUtJQNQFFtKWIRRuC7ojw',N'Sports'),       -- Robin Lundberg
  (N'UCgUOWrjLEhmHE9tqYtoeS_g',N'Video Games'),  -- Destin
  (N'UCt8CJiye87FMbEwt0npPMuQ',N'Video Games'),  -- Indie Games Hub
  (N'UCGghOrI5QA8TsnhN3tIcx4w',N'Video Games'),  -- Xbox Nation
  (N'UC6RNPkZr2YxjP6zbC9FaK0A',N'Video Games'),  -- ID@Xbox
  (N'UCjBp_7RuDBUYbd1LegWEJ8g',N'Video Games'),  -- Xbox
  (N'UCmXH6CX_juGwNFTj3PESdoA',N'Video Games'),  -- MrMatty Plays
  (N'UCyGYHo1qVIeGq3ZLnSDaEcg',N'Video Games'),  -- Colton Eastwood
  (N'UC7Cj7VhPm234Zclm0_rTGPg',N'Video Games'),  -- CultOfMush
  (N'UCEcrRXW3oEYfUctetZTAWLw',N'Tech'),         -- WaveForm Podcast
  (N'UCddiUEpeqJcYeBxX1IVBKvQ',N'Tech'),         -- The Verge
  (N'UC9M7-jzdU8CVrQo1JwmIdWA',N'Tech'),         -- Android
  (N'UCBJycsmduvYEL83R_U4JriQ',N'Tech'),         -- Marques Brownlee
  (N'UCey_c7U86mJGz1VJWH5CYPA',N'Tech'),         -- iJustine
  (N'UCSOpcUkE-is7u7c4AkLgqTw',N'Tech'),         -- MrMobile
  (N'UCdBK94H6oZT2Q7l0-b0xmMg',N'Tech'),         -- ShortCircuit
  (N'UC-mail1GYasziIlVjn7gRzw',N'Tech'),         -- C. Scott Brown
  (N'UCCKQStaJH5LuMGHHeM9Y-OA',N'Tech'),         -- Shane Craig
  (N'UC2J-0g_nxlwcD9JBK1eTleQ',N'Tech'),         -- Auto Focus
  (N'UC4m4jfyM0VrEdNkf7ZtRx9g',N'Tech'),         -- CodingBite
  (N'UC5UAwBUum7CPN5buc-_N1Fw',N'Tech'),         -- The Linux Experiment
  (N'UCmeU2DYiVy80wMBGZzEWnbw',N'Tech'),         -- Paul J Lipsky
  (N'UCG4Hp1KbGw4e02N7FpPXDgQ',N'News & Politics') -- Bulwark
 ) a(ChannelId,Category) ON a.ChannelId=f.ChannelId
 JOIN dbo.YoutubeCategories c ON c.Category_Name=a.Category
 WHERE f.YoutubeCategoryId IS NULL OR f.YoutubeCategoryId<>c.ID;
 COMMIT;
END TRY
BEGIN CATCH
 IF @@TRANCOUNT>0 ROLLBACK;
 THROW;
END CATCH;
