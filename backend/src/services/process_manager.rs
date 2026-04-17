use crate::error::AppError;
use crate::models::ServerStatus;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, RwLock};

pub struct ServerProcess {
    pub stdin_tx: tokio::sync::mpsc::UnboundedSender<String>,
    pub log_tx: broadcast::Sender<String>,
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
        autocreate: Option<u32>,
        world_name_for_create: &Option<String>,
    ) -> Result<(), AppError> {
        // Check if already running (short read lock)
        {
            let processes = self.processes.read().await;
            if processes.contains_key(server_id) {
                tracing::warn!(server_id = %server_id, "Server process already running");
                return Err(AppError::Conflict("Server already running".to_string()));
            }
        }

        // Ensure config directory exists
        std::fs::create_dir_all(config_path).map_err(|e| {
            tracing::error!(server_id = %server_id, config_path = %config_path, error = %e, "Failed to create config directory");
            AppError::ProcessError(format!("Failed to create config directory: {}", e))
        })?;

        // Detect TShock executable: self-contained binary (v6+) or DLL (v5 and earlier)
        let self_contained_bin = format!("{}/TShock.Server", version_path);
        let dll_path = format!("{}/TShock.Server.dll", version_path);

        let (executable, is_self_contained) = if std::path::Path::new(&self_contained_bin).exists() {
            (self_contained_bin.clone(), true)
        } else if std::path::Path::new(&dll_path).exists() {
            (dll_path.clone(), false)
        } else {
            let dir_contents = std::fs::read_dir(version_path)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|_| "UNREADABLE".to_string());
            tracing::error!(
                server_id = %server_id,
                version_dir = %version_path,
                dir_contents = %dir_contents,
                "TShock executable not found (checked TShock.Server and TShock.Server.dll)"
            );
            return Err(AppError::NotFound(format!(
                "TShock 可执行文件未找到，请检查版本 {} 是否正确安装。目录内容: [{}]",
                version_path, dir_contents
            )));
        };

        tracing::info!(
            server_id = %server_id,
            executable = %executable,
            is_self_contained = is_self_contained,
            config_path = %config_path,
            port = port,
            max_players = max_players,
            "Spawning TShock process"
        );

        // Build command based on executable type
        let mut cmd = if is_self_contained {
            // v6+: self-contained binary, run directly
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&executable) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(perms.mode() | 0o755);
                    let _ = std::fs::set_permissions(&executable, perms);
                }
            }
            let c = tokio::process::Command::new(&executable);
            c
        } else {
            // v5 and earlier: run via dotnet runtime
            let mut c = tokio::process::Command::new("dotnet");
            c.arg(&executable);
            c
        };

        cmd.arg("-configpath")
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

        // autocreate: 1=small, 2=medium, 3=large — tells TShock to create a new world
        if let Some(size) = autocreate {
            if size >= 1 && size <= 3 {
                tracing::info!(server_id = %server_id, autocreate = size, "Auto-creating world");
                cmd.arg("-autocreate").arg(size.to_string());
            }
        }

        if let Some(wn) = world_name_for_create {
            if !wn.is_empty() {
                cmd.arg("-worldname").arg(wn);
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

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::ProcessError("Failed to get stderr".to_string()))?;

        let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel();
        let (log_tx, _log_rx) = broadcast::channel(1000);

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
        let log_tx_stdout = log_tx.clone();
        let server_id_stdout = server_id.to_string();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::trace!(server_id = %server_id_stdout, line = %line, "stdout");
                let _ = log_tx_stdout.send(line);
            }
            tracing::info!(server_id = %server_id_stdout, "Stdout reader task ended");
        });

        // Spawn stderr reader task — merge into same log channel
        let log_tx_stderr = log_tx.clone();
        let server_id_stderr = server_id.to_string();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(server_id = %server_id_stderr, stderr = %line, "TShock stderr");
                let _ = log_tx_stderr.send(format!("[STDERR] {}", line));
            }
            tracing::info!(server_id = %server_id_stderr, "Stderr reader task ended");
        });

        let process = ServerProcess {
            stdin_tx,
            log_tx,
            status: ServerStatus::Running,
        };

        // Insert under write lock
        let mut processes = self.processes.write().await;
        processes.insert(server_id.to_string(), process);
        tracing::info!(server_id = %server_id, "Server process registered");

        // Now spawn the real exit watcher that owns the child
        let processes_ref = Arc::clone(&self.processes);
        let server_id_exit2 = server_id.to_string();
        tokio::spawn(async move {
            let status = child.wait().await;
            match &status {
                Ok(exit) => {
                    tracing::warn!(
                        server_id = %server_id_exit2,
                        exit_code = ?exit.code(),
                        "TShock process exited"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        server_id = %server_id_exit2,
                        error = %e,
                        "Error waiting for TShock process"
                    );
                }
            }

            // Remove from processes map
            {
                let mut processes = processes_ref.write().await;
                processes.remove(&server_id_exit2);
                tracing::info!(server_id = %server_id_exit2, "Process entry removed after exit");
            }
        });

        drop(processes); // release write lock

        Ok(())
    }

    pub async fn stop_server(&self, server_id: &str) -> Result<(), AppError> {
        let mut processes = self.processes.write().await;

        if let Some(process) = processes.remove(server_id) {
            tracing::info!(server_id = %server_id, "Stopping server: sending exit command");
            // Send exit command via stdin
            let _ = process.stdin_tx.send("exit".to_string());

            // The exit watcher task will detect the process exit and update DB.
            // We just wait a bit for graceful shutdown confirmation.
            drop(processes); // release lock so exit watcher can work

            // Give TShock a moment to process the exit command
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            tracing::info!(server_id = %server_id, "Stop command sent");
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

        if processes.contains_key(server_id) {
            Ok(ServerStatus::Running)
        } else {
            Ok(ServerStatus::Stopped)
        }
    }

    pub async fn subscribe_logs(&self, server_id: &str) -> Result<broadcast::Receiver<String>, AppError> {
        let processes = self.processes.read().await;

        if let Some(process) = processes.get(server_id) {
            Ok(process.log_tx.subscribe())
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
