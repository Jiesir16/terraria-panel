use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[allow(dead_code)]
pub async fn handle_ws_connection(
    ws: WebSocket,
    mut log_rx: broadcast::Receiver<String>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<String>,
) {
    let (ws_tx, mut ws_rx) = ws.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    // Task to send logs from broadcast channel to WebSocket
    let ws_tx_log = Arc::clone(&ws_tx);
    let log_task = tokio::spawn(async move {
        while let Ok(log_line) = log_rx.recv().await {
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

    // Task to receive commands from WebSocket and send to command channel
    let cmd_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(cmd) = payload.get("command").and_then(|v| v.as_str()) {
                            let _ = cmd_tx.send(cmd.to_string());
                        }
                    } else if !text.is_empty() {
                        let _ = cmd_tx.send(text.to_string());
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
