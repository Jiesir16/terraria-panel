use crate::error::AppError;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FrpRuntimeStatus {
    pub key: String,
    pub running: bool,
    pub pid: Option<u32>,
    pub config_path: Option<String>,
    pub remote_port: Option<u16>,
    pub last_error: Option<String>,
}

#[derive(Debug)]
struct FrpProcess {
    pid: u32,
    config_path: String,
    remote_port: Option<u16>,
    last_error: Option<String>,
}

#[derive(Clone)]
pub struct FrpManager {
    processes: Arc<RwLock<HashMap<String, FrpProcess>>>,
}

impl FrpManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_tunnel(
        &self,
        key: &str,
        frpc_bin: &str,
        config_path: &str,
        remote_port: Option<u16>,
    ) -> Result<(), AppError> {
        self.stop_tunnel(key).await.ok();

        let child = tokio::process::Command::new(frpc_bin)
            .arg("-c")
            .arg(config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| AppError::ProcessError(format!("Failed to start frpc: {}", e)))?;

        let pid = child.id().unwrap_or(0);
        let process = FrpProcess {
            pid,
            config_path: config_path.to_string(),
            remote_port,
            last_error: None,
        };

        let mut processes = self.processes.write().await;
        processes.insert(key.to_string(), process);
        Ok(())
    }

    pub async fn stop_tunnel(&self, key: &str) -> Result<(), AppError> {
        let process = {
            let mut processes = self.processes.write().await;
            processes.remove(key)
        };

        if let Some(process) = process {
            let _ = kill(Pid::from_raw(process.pid as i32), Signal::SIGTERM);
        }
        Ok(())
    }

    pub async fn restart_tunnel(
        &self,
        key: &str,
        frpc_bin: &str,
        config_path: &str,
        remote_port: Option<u16>,
    ) -> Result<(), AppError> {
        self.stop_tunnel(key).await?;
        self.start_tunnel(key, frpc_bin, config_path, remote_port)
            .await
    }

    pub async fn status(&self, key: &str) -> FrpRuntimeStatus {
        let processes = self.processes.read().await;
        if let Some(process) = processes.get(key) {
            // Check if process is still alive
            let running = Self::is_process_alive(process.pid);
            FrpRuntimeStatus {
                key: key.to_string(),
                running,
                pid: Some(process.pid),
                config_path: Some(process.config_path.clone()),
                remote_port: process.remote_port,
                last_error: process.last_error.clone(),
            }
        } else {
            FrpRuntimeStatus {
                key: key.to_string(),
                running: false,
                pid: None,
                config_path: None,
                remote_port: None,
                last_error: None,
            }
        }
    }

    pub async fn recover_from_config(
        &self,
        key: &str,
        config_path: &str,
        remote_port: Option<u16>,
    ) -> Result<bool, AppError> {
        // Try to find existing frpc process for this config
        if let Some(pid) = Self::find_frpc_pid_by_config(config_path).await {
            let process = FrpProcess {
                pid,
                config_path: config_path.to_string(),
                remote_port,
                last_error: None,
            };
            let mut processes = self.processes.write().await;
            processes.insert(key.to_string(), process);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn stop_all(&self) {
        let keys = {
            let processes = self.processes.read().await;
            processes.keys().cloned().collect::<Vec<_>>()
        };

        for key in keys {
            let _ = self.stop_tunnel(&key).await;
        }
    }

    fn is_process_alive(pid: u32) -> bool {
        kill(Pid::from_raw(pid as i32), None).is_ok()
    }

    pub async fn find_frpc_pid_by_config(config_path: &str) -> Option<u32> {
        let output = tokio::process::Command::new("pgrep")
            .arg("-f")
            .arg(config_path)
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().next()?.trim().parse().ok()
        } else {
            None
        }
    }
}
