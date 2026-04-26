use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use crate::{
    auth::Auth,
    config::TelegramConfig,
    error::AppError,
    handlers::{self, AppState},
    models::CommandRequest,
};

#[derive(Debug, Clone)]
struct TelegramServer {
    id: String,
    name: String,
    status: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct TelegramApiResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    chat: TelegramChat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

pub fn spawn_telegram_bot(config: TelegramConfig, state: AppState) {
    if !config.enabled {
        return;
    }

    if config.bot_token.trim().is_empty() {
        tracing::warn!("Telegram bot is enabled but bot_token is empty; skipping startup");
        return;
    }

    if config.allowed_chat_ids.is_empty() && config.admin_chat_ids.is_empty() {
        tracing::warn!(
            "Telegram bot is enabled but no allowed chat ids are configured; skipping startup"
        );
        return;
    }

    tokio::spawn(async move {
        tracing::info!("Telegram bot polling task started");
        run_telegram_bot(config, state).await;
    });
}

async fn run_telegram_bot(config: TelegramConfig, state: AppState) {
    let client = reqwest::Client::new();
    let mut offset: Option<i64> = None;

    loop {
        match fetch_updates(&client, &config, offset).await {
            Ok(updates) => {
                for update in updates {
                    offset = Some(update.update_id + 1);
                    if let Err(error) = handle_update(&client, &config, &state, update).await {
                        tracing::warn!(error = %error, "Failed to handle Telegram update");
                    }
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "Telegram polling failed");
                tokio::time::sleep(Duration::from_secs(config.retry_seconds.max(1))).await;
            }
        }
    }
}

async fn fetch_updates(
    client: &reqwest::Client,
    config: &TelegramConfig,
    offset: Option<i64>,
) -> Result<Vec<TelegramUpdate>, AppError> {
    let mut body = json!({
        "timeout": config.poll_timeout_seconds.max(1),
        "allowed_updates": ["message"]
    });
    if let Some(offset) = offset {
        body["offset"] = json!(offset);
    }

    let response = client
        .post(telegram_url(config, "getUpdates"))
        .json(&body)
        .send()
        .await
        .map_err(|error| AppError::InternalServerError(error.to_string()))?;

    let payload = response
        .json::<TelegramApiResponse<Vec<TelegramUpdate>>>()
        .await
        .map_err(|error| AppError::InternalServerError(error.to_string()))?;

    if !payload.ok {
        return Err(AppError::BadRequest(
            payload
                .description
                .unwrap_or_else(|| "Telegram getUpdates failed".to_string()),
        ));
    }

    Ok(payload.result.unwrap_or_default())
}

async fn handle_update(
    client: &reqwest::Client,
    config: &TelegramConfig,
    state: &AppState,
    update: TelegramUpdate,
) -> Result<(), AppError> {
    let Some(message) = update.message else {
        return Ok(());
    };

    let chat_id = message.chat.id;
    if !chat_allowed(config, chat_id) {
        tracing::warn!(
            chat_id = chat_id,
            "Rejected Telegram message from unauthorized chat"
        );
        return Ok(());
    }

    let Some(text) = message.text else {
        return Ok(());
    };

    if !text.trim_start().starts_with('/') {
        return Ok(());
    }

    let reply = handle_command(config, state, chat_id, &text).await;
    send_message(client, config, chat_id, &reply).await
}

async fn handle_command(
    config: &TelegramConfig,
    state: &AppState,
    chat_id: i64,
    text: &str,
) -> String {
    let (command, args) = split_command(text);
    match command.as_str() {
        "help" | "start" => help_text(),
        "servers" | "list" => match list_servers_text(state).await {
            Ok(text) => text,
            Err(error) => format!("读取服务器列表失败：{}", error),
        },
        "status" => match required_server_arg(args) {
            Ok(server_key) => match server_status_text(state, server_key).await {
                Ok(text) => text,
                Err(error) => format!("读取状态失败：{}", error),
            },
            Err(text) => text,
        },
        "start_server" | "startserver" => {
            run_server_action(
                config,
                state,
                chat_id,
                args,
                "启动",
                |state, auth, server_id| async move {
                    handlers::server::start_server(State(state), auth, Path(server_id)).await
                },
            )
            .await
        }
        "stop_server" | "stopserver" => {
            run_server_action(
                config,
                state,
                chat_id,
                args,
                "停止",
                |state, auth, server_id| async move {
                    handlers::server::stop_server(State(state), auth, Path(server_id)).await
                },
            )
            .await
        }
        "restart_server" | "restartserver" => {
            run_server_action(
                config,
                state,
                chat_id,
                args,
                "重启",
                |state, auth, server_id| async move {
                    handlers::server::restart_server(State(state), auth, Path(server_id)).await
                },
            )
            .await
        }
        "cmd" | "command" | "exec" => run_console_command(config, state, chat_id, args).await,
        _ => format!("未知命令：/{}\n\n{}", command, help_text()),
    }
}

async fn run_server_action<F, Fut>(
    config: &TelegramConfig,
    state: &AppState,
    chat_id: i64,
    args: &str,
    action: &str,
    run: F,
) -> String
where
    F: FnOnce(AppState, Auth, String) -> Fut,
    Fut: std::future::Future<Output = Result<Json<Value>, AppError>>,
{
    let server_key = match required_server_arg(args) {
        Ok(value) => value,
        Err(text) => return text,
    };

    let server = match find_server(state, server_key) {
        Ok(server) => server,
        Err(error) => return format!("找不到服务器：{}", error),
    };

    let auth = telegram_auth(config, chat_id);
    match run(state.clone(), auth, server.id.clone()).await {
        Ok(_) => format!("{}命令已执行：{} ({})", action, server.name, server.id),
        Err(error) => format!("{}失败：{}", action, error),
    }
}

async fn run_console_command(
    config: &TelegramConfig,
    state: &AppState,
    chat_id: i64,
    args: &str,
) -> String {
    let Some((server_key, command)) = split_first_arg(args) else {
        return "用法：/cmd <server_id|name> <command>\n示例：/cmd survival /save".to_string();
    };

    if command.trim().is_empty() {
        return "命令不能为空。示例：/cmd survival /save".to_string();
    }

    let server = match find_server(state, server_key) {
        Ok(server) => server,
        Err(error) => return format!("找不到服务器：{}", error),
    };

    let auth = telegram_auth(config, chat_id);
    let result = handlers::server::send_command(
        State(state.clone()),
        auth,
        Path(server.id.clone()),
        Json(CommandRequest {
            command: command.to_string(),
        }),
    )
    .await;

    match result {
        Ok(_) => format!("命令已发送到 {}：{}", server.name, command),
        Err(error) => format!("发送命令失败：{}", error),
    }
}

async fn list_servers_text(state: &AppState) -> Result<String, AppError> {
    let servers = query_servers(state)?;
    if servers.is_empty() {
        return Ok("暂无服务器。".to_string());
    }

    let mut lines = Vec::with_capacity(servers.len() + 1);
    lines.push("服务器列表：".to_string());
    for server in servers {
        let process_status = if state.process_manager.is_running(&server.id).await {
            "进程运行"
        } else {
            "进程停止"
        };
        lines.push(format!(
            "- {} ({})：{}，{}，端口 {}",
            server.name, server.id, server.status, process_status, server.port
        ));
    }
    Ok(lines.join("\n"))
}

async fn server_status_text(state: &AppState, server_key: &str) -> Result<String, AppError> {
    let server = find_server(state, server_key)?;
    let running = state.process_manager.is_running(&server.id).await;
    Ok(format!(
        "{} ({})\n状态：{}\n进程：{}\n端口：{}",
        server.name,
        server.id,
        server.status,
        if running { "运行中" } else { "未运行" },
        server.port
    ))
}

fn query_servers(state: &AppState) -> Result<Vec<TelegramServer>, AppError> {
    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;
    let mut statement = db
        .prepare("SELECT id, name, status, port FROM servers ORDER BY name ASC")
        .map_err(|error| AppError::DatabaseError(error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            Ok(TelegramServer {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                port: row.get(3)?,
            })
        })
        .map_err(|error| AppError::DatabaseError(error.to_string()))?;

    let mut servers = Vec::new();
    for row in rows {
        servers.push(row.map_err(|error| AppError::DatabaseError(error.to_string()))?);
    }
    Ok(servers)
}

fn find_server(state: &AppState, key: &str) -> Result<TelegramServer, AppError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::BadRequest("server_id/name 不能为空".to_string()));
    }

    let servers = query_servers(state)?;
    if let Some(server) = servers
        .iter()
        .find(|server| server.id == key || server.name.eq_ignore_ascii_case(key))
    {
        return Ok(server.clone());
    }

    let key_lower = key.to_lowercase();
    let matches: Vec<_> = servers
        .into_iter()
        .filter(|server| server.name.to_lowercase().contains(&key_lower))
        .collect();

    match matches.len() {
        1 => Ok(matches[0].clone()),
        0 => Err(AppError::NotFound(key.to_string())),
        _ => Err(AppError::Conflict(
            "匹配到多个服务器，请使用 /servers 中显示的 id".to_string(),
        )),
    }
}

fn split_command(text: &str) -> (String, &str) {
    let trimmed = text.trim();
    let Some((command, args)) = split_first_arg(trimmed) else {
        return (normalize_command_token(trimmed), "");
    };
    (normalize_command_token(command), args)
}

fn split_first_arg(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let first = parts.next()?;
    let rest = parts.next().unwrap_or("").trim();
    Some((first, rest))
}

fn normalize_command_token(token: &str) -> String {
    token
        .trim_start_matches('/')
        .split('@')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn required_server_arg(args: &str) -> Result<&str, String> {
    let Some((server_key, _)) = split_first_arg(args) else {
        return Err("用法：/status <server_id|name>".to_string());
    };
    Ok(server_key)
}

fn telegram_auth(config: &TelegramConfig, chat_id: i64) -> Auth {
    let role = if config.admin_chat_ids.contains(&chat_id) {
        "admin"
    } else {
        "operator"
    };

    Auth {
        user_id: format!("telegram:{}", chat_id),
        username: format!("telegram:{}", chat_id),
        role: role.to_string(),
    }
}

fn chat_allowed(config: &TelegramConfig, chat_id: i64) -> bool {
    config.allowed_chat_ids.contains(&chat_id) || config.admin_chat_ids.contains(&chat_id)
}

async fn send_message(
    client: &reqwest::Client,
    config: &TelegramConfig,
    chat_id: i64,
    text: &str,
) -> Result<(), AppError> {
    let payload = json!({
        "chat_id": chat_id,
        "text": truncate_message(text),
        "disable_web_page_preview": true
    });

    let response = client
        .post(telegram_url(config, "sendMessage"))
        .json(&payload)
        .send()
        .await
        .map_err(|error| AppError::InternalServerError(error.to_string()))?;

    let payload = response
        .json::<TelegramApiResponse<Value>>()
        .await
        .map_err(|error| AppError::InternalServerError(error.to_string()))?;

    if !payload.ok {
        return Err(AppError::BadRequest(
            payload
                .description
                .unwrap_or_else(|| "Telegram sendMessage failed".to_string()),
        ));
    }

    Ok(())
}

fn telegram_url(config: &TelegramConfig, method: &str) -> String {
    format!(
        "https://api.telegram.org/bot{}/{}",
        config.bot_token.trim(),
        method
    )
}

fn truncate_message(text: &str) -> String {
    const LIMIT: usize = 3900;
    if text.chars().count() <= LIMIT {
        return text.to_string();
    }
    let mut truncated = text.chars().take(LIMIT).collect::<String>();
    truncated.push_str("\n...");
    truncated
}

fn help_text() -> String {
    [
        "Terraria Panel Telegram 管理命令：",
        "/servers - 列出服务器",
        "/status <server_id|name> - 查看状态",
        "/start_server <server_id|name> - 启动服务器",
        "/stop_server <server_id|name> - 停止服务器",
        "/restart_server <server_id|name> - 重启服务器",
        "/cmd <server_id|name> <command> - 发送控制台命令",
        "",
        "示例：/cmd survival /save",
    ]
    .join("\n")
}
