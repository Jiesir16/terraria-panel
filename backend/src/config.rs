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
    #[serde(default = "default_backup_enabled")]
    pub enabled: bool,
    #[serde(default = "default_backup_interval_minutes")]
    pub interval_minutes: u64,
    #[serde(default = "default_max_backups_per_server")]
    pub max_backups_per_server: usize,
    #[serde(default = "default_local_retention_days")]
    pub local_retention_days: u64,
    #[serde(default = "default_archive_daily_enabled")]
    pub archive_daily_enabled: bool,
    #[serde(default = "default_archive_hour")]
    pub archive_hour: u8,
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: u64,
    #[serde(default)]
    pub oss: OssBackupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OssBackupConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_oss_provider")]
    pub provider: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub access_key_secret: String,
    #[serde(default)]
    pub local_path: String,
    #[serde(default = "default_oss_prefix")]
    pub prefix: String,
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

        let config_exists = std::path::Path::new(&path).exists();
        let mut config = if config_exists {
            Self::from_file(&path)?
        } else {
            Self::write_default_template(&path)?;
            tracing::info!(path = %path, "Default config.toml generated");
            Self::from_file(&path)?
        };

        // Canonicalize paths to absolute — avoids breakage when child
        // processes or different code paths resolve relative paths from
        // varying working directories.
        config.server.data_dir = Self::ensure_absolute(&config.server.data_dir);
        config.server.log_dir = Self::ensure_absolute(&config.server.log_dir);

        Ok(config)
    }

    fn write_default_template(path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, DEFAULT_CONFIG_TEMPLATE)?;
        Ok(())
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

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# Terraria Panel 后端配置
# 启动目录为 backend 时，默认读取当前目录下的 config.toml。
# 也可以用环境变量 TERRARIA_CONSOLE_CONFIG=/path/to/config.toml 指定配置文件。

[server]
# 后端监听地址。服务器部署一般用 0.0.0.0，本机调试可改成 127.0.0.1。
host = "0.0.0.0"

# 后端 HTTP API 端口。
port = 3000

# 面板数据目录，存放服务器实例、版本、世界存档、上传文件等。
data_dir = "./data"

# 后端日志目录。
log_dir = "./logs"

[auth]
# JWT 签名密钥。生产环境必须改成随机长字符串。
jwt_secret = "dev-secret-change-me-in-production"

# 登录 token 有效时间，单位小时。
jwt_expire_hours = 24

# 是否允许前端自助注册账号。公网环境建议保持 false，由管理员创建用户。
allow_register = false

[tshock]
# dotnet 可执行文件路径。TShock 6.x 通常需要 .NET runtime。
dotnet_path = "/usr/bin/dotnet"

# mono 可执行文件路径。旧版本 TShock/兼容场景可能会用到。
mono_path = "/usr/bin/mono"

# GitHub 下载镜像地址，留空表示直接访问 GitHub。
github_mirror = ""

# 自动分配服务器端口的起始端口。
default_port_range_start = 7777

# 自动分配服务器端口的结束端口。
default_port_range_end = 7800

[backup]
# 是否启用自动备份任务。
enabled = true

# 自动备份间隔，单位分钟。60 表示每小时生成一次世界存档备份。
interval_minutes = 60

# 每个服务器最多保留多少个未归档世界备份。
# 0 表示不按数量裁剪。启用每日归档时建议保持 0，否则小时备份可能在归档前被提前删除。
max_backups_per_server = 0

# 本地备份保留天数。超过该天数的本地备份/归档会被清理。
local_retention_days = 30

# 是否启用每日归档，把某一天的小时备份压缩成 zip。
archive_daily_enabled = true

# 每天几点执行归档，24 小时制。1 表示每天 01:00 执行。
archive_hour = 1

# 归档几天前的小时备份。
# 2 表示每天 01:00 归档“前天”的小时备份，归档成功后删除散文件，只保留 zip。
archive_after_days = 2

[backup.oss]
# 是否启用远端备份同步。本地备份永远会先生成，远端同步失败不会中断本地备份。
enabled = false

# 远端备份类型：
# nas/local：复制到本机或局域网 NAS 挂载目录。
# tencent_cos/cos：上传到腾讯云 COS。
provider = "nas"

# NAS 模式目标目录。先把 NAS 挂载到服务器，例如 /mnt/nas/terraria-panel-backups。
local_path = ""

# 腾讯云 COS Endpoint。可留空，程序会按 bucket + region 生成：
# https://<bucket>.cos.<region>.myqcloud.com
endpoint = ""

# 腾讯云 COS Bucket 名称，例如 my-bucket-1250000000。
bucket = ""

# 腾讯云 COS Region，例如 ap-guangzhou、ap-shanghai。
region = ""

# 腾讯云 SecretId。NAS 模式不需要。
access_key_id = ""

# 腾讯云 SecretKey。NAS 模式不需要。
access_key_secret = ""

# 远端对象前缀。NAS 模式会复制到 local_path/prefix/...；COS 模式会上传到 prefix/...。
prefix = "terraria-panel/saves"
"#;

fn default_backup_enabled() -> bool {
    true
}

fn default_backup_interval_minutes() -> u64 {
    60
}

fn default_max_backups_per_server() -> usize {
    0
}

fn default_local_retention_days() -> u64 {
    30
}

fn default_archive_daily_enabled() -> bool {
    true
}

fn default_archive_hour() -> u8 {
    1
}

fn default_archive_after_days() -> u64 {
    2
}

fn default_oss_provider() -> String {
    "nas".to_string()
}

fn default_oss_prefix() -> String {
    "terraria-panel/saves".to_string()
}

impl Default for OssBackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_oss_provider(),
            endpoint: String::new(),
            bucket: String::new(),
            region: String::new(),
            access_key_id: String::new(),
            access_key_secret: String::new(),
            local_path: String::new(),
            prefix: default_oss_prefix(),
        }
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
                interval_minutes: 60,
                max_backups_per_server: 0,
                local_retention_days: 30,
                archive_daily_enabled: true,
                archive_hour: 1,
                archive_after_days: 2,
                oss: OssBackupConfig::default(),
            },
        }
    }
}
