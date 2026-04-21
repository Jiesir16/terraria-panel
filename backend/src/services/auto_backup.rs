//! Automatic scheduled backup for running servers.
//!
//! Spawns a background task that periodically backs up world files
//! and SSC character databases for every running server.

use crate::config::{BackupConfig, Config, OssBackupConfig};
use crate::db::DbPool;
use crate::services::SaveManager;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_LENGTH, HOST};
use rusqlite::params;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

type HmacSha1 = Hmac<sha1::Sha1>;

const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Start the auto-backup background loop.
///
/// This should be called once from main.rs after constructing all services.
pub fn spawn_auto_backup(
    config: Config,
    db: DbPool,
    save_manager: Arc<SaveManager>,
) {
    if !config.backup.enabled {
        tracing::info!("Auto-backup is disabled in config");
        return;
    }

    let interval = Duration::from_secs(config.backup.interval_minutes * 60);
    let data_dir = config.server.data_dir.clone();
    let backup_config = config.backup.clone();
    let archive_db = db.clone();
    let archive_data_dir = data_dir.clone();
    let archive_config = backup_config.clone();

    tokio::spawn(async move {
        tracing::info!(
            interval_min = config.backup.interval_minutes,
            retention_days = config.backup.local_retention_days,
            oss_enabled = config.backup.oss.enabled,
            "Auto-backup task started"
        );

        loop {
            tokio::time::sleep(interval).await;
            tracing::info!("Auto-backup: running scheduled backup");

            if let Err(e) = run_backup_cycle(&db, &save_manager, &data_dir, &backup_config) {
                tracing::error!(error = %e, "Auto-backup cycle failed");
            }
        }
    });

    if archive_config.archive_daily_enabled {
        tokio::spawn(async move {
            tracing::info!(
                archive_hour = archive_config.archive_hour,
                archive_after_days = archive_config.archive_after_days,
                "Daily backup archive task started"
            );

            loop {
                tokio::time::sleep(duration_until_next_archive(
                    archive_config.archive_hour,
                ))
                .await;

                let target_date =
                    Local::now().date_naive()
                        - chrono::Duration::days(archive_config.archive_after_days as i64);
                tracing::info!(
                    target_date = %target_date,
                    "Daily backup archive: running scheduled archive"
                );

                if let Err(e) =
                    archive_world_backups_for_date(&archive_db, &archive_data_dir, &archive_config, target_date)
                {
                    tracing::error!(error = %e, "Daily backup archive failed");
                }
            }
        });
    }
}

/// Run one backup cycle: iterate all running servers, backup world + SSC.
fn run_backup_cycle(
    db: &DbPool,
    save_manager: &Arc<SaveManager>,
    data_dir: &Path,
    backup_config: &BackupConfig,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("DB lock: {}", e))?;

    // Get all servers that have a world_name set (regardless of running status,
    // we backup any server that has been configured with a world)
    let mut stmt = conn
        .prepare("SELECT id, name, world_name, status FROM servers WHERE world_name IS NOT NULL AND world_name != ''")
        .map_err(|e| format!("Query servers: {}", e))?;

    let servers: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("Map servers: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    drop(stmt);
    drop(conn);

    for (server_id, server_name, world_name, _status) in &servers {
        // 1) Backup world file
        match save_manager.backup_server(server_id, world_name) {
            Ok(backup) => {
                // Record in database
                if let Ok(conn) = db.lock() {
                    let _ = conn.execute(
                        "INSERT INTO saves (id, name, file_path, file_size, source_server_id, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            backup.id,
                            backup.name,
                            backup.file_path,
                            backup.file_size as i64,
                            backup.source_server_id,
                            backup.created_at
                        ],
                    );
                }
                tracing::info!(
                    server = %server_name,
                    world = %world_name,
                    "Auto-backup: world backed up"
                );
                sync_backup_to_oss(&backup_config.oss, &backup.file_path, &backup.name);
            }
            Err(e) => {
                tracing::warn!(
                    server = %server_name,
                    error = %e,
                    "Auto-backup: world backup failed (may not exist yet)"
                );
            }
        }

        // 2) Backup SSC database (tshock.sqlite) if it exists
        let tshock_db_path = data_dir
            .join("servers")
            .join(server_id)
            .join("tshock")
            .join("tshock.sqlite");

        if tshock_db_path.exists() {
            if let Err(e) = backup_ssc_database(&tshock_db_path, data_dir, server_id, backup_config) {
                tracing::warn!(
                    server = %server_name,
                    error = %e,
                    "Auto-backup: SSC database backup failed"
                );
            } else {
                tracing::info!(
                    server = %server_name,
                    "Auto-backup: SSC database backed up"
                );
            }
        }

        // 3) Prune old local backups by age and count.
        if backup_config.local_retention_days > 0 {
            prune_backups_older_than(db, server_id, backup_config.local_retention_days);
        }
        if backup_config.max_backups_per_server > 0 && !backup_config.archive_daily_enabled {
            prune_old_backups(db, server_id, backup_config.max_backups_per_server);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct BackupArchiveItem {
    id: String,
    name: String,
    file_path: String,
}

fn duration_until_next_archive(hour: u8) -> Duration {
    let now = Local::now();
    let hour = u32::from(hour.min(23));
    let today = now.date_naive();
    let mut next = local_datetime_at(today, hour);

    if next <= now {
        next = local_datetime_at(today + chrono::Duration::days(1), hour);
    }

    (next - now).to_std().unwrap_or_else(|_| Duration::from_secs(60))
}

fn local_datetime_at(date: NaiveDate, hour: u32) -> DateTime<Local> {
    let time = NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or(NaiveTime::MIN);
    let naive = date.and_time(time);
    Local
        .from_local_datetime(&naive)
        .earliest()
        .or_else(|| Local.from_local_datetime(&naive).latest())
        .unwrap_or_else(|| Local::now() + chrono::Duration::hours(1))
}

fn archive_world_backups_for_date(
    db: &DbPool,
    data_dir: &Path,
    backup_config: &BackupConfig,
    target_date: NaiveDate,
) -> Result<(), String> {
    let backups = collect_world_backups_for_date(db, target_date)?;
    if backups.is_empty() {
        tracing::info!(
            target_date = %target_date,
            "Daily backup archive: no backups to archive"
        );
        return Ok(());
    }

    let archive_dir = data_dir.join("saves").join("archives");
    std::fs::create_dir_all(&archive_dir)
        .map_err(|e| format!("Create archive dir: {}", e))?;

    for (server_id, items) in backups {
        if items.is_empty() {
            continue;
        }

        let date_key = target_date.format("%Y%m%d").to_string();
        let archive_name = format!("{}_{}_world_backups.zip", server_id, date_key);
        let archive_path = archive_dir.join(&archive_name);

        remove_existing_archive_record(db, &server_id, &archive_name, &archive_path)?;
        write_backup_archive(&archive_path, &items)?;

        let file_size = std::fs::metadata(&archive_path)
            .map(|metadata| metadata.len() as i64)
            .map_err(|e| format!("Read archive metadata: {}", e))?;
        let archive_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let archive_path_str = archive_path.to_string_lossy().to_string();

        let conn = db.lock().map_err(|e| format!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO saves (id, name, file_path, file_size, source_server_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                archive_id,
                archive_name,
                archive_path_str,
                file_size,
                server_id,
                created_at
            ],
        )
        .map_err(|e| format!("Insert archive save: {}", e))?;

        for item in &items {
            conn.execute("DELETE FROM saves WHERE id = ?1", params![item.id])
                .map_err(|e| format!("Delete archived save row: {}", e))?;
        }
        drop(conn);

        for item in &items {
            if Path::new(&item.file_path).exists() {
                let _ = std::fs::remove_file(&item.file_path);
            }
        }

        sync_backup_to_oss(
            &backup_config.oss,
            &archive_path.to_string_lossy(),
            &format!("archives/{}", archive_name),
        );

        tracing::info!(
            server_id = %server_id,
            target_date = %target_date,
            count = items.len(),
            archive = %archive_path.display(),
            "Daily backup archive: backups archived"
        );
    }

    Ok(())
}

fn collect_world_backups_for_date(
    db: &DbPool,
    target_date: NaiveDate,
) -> Result<HashMap<String, Vec<BackupArchiveItem>>, String> {
    let conn = db.lock().map_err(|e| format!("DB lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, file_path, source_server_id, created_at
             FROM saves
             WHERE source_server_id IS NOT NULL",
        )
        .map_err(|e| format!("Query backup saves: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| format!("Map backup saves: {}", e))?;

    let mut grouped: HashMap<String, Vec<BackupArchiveItem>> = HashMap::new();
    for row in rows.filter_map(Result::ok) {
        let (id, name, file_path, source_server_id, created_at) = row;
        if !is_world_backup_name(&name) || !Path::new(&file_path).exists() {
            continue;
        }

        let created_date = match parse_save_created_at(&created_at) {
            Some(created) => created.with_timezone(&Local).date_naive(),
            None => continue,
        };

        if created_date != target_date {
            continue;
        }

        grouped
            .entry(source_server_id.clone())
            .or_default()
            .push(BackupArchiveItem {
                id,
                name,
                file_path,
            });
    }

    Ok(grouped)
}

fn is_world_backup_name(name: &str) -> bool {
    name.ends_with(".wld") || name.ends_with(".wld.bak") || name.ends_with(".bak")
}

fn remove_existing_archive_record(
    db: &DbPool,
    server_id: &str,
    archive_name: &str,
    archive_path: &Path,
) -> Result<(), String> {
    if archive_path.exists() {
        std::fs::remove_file(archive_path)
            .map_err(|e| format!("Remove existing archive file: {}", e))?;
    }

    let conn = db.lock().map_err(|e| format!("DB lock: {}", e))?;
    conn.execute(
        "DELETE FROM saves WHERE source_server_id = ?1 AND name = ?2",
        params![server_id, archive_name],
    )
    .map_err(|e| format!("Remove existing archive row: {}", e))?;

    Ok(())
}

fn write_backup_archive(archive_path: &Path, items: &[BackupArchiveItem]) -> Result<(), String> {
    let file = File::create(archive_path)
        .map_err(|e| format!("Create archive file: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    for item in items {
        let entry_name = Path::new(&item.name)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("world.wld");
        let mut input = File::open(&item.file_path)
            .map_err(|e| format!("Open backup {}: {}", item.file_path, e))?;

        zip.start_file(entry_name, options)
            .map_err(|e| format!("Start archive entry: {}", e))?;
        io::copy(&mut input, &mut zip)
            .map_err(|e| format!("Write archive entry: {}", e))?;
    }

    zip.finish()
        .map_err(|e| format!("Finish archive: {}", e))?;

    Ok(())
}

/// Backup the tshock.sqlite file (contains SSC character data).
fn backup_ssc_database(
    tshock_db_path: &Path,
    data_dir: &Path,
    server_id: &str,
    backup_config: &BackupConfig,
) -> Result<PathBuf, String> {
    let backup_dir = data_dir.join("saves").join("ssc_backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Create SSC backup dir: {}", e))?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("{}_{}_tshock.sqlite", server_id, timestamp);
    let backup_path = backup_dir.join(&backup_name);

    // Use SQLite's backup API by opening both databases
    // For simplicity and safety (file may be in use), we use a file copy.
    // SQLite's WAL mode makes this safe as long as we copy in one operation.
    std::fs::copy(tshock_db_path, &backup_path)
        .map_err(|e| format!("Copy tshock.sqlite: {}", e))?;

    // Also backup WAL and SHM if they exist (for consistency)
    let wal_path = tshock_db_path.with_extension("sqlite-wal");
    if wal_path.exists() {
        let wal_backup = backup_path.with_extension("sqlite-wal");
        let _ = std::fs::copy(&wal_path, &wal_backup);
    }

    tracing::debug!(
        path = %backup_path.display(),
        "SSC database backed up"
    );

    sync_backup_to_oss(&backup_config.oss, &backup_path.to_string_lossy(), &backup_name);

    // Prune old SSC backups for this server (keep last 10)
    prune_ssc_backups(&backup_dir, server_id, 10);
    if backup_config.local_retention_days > 0 {
        prune_ssc_backups_older_than(&backup_dir, server_id, backup_config.local_retention_days);
    }

    Ok(backup_path)
}

pub fn sync_backup_to_oss(config: &OssBackupConfig, file_path: &str, object_name: &str) {
    if !config.enabled {
        return;
    }

    let object_key = if config.prefix.is_empty() {
        object_name.to_string()
    } else {
        format!("{}/{}", config.prefix.trim_matches('/'), object_name)
    };

    let result = match config.provider.as_str() {
        "nas" | "local" => sync_backup_to_nas(config, file_path, &object_key),
        "tencent_cos" | "cos" => sync_backup_to_tencent_cos(config, file_path, &object_key),
        other => Err(format!("Unsupported backup provider: {}", other)),
    };

    match result {
        Ok(destination) => {
            tracing::info!(
                provider = %config.provider,
                object_key = %object_key,
                file_path = %file_path,
                destination = %destination,
                "Remote backup sync completed"
            );
        }
        Err(error) => {
            tracing::warn!(
                provider = %config.provider,
                object_key = %object_key,
                file_path = %file_path,
                error = %error,
                "Remote backup sync failed"
            );
        }
    }
}

fn sync_backup_to_nas(
    config: &OssBackupConfig,
    file_path: &str,
    object_key: &str,
) -> Result<String, String> {
    if config.local_path.trim().is_empty() {
        return Err("backup.oss.local_path is required for provider=nas".to_string());
    }

    let source = Path::new(file_path);
    if !source.exists() {
        return Err(format!("Source file does not exist: {}", file_path));
    }

    let destination = Path::new(&config.local_path).join(object_key);
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create NAS backup directory: {}", e))?;
    }
    std::fs::copy(source, &destination)
        .map_err(|e| format!("Copy backup to NAS: {}", e))?;

    Ok(destination.display().to_string())
}

fn sync_backup_to_tencent_cos(
    config: &OssBackupConfig,
    file_path: &str,
    object_key: &str,
) -> Result<String, String> {
    if config.bucket.trim().is_empty() {
        return Err("backup.oss.bucket is required for provider=tencent_cos".to_string());
    }
    if config.region.trim().is_empty() && config.endpoint.trim().is_empty() {
        return Err("backup.oss.region or backup.oss.endpoint is required for provider=tencent_cos".to_string());
    }
    if config.access_key_id.trim().is_empty() || config.access_key_secret.trim().is_empty() {
        return Err("backup.oss.access_key_id/access_key_secret are required for provider=tencent_cos".to_string());
    }

    let source = Path::new(file_path);
    let metadata = source
        .metadata()
        .map_err(|e| format!("Read source metadata: {}", e))?;
    let file = File::open(source).map_err(|e| format!("Open source file: {}", e))?;

    let base_url = tencent_cos_base_url(config)?;
    let host = base_url
        .host_str()
        .ok_or_else(|| "COS endpoint host is invalid".to_string())?
        .to_string();
    let encoded_key = encode_cos_object_key(object_key);
    let url = format!(
        "{}://{}{}{}",
        base_url.scheme(),
        host,
        base_url
            .port()
            .map(|port| format!(":{}", port))
            .unwrap_or_default(),
        encoded_key
    );
    let authorization = tencent_cos_authorization(config, "put", &host, &encoded_key)?;

    let response = Client::new()
        .put(&url)
        .header(HOST, host)
        .header(CONTENT_LENGTH, metadata.len())
        .header("Authorization", authorization)
        .body(reqwest::blocking::Body::new(file))
        .send()
        .map_err(|e| format!("Upload to Tencent COS: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Tencent COS returned {}: {}", status, body));
    }

    Ok(url)
}

fn tencent_cos_base_url(config: &OssBackupConfig) -> Result<reqwest::Url, String> {
    let endpoint = if config.endpoint.trim().is_empty() {
        format!(
            "https://{}.cos.{}.myqcloud.com",
            config.bucket.trim(),
            config.region.trim()
        )
    } else if config.endpoint.starts_with("http://") || config.endpoint.starts_with("https://") {
        config.endpoint.trim().trim_end_matches('/').to_string()
    } else {
        format!("https://{}", config.endpoint.trim().trim_end_matches('/'))
    };

    reqwest::Url::parse(&endpoint).map_err(|e| format!("Parse COS endpoint: {}", e))
}

fn encode_cos_object_key(object_key: &str) -> String {
    let normalized = object_key.trim_start_matches('/');
    if normalized.is_empty() {
        return "/".to_string();
    }

    format!(
        "/{}",
        normalized
            .split('/')
            .map(|part| utf8_percent_encode(part, PATH_ENCODE_SET).to_string())
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn tencent_cos_authorization(
    config: &OssBackupConfig,
    method: &str,
    host: &str,
    encoded_path: &str,
) -> Result<String, String> {
    let start = Utc::now().timestamp();
    let end = start + 3600;
    let key_time = format!("{};{}", start, end);
    let sign_time = key_time.clone();
    let header_list = "host";
    let url_param_list = "";
    let http_string = format!(
        "{}\n{}\n\nhost={}\n",
        method.to_ascii_lowercase(),
        encoded_path,
        host.to_ascii_lowercase()
    );
    let http_string_sha1 = sha1_hex(http_string.as_bytes());
    let string_to_sign = format!("sha1\n{}\n{}\n", sign_time, http_string_sha1);
    let sign_key = hmac_sha1(config.access_key_secret.as_bytes(), key_time.as_bytes())?;
    let signature = hmac_sha1_hex(&sign_key, string_to_sign.as_bytes())?;

    Ok(format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list={}&q-signature={}",
        config.access_key_id.trim(),
        sign_time,
        key_time,
        header_list,
        url_param_list,
        signature
    ))
}

fn sha1_hex(data: &[u8]) -> String {
    use sha1::Digest;
    to_hex(&sha1::Sha1::digest(data))
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let mut mac = HmacSha1::new_from_slice(key)
        .map_err(|e| format!("Create HMAC-SHA1 key: {}", e))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_sha1_hex(key: &[u8], data: &[u8]) -> Result<String, String> {
    hmac_sha1(key, data).map(|bytes| to_hex(&bytes))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

/// Remove old SSC database backups, keeping only the newest `keep` entries.
fn prune_ssc_backups(backup_dir: &Path, server_id: &str, keep: usize) {
    let prefix = format!("{}_", server_id);
    let mut backups: Vec<_> = std::fs::read_dir(backup_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&prefix) && name.ends_with("_tshock.sqlite")
        })
        .collect();

    if backups.len() <= keep {
        return;
    }

    // Sort by name (which contains timestamp) ascending
    backups.sort_by_key(|e| e.file_name());

    let to_remove = backups.len() - keep;
    for entry in backups.into_iter().take(to_remove) {
        let path = entry.path();
        tracing::debug!(path = %path.display(), "Pruning old SSC backup");
        let _ = std::fs::remove_file(&path);
        // Also remove WAL/SHM companions
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }
}

fn prune_ssc_backups_older_than(backup_dir: &Path, server_id: &str, retention_days: u64) {
    let prefix = format!("{}_", server_id);
    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let entries = match std::fs::read_dir(backup_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(&prefix) || !name.ends_with("_tshock.sqlite") {
            continue;
        }

        let path = entry.path();
        let modified = match entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map(DateTime::<Utc>::from)
        {
            Ok(modified) => modified,
            Err(_) => continue,
        };

        if modified < cutoff {
            tracing::debug!(path = %path.display(), "Pruning expired SSC backup");
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
            let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
        }
    }
}

fn parse_save_created_at(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|dt| Local.from_local_datetime(&dt).single())
                .map(|dt| dt.with_timezone(&Utc))
        })
}

pub fn prune_backups_older_than(db: &DbPool, server_id: &str, retention_days: u64) {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return,
    };

    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let mut stmt = match conn.prepare(
        "SELECT id, file_path, created_at FROM saves WHERE source_server_id = ?1",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let expired: Vec<(String, String)> = stmt
        .query_map(params![server_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .into_iter()
        .flatten()
        .filter_map(|row| row.ok())
        .filter_map(|(save_id, file_path, created_at)| {
            parse_save_created_at(&created_at)
                .filter(|created| *created < cutoff)
                .map(|_| (save_id, file_path))
        })
        .collect();

    drop(stmt);

    for (save_id, file_path) in expired {
        let _ = conn.execute("DELETE FROM saves WHERE id = ?1", params![save_id]);
        if Path::new(&file_path).exists() {
            let _ = std::fs::remove_file(&file_path);
        }
        tracing::debug!(server_id = %server_id, save_id = %save_id, "Pruned expired world backup");
    }
}

/// Prune old world save backups for a server, keeping only the newest `max` entries.
fn prune_old_backups(db: &DbPool, server_id: &str, max: usize) {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return,
    };

    // Count backups for this server
    let count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM saves WHERE source_server_id = ?1",
            params![server_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count <= max {
        return;
    }

    let to_remove = count - max;

    // Get the oldest backups for this server
    let mut stmt = match conn.prepare(
        "SELECT id, file_path FROM saves WHERE source_server_id = ?1 ORDER BY created_at ASC LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let old_saves: Vec<(String, String)> = stmt
        .query_map(params![server_id, to_remove], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect();

    for (save_id, file_path) in old_saves {
        // Delete from DB
        let _ = conn.execute("DELETE FROM saves WHERE id = ?1", params![save_id]);
        // Delete file
        if Path::new(&file_path).exists() {
            let _ = std::fs::remove_file(&file_path);
        }
        tracing::debug!(save_id = %save_id, "Pruned old backup");
    }
}
