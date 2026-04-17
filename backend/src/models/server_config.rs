use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_name: Option<String>,
    pub port: Option<u16>,
    pub max_players: Option<i32>,
    pub world_name: Option<String>,
    pub difficulty: Option<u32>,
    pub auto_create: Option<bool>,
    pub world_width: Option<u32>,
    pub world_height: Option<u32>,
    pub seed: Option<String>,
    pub npc_spawn_protection_radius: Option<u32>,
    pub server_password: Option<String>,
    pub enable_whitelist: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfigTemplate {
    pub name: String,
    pub description: String,
    pub config: ServerConfig,
}

pub fn get_templates() -> Vec<ServerConfigTemplate> {
    vec![
        ServerConfigTemplate {
            name: "Survival".to_string(),
            description: "Standard survival mode".to_string(),
            config: ServerConfig {
                difficulty: Some(0),
                auto_create: Some(false),
                enable_whitelist: Some(false),
                ..Default::default()
            },
        },
        ServerConfigTemplate {
            name: "Creative".to_string(),
            description: "Creative/sandbox mode".to_string(),
            config: ServerConfig {
                difficulty: Some(0),
                auto_create: Some(true),
                npc_spawn_protection_radius: Some(0),
                ..Default::default()
            },
        },
        ServerConfigTemplate {
            name: "PvP".to_string(),
            description: "PvP-focused server".to_string(),
            config: ServerConfig {
                difficulty: Some(1),
                enable_whitelist: Some(true),
                ..Default::default()
            },
        },
    ]
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_name: None,
            port: None,
            max_players: Some(8),
            world_name: None,
            difficulty: None,
            auto_create: None,
            world_width: None,
            world_height: None,
            seed: None,
            npc_spawn_protection_radius: None,
            server_password: None,
            enable_whitelist: None,
        }
    }
}
