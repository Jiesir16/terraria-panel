//! TShock REST API client.
//!
//! Each running TShock instance exposes a REST API on a configurable port.
//! This module provides a thin async client that panel handlers use to talk
//! to a specific server instance.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

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

    let rest_enabled = settings
        .and_then(|s| s.get("RestApiEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !rest_enabled {
        return Err(AppError::BadRequest(
            "TShock REST API is not enabled. Enable it in server config first.".to_string(),
        ));
    }

    let rest_port = settings
        .and_then(|s| s.get("RestApiPort"))
        .and_then(|v| v.as_u64())
        .unwrap_or(7878) as u16;

    // TShock generates an application token in the REST config
    // Try to read from rest-tokens file or config
    let token = read_application_token(&config_dir)?;

    let base_url = format!("http://127.0.0.1:{}", rest_port);
    Ok((base_url, token))
}

/// Read the TShock application REST token.
///
/// TShock stores tokens in tshock/config.json under
/// `Settings.ApplicationRestTokens` (a JSON object mapping token→permissions).
/// If no application token exists, we look for one in a panel-managed file.
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

    // 2) Try to extract from TShock config.json ApplicationRestTokens
    let config_path = config_dir.join("config.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::FileError(e.to_string()))?;
        if let Ok(config) = serde_json::from_str::<Value>(&content) {
            let settings = config
                .get("Settings")
                .and_then(|v| v.as_object())
                .or_else(|| config.as_object());
            if let Some(tokens) = settings
                .and_then(|s| s.get("ApplicationRestTokens"))
                .and_then(|v| v.as_object())
            {
                // Return the first token that has superadmin-level permissions
                for (token, _perms) in tokens {
                    return Ok(token.clone());
                }
            }
        }
    }

    Err(AppError::BadRequest(
        "No REST API token found. Please set an ApplicationRestToken in TShock config.json, \
         or create a panel-rest-token.txt file in the server's tshock directory."
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
