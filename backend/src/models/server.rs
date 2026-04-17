use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl ServerStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            ServerStatus::Stopped => "stopped",
            ServerStatus::Starting => "starting",
            ServerStatus::Running => "running",
            ServerStatus::Stopping => "stopping",
            ServerStatus::Error => "error",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "starting" => ServerStatus::Starting,
            "running" => ServerStatus::Running,
            "stopping" => ServerStatus::Stopping,
            "error" => ServerStatus::Error,
            _ => ServerStatus::Stopped,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub tshock_version: String,
    pub world_name: Option<String>,
    pub status: String,
    pub password: Option<String>,
    pub max_players: i32,
    pub auto_start: bool,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub port: Option<u16>,
    pub tshock_version: String,
    pub world_name: Option<String>,
    pub password: Option<String>,
    pub max_players: Option<i32>,
    pub auto_start: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub max_players: Option<i32>,
    pub auto_start: Option<bool>,
    pub world_name: Option<String>,
    pub tshock_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ServerDetail {
    pub server: Server,
    pub player_count: i32,
    pub uptime_seconds: i64,
}

#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub command: String,
}
