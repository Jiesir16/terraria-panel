use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::json;

use crate::{
    auth::Auth,
    error::AppError,
    handlers::AppState,
    models::{
        TShockGroupDetail, TShockSscCharacter, TShockSscCharacterSummary,
    },
};

/// Open the tshock.sqlite database for a given server.
fn open_tshock_db(state: &AppState, server_id: &str) -> Result<Connection, AppError> {
    let sqlite_path = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(server_id)
        .join("tshock")
        .join("tshock.sqlite");

    if !sqlite_path.exists() {
        return Err(AppError::NotFound(
            "TShock database not found. Start the server at least once to create it.".to_string(),
        ));
    }

    Connection::open(&sqlite_path)
        .map_err(|e| AppError::DatabaseError(format!("Failed to open tshock.sqlite: {}", e)))
}

/// Detect which permissions table exists: "GroupPermissions" or "Permissions".
fn permissions_table(conn: &Connection) -> Option<&'static str> {
    let has_gp: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='GroupPermissions'")
        .and_then(|mut s| s.exists([]))
        .unwrap_or(false);
    if has_gp {
        return Some("GroupPermissions");
    }

    let has_permissions: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='Permissions'")
        .and_then(|mut s| s.exists([]))
        .unwrap_or(false);
    if has_permissions {
        Some("Permissions")
    } else {
        None
    }
}

fn groups_has_commands(conn: &Connection) -> bool {
    let mut stmt = match conn.prepare("PRAGMA table_info(Groups)") {
        Ok(stmt) => stmt,
        Err(_) => return false,
    };
    let rows = match stmt.query_map([], |row| row.get::<_, String>(1)) {
        Ok(rows) => rows,
        Err(_) => return false,
    };

    let has_commands = rows
        .filter_map(Result::ok)
        .any(|name| name.eq_ignore_ascii_case("Commands"));
    has_commands
}

fn parse_commands(commands: &str) -> Vec<String> {
    commands
        .split(',')
        .map(|permission| permission.trim())
        .filter(|permission| !permission.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn serialize_commands(permissions: &[String]) -> String {
    permissions.join(",")
}

// ─── User Management ───

/// Change a TShock user's group
pub async fn update_user_group(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, username)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only admins can change TShock user groups".to_string(),
        ));
    }

    let group = body
        .get("group")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'group' field".to_string()))?;

    let conn = open_tshock_db(&state, &server_id)?;

    // Check user exists
    let exists: bool = conn
        .prepare("SELECT 1 FROM Users WHERE Username = ?1")
        .and_then(|mut s| s.exists(params![&username]))
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "TShock user '{}' not found",
            username
        )));
    }

    conn.execute(
        "UPDATE Users SET Usergroup = ?1 WHERE Username = ?2",
        params![group, username],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "修改TShock用户组",
        Some(&server_id),
        Some(&format!("用户 {} → 组 {}", username, group)),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("User '{}' moved to group '{}'", username, group)
    })))
}

/// Delete a TShock user
pub async fn delete_user(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, username)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only admins can delete TShock users".to_string(),
        ));
    }

    let conn = open_tshock_db(&state, &server_id)?;

    let affected = conn
        .execute("DELETE FROM Users WHERE Username = ?1", params![username])
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "TShock user '{}' not found",
            username
        )));
    }

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "删除TShock用户",
        Some(&server_id),
        Some(&format!("用户: {}", username)),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("User '{}' deleted", username)
    })))
}

// ─── Group Management ───

/// Get group details including all permissions
pub async fn get_group(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, group_name)): Path<(String, String)>,
) -> Result<Json<TShockGroupDetail>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can view group details".to_string(),
        ));
    }

    let conn = open_tshock_db(&state, &server_id)?;
    let perm_table = permissions_table(&conn);

    // Get parent from Groups table
    let parent: Option<String> = conn
        .prepare("SELECT Parent FROM Groups WHERE GroupName = ?1")
        .and_then(|mut s| {
            s.query_row(params![&group_name], |row| row.get::<_, Option<String>>(0))
        })
        .unwrap_or(None);

    // Get permissions. Newer schemas use a permissions table; older schemas keep CSV in Groups.Commands.
    let permissions = if let Some(perm_table) = perm_table {
        let query = format!(
            "SELECT Permission FROM {} WHERE GroupName = ?1 ORDER BY Permission",
            perm_table
        );
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let rows = stmt.query_map(params![&group_name], |row| row.get::<_, String>(0))
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        rows
    } else if groups_has_commands(&conn) {
        let commands: Option<String> = conn
            .prepare("SELECT Commands FROM Groups WHERE GroupName = ?1")
            .and_then(|mut s| {
                s.query_row(params![&group_name], |row| row.get::<_, Option<String>>(0))
            })
            .unwrap_or(None);
        let mut permissions = parse_commands(commands.as_deref().unwrap_or(""));
        permissions.sort();
        permissions
    } else {
        Vec::new()
    };

    // Get member count
    let member_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Users WHERE Usergroup = ?1")
        .and_then(|mut s| s.query_row(params![&group_name], |row| row.get(0)))
        .unwrap_or(0);

    Ok(Json(TShockGroupDetail {
        name: group_name,
        parent,
        permissions,
        member_count: member_count as usize,
    }))
}

/// Create a new TShock group
pub async fn create_group(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only admins can create TShock groups".to_string(),
        ));
    }

    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'name' field".to_string()))?;

    let parent = body
        .get("parent")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let conn = open_tshock_db(&state, &server_id)?;

    // Check if group already exists
    let exists: bool = conn
        .prepare("SELECT 1 FROM Groups WHERE GroupName = ?1")
        .and_then(|mut s| s.exists(params![name]))
        .unwrap_or(false);

    if exists {
        return Err(AppError::Conflict(format!(
            "Group '{}' already exists",
            name
        )));
    }

    // TShock Groups table: GroupName, Parent, Commands (permissions CSV), ChatColor
    // But permissions are in a separate table, so we only insert the group here
    conn.execute(
        "INSERT INTO Groups (GroupName, Parent, Commands, ChatColor) VALUES (?1, ?2, '', '255,255,255')",
        params![name, parent],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "创建TShock组",
        Some(&server_id),
        Some(&format!("组: {}", name)),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Group '{}' created", name)
    })))
}

/// Delete a TShock group (also removes its permissions)
pub async fn delete_group(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, group_name)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only admins can delete TShock groups".to_string(),
        ));
    }

    // Prevent deleting built-in groups
    let protected = ["superadmin", "owner", "guest", "default"];
    if protected
        .iter()
        .any(|g| g.eq_ignore_ascii_case(&group_name))
    {
        return Err(AppError::BadRequest(format!(
            "Cannot delete built-in group '{}'",
            group_name
        )));
    }

    let conn = open_tshock_db(&state, &server_id)?;
    let perm_table = permissions_table(&conn);

    // Check if any users belong to this group
    let user_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Users WHERE Usergroup = ?1")
        .and_then(|mut s| s.query_row(params![&group_name], |row| row.get(0)))
        .unwrap_or(0);

    if user_count > 0 {
        return Err(AppError::BadRequest(format!(
            "Cannot delete group '{}': {} users still belong to it. Move them first.",
            group_name, user_count
        )));
    }

    // Delete permissions
    if let Some(perm_table) = perm_table {
        let delete_perms = format!("DELETE FROM {} WHERE GroupName = ?1", perm_table);
        conn.execute(&delete_perms, params![&group_name])
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Delete group
    let affected = conn
        .execute("DELETE FROM Groups WHERE GroupName = ?1", params![&group_name])
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Group '{}' not found",
            group_name
        )));
    }

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "删除TShock组",
        Some(&server_id),
        Some(&format!("组: {}", group_name)),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Group '{}' deleted", group_name)
    })))
}

// ─── Permission Management ───

/// Add a permission to a group
pub async fn add_permission(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, group_name)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only admins can manage permissions".to_string(),
        ));
    }

    let permission = body
        .get("permission")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'permission' field".to_string()))?;

    let conn = open_tshock_db(&state, &server_id)?;
    let perm_table = permissions_table(&conn);

    let group_exists: bool = conn
        .prepare("SELECT 1 FROM Groups WHERE GroupName = ?1")
        .and_then(|mut s| s.exists(params![&group_name]))
        .unwrap_or(false);
    if !group_exists {
        return Err(AppError::NotFound(format!(
            "Group '{}' not found",
            group_name
        )));
    }

    if let Some(perm_table) = perm_table {
        // Check if permission already exists for this group
        let check_query = format!(
            "SELECT 1 FROM {} WHERE GroupName = ?1 AND Permission = ?2",
            perm_table
        );
        let exists: bool = conn
            .prepare(&check_query)
            .and_then(|mut s| s.exists(params![&group_name, permission]))
            .unwrap_or(false);

        if exists {
            return Err(AppError::Conflict(format!(
                "Permission '{}' already exists for group '{}'",
                permission, group_name
            )));
        }

        let insert_query = format!(
            "INSERT INTO {} (GroupName, Permission) VALUES (?1, ?2)",
            perm_table
        );
        conn.execute(&insert_query, params![&group_name, permission])
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    } else if groups_has_commands(&conn) {
        let commands: Option<String> = conn
            .prepare("SELECT Commands FROM Groups WHERE GroupName = ?1")
            .and_then(|mut s| {
                s.query_row(params![&group_name], |row| row.get::<_, Option<String>>(0))
            })
            .unwrap_or(None);
        let mut permissions = parse_commands(commands.as_deref().unwrap_or(""));
        if permissions.iter().any(|p| p == permission) {
            return Err(AppError::Conflict(format!(
                "Permission '{}' already exists for group '{}'",
                permission, group_name
            )));
        }
        permissions.push(permission.to_string());
        permissions.sort();
        permissions.dedup();

        conn.execute(
            "UPDATE Groups SET Commands = ?1 WHERE GroupName = ?2",
            params![serialize_commands(&permissions), &group_name],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    } else {
        return Err(AppError::DatabaseError(
            "No supported TShock permission storage found".to_string(),
        ));
    }

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "添加TShock权限",
        Some(&server_id),
        Some(&format!("组 {} + 权限 {}", group_name, permission)),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Permission '{}' added to group '{}'", permission, group_name)
    })))
}

/// Remove a permission from a group
pub async fn remove_permission(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, group_name)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only admins can manage permissions".to_string(),
        ));
    }

    let permission = body
        .get("permission")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'permission' field".to_string()))?;

    let conn = open_tshock_db(&state, &server_id)?;
    let perm_table = permissions_table(&conn);

    let affected = if let Some(perm_table) = perm_table {
        let delete_query = format!(
            "DELETE FROM {} WHERE GroupName = ?1 AND Permission = ?2",
            perm_table
        );
        conn.execute(&delete_query, params![&group_name, permission])
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
    } else if groups_has_commands(&conn) {
        let commands: Option<String> = conn
            .prepare("SELECT Commands FROM Groups WHERE GroupName = ?1")
            .and_then(|mut s| {
                s.query_row(params![&group_name], |row| row.get::<_, Option<String>>(0))
            })
            .unwrap_or(None);
        let mut permissions = parse_commands(commands.as_deref().unwrap_or(""));
        let before = permissions.len();
        permissions.retain(|p| p != permission);
        if permissions.len() == before {
            0
        } else {
            conn.execute(
                "UPDATE Groups SET Commands = ?1 WHERE GroupName = ?2",
                params![serialize_commands(&permissions), &group_name],
            )
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
        }
    } else {
        return Err(AppError::DatabaseError(
            "No supported TShock permission storage found".to_string(),
        ));
    };

    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Permission '{}' not found for group '{}'",
            permission, group_name
        )));
    }

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "移除TShock权限",
        Some(&server_id),
        Some(&format!("组 {} - 权限 {}", group_name, permission)),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Permission '{}' removed from group '{}'", permission, group_name)
    })))
}

// ─── SSC Character Management ───

/// List SSC characters
pub async fn list_ssc_characters(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<TShockSscCharacterSummary>>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can view SSC characters".to_string(),
        ));
    }

    let conn = open_tshock_db(&state, &server_id)?;

    // Check if tsCharacter table exists
    let has_table: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='tsCharacter'")
        .and_then(|mut s| s.exists([]))
        .unwrap_or(false);

    if !has_table {
        return Ok(Json(Vec::new()));
    }

    let mut stmt = conn
        .prepare(
            "SELECT Account, Health, MaxHealth, Mana, MaxMana, questsCompleted
             FROM tsCharacter ORDER BY Account",
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let characters = stmt
        .query_map([], |row| {
            Ok(TShockSscCharacterSummary {
                account: row.get::<_, i64>(0)?,
                username: None,
                health: row.get::<_, i32>(1).unwrap_or(0),
                max_health: row.get::<_, i32>(2).unwrap_or(100),
                mana: row.get::<_, i32>(3).unwrap_or(0),
                max_mana: row.get::<_, i32>(4).unwrap_or(20),
                quests_completed: row.get::<_, i32>(5).unwrap_or(0),
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Enrich with usernames from Users table
    let enriched: Vec<TShockSscCharacterSummary> = characters
        .into_iter()
        .map(|mut c| {
            let username: Option<String> = conn
                .prepare("SELECT Username FROM Users WHERE rowid = ?1")
                .and_then(|mut s| s.query_row(params![c.account], |row| row.get(0)))
                .ok();
            c.username = username;
            c
        })
        .collect();

    Ok(Json(enriched))
}

/// Export a single SSC character's full data
pub async fn export_ssc_character(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, account_id)): Path<(String, i64)>,
) -> Result<Json<TShockSscCharacter>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can export SSC characters".to_string(),
        ));
    }

    let conn = open_tshock_db(&state, &server_id)?;

    let character = conn
        .prepare(
            "SELECT Account, Health, MaxHealth, Mana, MaxMana, Inventory, extraSlot,
                    spawnX, spawnY, skinVariant, hair, hairDye, hairColor,
                    pantsColor, shirtColor, underShirtColor, shoeColor, skinColor, eyeColor,
                    questsCompleted, hideVisuals
             FROM tsCharacter WHERE Account = ?1",
        )
        .and_then(|mut s| {
            s.query_row(params![account_id], |row| {
                Ok(TShockSscCharacter {
                    account: row.get(0)?,
                    health: row.get::<_, i32>(1).unwrap_or(0),
                    max_health: row.get::<_, i32>(2).unwrap_or(100),
                    mana: row.get::<_, i32>(3).unwrap_or(0),
                    max_mana: row.get::<_, i32>(4).unwrap_or(20),
                    inventory: row.get::<_, Option<String>>(5).unwrap_or(None),
                    extra_slot: row.get::<_, Option<i32>>(6).unwrap_or(None),
                    spawn_x: row.get::<_, Option<i32>>(7).unwrap_or(None),
                    spawn_y: row.get::<_, Option<i32>>(8).unwrap_or(None),
                    skin_variant: row.get::<_, Option<i32>>(9).unwrap_or(None),
                    hair: row.get::<_, Option<i32>>(10).unwrap_or(None),
                    hair_dye: row.get::<_, Option<i32>>(11).unwrap_or(None),
                    hair_color: row.get::<_, Option<i32>>(12).unwrap_or(None),
                    pants_color: row.get::<_, Option<i32>>(13).unwrap_or(None),
                    shirt_color: row.get::<_, Option<i32>>(14).unwrap_or(None),
                    under_shirt_color: row.get::<_, Option<i32>>(15).unwrap_or(None),
                    shoe_color: row.get::<_, Option<i32>>(16).unwrap_or(None),
                    skin_color: row.get::<_, Option<i32>>(17).unwrap_or(None),
                    eye_color: row.get::<_, Option<i32>>(18).unwrap_or(None),
                    quests_completed: row.get::<_, i32>(19).unwrap_or(0),
                    hide_visuals: row.get::<_, Option<String>>(20).unwrap_or(None),
                    username: None,
                })
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("SSC character with account {} not found", account_id))
            }
            _ => AppError::DatabaseError(e.to_string()),
        })?;

    // Get username
    let mut character = character;
    character.username = conn
        .prepare("SELECT Username FROM Users WHERE rowid = ?1")
        .and_then(|mut s| s.query_row(params![account_id], |row| row.get(0)))
        .ok();

    Ok(Json(character))
}

/// Backup all SSC characters to a JSON file in the saves directory
pub async fn backup_ssc_characters(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can backup SSC characters".to_string(),
        ));
    }

    let conn = open_tshock_db(&state, &server_id)?;

    // Check tsCharacter table exists
    let has_table: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='tsCharacter'")
        .and_then(|mut s| s.exists([]))
        .unwrap_or(false);

    if !has_table {
        return Err(AppError::NotFound(
            "No SSC character data found (tsCharacter table missing)".to_string(),
        ));
    }

    // Read all characters
    let mut stmt = conn
        .prepare(
            "SELECT Account, Health, MaxHealth, Mana, MaxMana, Inventory, extraSlot,
                    spawnX, spawnY, questsCompleted
             FROM tsCharacter",
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let characters: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let account: i64 = row.get(0)?;
            Ok(json!({
                "account": account,
                "health": row.get::<_, i32>(1).unwrap_or(0),
                "max_health": row.get::<_, i32>(2).unwrap_or(100),
                "mana": row.get::<_, i32>(3).unwrap_or(0),
                "max_mana": row.get::<_, i32>(4).unwrap_or(20),
                "inventory": row.get::<_, Option<String>>(5).unwrap_or(None),
                "extra_slot": row.get::<_, Option<i32>>(6).unwrap_or(None),
                "spawn_x": row.get::<_, Option<i32>>(7).unwrap_or(None),
                "spawn_y": row.get::<_, Option<i32>>(8).unwrap_or(None),
                "quests_completed": row.get::<_, i32>(9).unwrap_or(0),
            }))
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Also grab user mapping
    let mut user_stmt = conn
        .prepare("SELECT rowid, Username, Usergroup FROM Users")
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let users: Vec<serde_json::Value> = user_stmt
        .query_map([], |row| {
            Ok(json!({
                "account": row.get::<_, i64>(0)?,
                "username": row.get::<_, String>(1)?,
                "group": row.get::<_, Option<String>>(2)?,
            }))
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let backup_data = json!({
        "server_id": server_id,
        "backup_time": Utc::now().to_rfc3339(),
        "users": users,
        "characters": characters,
    });

    // Write to saves directory
    let backup_dir = state.config.server.data_dir.join("saves").join("ssc-backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| AppError::FileError(format!("Failed to create backup directory: {}", e)))?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("ssc-backup_{}_{}.json", server_id, timestamp);
    let backup_path = backup_dir.join(&filename);

    let backup_json = serde_json::to_string_pretty(&backup_data)
        .map_err(|e| AppError::BadRequest(format!("Failed to serialize backup: {}", e)))?;
    std::fs::write(&backup_path, backup_json)
        .map_err(|e| AppError::FileError(format!("Failed to write backup file: {}", e)))?;

    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "备份SSC角色数据",
        Some(&server_id),
        Some(&format!(
            "已备份 {} 个角色, 文件: {}",
            characters.len(),
            filename
        )),
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Backed up {} characters", characters.len()),
        "filename": filename,
        "character_count": characters.len(),
    })))
}
