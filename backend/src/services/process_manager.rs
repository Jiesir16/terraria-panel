use crate::error::AppError;
use crate::models::ServerStatus;
use serde_json::Value;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, RwLock};

pub struct ServerProcess {
    pub stdin_tx: tokio::sync::mpsc::UnboundedSender<String>,
    pub log_tx: broadcast::Sender<String>,
    pub log_history: Arc<tokio::sync::Mutex<VecDeque<String>>>,
    pub pid: u32,
    #[allow(dead_code)]
    pub status: ServerStatus,
}

#[derive(Clone)]
pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<String, ServerProcess>>>,
}

const MAX_LOG_HISTORY_LINES: usize = 2000;

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
        server_config_path: &Option<String>,
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

        // Detect launch mode.
        // Many .NET releases ship both an apphost binary and a DLL. If the DLL exists,
        // prefer `dotnet TShock.Server.dll`; only run the bare binary when no DLL is present.
        let self_contained_bin = format!("{}/TShock.Server", version_path);
        let dll_path = format!("{}/TShock.Server.dll", version_path);

        let version_dir = Path::new(version_path);
        let is_self_contained = crate::services::VersionManager::is_self_contained(version_dir);
        let (executable, launch_mode) = if std::path::Path::new(&dll_path).exists() {
            (dll_path.clone(), "dotnet")
        } else if is_self_contained && std::path::Path::new(&self_contained_bin).exists() {
            (self_contained_bin.clone(), "direct")
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

        Self::ensure_runtime_available(version_dir, launch_mode)?;

        tracing::info!(
            server_id = %server_id,
            executable = %executable,
            launch_mode = launch_mode,
            config_path = %config_path,
            port = port,
            max_players = max_players,
            "Spawning TShock process"
        );

        let server_dir = Path::new(config_path)
            .parent()
            .ok_or_else(|| AppError::ProcessError("Invalid config path".to_string()))?;
        let world_dir = server_dir.join("world");
        let logs_dir = server_dir.join("logs");

        // Build command based on executable type
        let mut cmd = if launch_mode == "direct" {
            // Self-contained binary, run directly
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
            // Framework-dependent .NET app: run via dotnet runtime
            let mut c = tokio::process::Command::new("dotnet");
            c.arg(&executable);
            c
        };

        cmd.arg("-configpath")
            .arg(config_path)
            .arg("-worldpath")
            .arg(&world_dir)
            .arg("-logpath")
            .arg(&logs_dir)
            .arg("-port")
            .arg(port.to_string())
            .arg("-maxplayers")
            .arg(max_players.to_string())
            .current_dir(server_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        if let Some(server_config_path) = server_config_path {
            if !server_config_path.is_empty() {
                cmd.arg("-config").arg(server_config_path);
            }
        }

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

        if autocreate.is_some() {
            if let Some(wn) = world_name_for_create {
                if !wn.is_empty() {
                    let normalized_name = if wn.ends_with(".wld") {
                        wn.trim_end_matches(".wld")
                    } else {
                        wn.as_str()
                    };
                    cmd.arg("-worldname").arg(normalized_name);
                }
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
        let log_history = Arc::new(tokio::sync::Mutex::new(VecDeque::with_capacity(
            MAX_LOG_HISTORY_LINES,
        )));

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
        let log_history_stdout = Arc::clone(&log_history);
        let server_id_stdout = server_id.to_string();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::trace!(server_id = %server_id_stdout, line = %line, "stdout");
                {
                    let mut history = log_history_stdout.lock().await;
                    if history.len() >= MAX_LOG_HISTORY_LINES {
                        history.pop_front();
                    }
                    history.push_back(line.clone());
                }
                let _ = log_tx_stdout.send(line);
            }
            tracing::info!(server_id = %server_id_stdout, "Stdout reader task ended");
        });

        // Spawn stderr reader task — merge into same log channel
        let log_tx_stderr = log_tx.clone();
        let log_history_stderr = Arc::clone(&log_history);
        let server_id_stderr = server_id.to_string();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(server_id = %server_id_stderr, stderr = %line, "TShock stderr");
                let stderr_line = format!("[STDERR] {}", line);
                {
                    let mut history = log_history_stderr.lock().await;
                    if history.len() >= MAX_LOG_HISTORY_LINES {
                        history.pop_front();
                    }
                    history.push_back(stderr_line.clone());
                }
                let _ = log_tx_stderr.send(stderr_line);
            }
            tracing::info!(server_id = %server_id_stderr, "Stderr reader task ended");
        });

        let process = ServerProcess {
            stdin_tx,
            log_tx,
            log_history,
            pid,
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
        let (stdin_tx, pid) = {
            let processes = self.processes.read().await;
            if let Some(process) = processes.get(server_id) {
                (process.stdin_tx.clone(), process.pid)
            } else {
                tracing::warn!(server_id = %server_id, "Cannot stop: server not running");
                return Err(AppError::NotFound(format!("Server {} not running", server_id)));
            }
        };

        tracing::info!(server_id = %server_id, pid = pid, "Stopping server: sending exit command");
        let _ = stdin_tx.send("exit".to_string());

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if !self.is_running(server_id).await {
                tracing::info!(server_id = %server_id, "Server exited after graceful stop");
                return Ok(());
            }

            if tokio::time::Instant::now() >= deadline {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        tracing::warn!(server_id = %server_id, pid = pid, "Graceful stop timed out, sending SIGTERM");
        Self::signal_process(pid, false)?;

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if !self.is_running(server_id).await {
                tracing::info!(server_id = %server_id, "Server exited after SIGTERM");
                return Ok(());
            }

            if tokio::time::Instant::now() >= deadline {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        tracing::warn!(server_id = %server_id, pid = pid, "SIGTERM timed out, sending SIGKILL");
        Self::signal_process(pid, true)?;
        Ok(())
    }

    pub async fn kill_server(&self, server_id: &str) -> Result<(), AppError> {
        let pid = {
            let processes = self.processes.read().await;
            if let Some(process) = processes.get(server_id) {
                process.pid
            } else {
                tracing::warn!(server_id = %server_id, "Cannot kill: server not running");
                return Err(AppError::NotFound(format!("Server {} not running", server_id)));
            }
        };

        tracing::warn!(server_id = %server_id, pid = pid, "Force killing server process");
        Self::signal_process(pid, true)?;
        Ok(())
    }

    fn signal_process(pid: u32, force: bool) -> Result<(), AppError> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let signal = if force { Signal::SIGKILL } else { Signal::SIGTERM };
            kill(Pid::from_raw(pid as i32), signal)
                .map_err(|e| AppError::ProcessError(format!("Failed to signal process {}: {}", pid, e)))?;
            Ok(())
        }

        #[cfg(not(unix))]
        {
            let _ = (pid, force);
            Err(AppError::ProcessError(
                "Force kill is only implemented on Unix-like systems".to_string(),
            ))
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

    pub async fn get_recent_logs(
        &self,
        server_id: &str,
        limit: usize,
    ) -> Result<Vec<String>, AppError> {
        let history = {
            let processes = self.processes.read().await;
            let process = processes
                .get(server_id)
                .ok_or_else(|| AppError::NotFound(format!("Server {} not running", server_id)))?;
            Arc::clone(&process.log_history)
        };

        let history = history.lock().await;
        let limit = limit.min(MAX_LOG_HISTORY_LINES);
        let start = history.len().saturating_sub(limit);
        Ok(history.iter().skip(start).cloned().collect())
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    fn ensure_runtime_available(version_dir: &Path, launch_mode: &str) -> Result<(), AppError> {
        let required_runtime = Self::detect_required_runtime_major(version_dir);
        let dotnet_output = std::process::Command::new("dotnet")
            .arg("--list-runtimes")
            .output();

        let output = match dotnet_output {
            Ok(output) => output,
            Err(e) => {
                let requirement = required_runtime
                    .map(|major| format!(".NET {} 运行时", major))
                    .unwrap_or_else(|| ".NET 运行时".to_string());
                return Err(AppError::ProcessError(format!(
                    "当前 TShock 版本需要 {}，但服务器上未检测到可用的 `dotnet`。请先安装对应运行时后再启动。原始错误: {}",
                    requirement, e
                )));
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AppError::ProcessError(format!(
                "无法检测服务器上的 .NET 运行时（执行 `dotnet --list-runtimes` 失败）。请先安装 .NET 运行时。{}",
                if stderr.is_empty() {
                    String::new()
                } else {
                    format!(" 错误输出: {}", stderr)
                }
            )));
        }

        if let Some(major) = required_runtime {
            let runtimes = String::from_utf8_lossy(&output.stdout);
            let expected_prefix = format!("Microsoft.NETCore.App {}.", major);
            if !runtimes.lines().any(|line| line.contains(&expected_prefix)) {
                return Err(AppError::ProcessError(format!(
                    "当前 TShock 版本需要 .NET {} 运行时，但服务器上未找到该版本。请安装 `dotnet-runtime-{}.0` 后再启动。",
                    major, major
                )));
            }
        } else if launch_mode == "direct" {
            tracing::warn!(
                version_dir = %version_dir.display(),
                "Runtime config not found; proceeding with direct launch because `dotnet --list-runtimes` is available"
            );
        }

        Ok(())
    }

    fn detect_required_runtime_major(version_dir: &Path) -> Option<u32> {
        let runtimeconfig_names = [
            "TShock.Server.runtimeconfig.json",
            "TerrariaServer.runtimeconfig.json",
        ];

        for name in runtimeconfig_names {
            let path = version_dir.join(name);
            if !path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&path).ok()?;
            let json: Value = serde_json::from_str(&content).ok()?;
            let runtime_options = json.get("runtimeOptions")?;

            if let Some(version) = runtime_options
                .get("framework")
                .and_then(|framework| framework.get("version"))
                .and_then(|version| version.as_str())
            {
                return version.split('.').next()?.parse::<u32>().ok();
            }

            if let Some(version) = runtime_options
                .get("frameworks")
                .and_then(|frameworks| frameworks.as_array())
                .and_then(|frameworks| frameworks.iter().find_map(|framework| {
                    framework
                        .get("version")
                        .and_then(|version| version.as_str())
                }))
            {
                return version.split('.').next()?.parse::<u32>().ok();
            }
        }

        None
    }
}
