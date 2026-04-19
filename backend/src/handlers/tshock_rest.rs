//! Axum handlers that proxy requests to TShock's built-in REST API.
//!
//! The panel frontend calls these endpoints; the backend resolves the REST
//! token for the target server and forwards the request to TShock.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;

use crate::{
    auth::Auth,
    error::AppError,
    handlers::AppState,
    services::{
        item_catalog,
        tshock_rest::{self, TShockRestClient},
    },
};

/// Helper: build a TShockRestClient for a given server id.
fn client_for(state: &AppState, server_id: &str) -> Result<TShockRestClient, AppError> {
    TShockRestClient::for_server(&state.config.server.data_dir, server_id)
}

fn require_operator(auth: &Auth) -> Result<(), AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "需要管理员或操作员权限".to_string(),
        ));
    }
    Ok(())
}

fn require_admin(auth: &Auth) -> Result<(), AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden("需要管理员权限".to_string()));
    }
    Ok(())
}

fn server_tshock_version(state: &AppState, server_id: &str) -> Result<String, AppError> {
    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;
    let version = db
        .query_row(
            "SELECT tshock_version FROM servers WHERE id = ?1",
            params![server_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound("Server not found".to_string())
            }
            _ => AppError::DatabaseError(e.to_string()),
        })?;
    Ok(version)
}

fn quote_tshock_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn tshock_response_text(value: &Value) -> String {
    if let Some(response) = value.get("response") {
        if let Some(text) = response.as_str() {
            return text.to_string();
        }
        if let Some(lines) = response.as_array() {
            return lines
                .iter()
                .filter_map(|line| line.as_str())
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
    value.to_string()
}

fn rawcmd_response_indicates_failure(value: &Value) -> bool {
    let text = tshock_response_text(value).to_ascii_lowercase();
    text.contains("invalid command")
        || text.contains("invalid syntax")
        || text.contains("not have permission")
        || text.contains("you do not have access")
        || text.contains("could not find")
        || text.contains("failed")
        || text.contains("error")
}

fn log_rest_operation(state: &AppState, server_id: &str, operation: &str, message: &str) {
    let logs_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(server_id)
        .join("logs");

    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
        tracing::error!(server_id = %server_id, error = %e, "Failed to create REST operation log directory");
        return;
    }

    let entry = json!({
        "time": Utc::now().to_rfc3339(),
        "operation": operation,
        "message": message,
    });

    let path = logs_dir.join("rest-operations.log");
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", entry) {
                tracing::error!(server_id = %server_id, path = %path.display(), error = %e, "Failed to write REST operation log");
            }
        }
        Err(e) => {
            tracing::error!(server_id = %server_id, path = %path.display(), error = %e, "Failed to open REST operation log");
        }
    }
}

fn log_rest_success(state: &AppState, server_id: &str, operation: &str, response: &Value) {
    log_rest_operation(state, server_id, operation, &tshock_response_text(response));
}

fn log_rest_error(state: &AppState, server_id: &str, operation: &str, error: &AppError) {
    log_rest_operation(state, server_id, operation, &error.message());
}

macro_rules! proxy_rest_action {
    ($state:expr, $id:expr, $operation:expr, $future:expr) => {{
        let result = $future.await;
        match &result {
            Ok(data) => log_rest_success(&$state, &$id, $operation, data),
            Err(error) => log_rest_error(&$state, &$id, $operation, error),
        }
        let data = result?;
        Ok(Json(data))
    }};
}

// ─── REST Setup ───

/// Check / auto-provision REST API token for a server.
/// Returns `{ ready: bool, message: string }`.
/// If `ready` is false, the server needs a restart.
pub async fn rest_setup(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let result = tshock_rest::ensure_rest_setup(&state.config.server.data_dir, &id);
    match &result {
        Ok((_, message)) => log_rest_operation(&state, &id, "REST 自动配置", message),
        Err(error) => log_rest_error(&state, &id, "REST 自动配置", error),
    }
    let (ready, message) = result?;
    Ok(Json(json!({ "ready": ready, "message": message })))
}

// ─── Server endpoints ───

pub async fn rest_server_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.server_status().await?;
    Ok(Json(data))
}

pub async fn rest_token_test(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
) -> Result<Json<Value>, AppError> {
    require_operator(&auth)?;
    let client = client_for(&state, &id)?;
    proxy_rest_action!(state, id, "REST Token 测试", client.token_test())
}

#[derive(Deserialize)]
pub struct BroadcastBody {
    pub msg: String,
}

pub async fn rest_server_broadcast(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<BroadcastBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 广播: {}", body.msg);
    proxy_rest_action!(state, id, &operation, client.server_broadcast(&body.msg))
}

pub async fn rest_server_reload(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    proxy_rest_action!(state, id, "REST 重载配置", client.server_reload())
}

pub async fn rest_server_restart(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
) -> Result<Json<Value>, AppError> {
    require_operator(&auth)?;
    let client = client_for(&state, &id)?;
    proxy_rest_action!(state, id, "REST 重启服务器", client.server_restart())
}

#[derive(Deserialize)]
pub struct RawCmdBody {
    pub cmd: String,
}

pub async fn rest_server_rawcmd(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<RawCmdBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 原始命令: {}", body.cmd);
    proxy_rest_action!(state, id, &operation, client.server_rawcmd(&body.cmd))
}

// ─── Item catalog / give ───

#[derive(Deserialize)]
pub struct ItemListQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

pub async fn rest_item_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ItemListQuery>,
    auth: Auth,
) -> Result<Json<Value>, AppError> {
    require_operator(&auth)?;
    let version = server_tshock_version(&state, &id)?;
    let catalog = item_catalog::ensure_catalog(&state.config.server.data_dir, &version).await?;
    let items = item_catalog::filter_items(&catalog, query.q.as_deref(), query.limit.unwrap_or(10000));
    Ok(Json(json!({
        "version": catalog.version,
        "source": catalog.source,
        "items": items,
    })))
}

pub async fn rest_item_sync(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
) -> Result<Json<Value>, AppError> {
    require_operator(&auth)?;
    let version = server_tshock_version(&state, &id)?;
    let result = item_catalog::download_catalog(&state.config.server.data_dir, &version).await;
    match &result {
        Ok(catalog) => log_rest_operation(
            &state,
            &id,
            "REST 重新下载物品清单",
            &format!("version={}, count={}", catalog.version, catalog.items.len()),
        ),
        Err(error) => log_rest_error(&state, &id, "REST 重新下载物品清单", error),
    }
    let catalog = result?;
    Ok(Json(json!({
        "version": catalog.version,
        "source": catalog.source,
        "items": catalog.items,
    })))
}

#[derive(Deserialize)]
pub struct GiveItemBody {
    pub player: String,
    pub item_id: Option<i32>,
    pub item_name: Option<String>,
    pub stack: Option<i32>,
}

pub async fn rest_item_give(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
    Json(body): Json<GiveItemBody>,
) -> Result<Json<Value>, AppError> {
    require_operator(&auth)?;

    let player = body.player.trim();
    if player.is_empty() {
        return Err(AppError::BadRequest("玩家名不能为空".to_string()));
    }

    let item_arg = if let Some(item_id) = body.item_id {
        if item_id <= 0 {
            return Err(AppError::BadRequest("物品 ID 必须大于 0".to_string()));
        }
        item_id.to_string()
    } else if let Some(item_name) = body.item_name.as_deref() {
        let item_name = item_name.trim();
        if item_name.is_empty() {
            return Err(AppError::BadRequest("物品不能为空".to_string()));
        }
        quote_tshock_arg(item_name)
    } else {
        return Err(AppError::BadRequest("请选择要发放的物品".to_string()));
    };

    let stack = body.stack.unwrap_or(1).clamp(1, 9999);
    let cmd = format!("/give {} {} {}", item_arg, quote_tshock_arg(player), stack);
    let client = client_for(&state, &id)?;
    let result = client.server_rawcmd(&cmd).await;
    match &result {
        Ok(data) => log_rest_success(&state, &id, &format!("REST 发放物品: {}", cmd), data),
        Err(error) => log_rest_error(&state, &id, &format!("REST 发放物品: {}", cmd), error),
    }
    let data = result?;
    let ok = !rawcmd_response_indicates_failure(&data);
    crate::db::log_operation(&state.db, &auth.user_id, "发放物品", Some(&id), Some(&cmd));

    Ok(Json(json!({
        "ok": ok,
        "command": cmd,
        "message": tshock_response_text(&data),
        "response": data,
    })))
}

#[derive(Deserialize)]
pub struct ServerOffBody {
    pub message: Option<String>,
    #[serde(default)]
    pub nosave: bool,
}

pub async fn rest_server_off(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<ServerOffBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!(
        "REST 停止服务器: nosave={}, message={}",
        body.nosave,
        body.message.as_deref().unwrap_or("")
    );
    proxy_rest_action!(state, id, &operation, client.server_off(body.message.as_deref(), body.nosave))
}

pub async fn rest_server_motd(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.server_motd().await?;
    Ok(Json(data))
}

pub async fn rest_server_rules(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.server_rules().await?;
    Ok(Json(data))
}

// ─── Player endpoints ───

pub async fn rest_player_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.player_list().await?;
    Ok(Json(data))
}

pub async fn rest_player_read(
    State(state): State<AppState>,
    Path((id, player)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.player_read(&player).await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct KickBody {
    pub player: String,
    pub reason: Option<String>,
}

pub async fn rest_player_kick(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<KickBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 踢出玩家: {}", body.player);
    proxy_rest_action!(state, id, &operation, client.player_kick(&body.player, body.reason.as_deref()))
}

pub async fn rest_player_ban(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
    Json(body): Json<KickBody>,
) -> Result<Json<Value>, AppError> {
    require_operator(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 封禁玩家: {}", body.player);
    let result = client.player_ban(&body.player, body.reason.as_deref()).await;
    match &result {
        Ok(data) => log_rest_success(&state, &id, &operation, data),
        Err(error) => log_rest_error(&state, &id, &operation, error),
    }
    let data = result?;
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "REST封禁玩家",
        Some(&id),
        Some(&body.player),
    );
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct PlayerNameBody {
    pub player: String,
}

pub async fn rest_player_kill(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<PlayerNameBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 击杀玩家: {}", body.player);
    proxy_rest_action!(state, id, &operation, client.player_kill(&body.player))
}

pub async fn rest_player_mute(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<PlayerNameBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 禁言玩家: {}", body.player);
    proxy_rest_action!(state, id, &operation, client.player_mute(&body.player))
}

pub async fn rest_player_unmute(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<PlayerNameBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 解除禁言: {}", body.player);
    proxy_rest_action!(state, id, &operation, client.player_unmute(&body.player))
}

// ─── User endpoints (REST) ───

pub async fn rest_user_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.user_list().await?;
    Ok(Json(data))
}

pub async fn rest_user_active_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.user_active_list().await?;
    Ok(Json(data))
}

pub async fn rest_user_read(
    State(state): State<AppState>,
    Path((id, user)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.user_read(&user).await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct UserCreateBody {
    pub user: String,
    pub password: String,
    pub group: Option<String>,
}

pub async fn rest_user_create(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
    Json(body): Json<UserCreateBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 创建用户: {}", body.user);
    proxy_rest_action!(state, id, &operation, client.user_create(&body.user, &body.password, body.group.as_deref()))
}

#[derive(Deserialize)]
pub struct UserUpdateBody {
    pub user: String,
    pub password: Option<String>,
    pub group: Option<String>,
}

pub async fn rest_user_update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
    Json(body): Json<UserUpdateBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 更新用户: {}", body.user);
    proxy_rest_action!(state, id, &operation, client.user_update(&body.user, body.password.as_deref(), body.group.as_deref()))
}

pub async fn rest_user_destroy(
    State(state): State<AppState>,
    Path((id, user)): Path<(String, String)>,
    auth: Auth,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 删除用户: {}", user);
    proxy_rest_action!(state, id, &operation, client.user_destroy(&user))
}

// ─── Group endpoints (REST) ───

pub async fn rest_group_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.group_list().await?;
    Ok(Json(data))
}

pub async fn rest_group_read(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.group_read(&name).await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct GroupCreateBody {
    pub group: String,
    pub parent: Option<String>,
    pub permissions: Option<String>,
}

pub async fn rest_group_create(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: Auth,
    Json(body): Json<GroupCreateBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 创建用户组: {}", body.group);
    proxy_rest_action!(state, id, &operation, client.group_create(&body.group, body.parent.as_deref(), body.permissions.as_deref()))
}

#[derive(Deserialize)]
pub struct GroupUpdateBody {
    pub parent: Option<String>,
    pub permissions: Option<String>,
}

pub async fn rest_group_update(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    auth: Auth,
    Json(body): Json<GroupUpdateBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 更新用户组: {}", name);
    proxy_rest_action!(state, id, &operation, client.group_update(&name, body.parent.as_deref(), body.permissions.as_deref()))
}

pub async fn rest_group_destroy(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    auth: Auth,
) -> Result<Json<Value>, AppError> {
    require_admin(&auth)?;
    let client = client_for(&state, &id)?;
    let operation = format!("REST 删除用户组: {}", name);
    proxy_rest_action!(state, id, &operation, client.group_destroy(&name))
}

// ─── Ban endpoints ───

pub async fn rest_ban_list(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.ban_list().await?;
    Ok(Json(data))
}

pub async fn rest_ban_read(
    State(state): State<AppState>,
    Path((id, ticket)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.ban_read(&ticket).await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct BanCreateBody {
    pub identifier: String,
    pub reason: Option<String>,
    pub duration: Option<String>,
}

pub async fn rest_ban_create(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<BanCreateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 创建封禁: {}", body.identifier);
    proxy_rest_action!(state, id, &operation, client.ban_create(&body.identifier, body.reason.as_deref(), body.duration.as_deref()))
}

pub async fn rest_ban_destroy(
    State(state): State<AppState>,
    Path((id, ticket)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 删除封禁: {}", ticket);
    proxy_rest_action!(state, id, &operation, client.ban_destroy(&ticket))
}

// ─── World endpoints ───

pub async fn rest_world_read(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.world_read().await?;
    Ok(Json(data))
}

pub async fn rest_world_save(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    proxy_rest_action!(state, id, "REST 保存世界", client.world_save())
}

#[derive(Deserialize)]
pub struct ButcherBody {
    #[serde(default)]
    pub kill_friendly: bool,
}

pub async fn rest_world_butcher(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<ButcherBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 清除NPC: kill_friendly={}", body.kill_friendly);
    proxy_rest_action!(state, id, &operation, client.world_butcher(body.kill_friendly))
}

#[derive(Deserialize)]
pub struct BoolStateBody {
    pub state: bool,
}

pub async fn rest_world_bloodmoon(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<BoolStateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 血月状态: {}", body.state);
    proxy_rest_action!(state, id, &operation, client.world_bloodmoon(body.state))
}

pub async fn rest_world_meteor(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    proxy_rest_action!(state, id, "REST 召唤陨石", client.world_meteor())
}

pub async fn rest_world_autosave(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<BoolStateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let operation = format!("REST 自动保存状态: {}", body.state);
    proxy_rest_action!(state, id, &operation, client.world_autosave(body.state))
}
