use crate::config::{BackupConfig, OssBackupConfig};
use crate::models::{BackupPolicyOverride, ServerConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const GLOBAL_BACKUP_POLICY_FILE: &str = "settings/backup-policy.json";
const SERVER_PANEL_CONFIG_FILE: &str = "tshock/panel-config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBackupPolicy {
    pub enabled: bool,
    pub interval_minutes: u64,
    pub max_backups_per_server: usize,
    pub local_retention_days: u64,
    pub backup_ssc: bool,
    pub archive_daily_enabled: bool,
    pub archive_hour: u8,
    pub archive_after_days: u64,
    pub oss: OssBackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveBackupPolicy {
    pub enabled: bool,
    pub interval_minutes: u64,
    pub max_backups_per_server: usize,
    pub local_retention_days: u64,
    pub backup_ssc: bool,
    pub archive_daily_enabled: bool,
    pub archive_hour: u8,
    pub archive_after_days: u64,
    pub oss: OssBackupConfig,
}

impl From<&BackupConfig> for GlobalBackupPolicy {
    fn from(value: &BackupConfig) -> Self {
        Self {
            enabled: value.enabled,
            interval_minutes: value.interval_minutes,
            max_backups_per_server: value.max_backups_per_server,
            local_retention_days: value.local_retention_days,
            backup_ssc: value.backup_ssc,
            archive_daily_enabled: value.archive_daily_enabled,
            archive_hour: value.archive_hour,
            archive_after_days: value.archive_after_days,
            oss: value.oss.clone(),
        }
    }
}

pub fn global_policy_path(data_dir: &Path) -> PathBuf {
    data_dir.join(GLOBAL_BACKUP_POLICY_FILE)
}

pub fn load_global_backup_policy(
    data_dir: &Path,
    fallback: &BackupConfig,
) -> Result<GlobalBackupPolicy, String> {
    let path = global_policy_path(data_dir);
    if !path.exists() {
        return Ok(GlobalBackupPolicy::from(fallback));
    }

    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("Read global backup policy: {}", e))?;
    serde_json::from_str::<GlobalBackupPolicy>(&raw)
        .map_err(|e| format!("Parse global backup policy: {}", e))
}

pub fn save_global_backup_policy(
    data_dir: &Path,
    policy: &GlobalBackupPolicy,
) -> Result<(), String> {
    let path = global_policy_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create backup policy directory: {}", e))?;
    }

    let raw = serde_json::to_string_pretty(policy)
        .map_err(|e| format!("Serialize global backup policy: {}", e))?;
    std::fs::write(path, raw).map_err(|e| format!("Write global backup policy: {}", e))
}

pub fn load_server_backup_override(
    data_dir: &Path,
    server_id: &str,
) -> Result<Option<BackupPolicyOverride>, String> {
    let path = data_dir
        .join("servers")
        .join(server_id)
        .join(SERVER_PANEL_CONFIG_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read server backup override: {}", e))?;
    let config = serde_json::from_str::<ServerConfig>(&raw)
        .map_err(|e| format!("Parse server backup override: {}", e))?;
    Ok(config.backup_policy_override)
}

pub fn resolve_effective_backup_policy(
    global: &GlobalBackupPolicy,
    override_policy: Option<&BackupPolicyOverride>,
) -> EffectiveBackupPolicy {
    let override_policy = override_policy.cloned().unwrap_or_default();

    EffectiveBackupPolicy {
        enabled: override_policy.enabled.unwrap_or(global.enabled),
        interval_minutes: override_policy
            .interval_minutes
            .unwrap_or(global.interval_minutes),
        max_backups_per_server: override_policy
            .max_backups_per_server
            .unwrap_or(global.max_backups_per_server),
        local_retention_days: override_policy
            .local_retention_days
            .unwrap_or(global.local_retention_days),
        backup_ssc: override_policy.backup_ssc.unwrap_or(global.backup_ssc),
        archive_daily_enabled: global.archive_daily_enabled,
        archive_hour: global.archive_hour,
        archive_after_days: global.archive_after_days,
        oss: global.oss.clone(),
    }
}
