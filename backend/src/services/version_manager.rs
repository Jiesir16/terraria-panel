use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub tag_name: String,
    pub name: String,
    pub download_url: String,
    pub published_at: String,
    pub size: u64,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVersion {
    pub version: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dotnet: bool,
    pub installed_at: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableVersionsResponse {
    pub versions: Vec<VersionInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub has_more: bool,
}

pub struct VersionManager {
    versions_dir: PathBuf,
    github_mirror: RwLock<String>,
}

impl VersionManager {
    pub fn new(versions_dir: PathBuf, github_mirror: String) -> Self {
        Self {
            versions_dir,
            github_mirror: RwLock::new(github_mirror),
        }
    }

    pub async fn get_github_mirror(&self) -> String {
        self.github_mirror.read().await.clone()
    }

    pub async fn set_github_mirror(&self, mirror: String) {
        let mut m = self.github_mirror.write().await;
        *m = mirror;
        tracing::info!("GitHub mirror updated");
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

                    // Get directory creation time as installed_at
                    let installed_at = std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let datetime: chrono::DateTime<chrono::Utc> = t.into();
                            datetime.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_default();

                    // Generate a display name from the tag
                    let name = format!("TShock {}", dir_name_str.trim_start_matches('v'));

                    versions.push(LocalVersion {
                        version: dir_name_str,
                        name,
                        path: path.to_string_lossy().to_string(),
                        size,
                        is_dotnet,
                        installed_at,
                    });
                }
            }
        }

        // Sort by version descending
        versions.sort_by(|a, b| b.version.cmp(&a.version));

        Ok(versions)
    }

    pub async fn fetch_available(&self, page: usize, per_page: usize) -> Result<AvailableVersionsResponse, AppError> {
        let github_mirror = self.github_mirror.read().await.clone();

        // GitHub API supports per_page (max 100) and page params
        let api_per_page = 100; // Fetch more from GitHub, paginate locally
        let api_url = format!(
            "https://api.github.com/repos/Pryaxis/TShock/releases?per_page={}&page=1",
            api_per_page
        );

        tracing::info!(url = %api_url, "Fetching TShock releases from GitHub API");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::ProcessError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(&api_url)
            .header("User-Agent", "terraria-console")
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "HTTP request to GitHub API failed");
                AppError::ProcessError(format!("无法连接 GitHub API，请检查网络或配置代理: {}", e))
            })?;

        tracing::debug!(status = %response.status(), "GitHub API response received");

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "GitHub API returned error");
            return Err(AppError::ProcessError(format!(
                "GitHub API 返回错误 ({}): {}",
                status, body
            )));
        }

        let releases: Vec<GitHubRelease> = response
            .json()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to parse GitHub API response JSON");
                AppError::ProcessError(format!("Failed to parse releases: {}", e))
            })?;

        tracing::info!(releases_count = releases.len(), "Parsed GitHub releases");

        let local_versions = self.list_local().unwrap_or_default();
        let local_version_set: std::collections::HashSet<String> =
            local_versions.iter().map(|v| v.version.clone()).collect();

        let mut all_versions = Vec::new();
        for release in releases {
            // Find the Linux zip asset (prefer linux, fallback to any zip)
            let zip_asset = release.assets.iter().find(|a| {
                a.name.ends_with(".zip")
            });

            if let Some(asset) = zip_asset {
                let mut download_url = asset.browser_download_url.clone();

                if !github_mirror.is_empty() {
                    download_url = Self::apply_mirror(&download_url, &github_mirror);
                }

                // Build a readable display name
                let display_name = release
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("TShock {}", release.tag_name.trim_start_matches('v')));

                all_versions.push(VersionInfo {
                    version: release.tag_name.clone(),
                    tag_name: release.tag_name.clone(),
                    name: display_name,
                    download_url,
                    published_at: release.published_at.clone(),
                    size: asset.size,
                    downloaded: local_version_set.contains(&release.tag_name),
                });
            }
        }

        let total = all_versions.len();
        let start = (page - 1) * per_page;
        let has_more = start + per_page < total;

        let versions = if start < total {
            all_versions[start..std::cmp::min(start + per_page, total)].to_vec()
        } else {
            Vec::new()
        };

        Ok(AvailableVersionsResponse {
            versions,
            total,
            page,
            per_page,
            has_more,
        })
    }

    /// Apply a mirror/proxy prefix to a GitHub download URL.
    /// Supports common mirror patterns:
    /// - `https://ghproxy.com/` style: prepend to the full URL
    /// - `https://mirror.example.com` style: replace `https://github.com`
    fn apply_mirror(url: &str, mirror: &str) -> String {
        let mirror = mirror.trim().trim_end_matches('/');
        if mirror.is_empty() {
            return url.to_string();
        }

        // If mirror looks like a full proxy (e.g. ghproxy.com, gh-proxy.com),
        // prepend it to the URL
        if mirror.contains("ghproxy") || mirror.contains("gh-proxy") || mirror.contains("mirror.ghproxy") {
            format!("{}/{}", mirror, url)
        } else {
            // Otherwise replace the github.com domain
            url.replace("https://github.com", mirror)
        }
    }

    pub async fn download_version(
        &self,
        tag_name: &str,
        download_url: &str,
    ) -> Result<PathBuf, AppError> {
        let version_dir = self.versions_dir.join(tag_name);

        if version_dir.exists() {
            tracing::info!(version = %tag_name, "Version already downloaded, skipping");
            return Ok(version_dir);
        }

        // Apply mirror to the download URL if configured
        let github_mirror = self.github_mirror.read().await.clone();
        let actual_url = if !github_mirror.is_empty() {
            Self::apply_mirror(download_url, &github_mirror)
        } else {
            download_url.to_string()
        };

        tracing::info!(version = %tag_name, url = %actual_url, "Starting version download");

        std::fs::create_dir_all(&version_dir)
            .map_err(|e| AppError::FileError(format!("Failed to create version directory: {}", e)))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| AppError::ProcessError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(&actual_url)
            .header("User-Agent", "terraria-console")
            .send()
            .await
            .map_err(|e| {
                // Clean up empty dir on failure
                let _ = std::fs::remove_dir_all(&version_dir);
                tracing::error!(version = %tag_name, error = %e, "Download HTTP request failed");
                AppError::ProcessError(format!("下载失败，请检查网络或代理设置: {}", e))
            })?;

        tracing::debug!(version = %tag_name, status = %response.status(), "Download response received");

        let bytes = response
            .bytes()
            .await
            .map_err(|e| {
                tracing::error!(version = %tag_name, error = %e, "Failed to read download response body");
                AppError::ProcessError(format!("Failed to read response: {}", e))
            })?;

        tracing::info!(version = %tag_name, size = bytes.len(), "Download complete, writing to disk");

        let zip_path = version_dir.join("release.zip");

        tokio::fs::write(&zip_path, bytes)
            .await
            .map_err(|e| AppError::FileError(format!("Failed to write zip file: {}", e)))?;

        tracing::info!(version = %tag_name, "Extracting zip archive");
        self.extract_zip(&zip_path, &version_dir)?;

        std::fs::remove_file(&zip_path)
            .map_err(|e| AppError::FileError(format!("Failed to delete zip: {}", e)))?;

        tracing::info!(version = %tag_name, path = %version_dir.display(), "Version downloaded and extracted successfully");

        Ok(version_dir)
    }

    fn extract_zip(&self, zip_path: &Path, extract_to: &Path) -> Result<(), AppError> {
        let file = std::fs::File::open(zip_path)
            .map_err(|e| AppError::FileError(format!("Failed to open zip: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::FileError(format!("Failed to read zip: {}", e)))?;

        // Detect common top-level prefix to flatten nested zips.
        // TShock releases often have all files under a single directory like
        // "TShock-5-2-0-Beta/..." — we strip that prefix so TShock.Server.dll
        // ends up directly in extract_to.
        let prefix_to_strip = Self::detect_common_prefix(&mut archive);

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::FileError(format!("Failed to get file from zip: {}", e)))?;

            let raw_name = file.name().to_string();

            // Strip common prefix if detected
            let relative = if let Some(ref prefix) = prefix_to_strip {
                raw_name.strip_prefix(prefix).unwrap_or(&raw_name)
            } else {
                &raw_name
            };

            // Skip empty paths (the prefix directory itself)
            if relative.is_empty() || relative == "/" {
                continue;
            }

            let output_path = extract_to.join(relative);

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

    /// If every entry in the zip shares a single top-level directory prefix,
    /// return it (with trailing `/`).  This lets us flatten archives like
    /// `TShock-Beta-v2024.05.31.0/TShock.Server.dll` → `TShock.Server.dll`.
    fn detect_common_prefix(archive: &mut zip::ZipArchive<std::fs::File>) -> Option<String> {
        let mut common: Option<String> = None;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                // Extract first path component
                let first = match name.find('/') {
                    Some(idx) => &name[..=idx], // e.g. "TShock-Beta/"
                    None => return None,        // top-level file → no prefix to strip
                };
                match &common {
                    None => common = Some(first.to_string()),
                    Some(existing) => {
                        if existing != first {
                            return None; // multiple top-level entries
                        }
                    }
                }
            }
        }
        // Only strip if the prefix is a directory (ends with /)
        common.filter(|p| p.ends_with('/'))
    }

    pub fn delete_version(&self, version: &str) -> Result<(), AppError> {
        let version_dir = self.versions_dir.join(version);

        if !version_dir.exists() {
            tracing::warn!(version = %version, "Cannot delete: version not found");
            return Err(AppError::NotFound(format!("Version {} not found", version)));
        }

        tracing::info!(version = %version, path = %version_dir.display(), "Deleting version directory");
        std::fs::remove_dir_all(&version_dir)
            .map_err(|e| AppError::FileError(format!("Failed to delete version: {}", e)))?;

        tracing::info!(version = %version, "Version deleted successfully");
        Ok(())
    }

    pub fn get_version_path(&self, version: &str) -> Option<PathBuf> {
        let path = self.versions_dir.join(version);
        if !path.exists() {
            return None;
        }
        // Return the directory containing TShock.Server.dll (recursive search up to depth 5)
        if let Some(dll_dir) = Self::find_dll_dir(&path, 5) {
            return Some(dll_dir);
        }
        // Fallback: return the base path (for Mono-based or unknown layouts)
        tracing::warn!(version = %version, path = %path.display(), "TShock.Server.dll not found in version directory tree");
        Some(path)
    }

    /// Recursively search for TShock.Server.dll, returning the directory that contains it.
    fn find_dll_dir(dir: &Path, max_depth: u32) -> Option<PathBuf> {
        if dir.join("TShock.Server.dll").exists() {
            return Some(dir.to_path_buf());
        }
        if max_depth == 0 {
            return None;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let sub = entry.path();
                if sub.is_dir() {
                    if let Some(found) = Self::find_dll_dir(&sub, max_depth - 1) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    pub fn is_dotnet_version(&self, version_path: &Path) -> bool {
        Self::find_dll_dir(version_path, 5).is_some()
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
        Self {
            versions_dir: PathBuf::from("./data/versions"),
            github_mirror: RwLock::new(String::new()),
        }
    }
}
