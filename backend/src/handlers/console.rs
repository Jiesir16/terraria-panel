use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query, State},
    response::Response,
};
use std::collections::HashMap;

use crate::{
    error::AppError,
    handlers::AppState,
};

pub async fn ws_console(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    // Extract token from query parameters
    let token = params
        .get("token")
        .ok_or_else(|| AppError::Unauthorized("Missing token".to_string()))?;

    // Verify token
    let _claims = state.token_manager.verify(token)?;

    // Check if server is running first
    if !state.process_manager.is_running(&id).await {
        return Err(AppError::NotFound("Server not found or offline".to_string()));
    }

    // Get server's broadcast channel
    let log_rx = state
        .process_manager
        .subscribe_logs(&id)
        .await?;

    let id_for_handler = id.clone();
    let process_manager = state.process_manager.clone();

    Ok(ws.on_upgrade(move |socket| async move {
        handle_ws_with_pm(socket, log_rx, id_for_handler, process_manager).await
    }))
}

async fn handle_ws_with_pm(
    socket: axum::extract::ws::WebSocket,
    log_rx: tokio::sync::broadcast::Receiver<String>,
    server_id: String,
    pm: std::sync::Arc<crate::services::ProcessManager>,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let (ws_tx, mut ws_rx) = socket.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    // Task to send logs from broadcast channel to WebSocket
    let ws_tx_log = Arc::clone(&ws_tx);
    let log_task = tokio::spawn(async move {
        let mut rx = log_rx;
        while let Ok(log_line) = rx.recv().await {
            let message = json!({
                "type": "log",
                "data": log_line
            });

            let mut tx = ws_tx_log.lock().await;
            if tx.send(Message::Text(message.to_string())).await.is_err() {
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
                            let _ = pm.send_command(&server_id_clone, cmd).await;
                        }
                    } else if !text.is_empty() {
                        let _ = pm.send_command(&server_id_clone, &text).await;
                    }
                }
                Message::Close(_) => {
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
}
