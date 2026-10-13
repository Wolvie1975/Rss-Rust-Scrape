#!/bin/bash
# Installed on Docker host as /usr/local/sbin/webscraper-sql-backup.
set -euo pipefail
backup_kind=${1:-log}
case "$backup_kind" in log|full) ;; *) echo 'Usage: webscraper-sql-backup log|full' >&2; exit 2;; esac
exec 9>/var/lock/webscraper-sql-backup.lock
flock -n 9 || exit 0
/usr/bin/docker exec sql2025 /bin/bash -c '
 set -euo pipefail
 umask 077
 export SQLCMDPASSWORD="$MSSQL_SA_PASSWORD"
 /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -C -b -i "/var/opt/mssql/backup/webscraper-managed/scripts/$1.sql"
 if [[ $1 == full ]]; then
  find /var/opt/mssql/backup/webscraper-managed/full -type f -name "WebScraper_full_*.bak" -mtime +7 -delete
  find /var/opt/mssql/backup/webscraper-managed/log -type f -name "WebScraper_log_*.trn" -mtime +9 -delete
 fi
' backup "$backup_kind"
