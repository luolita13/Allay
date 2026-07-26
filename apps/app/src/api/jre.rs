use crate::api::Result;
use dashmap::DashMap;
use std::path::PathBuf;
use tauri::plugin::TauriPlugin;
use theseus::prelude::JavaVersion;
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("jre")
        .invoke_handler(tauri::generate_handler![
            get_java_versions,
            set_java_version,
            jre_find_filtered_jres,
            jre_get_jre,
            jre_test_jre,
            jre_auto_install_java,
            jre_get_max_memory,
            jre_get_minimum_java_version,
            jre_get_recommended_java_major,
            jre_check_java_for_version,
            jre_find_suitable_java,
        ])
        .build()
}

#[tauri::command]
pub async fn get_java_versions() -> Result<DashMap<u32, JavaVersion>> {
    Ok(jre::get_java_versions().await?)
}

#[tauri::command]
pub async fn set_java_version(java_version: JavaVersion) -> Result<()> {
    jre::set_java_version(java_version).await?;
    Ok(())
}

// Finds the installation of Java 8, if it exists
#[tauri::command]
pub async fn jre_find_filtered_jres(
    version: Option<u32>,
) -> Result<Vec<JavaVersion>> {
    Ok(jre::find_filtered_jres(version).await?)
}

// Validates JRE at a given path
// Returns None if the path is not a valid JRE
#[tauri::command]
pub async fn jre_get_jre(path: PathBuf) -> Result<JavaVersion> {
    Ok(jre::check_jre(path).await?)
}

// Tests JRE of a certain version
#[tauri::command]
pub async fn jre_test_jre(path: PathBuf, major_version: u32) -> Result<bool> {
    Ok(jre::test_jre(path, major_version).await?)
}

// Auto installs java for the given java version
#[tauri::command]
pub async fn jre_auto_install_java(java_version: u32) -> Result<PathBuf> {
    Ok(jre::auto_install_java(java_version).await?)
}

// Gets the maximum memory a system has available.
#[tauri::command]
pub async fn jre_get_max_memory() -> Result<u64> {
    Ok(jre::get_max_memory().await?)
}

// --- Java version constraint system ---

/// Returns the minimum Java major version required by a vanilla Minecraft
/// version, or None if no specific minimum applies (e.g. MC < 1.13).
#[tauri::command]
pub fn jre_get_minimum_java_version(
    game_version: String,
) -> Result<Option<theseus::java_version::GameJavaVersion>> {
    Ok(theseus::java_version::get_minimum_java_version(&game_version))
}

/// Returns the recommended Java major version for a Minecraft version and
/// optional loader (e.g. "forge", "fabric"). Falls back to the minimum
/// if no suggested rule applies.
#[tauri::command]
pub fn jre_get_recommended_java_major(
    game_version: String,
    loader: Option<String>,
) -> Result<Option<u32>> {
    Ok(theseus::java_version::get_recommended_java_major(
        &game_version,
        loader.as_deref(),
    ))
}

/// Check whether a given Java runtime is suitable for a Minecraft version.
/// Returns a detailed result with satisfied/violated rules.
#[tauri::command]
pub async fn jre_check_java_for_version(
    game_version: String,
    java_path: PathBuf,
    loader: Option<String>,
) -> Result<theseus::java_version::ConstraintCheckResult> {
    let java = jre::check_jre(java_path).await?;
    Ok(theseus::java_version::check_java_for_version(
        &game_version,
        &java,
        loader.as_deref(),
    ))
}

/// Find the best Java runtime for a game version from all detected JREs.
/// Returns the selected JavaVersion, or None if no suitable Java is found.
#[tauri::command]
pub async fn jre_find_suitable_java(
    game_version: String,
    loader: Option<String>,
) -> Result<Option<JavaVersion>> {
    let all = jre::find_filtered_jres(None).await?;
    Ok(theseus::java_version::find_suitable_java(
        &game_version,
        &all,
        loader.as_deref(),
    )
    .cloned())
}
