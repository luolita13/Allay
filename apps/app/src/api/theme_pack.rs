use crate::api::Result;
use std::path::PathBuf;
use tauri::Runtime;
use theseus::prelude::theme_pack::{InstalledThemePack, ThemePackManifest};

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("theme-pack")
        .invoke_handler(tauri::generate_handler![
            theme_pack_install_from_path,
            theme_pack_uninstall,
            theme_pack_list_installed,
            theme_pack_get,
            theme_pack_export_to_zip,
            theme_pack_get_themes_dir_path,
        ])
        .build()
}

/// Install a theme pack from a zip file path.
#[tauri::command]
pub async fn theme_pack_install_from_path(
    zip_path: PathBuf,
) -> Result<ThemePackManifest> {
    Ok(theseus::prelude::theme_pack::install_from_path(&zip_path).await?)
}

/// Uninstall a theme pack by id.
#[tauri::command]
pub async fn theme_pack_uninstall(theme_id: String) -> Result<()> {
    Ok(theseus::prelude::theme_pack::uninstall(&theme_id).await?)
}

/// List all installed theme packs.
#[tauri::command]
pub async fn theme_pack_list_installed() -> Result<Vec<InstalledThemePack>> {
    Ok(theseus::prelude::theme_pack::list_installed().await?)
}

/// Get a single installed theme pack by id.
#[tauri::command]
pub async fn theme_pack_get(theme_id: String) -> Result<Option<InstalledThemePack>> {
    Ok(theseus::prelude::theme_pack::get(&theme_id).await?)
}

/// Export an installed theme pack back into a zip file at `dest_path`.
#[tauri::command]
pub async fn theme_pack_export_to_zip(
    theme_id: String,
    dest_path: PathBuf,
) -> Result<()> {
    Ok(theseus::prelude::theme_pack::export_to_zip(&theme_id, &dest_path).await?)
}

/// Returns the absolute path to the themes directory (so the frontend can
/// open it in the OS file explorer or save user-created themes there).
#[tauri::command]
pub async fn theme_pack_get_themes_dir_path() -> Result<String> {
    Ok(theseus::prelude::theme_pack::get_themes_dir_path().await?)
}
