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
        ("/ban", _) => "/ban".to_string(),
        ("/mute", _) => "/mute".to_string(),
        ("/unmute", _) => "/unmute".to_string(),
        ("/kill", _) => "/kill".to_string(),
        ("/slay", _) => "/kill".to_string(),
        ("/slap", _) => "/slap".to_string(),
        ("/heal", _) => "/heal".to_string(),
        ("/godmode", _) => "/godmode".to_string(),
        ("/god", _) => "/godmode".to_string(),
        ("/gbuff", _) => "/gbuff".to_string(),
        ("/give", _) => "/give".to_string(),
        ("/g", _) => "/give".to_string(),
        ("/worldevent", _) => "/worldevent".to_string(),
        ("/wind", _) => "/wind".to_string(),
        ("/hardmode", _) => "/hardmode".to_string(),
        ("/evil", _) => "/evil".to_string(),
        ("/worldmode", _) => "/worldmode".to_string(),
        ("/gamemode", _) => "/worldmode".to_string(),
        _ => first.to_string(),
    }
}

/// Commands that TShock's command registry marks as `AllowServer = false`.
/// They require an in-game player context and should not be sent from the panel.
pub fn requires_game_player_context(command: &str) -> bool {
    matches!(
        normalize_command(command).as_str(),
        "/setup"
            | "/login"
            | "/logout"
            | "/password"
            | "/register"
            | "/item"
            | "/i"
            | "/spawnboss"
            | "/sb"
            | "/spawnmob"
            | "/sm"
            | "/home"
            | "/spawn"
            | "/tp"
            | "/tphere"
            | "/tpnpc"
            | "/tppos"
            | "/pos"
            | "/tpallow"
            | "/grow"
            | "/setspawn"
            | "/setdungeon"
            | "/buff"
            | "/party"
            | "/p"
            | "/wallow"
            | "/wa"
            | "/death"
            | "/pvpdeath"
    )
}

pub fn required_scope(command: &str) -> CommandScope {
    let normalized = normalize_command(command);

    match normalized.as_str() {
        "/who" | "/online" | "/playing" | "/time" | "/world" | "/rules" | "/help" | "/motd" => {
            CommandScope::Viewer
        }
        "/save" | "/time day" | "/time night" | "/butcher" | "/broadcast" | "/bc" | "/say"
        | "/kick" | "/ban" | "/mute" | "/unmute" | "/kill" | "/slap" | "/heal" | "/godmode"
        | "/gbuff" | "/give" | "/settle" | "/worldevent" | "/wind" => CommandScope::Operator,
        "/off" | "/reload" | "/whitelist" | "/region" => CommandScope::Owner,
        "/group list" | "/user list" | "/antibuild" | "/hardmode" | "/evil" | "/worldmode" => {
            CommandScope::Admin
        }
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
    if requires_game_player_context(command) {
        return false;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(role: &str) -> Auth {
        Auth {
            user_id: "u1".to_string(),
            username: "tester".to_string(),
            role: role.to_string(),
        }
    }

    #[test]
    fn blocks_in_game_only_commands_for_admins() {
        let admin = auth("admin");
        for command in ["/tp Bob", "/spawnmob zombie 1", "/spawnboss eye", "/grow"] {
            assert!(!can_execute_command(&admin, Some("u1"), command));
        }
    }

    #[test]
    fn allows_operator_safe_remote_commands() {
        let operator = auth("operator");
        for command in [
            "/save",
            "/worldevent bloodmoon",
            "/give 29 Bob 1",
            "/heal Bob",
        ] {
            assert!(can_execute_command(&operator, Some("owner"), command));
        }
    }

    #[test]
    fn keeps_owner_scope_for_unknown_slash_commands() {
        let owner = auth("viewer");
        let other = auth("viewer");
        assert!(can_execute_command(&owner, Some("u1"), "/custom"));
        assert!(!can_execute_command(&other, Some("owner"), "/custom"));
    }
}
