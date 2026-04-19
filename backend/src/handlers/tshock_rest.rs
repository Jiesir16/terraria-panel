//! Axum handlers that proxy requests to TShock's built-in REST API.
//!
//! The panel frontend calls these endpoints; the backend resolves the REST
//! token for the target server and forwards the request to TShock.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::Auth,
    error::AppError,
    handlers::AppState,
    services::tshock_rest::{self, TShockRestClient},
};

/// Helper: build a TShockRestClient for a given server id.
fn client_for(state: &AppState, server_id: &str) -> Result<TShockRestClient, AppError> {
    TShockRestClient::for_server(&state.config.server.data_dir, server_id)
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
    let (ready, message) = tshock_rest::ensure_rest_setup(&state.config.server.data_dir, &id)?;
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
    let data = client.server_broadcast(&body.msg).await?;
    Ok(Json(data))
}

pub async fn rest_server_reload(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.server_reload().await?;
    Ok(Json(data))
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
    let data = client.server_rawcmd(&body.cmd).await?;
    Ok(Json(data))
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
    let data = client
        .server_off(body.message.as_deref(), body.nosave)
        .await?;
    Ok(Json(data))
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
    let data = client
        .player_kick(&body.player, body.reason.as_deref())
        .await?;
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
    let data = client.player_kill(&body.player).await?;
    Ok(Json(data))
}

pub async fn rest_player_mute(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<PlayerNameBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.player_mute(&body.player).await?;
    Ok(Json(data))
}

pub async fn rest_player_unmute(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<PlayerNameBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.player_unmute(&body.player).await?;
    Ok(Json(data))
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
    _auth: Auth,
    Json(body): Json<UserCreateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client
        .user_create(&body.user, &body.password, body.group.as_deref())
        .await?;
    Ok(Json(data))
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
    _auth: Auth,
    Json(body): Json<UserUpdateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client
        .user_update(&body.user, body.password.as_deref(), body.group.as_deref())
        .await?;
    Ok(Json(data))
}

pub async fn rest_user_destroy(
    State(state): State<AppState>,
    Path((id, user)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.user_destroy(&user).await?;
    Ok(Json(data))
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
    _auth: Auth,
    Json(body): Json<GroupCreateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client
        .group_create(&body.group, body.parent.as_deref(), body.permissions.as_deref())
        .await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct GroupUpdateBody {
    pub parent: Option<String>,
    pub permissions: Option<String>,
}

pub async fn rest_group_update(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    _auth: Auth,
    Json(body): Json<GroupUpdateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client
        .group_update(&name, body.parent.as_deref(), body.permissions.as_deref())
        .await?;
    Ok(Json(data))
}

pub async fn rest_group_destroy(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.group_destroy(&name).await?;
    Ok(Json(data))
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
    let data = client
        .ban_create(&body.identifier, body.reason.as_deref(), body.duration.as_deref())
        .await?;
    Ok(Json(data))
}

pub async fn rest_ban_destroy(
    State(state): State<AppState>,
    Path((id, ticket)): Path<(String, String)>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.ban_destroy(&ticket).await?;
    Ok(Json(data))
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
    let data = client.world_save().await?;
    Ok(Json(data))
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
    let data = client.world_butcher(body.kill_friendly).await?;
    Ok(Json(data))
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
    let data = client.world_bloodmoon(body.state).await?;
    Ok(Json(data))
}

pub async fn rest_world_meteor(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.world_meteor().await?;
    Ok(Json(data))
}

pub async fn rest_world_autosave(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _auth: Auth,
    Json(body): Json<BoolStateBody>,
) -> Result<Json<Value>, AppError> {
    let client = client_for(&state, &id)?;
    let data = client.world_autosave(body.state).await?;
    Ok(Json(data))
}
