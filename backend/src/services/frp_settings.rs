use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const GLOBAL_FRP_SETTINGS_FILE: &str = "settings/frp.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelFrpTunnelSettings {
    pub enabled: bool,
    pub local_port: u16,
    pub remote_port: u16,
    pub proxy_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFrpSettings {
    pub enabled: bool,
    pub frpc_bin: String,
    pub server_addr: String,
    pub server_port: u16,
    pub auth_token: String,
    pub transport_protocol: String,
    pub tls_enable: bool,
    pub log_level: String,
    pub panel_tunnel: PanelFrpTunnelSettings,
}

impl GlobalFrpSettings {
    pub fn with_panel_port(panel_port: u16) -> Self {
        Self {
            enabled: false,
            frpc_bin: "frpc".to_string(),
            server_addr: String::new(),
            server_port: 7000,
            auth_token: String::new(),
            transport_protocol: "tcp".to_string(),
            tls_enable: false,
            log_level: "info".to_string(),
            panel_tunnel: PanelFrpTunnelSettings {
                enabled: false,
                local_port: panel_port,
                remote_port: panel_port,
                proxy_name: "terraria-panel".to_string(),
            },
        }
    }
}

pub fn global_frp_settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join(GLOBAL_FRP_SETTINGS_FILE)
}

pub fn load_global_frp_settings(
    data_dir: &Path,
    panel_port: u16,
) -> Result<GlobalFrpSettings, String> {
    let path = global_frp_settings_path(data_dir);
    if !path.exists() {
        return Ok(GlobalFrpSettings::with_panel_port(panel_port));
    }

    let raw = std::fs::read_to_string(&path).map_err(|e| format!("Read FRP settings: {}", e))?;
    let mut settings = serde_json::from_str::<GlobalFrpSettings>(&raw)
        .map_err(|e| format!("Parse FRP settings: {}", e))?;
    if settings.panel_tunnel.local_port == 0 {
        settings.panel_tunnel.local_port = panel_port;
    }
    Ok(settings)
}

pub fn save_global_frp_settings(
    data_dir: &Path,
    settings: &GlobalFrpSettings,
) -> Result<(), String> {
    let path = global_frp_settings_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create FRP settings directory: {}", e))?;
    }

    let raw = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Serialize FRP settings: {}", e))?;
    std::fs::write(path, raw).map_err(|e| format!("Write FRP settings: {}", e))
}
