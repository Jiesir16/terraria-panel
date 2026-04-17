use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type DbPool = Arc<Mutex<Connection>>;

pub fn create_db(path: &Path) -> Result<DbPool, AppError> {
    std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;

    let conn = Connection::open(path).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    init_schema(&conn)?;

    Ok(Arc::new(Mutex::new(conn)))
}

fn init_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'viewer',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 7777,
            tshock_version TEXT NOT NULL,
            world_name TEXT,
            status TEXT NOT NULL DEFAULT 'stopped',
            password TEXT,
            max_players INTEGER DEFAULT 8,
            auto_start BOOLEAN DEFAULT 0,
            created_by TEXT REFERENCES users(id),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS saves (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER,
            source_server_id TEXT REFERENCES servers(id),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT REFERENCES users(id),
            action TEXT NOT NULL,
            target TEXT,
            details TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_servers_status ON servers(status);
        CREATE INDEX IF NOT EXISTS idx_servers_created_by ON servers(created_by);
        CREATE INDEX IF NOT EXISTS idx_saves_source ON saves(source_server_id);
        CREATE INDEX IF NOT EXISTS idx_logs_user ON operation_logs(user_id);
        "#,
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Create default admin user if not exists
    create_default_admin(conn)?;

    Ok(())
}

fn create_default_admin(conn: &Connection) -> Result<(), AppError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users WHERE role = 'admin'", [], |row| {
            row.get(0)
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if count == 0 {
        let admin_id = uuid::Uuid::new_v4().to_string();
        let password_hash =
            crate::auth::password::hash_password("admin123").map_err(|_| {
                AppError::InternalServerError("Failed to hash password".to_string())
            })?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![admin_id, "admin", password_hash, "admin", now, now],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        tracing::info!("Created default admin user: admin/admin123");
    }

    Ok(())
}
