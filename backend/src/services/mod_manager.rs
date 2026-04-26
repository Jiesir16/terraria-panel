use crate::error::AppError;
use crate::models::ModInfo;
use chrono::Local;
use std::path::PathBuf;

pub struct ModManager {
    data_dir: PathBuf,
}

impl ModManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn list_mods(&self, server_id: &str) -> Result<crate::models::ModList, AppError> {
        let plugins_path = self.get_server_plugins_path(server_id)?;

        let mut mods = Vec::new();

        if !plugins_path.exists() {
            return Ok(crate::models::ModList { mods, total: 0 });
        }

        let entries = std::fs::read_dir(&plugins_path)
            .map_err(|e| AppError::FileError(format!("Failed to read plugins directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::FileError(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();

            if path.is_file() {
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let is_disabled = filename.ends_with(".disabled");
                let display_name = if is_disabled {
                    filename.trim_end_matches(".disabled").to_string()
                } else {
                    filename.clone()
                };

                let metadata = std::fs::metadata(&path).map_err(|e| {
                    AppError::FileError(format!("Failed to get file metadata: {}", e))
                })?;

                let created_at = metadata
                    .modified()
                    .ok()
                    .and_then(|t| {
                        let duration = t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok()?;
                        chrono::DateTime::<Local>::from(
                            std::time::SystemTime::UNIX_EPOCH + duration,
                        )
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .into()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                mods.push(ModInfo {
                    name: display_name,
                    filename: filename,
                    size: metadata.len(),
                    enabled: !is_disabled,
                    created_at,
                });
            }
        }

        let total = mods.len();
        Ok(crate::models::ModList { mods, total })
    }

    pub fn upload_mod(
        &self,
        server_id: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<ModInfo, AppError> {
        let plugins_path = self.get_server_plugins_path(server_id)?;

        if !filename.ends_with(".dll") {
            return Err(AppError::BadRequest(
                "Only .dll files are allowed".to_string(),
            ));
        }

        if data.len() > 52_428_800 {
            // 50MB limit
            return Err(AppError::BadRequest(
                "Mod file too large (max 50MB)".to_string(),
            ));
        }

        let file_path = plugins_path.join(filename);

        std::fs::write(&file_path, data)
            .map_err(|e| AppError::FileError(format!("Failed to write mod file: {}", e)))?;

        let metadata = std::fs::metadata(&file_path)
            .map_err(|e| AppError::FileError(format!("Failed to get file metadata: {}", e)))?;

        let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        Ok(ModInfo {
            name: filename.to_string(),
            filename: filename.to_string(),
            size: metadata.len(),
            enabled: true,
            created_at,
        })
    }

    pub fn toggle_mod(&self, server_id: &str, mod_name: &str) -> Result<ModInfo, AppError> {
        let plugins_path = self.get_server_plugins_path(server_id)?;

        let file_path = plugins_path.join(mod_name);

        if !file_path.exists() {
            return Err(AppError::NotFound(format!("Mod {} not found", mod_name)));
        }

        let is_disabled = mod_name.ends_with(".disabled");

        let new_filename = if is_disabled {
            mod_name.trim_end_matches(".disabled").to_string()
        } else {
            format!("{}.disabled", mod_name)
        };

        let new_path = plugins_path.join(&new_filename);

        std::fs::rename(&file_path, &new_path)
            .map_err(|e| AppError::FileError(format!("Failed to toggle mod: {}", e)))?;

        let metadata = std::fs::metadata(&new_path)
            .map_err(|e| AppError::FileError(format!("Failed to get file metadata: {}", e)))?;

        let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        Ok(ModInfo {
            name: if is_disabled {
                new_filename.clone()
            } else {
                mod_name.trim_end_matches(".disabled").to_string()
            },
            filename: new_filename,
            size: metadata.len(),
            enabled: !is_disabled,
            created_at,
        })
    }

    pub fn delete_mod(&self, server_id: &str, mod_name: &str) -> Result<(), AppError> {
        let plugins_path = self.get_server_plugins_path(server_id)?;

        let file_path = plugins_path.join(mod_name);

        if !file_path.exists() {
            return Err(AppError::NotFound(format!("Mod {} not found", mod_name)));
        }

        std::fs::remove_file(&file_path)
            .map_err(|e| AppError::FileError(format!("Failed to delete mod: {}", e)))
    }

    pub fn get_server_plugins_path(&self, server_id: &str) -> Result<PathBuf, AppError> {
        let plugins_path = self
            .data_dir
            .join("servers")
            .join(server_id)
            .join("ServerPlugins");

        if !plugins_path.exists() {
            std::fs::create_dir_all(&plugins_path).map_err(|e| {
                AppError::FileError(format!("Failed to create plugins directory: {}", e))
            })?;
        }

        Ok(plugins_path)
    }
}

impl Default for ModManager {
    fn default() -> Self {
        Self::new(PathBuf::from("./data"))
    }
}
