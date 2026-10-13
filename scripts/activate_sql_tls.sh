#!/usr/bin/env bash
# Run on the Docker host after approving the shared SQL Server restart.
set -euo pipefail
if [[ ${1:-} != --activate ]]; then
  echo 'Usage: bash scripts/activate_sql_tls.sh --activate (restarts shared sql2025)' >&2
  exit 2
fi
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"
compose_dir=${SCRAPER_HOST_PROJECT_DIR:-$repo_dir}
compose=(docker compose --project-directory "$compose_dir" -f "$repo_dir/docker-compose.yml" -p webscraper)
backup_dir=.deployment-backups/20261011-audit-fixes
mkdir -p "$backup_dir"
chmod 700 "$backup_dir"
if ! test -e "$backup_dir/.env.before-tls"; then cp -p .env "$backup_dir/.env.before-tls"; fi
if ! test -e "$backup_dir/compose.before-tls.yml"; then cp -p docker-compose.yml "$backup_dir/compose.before-tls.yml"; fi
docker exec -u 0 sql2025 bash -c 'set -e; d=/var/opt/mssql/certs/webscraper-audit-fixes; test -s "$d/sql-server.crt"; test -s "$d/sql-server.key"; if ! test -e "$d/mssql.conf.before" && ! test -e "$d/mssql.conf.absent"; then if test -f /var/opt/mssql/mssql.conf; then cp -p /var/opt/mssql/mssql.conf "$d/mssql.conf.before"; else touch "$d/mssql.conf.absent"; fi; fi'
docker exec -u 0 sql2025 /opt/mssql/bin/mssql-conf set network.tlscert /var/opt/mssql/certs/webscraper-audit-fixes/sql-server.crt
docker exec -u 0 sql2025 /opt/mssql/bin/mssql-conf set network.tlskey /var/opt/mssql/certs/webscraper-audit-fixes/sql-server.key
docker restart sql2025
python3 - <<'PY'
from pathlib import Path
import re
p=Path('.env');text=p.read_text();text,n=re.subn(r'(?i)(;)\s*TrustServerCertificate\s*=\s*(true|yes)',r'\1TrustServerCertificateCA=certs/sql-ca.crt',text)
if n!=1 and 'TrustServerCertificateCA=certs/sql-ca.crt' not in text:raise SystemExit('Unexpected TLS options; review before switching.')
p.write_text(text);p.chmod(0o600)
p=Path('docker-compose.yml');text=p.read_text()
mount='      - ./certs/sql-ca.crt:/data/certs/sql-ca.crt:ro'
if mount not in text:text=text.replace('      - scraper-data:/data','      - scraper-data:/data\n'+mount)
p.write_text(text)
PY
# Test the authenticated connection without migrating schema or changing schedules.
ready=false
for attempt in {1..12}; do
  if "${compose[@]}" run --rm -T --no-deps scraper --health-check; then ready=true; break; fi
  sleep 5
done
if ! $ready; then echo 'Authenticated SQL connection failed; run scripts/rollback_sql_tls.sh --rollback' >&2; exit 1; fi
"${compose[@]}" up -d --no-deps scraper
echo 'SQL certificate authentication enabled. Verify /events and collector health.'
