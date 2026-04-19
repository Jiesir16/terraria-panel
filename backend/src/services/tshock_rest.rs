//! TShock REST API client.
//!
//! Each running TShock instance exposes a REST API on a configurable port.
//! This module provides a thin async client that panel handlers use to talk
//! to a specific server instance.

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
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
    serde_json::json!({
        "UserGroupName": "superadmin",
        "Username": "Panel"
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
    // 1) Check panel-managed token file first
    let token_file = config_dir.join("panel-rest-token.txt");
    if token_file.exists() {
        let token = std::fs::read_to_string(&token_file)
            .map_err(|e| AppError::FileError(e.to_string()))?
            .trim()
            .to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }

    let config_path = config_dir.join("config.json");
    if !config_path.exists() {
        return Err(AppError::NotFound(
            "TShock config.json not found".to_string(),
        ));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;
    let mut config: Value = serde_json::from_str(&content)
        .map_err(|e| AppError::BadRequest(format!("Invalid config.json: {}", e)))?;

    // 2) Try to extract an existing token from ApplicationRestTokens
    {
        let settings = config
            .get("Settings")
            .and_then(|v| v.as_object())
            .or_else(|| config.as_object());
        if let Some(tokens) = settings
            .and_then(|s| s.get("ApplicationRestTokens"))
            .and_then(|v| v.as_object())
        {
            for (token, _perms) in tokens {
                // Persist for fast lookup next time
                let _ = std::fs::write(&token_file, token.as_bytes());
                return Ok(token.clone());
            }
        }
    }

    // 3) No token exists — auto-provision one
    tracing::info!("No REST API token found for server, auto-provisioning one");
    let new_token = provision_token_in_config(&config_path, &mut config)?;

    // Also save to panel-rest-token.txt for fast future lookups
    let _ = std::fs::write(&token_file, new_token.as_bytes());

    Ok(new_token)
}

/// Generate a new REST token, inject it into config.json with proper format,
/// enable REST API, and write the file back. Returns the new token string.
fn provision_token_in_config(config_path: &Path, config: &mut Value) -> Result<String, AppError> {
    let new_token = Uuid::new_v4().to_string().replace('-', "");
    let perm_obj = token_permission_object();

    if let Some(settings) = config.get_mut("Settings").and_then(|v| v.as_object_mut()) {
        // Ensure RestApiEnabled is true
        settings.insert("RestApiEnabled".to_string(), Value::Bool(true));
        // Insert token with proper TShock object format
        let tokens_obj = settings
            .entry("ApplicationRestTokens")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(map) = tokens_obj.as_object_mut() {
            map.insert(new_token.clone(), perm_obj);
        }
    } else if let Some(root) = config.as_object_mut() {
        root.insert("RestApiEnabled".to_string(), Value::Bool(true));
        let tokens_obj = root
            .entry("ApplicationRestTokens")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(map) = tokens_obj.as_object_mut() {
            map.insert(new_token.clone(), perm_obj);
        }
    }

    // Write config back
    let pretty = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::FileError(format!("Failed to serialize config.json: {}", e)))?;
    std::fs::write(config_path, pretty.as_bytes())
        .map_err(|e| AppError::FileError(format!("Failed to write config.json: {}", e)))?;

    tracing::info!(
        "REST API token provisioned into config.json. Server restart required for TShock to load it."
    );

    Ok(new_token)
}

/// Check whether the REST API is set up and functional. Returns `(ready, message)`.
/// If not ready, auto-provisions a token and returns `needs_restart = true`.
pub fn ensure_rest_setup(data_dir: &Path, server_id: &str) -> Result<(bool, String), AppError> {
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

    let token_file = config_dir.join("panel-rest-token.txt");

    // Check if token already provisioned
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;
    let mut config: Value = serde_json::from_str(&content)
        .map_err(|e| AppError::BadRequest(format!("Invalid config.json: {}", e)))?;

    let settings = config
        .get("Settings")
        .and_then(|v| v.as_object())
        .or_else(|| config.as_object());

    let rest_enabled = settings
        .and_then(|s| s.get("RestApiEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let has_token = settings
        .and_then(|s| s.get("ApplicationRestTokens"))
        .and_then(|v| v.as_object())
        .map(|m| !m.is_empty())
        .unwrap_or(false);

    if rest_enabled && has_token {
        return Ok((true, "REST API is configured and ready.".to_string()));
    }

    // Need to provision
    let new_token = provision_token_in_config(&config_path, &mut config)?;
    let _ = std::fs::write(&token_file, new_token.as_bytes());

    Ok((
        false,
        "REST API token has been provisioned. Please restart the server for changes to take effect."
            .to_string(),
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

    pub async fn server_broadcast(&self, msg: &str) -> Result<Value, AppError> {
        self.post("/v2/server/broadcast", &[("msg", msg)]).await
    }

    pub async fn server_reload(&self) -> Result<Value, AppError> {
        self.post("/v3/server/reload", &[]).await
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
