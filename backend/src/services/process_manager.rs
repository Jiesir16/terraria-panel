use crate::error::AppError;
use crate::models::ServerStatus;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Child;
use tokio::sync::{broadcast, RwLock};

pub struct ServerProcess {
    pub child: Child,
    pub stdin_tx: tokio::sync::mpsc::UnboundedSender<String>,
    pub stdout_rx: broadcast::Receiver<String>,
    #[allow(dead_code)]
    pub status: ServerStatus,
}

#[derive(Clone)]
pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<String, ServerProcess>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_server(
        &self,
        server_id: &str,
        version_path: &str,
        config_path: &str,
        port: u16,
        max_players: i32,
        password: &Option<String>,
        world_path: &Option<String>,
    ) -> Result<(), AppError> {
        let mut processes = self.processes.write().await;

        if processes.contains_key(server_id) {
            tracing::warn!(server_id = %server_id, "Server process already running");
            return Err(AppError::Conflict("Server already running".to_string()));
        }

        // Check if TShock executable exists
        let tshock_dll = format!("{}/TShock.Server.dll", version_path);
        if !std::path::Path::new(&tshock_dll).exists() {
            tracing::error!(server_id = %server_id, path = %tshock_dll, "TShock executable not found");
            return Err(AppError::NotFound(format!(
                "TShock executable not found at {}",
                tshock_dll
            )));
        }

        // Build TShock command
        tracing::info!(
            server_id = %server_id,
            tshock_dll = %tshock_dll,
            config_path = %config_path,
            port = port,
            max_players = max_players,
            "Spawning TShock process"
        );

        let mut cmd = tokio::process::Command::new("dotnet");
        cmd.arg(&tshock_dll)
            .arg("-configpath")
            .arg(config_path)
            .arg("-port")
            .arg(port.to_string())
            .arg("-maxplayers")
            .arg(max_players.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        if let Some(pwd) = password {
            cmd.arg("-password").arg(pwd);
        }

        if let Some(world) = world_path {
            if std::path::Path::new(world).exists() {
                tracing::info!(server_id = %server_id, world = %world, "Loading world file");
                cmd.arg("-world").arg(world);
            } else {
                tracing::warn!(server_id = %server_id, world = %world, "World file not found, starting without -world flag");
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| {
                tracing::error!(server_id = %server_id, error = %e, "Failed to spawn TShock process");
                AppError::ProcessError(format!("Failed to start server: {}", e))
            })?;

        let pid = child.id().unwrap_or(0);
        tracing::info!(server_id = %server_id, pid = pid, "TShock process spawned");

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::ProcessError("Failed to get stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::ProcessError("Failed to get stdout".to_string()))?;

        let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stdout_tx, stdout_rx) = broadcast::channel(100);

        // Spawn stdin writer task
        let server_id_stdin = server_id.to_string();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(cmd) = stdin_rx.recv().await {
                tracing::trace!(server_id = %server_id_stdin, command = %cmd, "Writing to stdin");
                let _ = stdin.write_all(format!("{}\n", cmd).as_bytes()).await;
                let _ = stdin.flush().await;
            }
            tracing::debug!(server_id = %server_id_stdin, "Stdin writer task ended");
        });

        // Spawn stdout reader task
        let stdout_tx_clone = stdout_tx.clone();
        let server_id_stdout = server_id.to_string();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = stdout_tx_clone.send(line);
            }
            tracing::info!(server_id = %server_id_stdout, "Stdout reader task ended (process likely exited)");
        });

        let process = ServerProcess {
            child,
            stdin_tx,
            stdout_rx,
            status: ServerStatus::Running,
        };

        processes.insert(server_id.to_string(), process);
        tracing::info!(server_id = %server_id, "Server process registered");
        Ok(())
    }

    pub async fn stop_server(&self, server_id: &str) -> Result<(), AppError> {
        let mut processes = self.processes.write().await;

        if let Some(mut process) = processes.remove(server_id) {
            tracing::info!(server_id = %server_id, "Stopping server: sending exit command");
            // Send exit command
            let _ = process.stdin_tx.send("exit".to_string());

            // Wait for graceful shutdown
            let timeout = std::time::Duration::from_secs(10);
            let start = std::time::Instant::now();

            loop {
                match process.child.try_wait() {
                    Ok(Some(status)) => {
                        tracing::info!(server_id = %server_id, exit_code = ?status.code(), "Server process exited gracefully");
                        return Ok(());
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            tracing::warn!(server_id = %server_id, "Server did not exit within timeout, force killing");
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        tracing::error!(server_id = %server_id, error = %e, "Error waiting for server process");
                        return Err(AppError::ProcessError(format!(
                            "Failed to wait for process: {}",
                            e
                        )))
                    }
                }
            }

            // Force kill
            let _ = process.child.kill().await;
            tracing::warn!(server_id = %server_id, "Server process force killed");
            Ok(())
        } else {
            tracing::warn!(server_id = %server_id, "Cannot stop: server not running");
            Err(AppError::NotFound(format!("Server {} not running", server_id)))
        }
    }

    pub async fn send_command(&self, server_id: &str, command: &str) -> Result<(), AppError> {
        let processes = self.processes.read().await;

        if let Some(process) = processes.get(server_id) {
            tracing::debug!(server_id = %server_id, command = %command, "Sending command to server process");
            process
                .stdin_tx
                .send(command.to_string())
                .map_err(|_| {
                    tracing::error!(server_id = %server_id, command = %command, "Failed to send command to stdin");
                    AppError::ProcessError("Failed to send command".to_string())
                })
        } else {
            tracing::warn!(server_id = %server_id, command = %command, "Cannot send command: server not running");
            Err(AppError::NotFound(format!("Server {} not running", server_id)))
        }
    }

    #[allow(dead_code)]
    pub async fn get_status(&self, server_id: &str) -> Result<ServerStatus, AppError> {
        let processes = self.processes.read().await;

        if let Some(process) = processes.get(server_id) {
            Ok(process.status)
        } else {
            Ok(ServerStatus::Stopped)
        }
    }

    pub async fn subscribe_logs(&self, server_id: &str) -> Result<broadcast::Receiver<String>, AppError> {
        let processes = self.processes.read().await;

        if let Some(process) = processes.get(server_id) {
            Ok(process.stdout_rx.resubscribe())
        } else {
            Err(AppError::NotFound(format!("Server {} not running", server_id)))
        }
    }

    pub async fn is_running(&self, server_id: &str) -> bool {
        let processes = self.processes.read().await;
        processes.contains_key(server_id)
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
