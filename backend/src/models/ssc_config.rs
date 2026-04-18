use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscInventoryItem {
    #[serde(rename = "netID")]
    pub net_id: i32,
    pub prefix: i32,
    pub stack: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SscConfig {
    #[serde(rename = "Enabled")]
    pub enabled: bool,
    #[serde(rename = "ServerSideCharacterSave")]
    pub server_side_character_save: i32,
    #[serde(rename = "LogonDiscardThreshold")]
    pub logon_discard_threshold: i32,
    #[serde(rename = "StartingHealth")]
    pub starting_health: i32,
    #[serde(rename = "StartingMana")]
    pub starting_mana: i32,
    #[serde(rename = "StartingInventory")]
    pub starting_inventory: Vec<SscInventoryItem>,
}

impl Default for SscConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_side_character_save: 5,
            logon_discard_threshold: 250,
            starting_health: 100,
            starting_mana: 20,
            starting_inventory: vec![
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
            ],
        }
    }
}
