-- Run as deployment owner after article_identity.sql and upsert_article_identity.sql.
-- EXECUTE AS OWNER lets the procedure reliably inspect all incoming foreign keys.
GRANT EXECUTE ON dbo.UpsertArticleIdentity TO [WebScraperIngest];
GRANT SELECT ON dbo.ArticleUrlAliases TO [WebScraperIngest];
GRANT SELECT ON dbo.ArticleIdRedirects TO [WebScraperIngest];
GRANT UPDATE(LastCheckedAt) ON dbo.ArticleUrlAliases TO [WebScraperIngest];
-- The frontend's read principal needs SELECT on ArticleIdRedirects to recover retired saved IDs.
