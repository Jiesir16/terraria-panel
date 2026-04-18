use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    // === 基本设置 ===
    pub server_name: Option<String>,
    pub port: Option<u16>,
    pub max_players: Option<i32>,
    pub world_name: Option<String>,
    pub server_password: Option<String>,

    // === 世界设置 ===
    pub difficulty: Option<u32>,
    pub auto_create: Option<bool>,
    pub world_width: Option<u32>,
    pub world_height: Option<u32>,
    pub seed: Option<String>,

    // === 游戏规则 ===
    pub enable_whitelist: Option<bool>,
    pub pvp_mode: Option<String>, // "normal", "always", "disabled"
    pub spawn_protection: Option<bool>,
    pub spawn_protection_radius: Option<u32>,
    pub npc_spawn_protection_radius: Option<u32>,
    pub hard_core_only: Option<bool>,
    pub medium_core_only: Option<bool>,
    pub soft_core_only: Option<bool>,

    // === 权限与安全 ===
    pub server_side_character: Option<bool>, // SSC 强制服务端存档
    pub disable_build: Option<bool>,         // 禁止未登录玩家建造
    pub disable_clown_bombs: Option<bool>,   // 禁止小丑炸弹破坏
    pub disable_dungeon_guardian: Option<bool>, // 禁止地牢守卫
    pub disable_tombstones: Option<bool>,    // 禁止墓碑掉落
    pub force_time: Option<String>,          // "normal", "day", "night"
    pub disable_invisible_pvp: Option<bool>, // 禁止隐身PvP

    // === 反作弊 ===
    pub anti_cheat: Option<bool>,
    pub kick_on_damage_inflicted: Option<i32>, // 伤害阈值踢出
    pub kick_on_damage_received: Option<i32>,  // 受伤阈值踢出
    pub range_checks: Option<bool>,            // 范围检查
    pub disable_player_count_reporting: Option<bool>, // 隐藏玩家数

    // === REST API ===
    pub rest_api_enabled: Option<bool>,
    pub rest_api_port: Option<u16>,

    // === 消息设定 ===
    pub motd: Option<String>,                          // 进服欢迎消息
    pub server_full_no_reserve_reason: Option<String>, // 服务器满员消息
}

impl ServerConfig {
    /// Merge non-None fields from self into a TShock Settings JSON object
    pub fn apply_to_tshock_settings(
        &self,
        settings: &mut serde_json::Map<String, serde_json::Value>,
    ) {
        use serde_json::json;

        if let Some(ref v) = self.server_name {
            settings.insert("ServerName".to_string(), json!(v));
            settings.insert("UseServerName".to_string(), json!(true));
        }
        if let Some(v) = self.port {
            settings.insert("ServerPort".to_string(), json!(v));
        }
        if let Some(v) = self.max_players {
            settings.insert("MaxSlots".to_string(), json!(v));
        }
        if let Some(ref v) = self.server_password {
            settings.insert("ServerPassword".to_string(), json!(v));
        }
        if let Some(v) = self.enable_whitelist {
            settings.insert("EnableWhitelist".to_string(), json!(v));
        }
        if let Some(ref v) = self.pvp_mode {
            settings.insert("PvPMode".to_string(), json!(v));
        }
        if let Some(v) = self.spawn_protection {
            settings.insert("SpawnProtection".to_string(), json!(v));
        }
        if let Some(v) = self.spawn_protection_radius {
            settings.insert("SpawnProtectionRadius".to_string(), json!(v));
        }
        if let Some(v) = self.server_side_character {
            settings.insert("ServerSideCharacter".to_string(), json!(v));
        }
        if let Some(v) = self.disable_build {
            settings.insert("DisableBuild".to_string(), json!(v));
        }
        if let Some(v) = self.disable_clown_bombs {
            settings.insert("DisableClownBombs".to_string(), json!(v));
        }
        if let Some(v) = self.disable_dungeon_guardian {
            settings.insert("DisableDungeonGuardian".to_string(), json!(v));
        }
        if let Some(v) = self.disable_tombstones {
            settings.insert("DisableTombstones".to_string(), json!(v));
        }
        if let Some(ref v) = self.force_time {
            settings.insert("ForceTime".to_string(), json!(v));
        }
        if let Some(v) = self.disable_invisible_pvp {
            settings.insert("DisableInvisPvP".to_string(), json!(v));
        }
        if let Some(v) = self.anti_cheat {
            settings.insert("EnableAntiCheat".to_string(), json!(v));
        }
        if let Some(v) = self.kick_on_damage_inflicted {
            settings.insert("KickOnDamageInflicted".to_string(), json!(v));
        }
        if let Some(v) = self.kick_on_damage_received {
            settings.insert("KickOnDamageReceived".to_string(), json!(v));
        }
        if let Some(v) = self.range_checks {
            settings.insert("RangeChecks".to_string(), json!(v));
        }
        if let Some(v) = self.disable_player_count_reporting {
            settings.insert("DisablePlayerCountReporting".to_string(), json!(v));
        }
        if let Some(v) = self.rest_api_enabled {
            settings.insert("RestApiEnabled".to_string(), json!(v));
        }
        if let Some(v) = self.rest_api_port {
            settings.insert("RestApiPort".to_string(), json!(v));
        }
        if let Some(ref v) = self.motd {
            settings.insert(
                "ServerFullNoReserveReason".to_string(),
                json!("Server is full."),
            );
            settings.insert("Motd".to_string(), json!(v));
        }
        if let Some(ref v) = self.server_full_no_reserve_reason {
            settings.insert("ServerFullNoReserveReason".to_string(), json!(v));
        }
        if let Some(v) = self.hard_core_only {
            settings.insert("HardcoreOnly".to_string(), json!(v));
        }
        if let Some(v) = self.medium_core_only {
            settings.insert("MediumcoreOnly".to_string(), json!(v));
        }
        if let Some(v) = self.soft_core_only {
            settings.insert("SoftcoreOnly".to_string(), json!(v));
        }
    }

    pub fn from_tshock_settings(settings: &serde_json::Map<String, serde_json::Value>) -> Self {
        fn get_string(
            settings: &serde_json::Map<String, serde_json::Value>,
            key: &str,
        ) -> Option<String> {
            settings
                .get(key)
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        }

        fn get_bool(
            settings: &serde_json::Map<String, serde_json::Value>,
            key: &str,
        ) -> Option<bool> {
            settings.get(key).and_then(|v| v.as_bool())
        }

        fn get_u16(
            settings: &serde_json::Map<String, serde_json::Value>,
            key: &str,
        ) -> Option<u16> {
            settings
                .get(key)
                .and_then(|v| v.as_u64())
                .and_then(|v| u16::try_from(v).ok())
        }

        fn get_u32(
            settings: &serde_json::Map<String, serde_json::Value>,
            key: &str,
        ) -> Option<u32> {
            settings
                .get(key)
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok())
        }

        fn get_i32(
            settings: &serde_json::Map<String, serde_json::Value>,
            key: &str,
        ) -> Option<i32> {
            settings
                .get(key)
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok())
        }

        Self {
            server_name: get_string(settings, "ServerName"),
            port: get_u16(settings, "ServerPort"),
            max_players: get_i32(settings, "MaxSlots"),
            world_name: None,
            server_password: get_string(settings, "ServerPassword"),
            difficulty: None,
            auto_create: None,
            world_width: None,
            world_height: None,
            seed: None,
            enable_whitelist: get_bool(settings, "EnableWhitelist"),
            pvp_mode: get_string(settings, "PvPMode"),
            spawn_protection: get_bool(settings, "SpawnProtection"),
            spawn_protection_radius: get_u32(settings, "SpawnProtectionRadius"),
            npc_spawn_protection_radius: None,
            hard_core_only: get_bool(settings, "HardcoreOnly"),
            medium_core_only: get_bool(settings, "MediumcoreOnly"),
            soft_core_only: get_bool(settings, "SoftcoreOnly"),
            server_side_character: get_bool(settings, "ServerSideCharacter"),
            disable_build: get_bool(settings, "DisableBuild"),
            disable_clown_bombs: get_bool(settings, "DisableClownBombs"),
            disable_dungeon_guardian: get_bool(settings, "DisableDungeonGuardian"),
            disable_tombstones: get_bool(settings, "DisableTombstones"),
            force_time: get_string(settings, "ForceTime"),
            disable_invisible_pvp: get_bool(settings, "DisableInvisPvP"),
            anti_cheat: get_bool(settings, "EnableAntiCheat"),
            kick_on_damage_inflicted: get_i32(settings, "KickOnDamageInflicted"),
            kick_on_damage_received: get_i32(settings, "KickOnDamageReceived"),
            range_checks: get_bool(settings, "RangeChecks"),
            disable_player_count_reporting: get_bool(settings, "DisablePlayerCountReporting"),
            rest_api_enabled: get_bool(settings, "RestApiEnabled"),
            rest_api_port: get_u16(settings, "RestApiPort"),
            motd: get_string(settings, "Motd"),
            server_full_no_reserve_reason: get_string(settings, "ServerFullNoReserveReason"),
        }
    }

    pub fn from_tshock_config_value(config: &serde_json::Value) -> Option<Self> {
        let root = config.as_object()?;
        let settings = root
            .get("Settings")
            .and_then(|v| v.as_object())
            .unwrap_or(root);
        Some(Self::from_tshock_settings(settings))
    }
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
            server_password: None,
            difficulty: None,
            auto_create: None,
            world_width: None,
            world_height: None,
            seed: None,
            enable_whitelist: None,
            pvp_mode: Some("normal".to_string()),
            spawn_protection: Some(true),
            spawn_protection_radius: Some(10),
            npc_spawn_protection_radius: None,
            hard_core_only: Some(false),
            medium_core_only: Some(false),
            soft_core_only: Some(false),
            server_side_character: Some(false),
            disable_build: Some(false),
            disable_clown_bombs: Some(false),
            disable_dungeon_guardian: Some(false),
            disable_tombstones: Some(true),
            force_time: Some("normal".to_string()),
            disable_invisible_pvp: Some(false),
            anti_cheat: Some(true),
            kick_on_damage_inflicted: Some(0),
            kick_on_damage_received: Some(0),
            range_checks: Some(true),
            disable_player_count_reporting: Some(false),
            rest_api_enabled: Some(false),
            rest_api_port: Some(7878),
            motd: None,
            server_full_no_reserve_reason: None,
        }
    }
}
