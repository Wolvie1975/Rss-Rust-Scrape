# SQL Server Database Backup and Migration Guide

This guide details how to generate a full `.bak` backup of the `WebScraper` database from the `sql2025` Docker container, copy it to the host, and restore it on a new machine.

---

## 1. Create a Full Backup (`.bak`)

Run the following command on your Docker host. Using `WITH COPY_ONLY` ensures the backup does not break any existing automated transaction-log backup chains or differential baselines.

```bash
docker exec -it sql2025 /opt/mssql-tools18/bin/sqlcmd \
  -S localhost -U sa -P 'Fedora_Dev_2026!' -C \
  -Q "BACKUP DATABASE [WebScraper] TO DISK = N'/var/opt/mssql/backup/WebScraper_port.bak' WITH COPY_ONLY, FORMAT, COMPRESSION, CHECKSUM, STATS = 10;"
```

### Alternative: Host Backup Runner
If using the host backup script already configured in `/usr/local/sbin/webscraper-sql-backup`:

```bash
sudo /usr/local/sbin/webscraper-sql-backup full
```
Managed daily backups are stored inside the container at `/var/opt/mssql/backup/webscraper-managed/full/`.

---

## 2. Verify the Backup File

Verify that the backup media is readable and the checksums match:

```bash
docker exec -it sql2025 /opt/mssql-tools18/bin/sqlcmd \
  -S localhost -U sa -P 'Fedora_Dev_2026!' -C \
  -Q "RESTORE VERIFYONLY FROM DISK = N'/var/opt/mssql/backup/WebScraper_port.bak' WITH CHECKSUM;"
```

---

## 3. Copy the `.bak` Out of the Container

Copy the backup file to your host machine's current directory:

```bash
docker cp sql2025:/var/opt/mssql/backup/WebScraper_port.bak ./WebScraper_port.bak
```

You can now transfer `WebScraper_port.bak` to your new machine using `scp`, `rsync`, or external media.

---

## 4. Restore on the New Machine

### 4.1 Copy Backup into the New SQL Server Container
On the destination machine, copy the backup into the new SQL Server container:

```bash
docker cp ./WebScraper_port.bak <new_sql_container>:/var/opt/mssql/backup/
```

### 4.2 Inspect Logical File Names
Check the logical names and target file paths embedded in the backup:

```sql
RESTORE FILELISTONLY FROM DISK = N'/var/opt/mssql/backup/WebScraper_port.bak';
```

### 4.3 Restore the Database
Restore the database and move physical data/log files to match the new container's filesystem layout:

```sql
RESTORE DATABASE [WebScraper]
FROM DISK = N'/var/opt/mssql/backup/WebScraper_port.bak'
WITH MOVE 'WebScraper' TO '/var/opt/mssql/data/WebScraper.mdf',
     MOVE 'WebScraper_log' TO '/var/opt/mssql/data/WebScraper_log.ldf',
     REPLACE, STATS = 10;
```

---

## 5. Post-Restore Checklist

1. **TLS Certificates:**
   If the new SQL Server instance enforces TLS authentication (as configured in `.env`), ensure the CA certificate in `certs/sql-ca.crt` is deployed or trust is configured appropriately.

2. **Server Logins & Database Users:**
   Restoring a `.bak` restores database-level users (`mynewspaper_admin`, `webscraper_ingest`), but SQL Server server-level logins reside in `master`. If SIDs do not match on the new server:
   - Create the logins on the new instance if they do not exist.
   - Remap the orphaned database users to the new server logins:
     ```sql
     USE [WebScraper];
     ALTER USER [mynewspaper_admin] WITH LOGIN = [mynewspaper_admin];
     ALTER USER [webscraper_ingest] WITH LOGIN = [webscraper_ingest];
     ```

3. **Application `.env` Update:**
   Update `MSSQL_CONNECTION_STRING` in `.env` and `.env.migrations` to point to the new host/port if the hostname or credentials change.
