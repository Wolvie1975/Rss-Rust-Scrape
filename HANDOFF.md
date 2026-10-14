# Scraper handoff

## Documentation

Start with [README.md](README.md). Five topic guides replace the overlapping individual handoffs; three history files preserve dated deployment evidence. SQL examples, validation evidence and the previous long handoff remain available there.

## Latest documented production state

Last verified deployment: `web-scraper:cinema-posters-20261013`; scraper and front-end were healthy at verification. This document is not a live status check.

- Hourly collection includes news, rankings, standings, sports schedules, YouTube, movie releases, TV, actual box office and Boxoffice Pro forecasts.
- MLS schedule/results source 5525, standings 9; Championship schedule/results 5326, standings 11; EPL schedule 5357, standings 12; NFL standings 10.
- MLS and Championship current-season results are independent of schedule pruning; playoff shootouts are stored separately from goals.
- Latest actual box-office chart and domestic forecasts have poster URLs. Last checked: 10/10 chart films, 12/12 forecast films and 24/34 release films; unmatched films use placeholders.
- Dedicated Deadline/Variety box-office story sources are disabled. Structured charts and forecasts replace them.
- Current retention and backup rules: [operations guide](docs/operations.md#data-retention). SQL remains in FULL recovery with managed log/full backups.
- Initial performance changes deployed: ESPN batches of 100, stage/HTTP metrics, and filtering past movie releases before persistence. Further caching/concurrency work remains a candidate.
- Front-end work for the newer movie/sports sections is documented in the topic guides; scraper deployment alone does not implement those views.

## Development and deployment

Use runtime credentials for ingestion and migration credentials for explicit setup. Never print or commit credentials. Preserve existing environment, network, scheduler, retention and rollback settings during deployments. Use appropriate tests and verify the deployed image and targeted live imports. The user commits and pushes Git changes themselves unless they explicitly request otherwise.

Historical images, table counts, earlier open items and source audits: [operations history](reports/operations-history.md#previous-handoff), [sports history](reports/sports-history.md), [entertainment history](reports/entertainment-history.md).

## Working conventions with this user

- Confirm before anything outward-facing; the user pushes to GitHub, and restarts/pauses the scheduler
  on their own call.
- Prefer verifying against the live DB and live feeds (dry-run first, then a real run) and report
  exactly what was and wasn't checked.
