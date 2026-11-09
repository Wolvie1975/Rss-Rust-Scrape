-- After the two commerce migrations, using migration credentials.
-- Existing ingestion role and base movie permissions are provisioned separately.
GRANT SELECT,INSERT,UPDATE ON dbo.MovieCommerceLinks TO [WebScraperIngest];
GRANT SELECT ON dbo.MovieCommerce TO [WebScraperIngest];
-- Also grant SELECT on dbo.MovieCommerce to the application's existing reader
-- role during frontend deployment; its principal is managed in that application.
