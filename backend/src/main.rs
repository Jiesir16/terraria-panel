mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod models;
mod services;
mod websocket;

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
    Extension, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use handlers::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting Terraria Console Backend");

    // Load configuration
    let config = config::Config::from_env_or_default()?;
    tracing::info!(
        "Configuration loaded: host={}, port={}",
        config.server.host,
        config.server.port
    );

    // Create data directories
    std::fs::create_dir_all(&config.server.data_dir)?;
    std::fs::create_dir_all(&config.server.log_dir)?;
    std::fs::create_dir_all(config.server.data_dir.join("db"))?;
    std::fs::create_dir_all(config.server.data_dir.join("servers"))?;
    std::fs::create_dir_all(config.server.data_dir.join("versions"))?;
    std::fs::create_dir_all(config.server.data_dir.join("saves"))?;
    std::fs::create_dir_all(config.server.data_dir.join("uploads"))?;

    // Initialize database
    let db = db::create_db(
        &config
            .server
            .data_dir
            .join("db")
            .join("terraria-console.db"),
    )?;
    tracing::info!("Database initialized");

    // Initialize services
    let token_manager = Arc::new(auth::TokenManager::new(
        config.auth.jwt_secret.clone(),
        config.auth.jwt_expire_hours,
    ));

    let process_manager = Arc::new(services::ProcessManager::new());
    let version_manager = Arc::new(services::VersionManager::new(
        config.server.data_dir.join("versions"),
        config.tshock.github_mirror.clone(),
    ));
    let mod_manager = Arc::new(services::ModManager::new(config.server.data_dir.clone()));
    let save_manager = Arc::new(services::SaveManager::new(
        config.server.data_dir.join("saves"),
        config.server.data_dir.join("servers"),
    ));
    let system_monitor = Arc::new(tokio::sync::Mutex::new(services::SystemMonitor::new()));

    // Create application state
    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        token_manager: token_manager.clone(),
        process_manager,
        version_manager,
        mod_manager,
        save_manager,
        system_monitor,
    };

    // Build router
    let app = Router::new()
        // Auth endpoints
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/password", put(handlers::auth::change_password))
        .route("/api/auth/me", get(handlers::auth::me))
        // Server list + create
        .route(
            "/api/servers",
            get(handlers::server::list_servers).post(handlers::server::create_server),
        )
        // Server detail + update + delete
        .route(
            "/api/servers/:id",
            get(handlers::server::get_server)
                .put(handlers::server::update_server)
                .delete(handlers::server::delete_server),
        )
        .route(
            "/api/servers/:id/start",
            post(handlers::server::start_server),
        )
        .route("/api/servers/:id/stop", post(handlers::server::stop_server))
        .route("/api/servers/:id/kill", post(handlers::server::kill_server))
        .route(
            "/api/servers/:id/restart",
            post(handlers::server::restart_server),
        )
        .route(
            "/api/servers/:id/command",
            post(handlers::server::send_command),
        )
        .route(
            "/api/servers/:id/status",
            get(handlers::server::server_status),
        )
        .route(
            "/api/servers/:id/worlds",
            get(handlers::server::list_worlds),
        )
        .route(
            "/api/servers/:id/items",
            get(handlers::tshock_rest::rest_item_list),
        )
        .route(
            "/api/servers/:id/items/sync",
            post(handlers::tshock_rest::rest_item_sync),
        )
        .route(
            "/api/servers/:id/tshock-security",
            get(handlers::server::tshock_security_overview),
        )
        .route("/api/servers/:id/logs", get(handlers::console::recent_logs))
        .route(
            "/api/servers/:id/console",
            get(handlers::console::ws_console),
        )
        // Version endpoints
        .route("/api/versions", get(handlers::version::list_versions))
        .route(
            "/api/versions/available",
            get(handlers::version::available_versions),
        )
        .route(
            "/api/versions/download",
            post(handlers::version::download_version),
        )
        .route(
            "/api/versions/proxy",
            get(handlers::version::get_proxy).put(handlers::version::set_proxy),
        )
        .route(
            "/api/versions/:version",
            delete(handlers::version::delete_version),
        )
        // Mod endpoints
        .route(
            "/api/servers/:id/mods",
            get(handlers::mods::list_mods).post(handlers::mods::upload_mod),
        )
        .route(
            "/api/servers/:id/mods/:name/toggle",
            put(handlers::mods::toggle_mod),
        )
        .route(
            "/api/servers/:id/mods/:name",
            delete(handlers::mods::delete_mod),
        )
        // Save endpoints
        .route("/api/saves", get(handlers::saves::list_saves))
        .route("/api/saves/upload", post(handlers::saves::upload_save))
        .route(
            "/api/saves/:id/import/:server_id",
            post(handlers::saves::import_save),
        )
        .route(
            "/api/saves/:id/download",
            get(handlers::saves::download_save),
        )
        .route("/api/saves/:id", delete(handlers::saves::delete_save))
        .route(
            "/api/servers/:id/backup",
            post(handlers::saves::backup_server),
        )
        // TShock user/group/permission management
        .route(
            "/api/servers/:id/tshock-users/:username/group",
            put(handlers::tshock::update_user_group),
        )
        .route(
            "/api/servers/:id/tshock-users/:username",
            delete(handlers::tshock::delete_user),
        )
        .route(
            "/api/servers/:id/tshock-groups",
            post(handlers::tshock::create_group),
        )
        .route(
            "/api/servers/:id/tshock-groups/:name",
            get(handlers::tshock::get_group)
                .delete(handlers::tshock::delete_group),
        )
        .route(
            "/api/servers/:id/tshock-groups/:name/permissions",
            post(handlers::tshock::add_permission),
        )
        .route(
            "/api/servers/:id/tshock-groups/:name/permissions/remove",
            post(handlers::tshock::remove_permission),
        )
        // SSC character management
        .route(
            "/api/servers/:id/ssc-characters",
            get(handlers::tshock::list_ssc_characters),
        )
        .route(
            "/api/servers/:id/ssc-characters/backup",
            post(handlers::tshock::backup_ssc_characters),
        )
        .route(
            "/api/servers/:id/ssc-characters/:account_id",
            get(handlers::tshock::export_ssc_character)
                .put(handlers::tshock::update_ssc_character)
                .delete(handlers::tshock::delete_ssc_character),
        )
        // TShock REST API setup + proxy endpoints
        .route(
            "/api/servers/:id/rest/setup",
            post(handlers::tshock_rest::rest_setup),
        )
        .route(
            "/api/servers/:id/rest/tokentest",
            get(handlers::tshock_rest::rest_token_test),
        )
        .route(
            "/api/servers/:id/rest/server/status",
            get(handlers::tshock_rest::rest_server_status),
        )
        .route(
            "/api/servers/:id/rest/server/broadcast",
            post(handlers::tshock_rest::rest_server_broadcast),
        )
        .route(
            "/api/servers/:id/rest/server/reload",
            post(handlers::tshock_rest::rest_server_reload),
        )
        .route(
            "/api/servers/:id/rest/server/restart",
            post(handlers::tshock_rest::rest_server_restart),
        )
        .route(
            "/api/servers/:id/rest/server/rawcmd",
            post(handlers::tshock_rest::rest_server_rawcmd),
        )
        .route(
            "/api/servers/:id/rest/server/off",
            post(handlers::tshock_rest::rest_server_off),
        )
        .route(
            "/api/servers/:id/rest/items",
            get(handlers::tshock_rest::rest_item_list),
        )
        .route(
            "/api/servers/:id/rest/items/sync",
            post(handlers::tshock_rest::rest_item_sync),
        )
        .route(
            "/api/servers/:id/rest/items/give",
            post(handlers::tshock_rest::rest_item_give),
        )
        .route(
            "/api/servers/:id/rest/server/motd",
            get(handlers::tshock_rest::rest_server_motd),
        )
        .route(
            "/api/servers/:id/rest/server/rules",
            get(handlers::tshock_rest::rest_server_rules),
        )
        // REST: Players
        .route(
            "/api/servers/:id/rest/players/list",
            get(handlers::tshock_rest::rest_player_list),
        )
        .route(
            "/api/servers/:id/rest/players/:player",
            get(handlers::tshock_rest::rest_player_read),
        )
        .route(
            "/api/servers/:id/rest/players/kick",
            post(handlers::tshock_rest::rest_player_kick),
        )
        .route(
            "/api/servers/:id/rest/players/ban",
            post(handlers::tshock_rest::rest_player_ban),
        )
        .route(
            "/api/servers/:id/rest/players/kill",
            post(handlers::tshock_rest::rest_player_kill),
        )
        .route(
            "/api/servers/:id/rest/players/mute",
            post(handlers::tshock_rest::rest_player_mute),
        )
        .route(
            "/api/servers/:id/rest/players/unmute",
            post(handlers::tshock_rest::rest_player_unmute),
        )
        // REST: Users
        .route(
            "/api/servers/:id/rest/users/list",
            get(handlers::tshock_rest::rest_user_list),
        )
        .route(
            "/api/servers/:id/rest/users/activelist",
            get(handlers::tshock_rest::rest_user_active_list),
        )
        .route(
            "/api/servers/:id/rest/users/:user",
            get(handlers::tshock_rest::rest_user_read)
                .delete(handlers::tshock_rest::rest_user_destroy),
        )
        .route(
            "/api/servers/:id/rest/users/create",
            post(handlers::tshock_rest::rest_user_create),
        )
        .route(
            "/api/servers/:id/rest/users/update",
            post(handlers::tshock_rest::rest_user_update),
        )
        // REST: Groups
        .route(
            "/api/servers/:id/rest/groups/list",
            get(handlers::tshock_rest::rest_group_list),
        )
        .route(
            "/api/servers/:id/rest/groups/:name",
            get(handlers::tshock_rest::rest_group_read)
                .put(handlers::tshock_rest::rest_group_update)
                .delete(handlers::tshock_rest::rest_group_destroy),
        )
        .route(
            "/api/servers/:id/rest/groups/create",
            post(handlers::tshock_rest::rest_group_create),
        )
        // REST: Bans
        .route(
            "/api/servers/:id/rest/bans/list",
            get(handlers::tshock_rest::rest_ban_list),
        )
        .route(
            "/api/servers/:id/rest/bans/:ticket",
            get(handlers::tshock_rest::rest_ban_read)
                .delete(handlers::tshock_rest::rest_ban_destroy),
        )
        .route(
            "/api/servers/:id/rest/bans/create",
            post(handlers::tshock_rest::rest_ban_create),
        )
        // REST: World
        .route(
            "/api/servers/:id/rest/world/read",
            get(handlers::tshock_rest::rest_world_read),
        )
        .route(
            "/api/servers/:id/rest/world/save",
            post(handlers::tshock_rest::rest_world_save),
        )
        .route(
            "/api/servers/:id/rest/world/butcher",
            post(handlers::tshock_rest::rest_world_butcher),
        )
        .route(
            "/api/servers/:id/rest/world/bloodmoon",
            post(handlers::tshock_rest::rest_world_bloodmoon),
        )
        .route(
            "/api/servers/:id/rest/world/meteor",
            post(handlers::tshock_rest::rest_world_meteor),
        )
        .route(
            "/api/servers/:id/rest/world/autosave",
            post(handlers::tshock_rest::rest_world_autosave),
        )
        // Config endpoints
        .route(
            "/api/servers/:id/config",
            get(handlers::config::get_config).put(handlers::config::update_config),
        )
        .route(
            "/api/servers/:id/ssc-config",
            get(handlers::config::get_ssc_config).put(handlers::config::update_ssc_config),
        )
        .route(
            "/api/config/templates",
            get(handlers::config::list_templates),
        )
        .route(
            "/api/servers/:id/config/import",
            post(handlers::config::import_config),
        )
        .route(
            "/api/servers/:id/config/export",
            get(handlers::config::export_config),
        )
        // System endpoints
        .route("/api/system/info", get(handlers::system::system_info))
        .route("/api/system/logs", get(handlers::system::list_logs))
        .route(
            "/api/users",
            get(handlers::system::list_users).post(handlers::system::create_user),
        )
        .route(
            "/api/users/:id",
            put(handlers::system::update_user).delete(handlers::system::delete_user),
        )
        .layer(Extension(token_manager))
        .layer(CorsLayer::permissive())
        .layer(DefaultBodyLimit::max(512 * 1024 * 1024)) // 512MB for save/mod uploads
        .with_state(state);

    // Bind and serve
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.server.host, config.server.port))
            .await?;

    tracing::info!(
        "Server listening on {}:{}",
        config.server.host,
        config.server.port
    );

    axum::serve(listener, app).await?;

    Ok(())
}
