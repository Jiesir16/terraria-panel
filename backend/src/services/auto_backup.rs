//! Automatic scheduled backup for running servers.
//!
//! Spawns a background task that periodically backs up world files
//! and SSC character databases for every running server.

use crate::config::Config;
use crate::db::DbPool;
use crate::services::SaveManager;
use chrono::Local;
use rusqlite::params;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

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
    let max_backups = config.backup.max_backups_per_server;

    tokio::spawn(async move {
        tracing::info!(
            interval_min = config.backup.interval_minutes,
            "Auto-backup task started"
        );

        loop {
            tokio::time::sleep(interval).await;
            tracing::info!("Auto-backup: running scheduled backup");

            if let Err(e) = run_backup_cycle(&db, &save_manager, &data_dir, max_backups) {
                tracing::error!(error = %e, "Auto-backup cycle failed");
            }
        }
    });
}

/// Run one backup cycle: iterate all running servers, backup world + SSC.
fn run_backup_cycle(
    db: &DbPool,
    save_manager: &Arc<SaveManager>,
    data_dir: &Path,
    max_backups: usize,
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
            if let Err(e) = backup_ssc_database(&tshock_db_path, data_dir, server_id) {
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

        // 3) Prune old backups if exceeding max
        if max_backups > 0 {
            prune_old_backups(db, server_id, max_backups);
        }
    }

    Ok(())
}

/// Backup the tshock.sqlite file (contains SSC character data).
fn backup_ssc_database(
    tshock_db_path: &Path,
    data_dir: &Path,
    server_id: &str,
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

    // Prune old SSC backups for this server (keep last 10)
    prune_ssc_backups(&backup_dir, server_id, 10);

    Ok(backup_path)
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
