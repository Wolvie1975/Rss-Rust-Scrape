# NFL division standings — October 13, 2026

Production NFL source ID 10 now uses ESPN's standings?level=3 endpoint.
The deployed parser already supports nested groups; no binary rebuild or schema
change was necessary. Seed configuration was updated for new installations.
Existing configuration was explicitly updated with migration credentials.

Verification: production standings refresh exited 0, NFL LastError is NULL,
and the current snapshot has 32 unique teams across eight divisions with four
teams each. Old conference-group rows are historical (IsCurrent=0).
The existing /standings?sourceId=10 page renders all eight division names.
A new fixture test verifies unique membership, all eight group sizes, provider
order within divisions and Kansas City Chiefs membership in AFC West.

The frontend handoff now documents AFC/NFC parent headings, division ordering,
current-row queries, fallback handling and the compact NFL column mapping:
[my-newsfeed-nfl-standings.md](../docs/my-newsfeed-nfl-standings.md).
The frontend hierarchy change is documented for the site builder; the web app
was not modified or rebuilt in this task.

Rollback: set the NFL source URL back to the same endpoint without ?level=3
and refresh standings. Retained snapshots make the next valid import authoritative.
No commits or pushes performed.
