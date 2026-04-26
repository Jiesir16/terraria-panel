use crate::services::frp_settings::GlobalFrpSettings;
use std::path::{Path, PathBuf};

pub fn runtime_frp_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("runtime").join("frp")
}

pub fn panel_frp_config_path(data_dir: &Path) -> PathBuf {
    runtime_frp_dir(data_dir).join("panel.toml")
}

pub fn server_frp_config_path(data_dir: &Path, server_id: &str) -> PathBuf {
    runtime_frp_dir(data_dir).join(format!("server_{}.toml", server_id))
}

pub fn write_panel_frp_config(
    data_dir: &Path,
    settings: &GlobalFrpSettings,
) -> Result<PathBuf, String> {
    let path = panel_frp_config_path(data_dir);
    write_frp_config(
        &path,
        settings,
        &settings.panel_tunnel.proxy_name,
        settings.panel_tunnel.local_port,
        settings.panel_tunnel.remote_port,
    )?;
    Ok(path)
}

pub fn write_server_frp_config(
    data_dir: &Path,
    settings: &GlobalFrpSettings,
    server_id: &str,
    proxy_name: &str,
    local_port: u16,
    remote_port: u16,
) -> Result<PathBuf, String> {
    let path = server_frp_config_path(data_dir, server_id);
    write_frp_config(&path, settings, proxy_name, local_port, remote_port)?;
    Ok(path)
}

fn write_frp_config(
    path: &Path,
    settings: &GlobalFrpSettings,
    proxy_name: &str,
    local_port: u16,
    remote_port: u16,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create FRP runtime directory: {}", e))?;
    }

    let content = format!(
        "serverAddr = \"{}\"\nserverPort = {}\nauth.method = \"token\"\nauth.token = \"{}\"\nlog.level = \"{}\"\ntransport.protocol = \"{}\"\ntransport.tls.enable = {}\n\n[[proxies]]\nname = \"{}\"\ntype = \"tcp\"\nlocalIP = \"127.0.0.1\"\nlocalPort = {}\nremotePort = {}\n",
        settings.server_addr,
        settings.server_port,
        settings.auth_token.replace('"', "\\\""),
        settings.log_level,
        settings.transport_protocol,
        if settings.tls_enable { "true" } else { "false" },
        proxy_name,
        local_port,
        remote_port,
    );

    std::fs::write(path, content).map_err(|e| format!("Write FRP config {}: {}", path.display(), e))
}
