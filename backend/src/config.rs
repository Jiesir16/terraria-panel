use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub tshock: TShockConfig,
    pub backup: BackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub frontend_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expire_hours: u64,
    pub allow_register: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TShockConfig {
    pub dotnet_path: String,
    pub mono_path: String,
    pub github_mirror: String,
    pub default_port_range_start: u16,
    pub default_port_range_end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub interval_minutes: u64,
    pub max_backups_per_server: usize,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn from_env_or_default() -> Result<Self, Box<dyn std::error::Error>> {
        let path =
            std::env::var("TERRARIA_CONSOLE_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

        let mut config = if std::path::Path::new(&path).exists() {
            Self::from_file(&path)?
        } else {
            Self::default()
        };

        // Canonicalize paths to absolute — avoids breakage when child
        // processes or different code paths resolve relative paths from
        // varying working directories.
        config.server.data_dir = Self::ensure_absolute(&config.server.data_dir);
        config.server.log_dir = Self::ensure_absolute(&config.server.log_dir);
        config.server.frontend_dir = Self::ensure_absolute(&config.server.frontend_dir);

        Ok(config)
    }

    /// Convert a potentially relative path to absolute based on the current
    /// working directory.  Creates the directory first so `canonicalize` works.
    fn ensure_absolute(p: &PathBuf) -> PathBuf {
        if p.is_absolute() {
            return p.clone();
        }
        let _ = std::fs::create_dir_all(p);
        std::fs::canonicalize(p).unwrap_or_else(|_| {
            // fallback: manually prepend cwd
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(p)
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                data_dir: PathBuf::from("./data"),
                log_dir: PathBuf::from("./logs"),
                frontend_dir: PathBuf::from("./frontend/dist"),
            },
            auth: AuthConfig {
                jwt_secret: "dev-secret-change-me-in-production".to_string(),
                jwt_expire_hours: 24,
                allow_register: false,
            },
            tshock: TShockConfig {
                dotnet_path: "/usr/bin/dotnet".to_string(),
                mono_path: "/usr/bin/mono".to_string(),
                github_mirror: String::new(),
                default_port_range_start: 7777,
                default_port_range_end: 7800,
            },
            backup: BackupConfig {
                enabled: true,
                interval_minutes: 30,
                max_backups_per_server: 10,
            },
        }
    }
}
