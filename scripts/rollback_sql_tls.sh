#!/usr/bin/env bash
set -euo pipefail
if [[ ${1:-} != --rollback ]]; then echo 'Usage: bash scripts/rollback_sql_tls.sh --rollback (restarts shared sql2025)' >&2; exit 2; fi
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"
compose_dir=${SCRAPER_HOST_PROJECT_DIR:-$repo_dir}
compose=(docker compose --project-directory "$compose_dir" -f "$repo_dir/docker-compose.yml" -p webscraper)
backup_dir=.deployment-backups/20261011-audit-fixes
docker run --rm --user 0 --volumes-from sql2025 --entrypoint bash rust:1.97-bookworm -c 'set -e; d=/var/opt/mssql/certs/webscraper-audit-fixes; if test -e "$d/mssql.conf.before"; then cp -p "$d/mssql.conf.before" /var/opt/mssql/mssql.conf; elif test -e "$d/mssql.conf.absent"; then rm -f /var/opt/mssql/mssql.conf; else exit 1; fi'
docker restart sql2025
cp -p "$backup_dir/.env.before-tls" .env
cp -p "$backup_dir/compose.before-tls.yml" docker-compose.yml
"${compose[@]}" up -d --no-deps scraper
echo 'TLS configuration rolled back; restricted ingestion login retained.'
