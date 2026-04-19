//! TShock REST API client.
//!
//! Each running TShock instance exposes a REST API on a configurable port.
//! This module provides a thin async client that panel handlers use to talk
//! to a specific server instance.

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

use crate::error::AppError;

// ─── REST Token Management ───

/// Resolve the REST API base URL and application token for a given server.
pub fn resolve_rest_info(
    data_dir: &Path,
    server_id: &str,
) -> Result<(String, String), AppError> {
    let config_dir = data_dir
        .join("servers")
        .join(server_id)
        .join("tshock");
    let config_path = config_dir.join("config.json");

    if !config_path.exists() {
        return Err(AppError::NotFound(
            "TShock config.json not found — server may not have been started yet".to_string(),
        ));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(format!("Failed to read config.json: {}", e)))?;
    let config: Value = serde_json::from_str(&content)
        .map_err(|e| AppError::BadRequest(format!("Invalid config.json: {}", e)))?;

    let settings = config
        .get("Settings")
        .and_then(|v| v.as_object())
        .or_else(|| config.as_object());

    let rest_port = settings
        .and_then(|s| s.get("RestApiPort"))
        .and_then(|v| v.as_u64())
        .unwrap_or(7878) as u16;

    // read_application_token will auto-enable REST API + auto-provision a token
    // if neither exists yet.
    let token = read_application_token(&config_dir)?;

    let base_url = format!("http://127.0.0.1:{}", rest_port);
    Ok((base_url, token))
}

/// Build the TShock-expected token permission object.
///
/// TShock's `ApplicationRestTokens` values are objects like:
/// ```json
/// { "UserGroupName": "superadmin", "Username": "Panel" }
/// ```
fn token_permission_object() -> Value {
    json!({
        "UserGroupName": "superadmin",
        "Username": "Panel"
    })
}

fn token_permission_object_for_group(group: &str) -> Value {
    json!({
        "UserGroupName": group,
        "Username": "Panel"
    })
}

fn read_config_json(config_path: &Path) -> Result<Value, AppError> {
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| AppError::FileError(format!("Failed to read config.json: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| AppError::BadRequest(format!("Invalid config.json: {}", e)))
}

fn write_config_json(config_path: &Path, config: &Value) -> Result<(), AppError> {
    let pretty = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::FileError(format!("Failed to serialize config.json: {}", e)))?;
    std::fs::write(config_path, pretty.as_bytes())
        .map_err(|e| AppError::FileError(format!("Failed to write config.json: {}", e)))
}

fn settings_obj(config: &Value) -> Option<&Map<String, Value>> {
    config
        .get("Settings")
        .and_then(|v| v.as_object())
        .or_else(|| config.as_object())
}

fn settings_obj_mut(config: &mut Value) -> Result<&mut Map<String, Value>, AppError> {
    if config.get("Settings").is_some() {
        if !config.get("Settings").is_some_and(|v| v.is_object()) {
            if let Some(root) = config.as_object_mut() {
                root.insert("Settings".to_string(), Value::Object(Map::new()));
            }
        }
        return config
            .get_mut("Settings")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| AppError::BadRequest("Invalid config.json Settings object".to_string()));
    }

    config
        .as_object_mut()
        .ok_or_else(|| AppError::BadRequest("Invalid config.json root object".to_string()))
}

fn tokens_obj(settings: &Map<String, Value>) -> Option<&Map<String, Value>> {
    settings
        .get("ApplicationRestTokens")
        .and_then(|v| v.as_object())
}

fn tokens_obj_mut(settings: &mut Map<String, Value>) -> &mut Map<String, Value> {
    let tokens = settings
        .entry("ApplicationRestTokens")
        .or_insert_with(|| Value::Object(Map::new()));
    if !tokens.is_object() {
        *tokens = Value::Object(Map::new());
    }
    tokens.as_object_mut().expect("tokens object just initialized")
}

fn cached_token(token_file: &Path) -> Result<Option<String>, AppError> {
    if !token_file.exists() {
        return Ok(None);
    }

    let token = std::fs::read_to_string(token_file)
        .map_err(|e| AppError::FileError(e.to_string()))?
        .trim()
        .to_string();
    if token.is_empty() {
        Ok(None)
    } else {
        Ok(Some(token))
    }
}

fn cache_token(token_file: &Path, token: &str) {
    let _ = std::fs::write(token_file, token.as_bytes());
}

fn inject_token_in_config(
    config_path: &Path,
    config: &mut Value,
    token: &str,
) -> Result<(), AppError> {
    let settings = settings_obj_mut(config)?;
    settings.insert("RestApiEnabled".to_string(), Value::Bool(true));
    if settings.get("RestApiPort").is_none() {
        settings.insert("RestApiPort".to_string(), Value::Number(7878.into()));
    }
    tokens_obj_mut(settings).insert(token.to_string(), token_permission_object());
    write_config_json(config_path, config)
}

fn ensure_rest_api_enabled(config_path: &Path, config: &mut Value) -> Result<(), AppError> {
    let mut changed = false;
    {
        let settings = settings_obj_mut(config)?;
        if settings.get("RestApiEnabled").and_then(|v| v.as_bool()) != Some(true) {
            settings.insert("RestApiEnabled".to_string(), Value::Bool(true));
            changed = true;
        }
        if settings.get("RestApiPort").is_none() {
            settings.insert("RestApiPort".to_string(), Value::Number(7878.into()));
            changed = true;
        }
    }

    if changed {
        write_config_json(config_path, config)?;
    }
    Ok(())
}

fn fix_legacy_token_format(
    config_path: &Path,
    config: &mut Value,
    token_key: &str,
) -> Result<(), AppError> {
    let settings = settings_obj_mut(config)?;
    settings.insert("RestApiEnabled".to_string(), Value::Bool(true));
    if settings.get("RestApiPort").is_none() {
        settings.insert("RestApiPort".to_string(), Value::Number(7878.into()));
    }
    let tokens = tokens_obj_mut(settings);
    let group = tokens
        .get(token_key)
        .and_then(|value| value.as_str())
        .filter(|group| !group.trim().is_empty())
        .unwrap_or("superadmin")
        .to_string();
    tokens.insert(token_key.to_string(), token_permission_object_for_group(&group));
    write_config_json(config_path, config)
}

fn token_entry_is_object(config: &Value, token: &str) -> bool {
    settings_obj(config)
        .and_then(tokens_obj)
        .and_then(|tokens| tokens.get(token))
        .is_some_and(|value| value.is_object())
}

fn token_entry_is_legacy(config: &Value, token: &str) -> bool {
    settings_obj(config)
        .and_then(tokens_obj)
        .and_then(|tokens| tokens.get(token))
        .is_some_and(|value| value.is_string())
}

fn first_config_token(config: &Value) -> Option<(String, bool)> {
    settings_obj(config)
        .and_then(tokens_obj)
        .and_then(|tokens| {
            tokens.iter().find_map(|(token, value)| {
                if value.is_object() {
                    Some((token.clone(), false))
                } else if value.is_string() {
                    Some((token.clone(), true))
                } else {
                    None
                }
            })
        })
}

/// Read the TShock application REST token.
///
/// Lookup order:
/// 1. Panel-managed `panel-rest-token.txt` (fastest, no config parse).
/// 2. First key in `Settings.ApplicationRestTokens` inside config.json.
/// 3. **Auto-provision**: generate a new token, inject it into config.json
///    `ApplicationRestTokens`, and persist it in `panel-rest-token.txt`.
///    The server must be restarted for TShock to load the new token.
fn read_application_token(config_dir: &Path) -> Result<String, AppError> {
    let token_file = config_dir.join("panel-rest-token.txt");
    let config_path = config_dir.join("config.json");
    if !config_path.exists() {
        return Err(AppError::NotFound(
            "TShock config.json not found".to_string(),
        ));
    }

    let mut config = read_config_json(&config_path)?;

    // 1) Use panel-managed token only after verifying config.json still contains it.
    if let Some(token) = cached_token(&token_file)? {
        if token_entry_is_object(&config, &token) {
            ensure_rest_api_enabled(&config_path, &mut config)?;
            return Ok(token);
        }
        if token_entry_is_legacy(&config, &token) {
            fix_legacy_token_format(&config_path, &mut config, &token)?;
            return Ok(token);
        }

        inject_token_in_config(&config_path, &mut config, &token)?;
        return Ok(token);
    }

    // 2) No cached token: use the first valid config token and cache it.
    if let Some((token, legacy)) = first_config_token(&config) {
        if legacy {
            fix_legacy_token_format(&config_path, &mut config, &token)?;
        } else {
            ensure_rest_api_enabled(&config_path, &mut config)?;
        }
        cache_token(&token_file, &token);
        return Ok(token);
    }

    // 3) No token exists: auto-provision one.
    tracing::info!("No REST API token found for server, auto-provisioning one");
    let new_token = provision_token_in_config(&config_path, &mut config)?;
    cache_token(&token_file, &new_token);

    Ok(new_token)
}

/// Generate a new REST token, inject it into config.json with proper format,
/// enable REST API, and write the file back. Returns the new token string.
fn provision_token_in_config(config_path: &Path, config: &mut Value) -> Result<String, AppError> {
    let new_token = Uuid::new_v4().to_string().replace('-', "");
    inject_token_in_config(config_path, config, &new_token)?;

    tracing::info!(
        "REST API token provisioned into config.json. Server restart required for TShock to load it."
    );

    Ok(new_token)
}

/// Check whether the REST API is set up and functional. Returns `(ready, message)`.
/// If not ready, auto-provisions a token and returns `needs_restart = true`.
pub fn ensure_rest_setup(data_dir: &Path, server_id: &str) -> Result<(bool, String, String), AppError> {
    let config_dir = data_dir
        .join("servers")
        .join(server_id)
        .join("tshock");
    let config_path = config_dir.join("config.json");

    if !config_path.exists() {
        return Err(AppError::NotFound(
            "TShock config.json not found — server may not have been started yet".to_string(),
        ));
    }

    let config = read_config_json(&config_path)?;

    let rest_enabled = settings_obj(&config)
        .and_then(|s| s.get("RestApiEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rest_port_ok = settings_obj(&config)
        .and_then(|s| s.get("RestApiPort"))
        .is_some();
    let has_clean_token = first_config_token(&config)
        .map(|(_, legacy)| !legacy)
        .unwrap_or(false);

    if rest_enabled && rest_port_ok && has_clean_token {
        let token = read_application_token(&config_dir)?;
        cache_token(&config_dir.join("panel-rest-token.txt"), &token);
        return Ok((true, "REST API is configured and ready.".to_string(), token));
    }

    let token = read_application_token(&config_dir)?;
    cache_token(&config_dir.join("panel-rest-token.txt"), &token);

    Ok((
        false,
        "REST API token has been provisioned. Please restart the server for changes to take effect."
            .to_string(),
        token,
    ))
}

// ─── REST Client ───

pub struct TShockRestClient {
    client: Client,
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct RestResponse {
    pub status: String,
    #[serde(flatten)]
    pub data: Value,
}

impl TShockRestClient {
    pub fn new(base_url: String, token: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to build HTTP client");
        Self {
            client,
            base_url,
            token,
        }
    }

    /// Create a client for a specific server instance.
    pub fn for_server(data_dir: &Path, server_id: &str) -> Result<Self, AppError> {
        let (base_url, token) = resolve_rest_info(data_dir, server_id)?;
        Ok(Self::new(base_url, token))
    }

    /// Generic GET request
    async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, AppError> {
        let mut query: Vec<(&str, &str)> = vec![("token", &self.token)];
        query.extend_from_slice(params);

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .query(&query)
            .send()
            .await
            .map_err(|e| {
                AppError::ProcessError(format!("TShock REST request failed: {}", e))
            })?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            AppError::ProcessError(format!("Failed to parse TShock REST response: {}", e))
        })?;

        if !status.is_success() {
            let msg = body
                .get("error")
                .or_else(|| body.get("response"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(AppError::ProcessError(format!(
                "TShock REST API error ({}): {}",
                status, msg
            )));
        }

        Ok(body)
    }

    /// Generic POST request
    async fn post(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, AppError> {
        let mut query: Vec<(&str, &str)> = vec![("token", &self.token)];
        query.extend_from_slice(params);

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .query(&query)
            .send()
            .await
            .map_err(|e| {
                AppError::ProcessError(format!("TShock REST request failed: {}", e))
            })?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            AppError::ProcessError(format!("Failed to parse TShock REST response: {}", e))
        })?;

        if !status.is_success() {
            let msg = body
                .get("error")
                .or_else(|| body.get("response"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(AppError::ProcessError(format!(
                "TShock REST API error ({}): {}",
                status, msg
            )));
        }

        Ok(body)
    }

    // ─── Server ───

    pub async fn server_status(&self) -> Result<Value, AppError> {
        self.get("/v2/server/status", &[("players", "true"), ("rules", "true")])
            .await
    }

    pub async fn token_test(&self) -> Result<Value, AppError> {
        self.get("/tokentest", &[]).await
    }

    pub async fn server_broadcast(&self, msg: &str) -> Result<Value, AppError> {
        self.post("/v2/server/broadcast", &[("msg", msg)]).await
    }

    pub async fn server_reload(&self) -> Result<Value, AppError> {
        self.post("/v3/server/reload", &[]).await
    }

    pub async fn server_restart(&self) -> Result<Value, AppError> {
        self.get("/v3/server/restart", &[]).await
    }

    pub async fn server_rawcmd(&self, cmd: &str) -> Result<Value, AppError> {
        self.post("/v3/server/rawcmd", &[("cmd", cmd)]).await
    }

    pub async fn server_off(
        &self,
        message: Option<&str>,
        nosave: bool,
    ) -> Result<Value, AppError> {
        let nosave_str = nosave.to_string();
        let mut params: Vec<(&str, &str)> = vec![("confirm", "true"), ("nosave", &nosave_str)];
        if let Some(msg) = message {
            params.push(("message", msg));
        }
        self.post("/v2/server/off", &params).await
    }

    pub async fn server_motd(&self) -> Result<Value, AppError> {
        self.get("/v3/server/motd", &[]).await
    }

    pub async fn server_rules(&self) -> Result<Value, AppError> {
        self.get("/v3/server/rules", &[]).await
    }

    // ─── Players ───

    pub async fn player_list(&self) -> Result<Value, AppError> {
        self.get("/v2/players/list", &[]).await
    }

    pub async fn player_read(&self, player: &str) -> Result<Value, AppError> {
        self.get("/v4/players/read", &[("player", player)]).await
    }

    pub async fn player_kick(&self, player: &str, reason: Option<&str>) -> Result<Value, AppError> {
        let mut params = vec![("player", player)];
        if let Some(r) = reason {
            params.push(("reason", r));
        }
        self.post("/v2/players/kick", &params).await
    }

    pub async fn player_ban(&self, player: &str, reason: Option<&str>) -> Result<Value, AppError> {
        let mut params = vec![("player", player)];
        if let Some(r) = reason {
            params.push(("reason", r));
        }
        self.post("/v2/players/ban", &params).await
    }

    pub async fn player_kill(&self, player: &str) -> Result<Value, AppError> {
        self.post("/v2/players/kill", &[("player", player)]).await
    }

    pub async fn player_mute(&self, player: &str) -> Result<Value, AppError> {
        self.post("/v2/players/mute", &[("player", player)]).await
    }

    pub async fn player_unmute(&self, player: &str) -> Result<Value, AppError> {
        self.post("/v2/players/unmute", &[("player", player)])
            .await
    }

    // ─── Users ───

    pub async fn user_list(&self) -> Result<Value, AppError> {
        self.get("/v2/users/list", &[]).await
    }

    pub async fn user_active_list(&self) -> Result<Value, AppError> {
        self.get("/v2/users/activelist", &[]).await
    }

    pub async fn user_read(&self, user: &str) -> Result<Value, AppError> {
        self.get("/v2/users/read", &[("user", user), ("type", "name")])
            .await
    }

    pub async fn user_create(
        &self,
        user: &str,
        password: &str,
        group: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut params = vec![("user", user), ("password", password)];
        if let Some(g) = group {
            params.push(("group", g));
        }
        self.post("/v2/users/create", &params).await
    }

    pub async fn user_update(
        &self,
        user: &str,
        password: Option<&str>,
        group: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut params = vec![("user", user), ("type", "name")];
        if let Some(p) = password {
            params.push(("password", p));
        }
        if let Some(g) = group {
            params.push(("group", g));
        }
        self.post("/v2/users/update", &params).await
    }

    pub async fn user_destroy(&self, user: &str) -> Result<Value, AppError> {
        self.post("/v2/users/destroy", &[("user", user), ("type", "name")])
            .await
    }

    // ─── Groups ───

    pub async fn group_list(&self) -> Result<Value, AppError> {
        self.get("/v2/groups/list", &[]).await
    }

    pub async fn group_read(&self, name: &str) -> Result<Value, AppError> {
        self.get("/v2/groups/read", &[("group", name)]).await
    }

    pub async fn group_create(
        &self,
        name: &str,
        parent: Option<&str>,
        permissions: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut params = vec![("group", name)];
        if let Some(p) = parent {
            params.push(("parent", p));
        }
        if let Some(perms) = permissions {
            params.push(("permissions", perms));
        }
        self.post("/v2/groups/create", &params).await
    }

    pub async fn group_update(
        &self,
        name: &str,
        parent: Option<&str>,
        permissions: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut params = vec![("group", name)];
        if let Some(p) = parent {
            params.push(("parent", p));
        }
        if let Some(perms) = permissions {
            params.push(("permissions", perms));
        }
        self.post("/v2/groups/update", &params).await
    }

    pub async fn group_destroy(&self, name: &str) -> Result<Value, AppError> {
        self.post("/v2/groups/destroy", &[("group", name)]).await
    }

    // ─── Bans ───

    pub async fn ban_list(&self) -> Result<Value, AppError> {
        self.get("/v3/bans/list", &[]).await
    }

    pub async fn ban_read(&self, ticket: &str) -> Result<Value, AppError> {
        self.get("/v3/bans/read", &[("ticketNumber", ticket)]).await
    }

    pub async fn ban_create(
        &self,
        identifier: &str,
        reason: Option<&str>,
        duration: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut params = vec![("identifier", identifier)];
        if let Some(r) = reason {
            params.push(("reason", r));
        }
        if let Some(d) = duration {
            params.push(("end", d));
        }
        self.post("/v3/bans/create", &params).await
    }

    pub async fn ban_destroy(&self, ticket: &str) -> Result<Value, AppError> {
        self.post(
            "/v3/bans/destroy",
            &[("ticketNumber", ticket), ("fullDelete", "true")],
        )
        .await
    }

    // ─── World ───

    pub async fn world_read(&self) -> Result<Value, AppError> {
        self.get("/world/read", &[]).await
    }

    pub async fn world_save(&self) -> Result<Value, AppError> {
        self.post("/v2/world/save", &[]).await
    }

    pub async fn world_butcher(&self, kill_friendly: bool) -> Result<Value, AppError> {
        let kf = kill_friendly.to_string();
        self.post("/v2/world/butcher", &[("killfriendly", &kf)])
            .await
    }

    pub async fn world_bloodmoon(&self, state: bool) -> Result<Value, AppError> {
        let s = state.to_string();
        self.post("/v3/world/bloodmoon", &[("state", &s)]).await
    }

    pub async fn world_meteor(&self) -> Result<Value, AppError> {
        self.post("/world/meteor", &[]).await
    }

    pub async fn world_autosave(&self, state: bool) -> Result<Value, AppError> {
        let s = state.to_string();
        self.post("/v3/world/autosave", &[("state", &s)]).await
    }
}
