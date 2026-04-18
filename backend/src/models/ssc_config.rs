use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

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

fn default_warn_players_about_bypass_permission() -> bool {
    true
}

fn default_keep_player_appearance() -> bool {
    false
}

fn default_favorited() -> bool {
    false
}

fn default_starting_inventory() -> Vec<SscInventoryItem> {
    vec![
        SscInventoryItem {
            net_id: -15,
            prefix: 0,
            stack: 1,
            favorited: false,
        },
        SscInventoryItem {
            net_id: -13,
            prefix: 0,
            stack: 1,
            favorited: false,
        },
        SscInventoryItem {
            net_id: -16,
            prefix: 0,
            stack: 1,
            favorited: false,
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
    #[serde(default = "default_favorited")]
    pub favorited: bool,
}

impl Default for SscInventoryItem {
    fn default() -> Self {
        Self {
            net_id: 0,
            prefix: 0,
            stack: 1,
            favorited: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SscSettings {
    #[serde(rename = "Enabled", alias = "enabled", default = "default_enabled")]
    pub enabled: bool,
    #[serde(
        rename = "ServerSideCharacterSave",
        alias = "server_side_character_save",
        default = "default_server_side_character_save"
    )]
    pub server_side_character_save: i32,
    #[serde(
        rename = "LogonDiscardThreshold",
        alias = "logon_discard_threshold",
        default = "default_logon_discard_threshold"
    )]
    pub logon_discard_threshold: i32,
    #[serde(
        rename = "StartingHealth",
        alias = "starting_health",
        default = "default_starting_health"
    )]
    pub starting_health: i32,
    #[serde(
        rename = "StartingMana",
        alias = "starting_mana",
        default = "default_starting_mana"
    )]
    pub starting_mana: i32,
    #[serde(
        rename = "StartingInventory",
        alias = "starting_inventory",
        default = "default_starting_inventory"
    )]
    pub starting_inventory: Vec<SscInventoryItem>,
    #[serde(
        rename = "WarnPlayersAboutBypassPermission",
        alias = "warn_players_about_bypass_permission",
        default = "default_warn_players_about_bypass_permission"
    )]
    pub warn_players_about_bypass_permission: bool,
    #[serde(
        rename = "KeepPlayerAppearance",
        alias = "keep_player_appearance",
        default = "default_keep_player_appearance"
    )]
    pub keep_player_appearance: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for SscSettings {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            server_side_character_save: default_server_side_character_save(),
            logon_discard_threshold: default_logon_discard_threshold(),
            starting_health: default_starting_health(),
            starting_mana: default_starting_mana(),
            starting_inventory: default_starting_inventory(),
            warn_players_about_bypass_permission: default_warn_players_about_bypass_permission(),
            keep_player_appearance: default_keep_player_appearance(),
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SscConfig {
    #[serde(rename = "Settings")]
    pub settings: SscSettings,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for SscConfig {
    fn default() -> Self {
        Self {
            settings: SscSettings::default(),
            extra: BTreeMap::new(),
        }
    }
}

impl SscConfig {
    pub fn enabled(&self) -> bool {
        self.settings.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.settings.enabled = enabled;
    }
}

impl<'de> Deserialize<'de> for SscConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Wrapped {
                #[serde(rename = "Settings")]
                settings: SscSettings,
                #[serde(flatten)]
                extra: BTreeMap<String, Value>,
            },
            Flat(SscSettings),
        }

        match Wire::deserialize(deserializer)? {
            Wire::Wrapped { settings, extra } => Ok(Self { settings, extra }),
            Wire::Flat(settings) => Ok(Self {
                settings,
                extra: BTreeMap::new(),
            }),
        }
    }
}
