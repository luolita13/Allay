//! Mod compatibility checking.
//!
//! Compares the local instance's mod list against the host's mod list
//! and reports differences.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::types::{HostModInfo, ModCompatibilityResult};

/// Read the mod list from an instance's mods directory.
/// Each `.jar` file is parsed to extract mod_id and version from the JAR's
/// `fabric.mod.json` or `META-INF/mods.toml`.
pub async fn get_local_mods(mods_dir: &Path) -> Result<Vec<HostModInfo>, String> {
    let mut mods = Vec::new();

    if !mods_dir.exists() {
        return Ok(mods);
    }

    let mut reader = tokio::fs::read_dir(mods_dir)
        .await
        .map_err(|e| format!("read mods dir: {e}"))?;

    while let Some(entry) = reader
        .next_entry()
        .await
        .map_err(|e| format!("read dir entry: {e}"))?
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jar") {
            if let Some(mod_info) = parse_jar_mod_info(&path).await {
                mods.push(mod_info);
            }
        }
    }

    Ok(mods)
}

/// Parse a JAR file to extract mod ID, version, and name.
/// Tries `fabric.mod.json` first, then falls back to filename.
async fn parse_jar_mod_info(jar_path: &Path) -> Option<HostModInfo> {
    let zip = async_zip::tokio::read::fs::ZipFileReader::new(jar_path)
        .await
        .ok()?;

    // Find fabric.mod.json entry.
    let entry_idx = zip
        .file()
        .entries()
        .iter()
        .position(|entry| matches!(entry.filename().as_str(), Ok("fabric.mod.json")))?;

    // Read entry content.
    let mut buf = Vec::new();
    zip.reader_with_entry(entry_idx)
        .await
        .ok()?
        .read_to_end_checked(&mut buf)
        .await
        .ok()?;

    if let Ok(fabric_mod) = serde_json::from_slice::<FabricModJson>(&buf) {
        return Some(HostModInfo {
            mod_id: fabric_mod.id,
            version: fabric_mod.version,
            name: fabric_mod.name.unwrap_or_default(),
        });
    }

    // Fallback: use the filename as mod_id.
    let filename = jar_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    Some(HostModInfo {
        mod_id: filename.to_string(),
        version: "unknown".to_string(),
        name: filename.to_string(),
    })
}

/// Compare local mods against host mods.
pub fn compare_mods(
    local: &[HostModInfo],
    host: &[HostModInfo],
) -> ModCompatibilityResult {
    let local_map: HashMap<&str, &HostModInfo> =
        local.iter().map(|m| (m.mod_id.as_str(), m)).collect();
    let host_map: HashMap<&str, &HostModInfo> =
        host.iter().map(|m| (m.mod_id.as_str(), m)).collect();

    let mut local_only = Vec::new();
    let mut host_only = Vec::new();
    let mut version_mismatch = Vec::new();

    for m in local {
        match host_map.get(m.mod_id.as_str()) {
            None => local_only.push(m.clone()),
            Some(host_mod) if host_mod.version != m.version => {
                version_mismatch.push((m.clone(), (*host_mod).clone()));
            }
            _ => {}
        }
    }

    for m in host {
        if !local_map.contains_key(m.mod_id.as_str()) {
            host_only.push(m.clone());
        }
    }

    let is_compatible =
        local_only.is_empty() && host_only.is_empty() && version_mismatch.is_empty();

    ModCompatibilityResult {
        is_compatible,
        local_only,
        host_only,
        version_mismatch,
    }
}

/// Fabric mod metadata (partial).
#[derive(Debug, Deserialize, Serialize)]
struct FabricModJson {
    id: String,
    version: String,
    name: Option<String>,
}
