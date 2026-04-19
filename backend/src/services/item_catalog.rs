use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrariaItem {
    pub id: i32,
    pub name: String,
    pub internal_name: String,
    #[serde(default)]
    pub zh_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemCatalog {
    pub version: String,
    pub source: String,
    pub items: Vec<TerrariaItem>,
}

fn cache_path(data_dir: &Path, version: &str) -> std::path::PathBuf {
    let safe_version = version
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    data_dir.join("items").join(format!("{}.json", safe_version))
}

fn parse_cargo_items(value: Value) -> Result<Vec<TerrariaItem>, AppError> {
    let rows = value
        .get("cargoquery")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::BadRequest("Invalid Terraria item catalog response".to_string()))?;

    let mut items = Vec::new();
    for row in rows {
        let Some(title) = row.get("title").and_then(|v| v.as_object()) else {
            continue;
        };
        let Some(id) = title
            .get("itemid")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<i32>().ok())
            .or_else(|| title.get("itemid").and_then(|v| v.as_i64()).map(|v| v as i32))
        else {
            continue;
        };
        let name = title
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let internal_name = title
            .get("internalname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if !name.is_empty() && !internal_name.is_empty() && internal_name != "None" {
            items.push(TerrariaItem {
                id,
                name,
                internal_name,
                zh_name: None,
            });
        }
    }

    Ok(items)
}

async fn fetch_cargo_chunk(endpoint: &str, where_clause: &str) -> Result<Vec<TerrariaItem>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::ProcessError(format!("Failed to create HTTP client: {}", e)))?;

    let mut offset = 0;
    let mut items = Vec::new();

    loop {
        let limit = "500".to_string();
        let offset_value = offset.to_string();
        let response = client
            .get(endpoint)
            .query(&[
                ("action", "cargoquery"),
                ("tables", "Items"),
                ("fields", "itemid,name,internalname"),
                ("where", where_clause),
                ("group_by", "itemid"),
                ("order_by", "itemid"),
                ("limit", limit.as_str()),
                ("offset", offset_value.as_str()),
                ("format", "json"),
            ])
            .header("User-Agent", "terraria-panel")
            .send()
            .await
            .map_err(|e| AppError::ProcessError(format!("Failed to download Terraria item IDs: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ProcessError(format!(
                "Terraria item catalog download failed ({}): {}",
                status, body
            )));
        }

        let value: Value = response
            .json()
            .await
            .map_err(|e| AppError::ProcessError(format!("Failed to parse Terraria item catalog: {}", e)))?;
        let chunk = parse_cargo_items(value)?;
        let chunk_len = chunk.len();
        items.extend(chunk);

        if chunk_len < 500 {
            break;
        }
        offset += 500;
    }

    Ok(items)
}

async fn fetch_catalog_from(endpoint: &str) -> Result<Vec<TerrariaItem>, AppError> {
    let mut items = Vec::new();
    items.extend(
        fetch_cargo_chunk(endpoint, "itemid IS NOT NULL AND itemid <= 4000 AND internalname <> \"None\" AND internalname <> \"\"")
            .await?,
    );
    items.extend(
        fetch_cargo_chunk(endpoint, "itemid IS NOT NULL AND itemid > 4000 AND internalname <> \"None\" AND internalname <> \"\"")
            .await?,
    );
    Ok(items)
}

pub async fn download_catalog(data_dir: &Path, version: &str) -> Result<ItemCatalog, AppError> {
    let mut items = fetch_catalog_from("https://terraria.wiki.gg/api.php").await?;
    items.sort_by_key(|item| item.id);
    items.dedup_by_key(|item| item.id);

    if items.is_empty() {
        return Err(AppError::ProcessError(
            "Downloaded Terraria item catalog is empty".to_string(),
        ));
    }

    let mut source = "terraria.wiki.gg Cargo Items".to_string();
    match fetch_catalog_from("https://terraria.wiki.gg/zh/api.php").await {
        Ok(zh_items) => {
            let zh_names = zh_items
                .into_iter()
                .map(|item| (item.id, item.name))
                .collect::<HashMap<_, _>>();
            for item in &mut items {
                if let Some(zh_name) = zh_names.get(&item.id) {
                    if !zh_name.is_empty() && zh_name != &item.name {
                        item.zh_name = Some(zh_name.clone());
                    }
                }
            }
            source = "terraria.wiki.gg Cargo Items + zh Cargo Items".to_string();
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to download Chinese Terraria item names");
        }
    }

    let catalog = ItemCatalog {
        version: version.to_string(),
        source,
        items,
    };

    let path = cache_path(data_dir, version);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::FileError(e.to_string()))?;
    }
    let json = serde_json::to_string_pretty(&catalog)
        .map_err(|e| AppError::BadRequest(format!("Failed to serialize item catalog: {}", e)))?;
    std::fs::write(&path, json).map_err(|e| AppError::FileError(e.to_string()))?;

    Ok(catalog)
}

pub fn load_catalog(data_dir: &Path, version: &str) -> Result<Option<ItemCatalog>, AppError> {
    let path = cache_path(data_dir, version);
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path).map_err(|e| AppError::FileError(e.to_string()))?;
    let catalog = serde_json::from_str::<ItemCatalog>(&content)
        .map_err(|e| AppError::BadRequest(format!("Invalid item catalog cache: {}", e)))?;
    Ok(Some(catalog))
}

pub async fn ensure_catalog(data_dir: &Path, version: &str) -> Result<ItemCatalog, AppError> {
    if let Some(catalog) = load_catalog(data_dir, version)? {
        if catalog.source.contains("zh Cargo") || catalog.items.iter().any(|item| item.zh_name.is_some()) {
            return Ok(catalog);
        }
    }
    download_catalog(data_dir, version).await
}

pub fn filter_items(catalog: &ItemCatalog, query: Option<&str>, limit: usize) -> Vec<TerrariaItem> {
    let query = query.unwrap_or("").trim().to_ascii_lowercase();
    let limit = limit.clamp(1, 10000);

    catalog
        .items
        .iter()
        .filter(|item| {
            if query.is_empty() {
                return true;
            }
            item.id.to_string() == query
                || item.name.to_ascii_lowercase().contains(&query)
                || item.internal_name.to_ascii_lowercase().contains(&query)
                || item
                    .zh_name
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&query)
        })
        .take(limit)
        .cloned()
        .collect()
}
