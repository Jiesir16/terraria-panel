use crate::error::AppError;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInfo {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub file_size: u64,
    pub source_server_id: Option<String>,
    pub created_at: String,
}

pub struct SaveManager {
    saves_dir: PathBuf,
    servers_dir: PathBuf,
}

impl SaveManager {
    pub fn new(saves_dir: PathBuf, servers_dir: PathBuf) -> Self {
        Self {
            saves_dir,
            servers_dir,
        }
    }

    pub fn upload_save(&self, name: &str, data: &[u8]) -> Result<SaveInfo, AppError> {
        if !name.ends_with(".wld") {
            return Err(AppError::BadRequest(
                "Only .wld files are allowed".to_string(),
            ));
        }

        if data.len() > 536_870_912 {
            // 500MB limit
            return Err(AppError::BadRequest(
                "Save file too large (max 500MB)".to_string(),
            ));
        }

        if !self.saves_dir.exists() {
            std::fs::create_dir_all(&self.saves_dir)
                .map_err(|e| AppError::FileError(format!("Failed to create saves directory: {}", e)))?;
        }

        let save_id = Uuid::new_v4().to_string();
        let file_name = format!("{}_{}", save_id, name);
        let file_path = self.saves_dir.join(&file_name);

        std::fs::write(&file_path, data)
            .map_err(|e| AppError::FileError(format!("Failed to write save file: {}", e)))?;

        let metadata = std::fs::metadata(&file_path)
            .map_err(|e| AppError::FileError(format!("Failed to get file metadata: {}", e)))?;

        let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        Ok(SaveInfo {
            id: save_id,
            name: name.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            file_size: metadata.len(),
            source_server_id: None,
            created_at,
        })
    }

    pub fn import_save(
        &self,
        save_path: &str,
        server_id: &str,
    ) -> Result<(), AppError> {
        let save_file_path = PathBuf::from(save_path);

        if !save_file_path.exists() {
            return Err(AppError::NotFound(format!("Save file not found: {}", save_path)));
        }

        let world_dir = self
            .servers_dir
            .join(server_id)
            .join("world");

        if !world_dir.exists() {
            std::fs::create_dir_all(&world_dir)
                .map_err(|e| AppError::FileError(format!("Failed to create world directory: {}", e)))?;
        }

        if let Some(filename) = save_file_path.file_name() {
            let dest_path = world_dir.join(filename);
            std::fs::copy(&save_file_path, &dest_path)
                .map_err(|e| AppError::FileError(format!("Failed to copy save: {}", e)))?;
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "Invalid save file path".to_string(),
            ))
        }
    }

    pub fn backup_server(
        &self,
        server_id: &str,
        world_name: &str,
    ) -> Result<SaveInfo, AppError> {
        let world_file_path = self
            .servers_dir
            .join(server_id)
            .join("world")
            .join(format!("{}.wld", world_name));

        if !world_file_path.exists() {
            return Err(AppError::NotFound(format!(
                "World file not found: {}",
                world_file_path.display()
            )));
        }

        if !self.saves_dir.exists() {
            std::fs::create_dir_all(&self.saves_dir)
                .map_err(|e| AppError::FileError(format!("Failed to create saves directory: {}", e)))?;
        }

        let save_id = Uuid::new_v4().to_string();
        let backup_name = format!("{}_backup_{}.wld", world_name, Local::now().format("%Y%m%d_%H%M%S"));
        let backup_path = self.saves_dir.join(&backup_name);

        std::fs::copy(&world_file_path, &backup_path)
            .map_err(|e| AppError::FileError(format!("Failed to backup save: {}", e)))?;

        let metadata = std::fs::metadata(&backup_path)
            .map_err(|e| AppError::FileError(format!("Failed to get file metadata: {}", e)))?;

        let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        Ok(SaveInfo {
            id: save_id,
            name: backup_name,
            file_path: backup_path.to_string_lossy().to_string(),
            file_size: metadata.len(),
            source_server_id: Some(server_id.to_string()),
            created_at,
        })
    }

    #[allow(dead_code)]
    pub fn list_saves_from_dir(&self) -> Result<Vec<SaveInfo>, AppError> {
        let mut saves = Vec::new();

        if !self.saves_dir.exists() {
            return Ok(saves);
        }

        let entries = std::fs::read_dir(&self.saves_dir)
            .map_err(|e| AppError::FileError(format!("Failed to read saves directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::FileError(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "wld") {
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let metadata = std::fs::metadata(&path)
                    .map_err(|e| AppError::FileError(format!("Failed to get file metadata: {}", e)))?;

                let created_at = metadata
                    .modified()
                    .ok()
                    .and_then(|t| {
                        let duration = t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok()?;
                        chrono::DateTime::<Local>::from(std::time::SystemTime::UNIX_EPOCH + duration)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                            .into()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                let save_id = filename
                    .split('_')
                    .next()
                    .unwrap_or(&filename)
                    .to_string();

                let source_server_id = if filename.contains("_backup_") {
                    filename
                        .split('_')
                        .nth(1)
                        .map(|s| s.to_string())
                } else {
                    None
                };

                saves.push(SaveInfo {
                    id: save_id,
                    name: filename,
                    file_path: path.to_string_lossy().to_string(),
                    file_size: metadata.len(),
                    source_server_id,
                    created_at,
                });
            }
        }

        Ok(saves)
    }

    #[allow(dead_code)]
    pub fn delete_save(&self, file_path: &str) -> Result<(), AppError> {
        let path = PathBuf::from(file_path);

        if !path.exists() {
            return Err(AppError::NotFound(format!("Save file not found: {}", file_path)));
        }

        std::fs::remove_file(&path)
            .map_err(|e| AppError::FileError(format!("Failed to delete save: {}", e)))
    }

    #[allow(dead_code)]
    pub fn get_save_path(&self, save_id: &str) -> Option<PathBuf> {
        if !self.saves_dir.exists() {
            return None;
        }

        let entries = std::fs::read_dir(&self.saves_dir).ok()?;

        for entry in entries {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if filename_str.starts_with(save_id) && filename_str.ends_with(".wld") {
                        return Some(path);
                    }
                }
            }
        }

        None
    }
}

impl Default for SaveManager {
    fn default() -> Self {
        Self::new(
            PathBuf::from("./data/saves"),
            PathBuf::from("./data/servers"),
        )
    }
}
