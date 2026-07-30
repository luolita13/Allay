//! Theme pack system - load / install / uninstall / list theme packages
//!
//! Theme packs are zip archives containing a `modrinth-theme.json` manifest
//! along with optional asset files (background image, accent swatches, etc.).
//! Adapted from HMCL's ThemePackManager with security hardening:
//!
//! - zip entry names are normalized and path-traversal-safe
//! - atomic moves during install
//! - temporary directories are cleaned up on failure
//!
//! This module is intentionally self-contained - it does not depend on the
//! frontend themeStore and can be exercised from CLI / tests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Folder name (under the global config dir) where installed theme packs live.
pub const THEME_PACKS_FOLDER_NAME: &str = "theme_packs";

/// Manifest file name expected inside a theme pack zip.
pub const MANIFEST_FILENAME: &str = "modrinth-theme.json";

/// Schema version of the theme pack manifest. Bumped when the format changes.
pub const CURRENT_MANIFEST_VERSION: u32 = 1;

/// Maximum allowed decompressed size for a single zip entry (32 MB).
/// Prevents zip-bomb attacks.
pub const MAX_ENTRY_SIZE: u64 = 32 * 1024 * 1024;

/// Maximum allowed total decompressed size for a whole theme pack (128 MB).
pub const MAX_TOTAL_SIZE: u64 = 128 * 1024 * 1024;

/// A complete theme pack manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePackManifest {
    /// Manifest schema version. Must equal CURRENT_MANIFEST_VERSION.
    pub manifest_version: u32,
    /// Internal theme id. Must be unique among installed themes.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Optional short description.
    pub description: Option<String>,
    /// Optional author / copyright string.
    pub author: Option<String>,
    /// Optional theme pack version string.
    pub version: Option<String>,
    /// Path (inside the zip) to the background image file, if any.
    pub background_image: Option<String>,
    /// Accent color override, hex `#rrggbb` (e.g. "#ff6b9d").
    pub accent_color: Option<String>,
    /// Optional secondary color (for highlights) - hex.
    pub secondary_color: Option<String>,
    /// Background blur in pixels (0-40). Applied when background_image is set.
    pub background_blur: Option<u32>,
    /// Background opacity 10-100 (percent). Applied when background_image is set.
    pub background_opacity: Option<u32>,
    /// CSS custom properties to inject, e.g. `["--color-brand", "#ff6b9d"]`.
    /// The frontend applies these as inline style overrides.
    pub css_variables: Option<HashMap<String, String>>,
    /// Optional font family stack (CSS string).
    pub font_family: Option<String>,
}

/// A theme pack as exposed to the frontend. Includes the resolved filesystem
/// path so the frontend can build a `file://` or `asset://` URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledThemePack {
    /// Manifest id (unique).
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    /// Absolute filesystem path to the installed theme pack directory.
    pub dir: String,
    /// Absolute path to the background image, if present.
    pub background_image_path: Option<String>,
    pub accent_color: Option<String>,
    pub secondary_color: Option<String>,
    pub background_blur: Option<u32>,
    pub background_opacity: Option<u32>,
    pub css_variables: Option<HashMap<String, String>>,
    pub font_family: Option<String>,
}

static THEMES_DIR_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Returns the directory where installed theme packs live.
pub async fn themes_dir() -> crate::Result<PathBuf> {
    let state = crate::State::get().await?;
    Ok(state.directories.config_dir.join(THEME_PACKS_FOLDER_NAME))
}

/// Ensure the themes directory exists.
pub async fn ensure_themes_dir() -> crate::Result<PathBuf> {
    let dir = themes_dir().await?;
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            crate::util::io::IOError::with_path(e, &dir)
        })?;
    }
    Ok(dir)
}

/// Validates a zip entry name to prevent path traversal attacks.
///
/// Returns the normalized relative path if safe, or an error describing the
/// violation. Inspired by HMCL's ThemePackManager.
pub fn validate_zip_entry(name: &str) -> crate::Result<String> {
    if name.is_empty() {
        return Err(crate::ErrorKind::InputError(
            "empty zip entry name".into(),
        )
        .into());
    }

    // Normalize backslashes to forward slashes
    let normalized = name.replace('\\', "/");

    // Reject absolute paths (leading slash or drive letter like C:)
    if normalized.starts_with('/') {
        return Err(crate::ErrorKind::InputError(format!(
            "absolute path in zip entry: {name}"
        ))
        .into());
    }
    if normalized.len() >= 2
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(crate::ErrorKind::InputError(format!(
            "drive letter in zip entry: {name}"
        ))
        .into());
    }

    // Reject any component that is `..` or `.`
    for component in normalized.split('/') {
        if component == ".." || component == "." {
            return Err(crate::ErrorKind::InputError(format!(
                "path traversal component in zip entry: {name}"
            ))
            .into());
        }
    }

    // Reject NUL bytes
    if normalized.contains('\0') {
        return Err(crate::ErrorKind::InputError(format!(
            "NUL byte in zip entry: {name}"
        ))
        .into());
    }

    Ok(normalized)
}

/// Extract a zip archive to a destination directory safely.
///
/// - Each entry name is validated via [`validate_zip_entry`].
/// - Entries larger than [`MAX_ENTRY_SIZE`] are rejected.
/// - The total decompressed size is capped at [`MAX_TOTAL_SIZE`].
/// - Files are written to a temp file first, then atomically renamed.
pub async fn extract_zip_safe(
    zip_path: &Path,
    dest: &Path,
) -> crate::Result<()> {
    use async_zip::tokio::read::fs::ZipFileReader;
    use futures_lite::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    let zip = ZipFileReader::new(zip_path)
        .await
        .map_err(|e| crate::ErrorKind::OtherError(format!("failed to read zip: {e}")))?;

    tokio::fs::create_dir_all(dest)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, dest))?;

    let mut total_written: u64 = 0;
    let mut buffer = vec![0u8; 262144];

    for index in 0..zip.file().entries().len() {
        let entry = zip.file().entries().get(index).unwrap();
        let raw_name = entry
            .filename()
            .as_str()
            .map_err(|e| crate::ErrorKind::OtherError(format!("invalid utf8 in zip entry: {e}")))?
            .to_string();
        let safe_name = validate_zip_entry(&raw_name)?;

        // Reject oversized entries upfront
        let uncompressed = entry.uncompressed_size();
        if uncompressed > MAX_ENTRY_SIZE {
            return Err(crate::ErrorKind::InputError(format!(
                "zip entry too large: {raw_name} ({uncompressed} bytes)"
            ))
            .into());
        }
        if uncompressed > 0 {
            total_written = total_written.saturating_add(uncompressed);
            if total_written > MAX_TOTAL_SIZE {
                return Err(crate::ErrorKind::InputError(format!(
                    "zip total uncompressed size exceeds limit ({MAX_TOTAL_SIZE} bytes)"
                ))
                .into());
            }
        }

        let out_path = dest.join(&safe_name);

        if entry.dir().unwrap_or(false) {
            tokio::fs::create_dir_all(&out_path)
                .await
                .map_err(|e| crate::util::io::IOError::with_path(e, &out_path))?;
            continue;
        }

        // Ensure parent dir exists
        if let Some(parent) = out_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| crate::util::io::IOError::with_path(e, parent))?;
        }

        // Write to a temp file then atomically rename
        let tmp_path = out_path.with_extension("__tmp_theme_pack_extract__");

        {
            let mut entry_reader = zip
                .reader_with_entry(index)
                .await
                .map_err(|e| crate::ErrorKind::OtherError(format!("failed to open zip entry {raw_name}: {e}")))?;

            let mut out_file = tokio::fs::File::create(&tmp_path)
                .await
                .map_err(|e| crate::util::io::IOError::with_path(e, &tmp_path))?;

            loop {
                let bytes_read = AsyncReadExt::read(&mut entry_reader, &mut buffer)
                    .await
                    .map_err(|e| crate::util::io::IOError::with_path(e, &tmp_path))?;
                if bytes_read == 0 {
                    break;
                }
                out_file
                    .write_all(&buffer[..bytes_read])
                    .await
                    .map_err(|e| crate::util::io::IOError::with_path(e, &tmp_path))?;
            }

            out_file
                .flush()
                .await
                .map_err(|e| crate::util::io::IOError::with_path(e, &tmp_path))?;
        }

        // Atomic rename (same-filesystem)
        tokio::fs::rename(&tmp_path, &out_path)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &out_path))?;
    }

    Ok(())
}

/// Install a theme pack from a zip file path.
///
/// Returns the manifest of the newly installed theme.
pub async fn install_from_path(zip_path: &Path) -> crate::Result<ThemePackManifest> {
    let _guard = THEMES_DIR_LOCK.lock().await;

    let themes_dir = ensure_themes_dir().await?;

    // Stage to a temp dir first, then move into themes_dir/<id> atomically.
    let staging = tempfile::Builder::new()
        .prefix("modrinth-theme-pack-")
        .tempdir_in(&themes_dir)
        .map_err(|e| {
            crate::util::io::IOError::with_path(e, &themes_dir)
        })?;

    extract_zip_safe(zip_path, staging.path()).await?;

    // Load manifest
    let manifest_path = staging.path().join(MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Err(crate::ErrorKind::InputError(format!(
            "theme pack is missing {MANIFEST_FILENAME}"
        ))
        .into());
    }
    let manifest_bytes = tokio::fs::read(&manifest_path)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, &manifest_path))?;
    let manifest: ThemePackManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| {
            crate::ErrorKind::InputError(format!(
                "failed to parse {MANIFEST_FILENAME}: {e}"
            ))
        })?;

    if manifest.manifest_version != CURRENT_MANIFEST_VERSION {
        return Err(crate::ErrorKind::InputError(format!(
            "unsupported theme pack manifest_version: {} (expected {CURRENT_MANIFEST_VERSION})",
            manifest.manifest_version
        ))
        .into());
    }

    // Validate id - must be a simple identifier
    if !manifest
        .id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || manifest.id.is_empty()
    {
        return Err(crate::ErrorKind::InputError(format!(
            "invalid theme pack id: {:?}",
            manifest.id
        ))
        .into());
    }

    // If target already exists, remove the old copy first (reinstall / upgrade)
    let target_dir = themes_dir.join(&manifest.id);
    if target_dir.exists() {
        tokio::fs::remove_dir_all(&target_dir)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &target_dir))?;
    }

    // Move staging into target (same filesystem → atomic). If the rename
    // fails (e.g. cross-device staging tempdir), fall back to a recursive
    // copy + cleanup, executed in a blocking task.
    let staging_path = staging.path().to_path_buf();
    let target_clone = target_dir.clone();
    let rename_result = tokio::fs::rename(&staging_path, &target_dir).await;

    if let Err(e) = rename_result {
        tracing::debug!(
            "atomic rename failed ({}); falling back to recursive copy",
            e
        );
        let src = staging_path.clone();
        let dest = target_clone.clone();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            std::fs::create_dir_all(&dest)?;
            copy_dir_recursive(&src, &dest)?;
            let _ = std::fs::remove_dir_all(&src);
            Ok(())
        })
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!("install copy task panicked: {e}"))
        })??;
    } else {
        // Rename succeeded: staging tempdir was consumed; mark it so
        // `TempDir`'s Drop does not double-remove. We achieve this by leaking
        // it from the wrapper, then cleaning up manually if any later step
        // returned. Here there is no later step, so we can just close it.
        // `tempfile::TempDir::close` consumes self and removes the dir; but
        // we already moved it. Use `into_path` to detach.
        let _ = staging.keep();
    }

    Ok(manifest)
}

/// Synchronous recursive directory copy (fallback when rename is cross-device).
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(file_name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else if ft.is_file() {
            std::fs::copy(&entry_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Uninstall a theme pack by id.
pub async fn uninstall(theme_id: &str) -> crate::Result<()> {
    let _guard = THEMES_DIR_LOCK.lock().await;
    let themes_dir = ensure_themes_dir().await?;
    let target = themes_dir.join(theme_id);
    if !target.exists() {
        return Err(crate::ErrorKind::InputError(format!(
            "theme pack not installed: {theme_id}"
        ))
        .into());
    }
    tokio::fs::remove_dir_all(&target)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, &target))?;
    Ok(())
}

/// List all installed theme packs.
pub async fn list_installed() -> crate::Result<Vec<InstalledThemePack>> {
    let themes_dir = ensure_themes_dir().await?;
    let mut result = Vec::new();

    let mut reader = match tokio::fs::read_dir(&themes_dir).await {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![]);
        }
        Err(e) => return Err(crate::util::io::IOError::with_path(e, &themes_dir).into()),
    };

    while let Some(entry) = reader.next_entry().await.map_err(|e| {
        crate::util::io::IOError::with_path(e, &themes_dir)
    })? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join(MANIFEST_FILENAME);
        if !manifest_path.exists() {
            continue;
        }
        let bytes = match tokio::fs::read(&manifest_path).await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let manifest: ThemePackManifest = match serde_json::from_slice(&bytes) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let background_image_path = manifest
            .background_image
            .as_ref()
            .map(|rel| path.join(rel).to_string_lossy().to_string());

        result.push(InstalledThemePack {
            id: manifest.id,
            name: manifest.name,
            description: manifest.description,
            author: manifest.author,
            version: manifest.version,
            dir: path.to_string_lossy().to_string(),
            background_image_path,
            accent_color: manifest.accent_color,
            secondary_color: manifest.secondary_color,
            background_blur: manifest.background_blur,
            background_opacity: manifest.background_opacity,
            css_variables: manifest.css_variables,
            font_family: manifest.font_family,
        });
    }

    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

/// Get a single installed theme pack by id.
pub async fn get(theme_id: &str) -> crate::Result<Option<InstalledThemePack>> {
    let all = list_installed().await?;
    Ok(all.into_iter().find(|t| t.id == theme_id))
}

/// Export an existing theme pack back into a zip file at `dest_path`.
///
/// Useful for sharing or backup. Reads the installed theme pack at
/// `<themes_dir>/<theme_id>/` and zips its contents.
pub async fn export_to_zip(
    theme_id: &str,
    dest_path: &Path,
) -> crate::Result<()> {
    let themes_dir = ensure_themes_dir().await?;
    let src_dir = themes_dir.join(theme_id);
    if !src_dir.exists() {
        return Err(crate::ErrorKind::InputError(format!(
            "theme pack not installed: {theme_id}"
        ))
        .into());
    }

    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, parent))?;
    }

    // Use the `zip` crate (sync) inside a blocking task
    let src = src_dir.clone();
    let dest = dest_path.to_path_buf();
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        let file = std::fs::File::create(&dest)?;
        let mut writer = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions =
            zip::write::SimpleFileOptions::default();

        let mut stack = vec![(std::path::PathBuf::from(""), src.clone())];
        while let Some((base, dir)) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                let rel = if base.as_os_str().is_empty() {
                    std::path::PathBuf::from(entry.file_name())
                } else {
                    base.join(entry.file_name())
                };
                if path.is_dir() {
                    writer.add_directory(
                        rel.to_string_lossy().replace('\\', "/"),
                        opts,
                    )?;
                    stack.push((rel, path));
                } else if path.is_file() {
                    writer.start_file(
                        rel.to_string_lossy().replace('\\', "/"),
                        opts,
                    )?;
                    let bytes = std::fs::read(&path)?;
                    std::io::Write::write_all(&mut writer, &bytes)?;
                }
            }
        }
        writer.finish()?;
        Ok(())
    })
    .await
    .map_err(|e| {
        crate::ErrorKind::OtherError(format!("export task panicked: {e}"))
    })??;

    Ok(())
}

/// Helper for the frontend: get the absolute path to the themes directory.
pub async fn get_themes_dir_path() -> crate::Result<String> {
    let dir = ensure_themes_dir().await?;
    Ok(dir.to_string_lossy().to_string())
}
