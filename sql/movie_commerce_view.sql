-- Separate batch after movie_commerce.sql.
CREATE OR ALTER VIEW dbo.MovieCommerce AS
SELECT c.MovieId,c.Provider,
       CASE WHEN v.IsDirect=1 THEN c.DirectUrl ELSE c.SearchUrl END AS ActionUrl,
       CASE WHEN v.IsDirect=1 THEN 'direct' ELSE 'search' END AS LinkKind,
       CASE WHEN v.IsDirect=0 AND c.Provider='fandango' THEN 'Search Fandango'
            WHEN v.IsDirect=0 THEN 'Search Fandango at Home'
            WHEN c.Provider='fandango' THEN 'Get tickets on Fandango'
            WHEN c.PurchaseVerified=1 THEN 'Buy on Fandango at Home'
            ELSE 'View on Fandango at Home' END AS ActionLabel,
       c.CheckedAt,c.VerifiedUntil
FROM dbo.MovieCommerceLinks c JOIN dbo.Movies m ON m.ID=c.MovieId
CROSS APPLY (SELECT CASE WHEN c.DirectUrl IS NOT NULL AND c.VerifiedUntil>SYSUTCDATETIME()
    THEN 1 ELSE 0 END AS IsDirect) v
WHERE c.IdentityTitle COLLATE Latin1_General_100_BIN2=m.Title COLLATE Latin1_General_100_BIN2
  AND (c.IdentityYear=m.OriginalYear OR (c.IdentityYear IS NULL AND m.OriginalYear IS NULL))
  AND (c.IdentityImdbId=m.ImdbId OR (c.IdentityImdbId IS NULL AND m.ImdbId IS NULL));
