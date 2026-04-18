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

/// Recognised archive formats.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
}

impl ArchiveFormat {
    fn from_filename(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        if lower.ends_with(".zip") {
            Some(Self::Zip)
        } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if lower.ends_with(".tar") {
            Some(Self::Tar)
        } else {
            None
        }
    }

    fn temp_filename(&self) -> &'static str {
        match self {
            Self::Zip => "release.zip",
            Self::Tar => "release.tar",
            Self::TarGz => "release.tar.gz",
        }
    }
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

        let entries = std::fs::read_dir(&self.versions_dir).map_err(|e| {
            AppError::FileError(format!("Failed to read versions directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::FileError(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    let dir_name_str = dir_name.to_string_lossy().to_string();
                    let size = self.get_dir_size(&path).unwrap_or(0);
                    let is_dotnet = self.is_dotnet_version(&path);

                    let installed_at = std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let datetime: chrono::DateTime<chrono::Utc> = t.into();
                            datetime.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_default();

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

        versions.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(versions)
    }

    pub async fn fetch_available(
        &self,
        page: usize,
        per_page: usize,
    ) -> Result<AvailableVersionsResponse, AppError> {
        let github_mirror = self.github_mirror.read().await.clone();

        let api_per_page = 100;
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

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "GitHub API returned error");
            return Err(AppError::ProcessError(format!(
                "GitHub API 返回错误 ({}): {}",
                status, body
            )));
        }

        let releases: Vec<GitHubRelease> = response.json().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to parse GitHub API response JSON");
            AppError::ProcessError(format!("Failed to parse releases: {}", e))
        })?;

        tracing::info!(releases_count = releases.len(), "Parsed GitHub releases");

        let local_versions = self.list_local().unwrap_or_default();
        let local_version_set: std::collections::HashSet<String> =
            local_versions.iter().map(|v| v.version.clone()).collect();

        // Detect current platform to prefer matching assets
        let arch = std::env::consts::ARCH; // "x86_64", "aarch64", …
        let is_arm = arch.contains("arm") || arch.contains("aarch64");

        let mut all_versions = Vec::new();
        for release in releases {
            // Pick best asset: prefer platform-matching archive, fallback to any archive
            let asset = Self::pick_best_asset(&release.assets, is_arm);

            if let Some(asset) = asset {
                let mut download_url = asset.browser_download_url.clone();

                if !github_mirror.is_empty() {
                    download_url = Self::apply_mirror(&download_url, &github_mirror);
                }

                let display_name = release.name.clone().unwrap_or_else(|| {
                    format!("TShock {}", release.tag_name.trim_start_matches('v'))
                });

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

    /// Pick the best downloadable asset from a release.
    /// Prefers: platform-matching linux archive > any linux archive > any archive.
    fn pick_best_asset<'a>(assets: &'a [GitHubAsset], is_arm: bool) -> Option<&'a GitHubAsset> {
        let is_archive = |name: &str| {
            let l = name.to_lowercase();
            l.ends_with(".zip")
                || l.ends_with(".tar")
                || l.ends_with(".tar.gz")
                || l.ends_with(".tgz")
        };
        let is_linux = |name: &str| {
            let l = name.to_lowercase();
            l.contains("linux")
        };
        let is_arm_asset = |name: &str| {
            let l = name.to_lowercase();
            l.contains("arm") || l.contains("aarch64")
        };
        let is_x64 = |name: &str| {
            let l = name.to_lowercase();
            l.contains("x64") || l.contains("amd64") || l.contains("x86_64")
            // Also match assets that say "linux" but don't specify arch (assumed x64)
            || (is_linux(&l) && !is_arm_asset(&l) && !l.contains("arm"))
        };

        let archives: Vec<&GitHubAsset> = assets.iter().filter(|a| is_archive(&a.name)).collect();
        if archives.is_empty() {
            return None;
        }

        // Try exact platform match first
        let platform_match = archives.iter().find(|a| {
            let l = a.name.to_lowercase();
            is_linux(&l) && if is_arm { is_arm_asset(&l) } else { is_x64(&l) }
        });
        if let Some(a) = platform_match {
            return Some(a);
        }

        // Fallback: any linux archive
        let linux = archives.iter().find(|a| is_linux(&a.name));
        if let Some(a) = linux {
            return Some(a);
        }

        // Fallback: first archive
        Some(archives[0])
    }

    /// Apply a mirror/proxy prefix to a GitHub download URL.
    fn apply_mirror(url: &str, mirror: &str) -> String {
        let mirror = mirror.trim().trim_end_matches('/');
        if mirror.is_empty() {
            return url.to_string();
        }

        if mirror.contains("ghproxy")
            || mirror.contains("gh-proxy")
            || mirror.contains("mirror.ghproxy")
        {
            format!("{}/{}", mirror, url)
        } else {
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
            // If there are leftover unextracted archives, try to extract them now
            if Self::find_tshock_dir(&version_dir, 5).is_none() {
                tracing::info!(version = %tag_name, "Version dir exists but DLL not found, attempting re-extraction");
                self.try_extract_existing_archives(&version_dir)?;
            }
            if Self::find_tshock_dir(&version_dir, 5).is_some() {
                return Ok(version_dir);
            }
            // Still nothing — remove and re-download
            tracing::warn!(version = %tag_name, "Removing broken version dir for re-download");
            let _ = std::fs::remove_dir_all(&version_dir);
        }

        // Apply mirror to the download URL if configured
        let github_mirror = self.github_mirror.read().await.clone();
        let actual_url = if !github_mirror.is_empty() {
            Self::apply_mirror(download_url, &github_mirror)
        } else {
            download_url.to_string()
        };

        tracing::info!(version = %tag_name, url = %actual_url, "Starting version download");

        std::fs::create_dir_all(&version_dir).map_err(|e| {
            AppError::FileError(format!("Failed to create version directory: {}", e))
        })?;

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
                let _ = std::fs::remove_dir_all(&version_dir);
                tracing::error!(version = %tag_name, error = %e, "Download HTTP request failed");
                AppError::ProcessError(format!("下载失败，请检查网络或代理设置: {}", e))
            })?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| {
                tracing::error!(version = %tag_name, error = %e, "Failed to read download response body");
                AppError::ProcessError(format!("Failed to read response: {}", e))
            })?;

        tracing::info!(version = %tag_name, size = bytes.len(), "Download complete");

        // Detect format from URL
        let format = Self::detect_format_from_url(download_url);
        let temp_name = format.temp_filename();
        let archive_path = version_dir.join(temp_name);

        tokio::fs::write(&archive_path, bytes)
            .await
            .map_err(|e| AppError::FileError(format!("Failed to write archive file: {}", e)))?;

        tracing::info!(version = %tag_name, format = ?format, "Extracting archive");
        Self::extract_archive(&archive_path, &version_dir, format)?;

        std::fs::remove_file(&archive_path)
            .map_err(|e| AppError::FileError(format!("Failed to delete archive: {}", e)))?;

        tracing::info!(version = %tag_name, path = %version_dir.display(), "Version downloaded and extracted successfully");

        Ok(version_dir)
    }

    /// Detect archive format from download URL filename.
    fn detect_format_from_url(url: &str) -> ArchiveFormat {
        // Get the last path segment
        let filename = url.rsplit('/').next().unwrap_or(url);
        ArchiveFormat::from_filename(filename).unwrap_or(ArchiveFormat::Zip)
    }

    /// Try to extract any unextracted archives found inside a version directory.
    /// This handles the case where a previous download saved the archive but
    /// extraction failed (e.g. tar file treated as zip).
    fn try_extract_existing_archives(&self, version_dir: &Path) -> Result<(), AppError> {
        let entries =
            std::fs::read_dir(version_dir).map_err(|e| AppError::FileError(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let fname = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if let Some(format) = ArchiveFormat::from_filename(&fname) {
                tracing::info!(file = %fname, format = ?format, "Found unextracted archive, extracting now");
                match Self::extract_archive(&path, version_dir, format) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&path);
                        tracing::info!(file = %fname, "Archive extracted and removed");
                    }
                    Err(e) => {
                        tracing::error!(file = %fname, error = %e, "Failed to extract archive");
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract an archive (zip, tar, or tar.gz) into `extract_to`, flattening
    /// a common top-level prefix directory if one exists.
    fn extract_archive(
        archive_path: &Path,
        extract_to: &Path,
        format: ArchiveFormat,
    ) -> Result<(), AppError> {
        match format {
            ArchiveFormat::Zip => Self::extract_zip(archive_path, extract_to),
            ArchiveFormat::Tar => Self::extract_tar(archive_path, extract_to, false),
            ArchiveFormat::TarGz => Self::extract_tar(archive_path, extract_to, true),
        }
    }

    fn extract_zip(zip_path: &Path, extract_to: &Path) -> Result<(), AppError> {
        let file = std::fs::File::open(zip_path)
            .map_err(|e| AppError::FileError(format!("Failed to open zip: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::FileError(format!("Failed to read zip: {}", e)))?;

        let prefix_to_strip = Self::detect_zip_prefix(&mut archive);

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::FileError(format!("Failed to get file from zip: {}", e)))?;

            let raw_name = file.name().to_string();
            let relative = if let Some(ref prefix) = prefix_to_strip {
                raw_name.strip_prefix(prefix).unwrap_or(&raw_name)
            } else {
                &raw_name
            };

            if relative.is_empty() || relative == "/" {
                continue;
            }

            let output_path = extract_to.join(relative);

            if file.is_dir() {
                std::fs::create_dir_all(&output_path).map_err(|e| {
                    AppError::FileError(format!("Failed to create directory: {}", e))
                })?;
            } else {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        AppError::FileError(format!("Failed to create parent dir: {}", e))
                    })?;
                }
                let mut output_file = std::fs::File::create(&output_path)
                    .map_err(|e| AppError::FileError(format!("Failed to create file: {}", e)))?;
                std::io::copy(&mut file, &mut output_file)
                    .map_err(|e| AppError::FileError(format!("Failed to write file: {}", e)))?;
            }
        }

        Ok(())
    }

    fn extract_tar(tar_path: &Path, extract_to: &Path, is_gzipped: bool) -> Result<(), AppError> {
        let file = std::fs::File::open(tar_path)
            .map_err(|e| AppError::FileError(format!("Failed to open tar: {}", e)))?;

        // First pass: detect common prefix
        let prefix = {
            let reader: Box<dyn std::io::Read> = if is_gzipped {
                Box::new(flate2::read::GzDecoder::new(
                    std::fs::File::open(tar_path)
                        .map_err(|e| AppError::FileError(format!("Failed to open tar: {}", e)))?,
                ))
            } else {
                Box::new(std::io::BufReader::new(
                    std::fs::File::open(tar_path)
                        .map_err(|e| AppError::FileError(format!("Failed to open tar: {}", e)))?,
                ))
            };
            Self::detect_tar_prefix(reader)?
        };

        if let Some(ref p) = prefix {
            tracing::debug!(prefix = %p, "Detected tar common prefix, will flatten");
        }

        // Second pass: extract
        let reader: Box<dyn std::io::Read> = if is_gzipped {
            Box::new(flate2::read::GzDecoder::new(file))
        } else {
            Box::new(std::io::BufReader::new(file))
        };

        let mut archive = tar::Archive::new(reader);
        let entries = archive
            .entries()
            .map_err(|e| AppError::FileError(format!("Failed to read tar entries: {}", e)))?;

        for entry_result in entries {
            let mut entry = entry_result
                .map_err(|e| AppError::FileError(format!("Failed to read tar entry: {}", e)))?;

            let raw_path = entry
                .path()
                .map_err(|e| AppError::FileError(format!("Failed to get entry path: {}", e)))?
                .to_path_buf();

            let raw_str = raw_path.to_string_lossy().to_string();

            // Strip common prefix
            let relative = if let Some(ref pfx) = prefix {
                raw_str.strip_prefix(pfx).unwrap_or(&raw_str).to_string()
            } else {
                raw_str
            };

            if relative.is_empty() || relative == "/" || relative == "." {
                continue;
            }

            let output_path = extract_to.join(&relative);

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&output_path).map_err(|e| {
                    AppError::FileError(format!("Failed to create directory: {}", e))
                })?;
            } else if entry.header().entry_type().is_file() {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        AppError::FileError(format!("Failed to create parent dir: {}", e))
                    })?;
                }
                let mut output_file = std::fs::File::create(&output_path)
                    .map_err(|e| AppError::FileError(format!("Failed to create file: {}", e)))?;
                std::io::copy(&mut entry, &mut output_file)
                    .map_err(|e| AppError::FileError(format!("Failed to write file: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Detect a common prefix directory in a tar archive.
    fn detect_tar_prefix(reader: Box<dyn std::io::Read>) -> Result<Option<String>, AppError> {
        let mut archive = tar::Archive::new(reader);
        let entries = archive.entries().map_err(|e| {
            AppError::FileError(format!("Failed to read tar for prefix detection: {}", e))
        })?;

        let mut common: Option<String> = None;
        for entry_result in entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = match entry.path() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            let first = match path.find('/') {
                Some(idx) => path[..=idx].to_string(),
                None => return Ok(None), // top-level file
            };

            match &common {
                None => common = Some(first),
                Some(existing) => {
                    if *existing != first {
                        return Ok(None);
                    }
                }
            }
        }

        Ok(common.filter(|p| p.ends_with('/')))
    }

    /// Detect a common prefix directory in a zip archive.
    fn detect_zip_prefix(archive: &mut zip::ZipArchive<std::fs::File>) -> Option<String> {
        let mut common: Option<String> = None;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                let first = match name.find('/') {
                    Some(idx) => &name[..=idx],
                    None => return None,
                };
                match &common {
                    None => common = Some(first.to_string()),
                    Some(existing) => {
                        if existing != first {
                            return None;
                        }
                    }
                }
            }
        }
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

        // If there are leftover unextracted archives, extract them first
        if Self::find_tshock_dir(&path, 5).is_none() {
            tracing::info!(version = %version, "DLL not found, checking for unextracted archives");
            let mgr = VersionManager {
                versions_dir: self.versions_dir.clone(),
                github_mirror: RwLock::new(String::new()),
            };
            let _ = mgr.try_extract_existing_archives(&path);
        }

        if let Some(tshock_dir) = Self::find_tshock_dir(&path, 5) {
            return Some(tshock_dir);
        }

        tracing::warn!(version = %version, path = %path.display(), "TShock executable not found in version directory tree");
        Some(path)
    }

    /// Recursively search for TShock executable, returning the directory that contains it.
    /// Supports both old format (TShock.Server.dll) and new v6+ format (TShock.Server binary).
    fn find_tshock_dir(dir: &Path, max_depth: u32) -> Option<PathBuf> {
        // Check for new-style self-contained binary (v6+): "TShock.Server" without extension
        let self_contained = dir.join("TShock.Server");
        if self_contained.exists() && self_contained.is_file() {
            return Some(dir.to_path_buf());
        }
        // Check for old-style DLL: "TShock.Server.dll" (needs dotnet runtime)
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
                    if let Some(found) = Self::find_tshock_dir(&sub, max_depth - 1) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    /// Check if TShock executable exists (either .dll or self-contained binary).
    pub fn is_dotnet_version(&self, version_path: &Path) -> bool {
        Self::find_tshock_dir(version_path, 5).is_some()
    }

    /// Determine the executable type in a TShock directory.
    /// Returns `true` if it's a self-contained binary (v6+), `false` if it's a DLL needing dotnet.
    pub fn is_self_contained(version_path: &Path) -> bool {
        let binary = version_path.join("TShock.Server");
        binary.exists() && binary.is_file() && !version_path.join("TShock.Server.dll").exists()
    }

    fn get_dir_size(&self, path: &Path) -> Result<u64, AppError> {
        let mut total_size = 0u64;

        if !path.is_dir() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(path).map_err(|e| AppError::FileError(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::FileError(e.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|e| AppError::FileError(e.to_string()))?;

            if file_type.is_file() {
                let metadata = entry
                    .metadata()
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
