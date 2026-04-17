use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub tag_name: String,
    pub download_url: String,
    pub published_at: String,
    pub size: u64,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVersion {
    pub version: String,
    pub path: String,
    pub size: u64,
    pub is_dotnet: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: Option<String>,
    published_at: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub struct VersionManager {
    versions_dir: PathBuf,
    github_mirror: String,
}

impl VersionManager {
    pub fn new(versions_dir: PathBuf, github_mirror: String) -> Self {
        Self {
            versions_dir,
            github_mirror,
        }
    }

    pub fn list_local(&self) -> Result<Vec<LocalVersion>, AppError> {
        let mut versions = Vec::new();

        if !self.versions_dir.exists() {
            return Ok(versions);
        }

        let entries = std::fs::read_dir(&self.versions_dir)
            .map_err(|e| AppError::FileError(format!("Failed to read versions directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::FileError(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    let dir_name_str = dir_name.to_string_lossy().to_string();
                    let size = self.get_dir_size(&path)
                        .unwrap_or(0);
                    let is_dotnet = self.is_dotnet_version(&path);

                    versions.push(LocalVersion {
                        version: dir_name_str,
                        path: path.to_string_lossy().to_string(),
                        size,
                        is_dotnet,
                    });
                }
            }
        }

        Ok(versions)
    }

    pub async fn fetch_available(&self) -> Result<Vec<VersionInfo>, AppError> {
        let api_url = "https://api.github.com/repos/Pryaxis/TShock/releases";

        let client = reqwest::Client::new();
        let response = client
            .get(api_url)
            .header("User-Agent", "terraria-console")
            .send()
            .await
            .map_err(|e| AppError::ProcessError(format!("Failed to fetch releases: {}", e)))?;

        let releases: Vec<GitHubRelease> = response
            .json()
            .await
            .map_err(|e| AppError::ProcessError(format!("Failed to parse releases: {}", e)))?;

        let local_versions = self.list_local().unwrap_or_default();
        let local_version_set: std::collections::HashSet<String> =
            local_versions.iter().map(|v| v.version.clone()).collect();

        let mut versions = Vec::new();
        for release in releases {
            for asset in release.assets.iter() {
                if asset.name.ends_with(".zip") {
                    let mut download_url = asset.browser_download_url.clone();

                    if !self.github_mirror.is_empty() {
                        download_url = download_url.replace("https://github.com", &self.github_mirror);
                    }

                    versions.push(VersionInfo {
                        version: release.tag_name.clone(),
                        tag_name: release.tag_name.clone(),
                        download_url,
                        published_at: release.published_at.clone(),
                        size: asset.size,
                        downloaded: local_version_set.contains(&release.tag_name),
                    });
                }
            }
        }

        Ok(versions)
    }

    pub async fn download_version(
        &self,
        tag_name: &str,
        download_url: &str,
    ) -> Result<PathBuf, AppError> {
        let version_dir = self.versions_dir.join(tag_name);

        if version_dir.exists() {
            return Ok(version_dir);
        }

        std::fs::create_dir_all(&version_dir)
            .map_err(|e| AppError::FileError(format!("Failed to create version directory: {}", e)))?;

        let client = reqwest::Client::new();
        let response = client
            .get(download_url)
            .header("User-Agent", "terraria-console")
            .send()
            .await
            .map_err(|e| AppError::ProcessError(format!("Failed to download: {}", e)))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::ProcessError(format!("Failed to read response: {}", e)))?;

        let zip_path = version_dir.join("release.zip");

        tokio::fs::write(&zip_path, bytes)
            .await
            .map_err(|e| AppError::FileError(format!("Failed to write zip file: {}", e)))?;

        self.extract_zip(&zip_path, &version_dir)?;

        std::fs::remove_file(&zip_path)
            .map_err(|e| AppError::FileError(format!("Failed to delete zip: {}", e)))?;

        Ok(version_dir)
    }

    fn extract_zip(&self, zip_path: &Path, extract_to: &Path) -> Result<(), AppError> {
        let file = std::fs::File::open(zip_path)
            .map_err(|e| AppError::FileError(format!("Failed to open zip: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::FileError(format!("Failed to read zip: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::FileError(format!("Failed to get file from zip: {}", e)))?;

            let output_path = extract_to.join(file.name());

            if file.is_dir() {
                std::fs::create_dir_all(&output_path)
                    .map_err(|e| AppError::FileError(format!("Failed to create directory: {}", e)))?;
            } else {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| AppError::FileError(format!("Failed to create parent dir: {}", e)))?;
                }

                let mut output_file = std::fs::File::create(&output_path)
                    .map_err(|e| AppError::FileError(format!("Failed to create file: {}", e)))?;

                std::io::copy(&mut file, &mut output_file)
                    .map_err(|e| AppError::FileError(format!("Failed to write file: {}", e)))?;
            }
        }

        Ok(())
    }

    pub fn delete_version(&self, version: &str) -> Result<(), AppError> {
        let version_dir = self.versions_dir.join(version);

        if !version_dir.exists() {
            return Err(AppError::NotFound(format!("Version {} not found", version)));
        }

        std::fs::remove_dir_all(&version_dir)
            .map_err(|e| AppError::FileError(format!("Failed to delete version: {}", e)))
    }

    pub fn get_version_path(&self, version: &str) -> Option<PathBuf> {
        let path = self.versions_dir.join(version);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn is_dotnet_version(&self, version_path: &Path) -> bool {
        let dll_path = version_path.join("TShock.Server.dll");
        dll_path.exists()
    }

    fn get_dir_size(&self, path: &Path) -> Result<u64, AppError> {
        let mut total_size = 0u64;

        if !path.is_dir() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(path)
            .map_err(|e| AppError::FileError(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::FileError(e.to_string()))?;
            let file_type = entry.file_type()
                .map_err(|e| AppError::FileError(e.to_string()))?;

            if file_type.is_file() {
                let metadata = entry.metadata()
                    .map_err(|e| AppError::FileError(e.to_string()))?;
                total_size += metadata.len();
            } else if file_type.is_dir() {
                total_size += self.get_dir_size(&entry.path())?;
            }
        }

        Ok(total_size)
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new(PathBuf::from("./data/versions"), String::new())
    }
}
