use crate::auth::Auth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    Viewer,
    Operator,
    Owner,
    Admin,
}

fn normalize_command(command: &str) -> String {
    let trimmed = command.trim().to_lowercase();
    let mut parts = trimmed.split_whitespace();
    let first = parts.next().unwrap_or_default();
    let second = parts.next().unwrap_or_default();

    match (first, second) {
        ("/time", "day") => "/time day".to_string(),
        ("/time", "night") => "/time night".to_string(),
        ("/group", "list") => "/group list".to_string(),
        ("/user", "list") => "/user list".to_string(),
        ("/whitelist", _) => "/whitelist".to_string(),
        ("/region", _) => "/region".to_string(),
        _ => first.to_string(),
    }
}

pub fn required_scope(command: &str) -> CommandScope {
    let normalized = normalize_command(command);

    match normalized.as_str() {
        "/who" | "/playing" | "/time" | "/world" | "/rules" | "/help" => CommandScope::Viewer,
        "/save" | "/time day" | "/time night" | "/butcher" | "/broadcast" | "/kick" | "/ban"
        | "/mute" | "/tp" | "/tphere" | "/settle" => CommandScope::Operator,
        "/off" | "/reload" | "/whitelist" | "/region" => CommandScope::Owner,
        "/group list" | "/user list" | "/gbuff" | "/grow" | "/spawnmob" | "/give"
        | "/antibuild" | "/godmode" => CommandScope::Admin,
        _ => {
            if normalized.starts_with('/') {
                CommandScope::Owner
            } else {
                CommandScope::Admin
            }
        }
    }
}

pub fn can_execute_command(auth: &Auth, server_owner_id: Option<&str>, command: &str) -> bool {
    match required_scope(command) {
        CommandScope::Viewer => true,
        CommandScope::Operator => auth.is_operator_or_admin(),
        CommandScope::Owner => {
            auth.is_admin()
                || server_owner_id
                    .map(|owner_id| owner_id == auth.user_id)
                    .unwrap_or(false)
        }
        CommandScope::Admin => auth.is_admin(),
    }
}
