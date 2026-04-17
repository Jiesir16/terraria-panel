use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query, State},
    Json,
    response::Response,
};
use std::collections::VecDeque;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::{
    auth::Auth,
    error::AppError,
    handlers::AppState,
};

pub async fn ws_console(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    tracing::info!(server_id = %id, "WebSocket console connection attempt");

    // Extract token from query parameters
    let token = params
        .get("token")
        .ok_or_else(|| {
            tracing::warn!(server_id = %id, "WebSocket rejected: missing token");
            AppError::Unauthorized("Missing token".to_string())
        })?;

    // Verify token
    let claims = state.token_manager.verify(token)?;
    tracing::info!(server_id = %id, user = %claims.username, "WebSocket token verified");

    // Check if server is running first
    if !state.process_manager.is_running(&id).await {
        tracing::warn!(server_id = %id, "WebSocket rejected: server not running");
        return Err(AppError::NotFound("Server not found or offline".to_string()));
    }

    // Get server's broadcast channel
    let log_rx = state
        .process_manager
        .subscribe_logs(&id)
        .await?;
    let history_limit = params
        .get("history")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200);
    let history = state
        .process_manager
        .get_recent_logs(&id, history_limit)
        .await
        .unwrap_or_default();

    let id_for_handler = id.clone();
    let process_manager = state.process_manager.clone();

    tracing::info!(server_id = %id, user = %claims.username, "WebSocket console connected");

    Ok(ws.on_upgrade(move |socket| async move {
        handle_ws_with_pm(socket, log_rx, history, id_for_handler, process_manager).await
    }))
}

async fn handle_ws_with_pm(
    socket: axum::extract::ws::WebSocket,
    log_rx: tokio::sync::broadcast::Receiver<String>,
    history: Vec<String>,
    server_id: String,
    pm: std::sync::Arc<crate::services::ProcessManager>,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    tracing::debug!(server_id = %server_id, "WebSocket handler started");

    let (ws_tx, mut ws_rx) = socket.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    // Send recent history first so refreshing the page keeps context.
    {
        let mut tx = ws_tx.lock().await;
        for log_line in history {
            let message = json!({
                "type": "log",
                "data": log_line
            });
            if tx.send(Message::Text(message.to_string())).await.is_err() {
                tracing::debug!(server_id = %server_id, "WebSocket disconnected while sending history");
                return;
            }
        }
    }

    // Task to send logs from broadcast channel to WebSocket
    let ws_tx_log = Arc::clone(&ws_tx);
    let server_id_log = server_id.clone();
    let log_task = tokio::spawn(async move {
        let mut rx = log_rx;
        while let Ok(log_line) = rx.recv().await {
            let message = json!({
                "type": "log",
                "data": log_line
            });

            let mut tx = ws_tx_log.lock().await;
            if tx.send(Message::Text(message.to_string())).await.is_err() {
                tracing::debug!(server_id = %server_id_log, "WebSocket log sender disconnected");
                break;
            }
        }
    });

    // Task to receive commands from WebSocket and send to process
    let server_id_clone = server_id.clone();
    let cmd_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(cmd) = payload.get("command").and_then(|v| v.as_str()) {
                            tracing::debug!(server_id = %server_id_clone, command = %cmd, "WebSocket command received");
                            let _ = pm.send_command(&server_id_clone, cmd).await;
                        }
                    } else if !text.is_empty() {
                        tracing::debug!(server_id = %server_id_clone, command = %text, "WebSocket raw command received");
                        let _ = pm.send_command(&server_id_clone, &text).await;
                    }
                }
                Message::Close(_) => {
                    tracing::info!(server_id = %server_id_clone, "WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = log_task => {},
        _ = cmd_task => {},
    }

    tracing::info!(server_id = %server_id, "WebSocket console session ended");
}

pub async fn recent_logs(
    State(state): State<AppState>,
    _auth: Auth,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<String>>, AppError> {
    let limit = params
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200);

    let logs = match state.process_manager.get_recent_logs(&id, limit).await {
        Ok(logs) => logs,
        Err(_) => read_recent_logs_from_disk(&state, &id, limit)?,
    };
    Ok(Json(logs))
}

fn read_recent_logs_from_disk(
    state: &AppState,
    server_id: &str,
    limit: usize,
) -> Result<Vec<String>, AppError> {
    let logs_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(server_id)
        .join("logs");

    if !logs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(&logs_dir)
        .map_err(|e| AppError::FileError(format!("Failed to read logs directory: {}", e)))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();

    files.sort_by(|a, b| {
        let a_modified = std::fs::metadata(a)
            .and_then(|m| m.modified())
            .ok();
        let b_modified = std::fs::metadata(b)
            .and_then(|m| m.modified())
            .ok();
        b_modified.cmp(&a_modified)
    });

    let mut lines = VecDeque::with_capacity(limit);
    for file in files {
        let content = std::fs::read_to_string(&file).unwrap_or_default();
        for line in content.lines() {
            if lines.len() >= limit {
                lines.pop_front();
            }
            lines.push_back(line.to_string());
        }
    }

    Ok(lines.into_iter().collect())
}
