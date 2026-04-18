use serde::{Deserialize, Serialize};

fn default_enabled() -> bool {
    false
}

fn default_server_side_character_save() -> i32 {
    5
}

fn default_logon_discard_threshold() -> i32 {
    250
}

fn default_starting_health() -> i32 {
    100
}

fn default_starting_mana() -> i32 {
    20
}

fn default_starting_inventory() -> Vec<SscInventoryItem> {
    vec![
        SscInventoryItem {
            net_id: -15,
            prefix: 0,
            stack: 1,
        },
        SscInventoryItem {
            net_id: -13,
            prefix: 0,
            stack: 1,
        },
        SscInventoryItem {
            net_id: -16,
            prefix: 0,
            stack: 1,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SscInventoryItem {
    #[serde(rename = "netID")]
    pub net_id: i32,
    pub prefix: i32,
    pub stack: i32,
}

impl Default for SscInventoryItem {
    fn default() -> Self {
        Self {
            net_id: 0,
            prefix: 0,
            stack: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SscConfig {
    #[serde(rename = "Enabled")]
    #[serde(alias = "enabled", default = "default_enabled")]
    pub enabled: bool,
    #[serde(rename = "ServerSideCharacterSave")]
    #[serde(alias = "server_side_character_save", default = "default_server_side_character_save")]
    pub server_side_character_save: i32,
    #[serde(rename = "LogonDiscardThreshold")]
    #[serde(alias = "logon_discard_threshold", default = "default_logon_discard_threshold")]
    pub logon_discard_threshold: i32,
    #[serde(rename = "StartingHealth")]
    #[serde(alias = "starting_health", default = "default_starting_health")]
    pub starting_health: i32,
    #[serde(rename = "StartingMana")]
    #[serde(alias = "starting_mana", default = "default_starting_mana")]
    pub starting_mana: i32,
    #[serde(rename = "StartingInventory")]
    #[serde(alias = "starting_inventory", default = "default_starting_inventory")]
    pub starting_inventory: Vec<SscInventoryItem>,
}

impl Default for SscConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            server_side_character_save: default_server_side_character_save(),
            logon_discard_threshold: default_logon_discard_threshold(),
            starting_health: default_starting_health(),
            starting_mana: default_starting_mana(),
            starting_inventory: default_starting_inventory(),
        }
    }
}
