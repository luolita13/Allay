use super::io;
use crate::state::JavaVersion;
use futures::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use std::time::SystemTime;
use std::{collections::HashSet, path::Path};
use tokio::task::JoinError;

use crate::{State, get_resource_file};
#[cfg(target_os = "windows")]
use winreg::{
    RegKey,
    enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
};

// ---------------------------------------------------------------------------
// Java runtime cache (ported from HMCL's JavaManager.Searcher)
// ---------------------------------------------------------------------------
//
// Computing a Java version requires spawning a JVM process (`java -cp theseus.jar
// ...`), which is slow. To avoid re-checking every path on each call, we cache
// results keyed by a fingerprint derived from the executable's file size, last
// modification time, and (if present) the SHA-1 of the `release` file in the
// Java home directory. If the fingerprint is unchanged, the cached JavaVersion
// is reused without spawning a process.
//
// The cache file lives in the global config directory as `java_cache.json`.

const JAVA_CACHE_FILENAME: &str = "java_cache.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaCacheEntry {
    /// Canonical path to the java executable.
    path: String,
    /// Fingerprint string (file size + mtime + optional release SHA-1).
    key: String,
    /// Cached Java version string (e.g. "17.0.9").
    java_version: String,
    /// Cached os.arch value (e.g. "x86_64", "aarch64").
    java_arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaCacheFile {
    /// Schema version, currently 1.
    version: u32,
    caches: Vec<JavaCacheEntry>,
}

impl Default for JavaCacheFile {
    fn default() -> Self {
        Self {
            version: 1,
            caches: Vec::new(),
        }
    }
}

static JAVA_CACHE: LazyLock<tokio::sync::RwLock<JavaCacheFile>> =
    LazyLock::new(|| tokio::sync::RwLock::new(JavaCacheFile::default()));

/// Returns the path to the java cache file inside the config directory.
async fn cache_file_path() -> crate::Result<PathBuf> {
    let state = State::get().await?;
    Ok(state.directories.config_dir.join(JAVA_CACHE_FILENAME))
}

/// Load the cache file from disk into the in-memory cache. Called once at
/// startup. If the file is missing or corrupt, a fresh empty cache is used.
pub async fn load_cache() {
    let path = match cache_file_path().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Failed to resolve java cache path: {e}");
            return;
        }
    };
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::warn!("Failed to read java cache file: {e}");
            return;
        }
    };
    let parsed: JavaCacheFile = match serde_json::from_slice(&bytes) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Corrupt java cache file, starting fresh: {e}");
            return;
        }
    };
    let mut guard = JAVA_CACHE.write().await;
    *guard = parsed;
    tracing::debug!("Loaded {} cached java entries", guard.caches.len());
}

/// Persist the cache to disk if any entries changed since load.
pub async fn save_cache() {
    let (path, caches) = {
        let guard = JAVA_CACHE.read().await;
        if guard.caches.is_empty() {
            return;
        }
        match cache_file_path().await {
            Ok(p) => (p, guard.clone()),
            Err(_) => return,
        }
    };
    let bytes = match serde_json::to_vec_pretty(&caches) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to serialize java cache: {e}");
            return;
        }
    };
    if let Err(e) = tokio::fs::write(&path, bytes).await {
        tracing::warn!("Failed to write java cache file: {e}");
    }
}

/// Compute a fingerprint for a java executable path.
///
/// The fingerprint combines:
/// - the executable's file size
/// - the executable's last modification time
/// - the SHA-1 hash of the `release` file in the java home (if present)
/// - or, as a fallback, the `rt.jar` file size + mtime (legacy Java 8 layout)
///
/// Returns `None` if the path structure is not a recognized Java layout.
fn compute_cache_key(java_executable: &Path) -> Option<String> {
    let bin_dir = java_executable.parent()?;
    if bin_dir.file_name()?.to_str()? != "bin" {
        return None;
    }
    let java_home = bin_dir.parent()?;

    let lib_dir = java_home.join("lib");

    let exec_meta = std::fs::metadata(java_executable).ok()?;
    let exec_size = exec_meta.len();
    let exec_mtime = exec_meta
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_millis();

    let release_file = java_home.join("release");

    // Prefer SHA-1 of the `release` file (modern JDKs all have this)
    if release_file.is_file() {
        if let Ok(release_bytes) = std::fs::read(&release_file) {
            let hex = sha1_smol::Sha1::from(release_bytes).hexdigest();
            return Some(format!("sz:{exec_size},lm:{exec_mtime},{hex}"));
        }
    }

    // Fallback: rt.jar attributes (Java 8 JRE layout)
    let rt_jar = if lib_dir.join("rt.jar").is_file() {
        lib_dir.join("rt.jar")
    } else if java_home.join("jre/lib/rt.jar").is_file() {
        java_home.join("jre/lib/rt.jar")
    } else {
        return None;
    };

    let rt_meta = std::fs::metadata(&rt_jar).ok()?;
    let rt_size = rt_meta.len();
    let rt_mtime = rt_meta
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_millis();

    Some(format!(
        "sz:{exec_size},lm:{exec_mtime},rsz:{rt_size},rlm:{rt_mtime}"
    ))
}

/// Try to look up a cached JavaVersion for the given executable path.
/// Returns `None` if not cached or if the fingerprint has changed.
async fn lookup_cache(java_executable: &Path) -> Option<JavaVersion> {
    let key = compute_cache_key(java_executable)?;
    let canonical = java_executable.to_string_lossy().to_string();

    let guard = JAVA_CACHE.read().await;
    for entry in &guard.caches {
        if entry.path == canonical && entry.key == key {
            let parsed = extract_java_version(&entry.java_version).ok()?;
            return Some(JavaVersion {
                parsed_version: parsed,
                path: canonical.clone(),
                version: entry.java_version.clone(),
                architecture: entry.java_arch.clone(),
            });
        }
    }
    None
}

/// Store (or update) a cache entry for the given executable + JavaVersion.
async fn store_cache(java_executable: &Path, version: &JavaVersion) {
    if let Some(key) = compute_cache_key(java_executable) {
        let canonical = java_executable.to_string_lossy().to_string();
        let entry = JavaCacheEntry {
            path: canonical.clone(),
            key,
            java_version: version.version.clone(),
            java_arch: version.architecture.clone(),
        };

        let mut guard = JAVA_CACHE.write().await;
        // Replace existing entry for the same path, or append.
        if let Some(existing) = guard.caches.iter_mut().find(|e| e.path == canonical) {
            *existing = entry;
        } else {
            guard.caches.push(entry);
        }
    }
}

// ---------------------------------------------------------------------------
// Entrypoint: get_all_jre
// ---------------------------------------------------------------------------

// Entrypoint function (Windows)
// Returns a Vec of unique JavaVersions from PATH, Windows Registry, common
// locations, .jdks, Minecraft bundled runtimes, and the THESEUS_JRES env var.
#[cfg(target_os = "windows")]
#[tracing::instrument]
pub async fn get_all_jre() -> Result<Vec<JavaVersion>, JREError> {
    let mut jre_paths = HashSet::new();

    // Add JREs directly on PATH (filter out Oracle Common Files known pitfall)
    for p in get_all_jre_path().await {
        let lower = p.to_string_lossy().to_lowercase();
        if lower.contains(r"\common files\oracle\java\") {
            continue;
        }
        jre_paths.insert(p);
    }
    jre_paths.extend(get_all_autoinstalled_jre_path().await?);
    if let Ok(java_home) = env::var("JAVA_HOME") {
        jre_paths.insert(PathBuf::from(java_home));
    }

    // Hard paths for commonly installed Java distributions (expanded from HMCL)
    let java_paths = [
        r"C:\Program Files\Java",
        r"C:\Program Files (x86)\Java",
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files (x86)\Eclipse Adoptium",
        r"C:\Program Files\Eclipse Foundation",
        r"C:\Program Files (x86)\Eclipse Foundation",
        r"C:\Program Files\Microsoft\jdk",
        r"C:\Program Files\Microsoft\jre",
        r"C:\Program Files\Zulu",
        r"C:\Program Files (x86)\Zulu",
        r"C:\Program Files\BellSoft",
        r"C:\Program Files (x86)\BellSoft",
        r"C:\Program Files\AdoptOpenJDK",
        r"C:\Program Files (x86)\AdoptOpenJDK",
        r"C:\Program Files\Semeru",
        r"C:\Program Files (x86)\Semeru",
        r"C:\Program Files\Amazon Corretto",
        r"C:\Program Files (x86)\Amazon Corretto",
        r"C:\Program Files\GraalVM",
        r"C:\Program Files (x86)\GraalVM",
    ];
    for java_path in java_paths {
        let Ok(java_subpaths) = std::fs::read_dir(java_path) else {
            continue;
        };
        for java_subpath in java_subpaths.flatten() {
            let path = java_subpath.path();
            jre_paths.insert(path.join("bin"));
        }
    }

    // Windows Registry Keys (expanded to cover more vendors)
    let key_paths = [
        r"SOFTWARE\JavaSoft\Java Runtime Environment", // Oracle
        r"SOFTWARE\JavaSoft\Java Development Kit",
        r"SOFTWARE\JavaSoft\JRE", // Oracle (newer naming)
        r"SOFTWARE\JavaSoft\JDK",
        r"SOFTWARE\Eclipse Foundation\JDK",   // Eclipse
        r"SOFTWARE\Eclipse Adoptium\JRE",     // Adoptium
        r"SOFTWARE\Eclipse Adoptium\JDK",     // Adoptium
        r"SOFTWARE\Microsoft\JDK",            // Microsoft
        r"SOFTWARE\Microsoft\JRE",            // Microsoft
        r"SOFTWARE\Azul Systems\Zulu",         // Zulu
        r"SOFTWARE\BellSoft\JDK",             // Liberica
        r"SOFTWARE\BellSoft\JRE",             // Liberica
        r"SOFTWARE\AdoptOpenJDK\JDK",         // AdoptOpenJDK (legacy)
        r"SOFTWARE\AdoptOpenJDK\JRE",        // AdoptOpenJDK (legacy)
        r"SOFTWARE\Amazon\Corretto",          // Corretto
        r"SOFTWARE\Semeru\JDK",               // Semeru
    ];

    for key in key_paths {
        if let Ok(jre_key) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_32KEY)
        {
            jre_paths.extend(get_paths_from_jre_winregkey(jre_key));
        }
        if let Ok(jre_key) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY)
        {
            jre_paths.extend(get_paths_from_jre_winregkey(jre_key));
        }
    }

    // Minecraft bundled runtimes (Microsoft Store + Minecraft Launcher)
    if let Ok(localappdata) = env::var("localappdata") {
        let mc_store_runtime = PathBuf::from(&localappdata)
            .join("Packages")
            .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
            .join("LocalCache")
            .join("Local")
            .join("runtime");
        if mc_store_runtime.is_dir() {
            add_official_java_runtimes(&mc_store_runtime, &mut jre_paths);
        }
    }
    let program_files_x86 =
        env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let mc_launcher_runtime = PathBuf::from(&program_files_x86)
        .join("Minecraft Launcher")
        .join("runtime");
    if mc_launcher_runtime.is_dir() {
        add_official_java_runtimes(&mc_launcher_runtime, &mut jre_paths);
    }

    // .jdks directory (IntelliJ IDEA downloaded JDKs)
    if let Ok(home) = env::var("USERPROFILE") {
        let jdks_dir = PathBuf::from(&home).join(".jdks");
        if jdks_dir.is_dir() {
            add_jdks_runtimes(&jdks_dir, &mut jre_paths);
        }
    }

    // THESEUS_JRES env var (custom java path list, like HMCL_JRES)
    if let Ok(jres) = env::var("THESEUS_JRES") {
        for path in jres.split(';') {
            if !path.is_empty() {
                jre_paths.insert(PathBuf::from(path));
            }
        }
    }

    // Get JRE versions from potential paths concurrently (with cache)
    let j = check_java_at_filepaths(jre_paths)
        .await
        .into_iter()
        .collect();
    save_cache().await;
    Ok(j)
}

// Gets paths rather than search directly as RegKeys should not be passed asynchronously (do not impl Send)
#[cfg(target_os = "windows")]
#[tracing::instrument]
pub fn get_paths_from_jre_winregkey(jre_key: RegKey) -> HashSet<PathBuf> {
    let mut jre_paths = HashSet::new();

    for subkey in jre_key.enum_keys().flatten() {
        if let Ok(subkey) = jre_key.open_subkey(subkey) {
            let subkey_value_names =
                [r"JavaHome", r"InstallationPath", r"\hotspot\MSI"];

            for subkey_value in subkey_value_names {
                let path: Result<String, std::io::Error> =
                    subkey.get_value(subkey_value);
                let Ok(path) = path else { continue };

                jre_paths.insert(PathBuf::from(path).join("bin"));
            }
        }
    }
    jre_paths
}

// Entrypoint function (Mac)
// Returns a Vec of unique JavaVersions from the PATH, common locations,
// .jdks, Homebrew, and the THESEUS_JRES env var.
#[cfg(target_os = "macos")]
#[tracing::instrument]
pub async fn get_all_jre() -> Result<Vec<JavaVersion>, JREError> {
    // Use HashSet to avoid duplicates
    let mut jre_paths = HashSet::new();

    // Add JREs directly on PATH
    jre_paths.extend(get_all_jre_path().await);
    jre_paths.extend(get_all_autoinstalled_jre_path().await?);
    if let Ok(java_home) = env::var("JAVA_HOME") {
        jre_paths.insert(PathBuf::from(java_home));
    }

    // Hard paths for locations for commonly installed .exes
    let java_paths = [
        r"/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java",
        r"/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home",
        r"/System/Library/Frameworks/JavaVM.framework/Versions/Current/Commands",
    ];
    for path in java_paths {
        jre_paths.insert(PathBuf::from(path));
    }
    // Iterate over JavaVirtualMachines/(something)/Contents/Home/bin
    let base_path = PathBuf::from("/Library/Java/JavaVirtualMachines/");
    if let Ok(dir) = std::fs::read_dir(base_path) {
        for entry in dir.flatten() {
            let entry = entry.path().join("Contents/Home/bin");
            jre_paths.insert(entry);
        }
    }
    // User-level JavaVirtualMachines
    if let Ok(home) = env::var("HOME") {
        let user_jvm = PathBuf::from(&home).join("Library/Java/JavaVirtualMachines");
        if let Ok(dir) = std::fs::read_dir(user_jvm) {
            for entry in dir.flatten() {
                let entry = entry.path().join("Contents/Home/bin");
                jre_paths.insert(entry);
            }
        }

        // .jdks directory
        let jdks_dir = PathBuf::from(&home).join(".jdks");
        if jdks_dir.is_dir() {
            add_jdks_runtimes(&jdks_dir, &mut jre_paths);
        }

        // Minecraft bundled runtime
        let mc_runtime = PathBuf::from(&home)
            .join("Library/Application Support/minecraft/runtime");
        if mc_runtime.is_dir() {
            add_official_java_runtimes(&mc_runtime, &mut jre_paths);
        }
    }

    // Homebrew locations
    jre_paths.insert(PathBuf::from("/opt/homebrew/opt/java/bin/java"));
    let homebrew_cellar = PathBuf::from("/opt/homebrew/Cellar/openjdk");
    if homebrew_cellar.is_dir() {
        add_jdks_runtimes(&homebrew_cellar, &mut jre_paths);
    }
    if let Ok(dir) = std::fs::read_dir("/opt/homebrew/Cellar") {
        for entry in dir.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("openjdk@") {
                add_jdks_runtimes(&entry.path(), &mut jre_paths);
            }
        }
    }

    // THESEUS_JRES env var
    if let Ok(jres) = env::var("THESEUS_JRES") {
        for path in jres.split(':') {
            if !path.is_empty() {
                jre_paths.insert(PathBuf::from(path));
            }
        }
    }

    // Get JRE versions from potential paths concurrently (with cache)
    let j = check_java_at_filepaths(jre_paths)
        .await
        .into_iter()
        .collect();
    save_cache().await;
    Ok(j)
}

// Entrypoint function (Linux)
// Returns a Vec of unique JavaVersions from the PATH, common locations,
// .jdks, SDKMAN, and the THESEUS_JRES env var.
#[cfg(target_os = "linux")]
#[tracing::instrument]
pub async fn get_all_jre() -> Result<Vec<JavaVersion>, JREError> {
    // Use HashSet to avoid duplicates
    let mut jre_paths = HashSet::new();

    // Add JREs directly on PATH
    jre_paths.extend(get_all_jre_path().await);
    jre_paths.extend(get_all_autoinstalled_jre_path().await?);
    if let Ok(java_home) = env::var("JAVA_HOME") {
        jre_paths.insert(PathBuf::from(java_home));
    }

    // Hard paths for locations for commonly installed locations
    let java_paths = [
        r"/usr",
        r"/usr/java",
        r"/usr/lib/jvm",
        r"/usr/lib32/jvm",
        r"/usr/lib64/jvm",
        r"/opt/jdk",
        r"/opt/jdks",
    ];
    for path in java_paths {
        let path = PathBuf::from(path);
        jre_paths.insert(PathBuf::from(&path).join("jre").join("bin"));
        jre_paths.insert(PathBuf::from(&path).join("bin"));
        if let Ok(dir) = std::fs::read_dir(path) {
            for entry in dir.flatten() {
                let entry_path = entry.path();
                jre_paths.insert(entry_path.join("jre").join("bin"));
                jre_paths.insert(entry_path.join("bin"));
            }
        }
    }

    // SDKMAN candidates
    if let Ok(home) = env::var("HOME") {
        let sdkman_dir = PathBuf::from(&home).join(".sdkman/candidates/java");
        if sdkman_dir.is_dir() {
            add_jdks_runtimes(&sdkman_dir, &mut jre_paths);
        }

        // .jdks directory (IntelliJ IDEA)
        let jdks_dir = PathBuf::from(&home).join(".jdks");
        if jdks_dir.is_dir() {
            add_jdks_runtimes(&jdks_dir, &mut jre_paths);
        }

        // Minecraft bundled runtime
        let mc_runtime = PathBuf::from(&home).join(".minecraft/runtime");
        if mc_runtime.is_dir() {
            add_official_java_runtimes(&mc_runtime, &mut jre_paths);
        }
    }

    // THESEUS_JRES env var
    if let Ok(jres) = env::var("THESEUS_JRES") {
        for path in jres.split(':') {
            if !path.is_empty() {
                jre_paths.insert(PathBuf::from(path));
            }
        }
    }

    // Get JRE versions from potential paths concurrently (with cache)
    let j = check_java_at_filepaths(jre_paths)
        .await
        .into_iter()
        .collect();
    save_cache().await;
    Ok(j)
}

/// Scan a directory containing JDK home folders (e.g. `.jdks/jdk-17.0.1/`).
/// Each subdirectory is expected to be a Java home with a `bin` folder.
fn add_jdks_runtimes(dir: &Path, jre_paths: &mut HashSet<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                jre_paths.insert(path.join("bin"));
            }
        }
    }
}

/// Scan Minecraft's official runtime directory structure.
/// Minecraft Launcher stores runtimes as `runtime/<component>/<arch>/<...>`.
fn add_official_java_runtimes(dir: &Path, jre_paths: &mut HashSet<PathBuf>) {
    // Walk two levels deep: runtime/<component>/<arch>/ then find bin
    if let Ok(components) = std::fs::read_dir(dir) {
        for component in components.flatten() {
            let component_path = component.path();
            if !component_path.is_dir() {
                continue;
            }
            if let Ok(arch_entries) = std::fs::read_dir(&component_path) {
                for arch_entry in arch_entries.flatten() {
                    let arch_path = arch_entry.path();
                    if !arch_path.is_dir() {
                        continue;
                    }
                    // Look for bin/java directly or nested deeper
                    let bin = arch_path.join("bin");
                    if bin.is_dir() {
                        jre_paths.insert(bin);
                    } else {
                        // Some layouts nest further (e.g. arch/<version>/bin)
                        if let Ok(sub) = std::fs::read_dir(&arch_path) {
                            for s in sub.flatten() {
                                let sp = s.path();
                                if sp.is_dir() && sp.join("bin").is_dir() {
                                    jre_paths.insert(sp.join("bin"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Gets all JREs from the PATH env variable
#[tracing::instrument]
async fn get_all_autoinstalled_jre_path() -> Result<HashSet<PathBuf>, JREError>
{
    Box::pin(async move {
        let state = State::get().await.map_err(|_| JREError::StateError)?;

        let mut jre_paths = HashSet::new();
        let base_path = state.directories.java_versions_dir();

        if base_path.is_dir()
            && let Ok(dir) = std::fs::read_dir(base_path)
        {
            for entry in dir.flatten() {
                let file_path = entry.path().join("bin");

                if let Ok(contents) = std::fs::read_to_string(file_path.clone())
                {
                    let entry = entry.path().join(contents);
                    jre_paths.insert(entry);
                } else {
                    #[cfg(not(target_os = "macos"))]
                    {
                        let file_path = file_path.join(JAVA_BIN);
                        jre_paths.insert(file_path);
                    }
                }
            }
        }

        Ok(jre_paths)
    })
    .await
}

// Gets all JREs from the PATH env variable
#[tracing::instrument]
async fn get_all_jre_path() -> HashSet<PathBuf> {
    // Iterate over values in PATH variable, where accessible JREs are referenced
    let paths =
        env::var("PATH").map(|x| env::split_paths(&x).collect::<HashSet<_>>());
    paths.unwrap_or_else(|_| HashSet::new())
}

pub const JAVA_BIN: &str = if cfg!(target_os = "windows") {
    "javaw.exe"
} else {
    "java"
};

// For each example filepath in 'paths', perform check_java_at_filepath, checking each one concurrently
// and returning a JavaVersion for every valid path that points to a java bin
#[tracing::instrument]
pub async fn check_java_at_filepaths(
    paths: HashSet<PathBuf>,
) -> HashSet<JavaVersion> {
    stream::iter(paths.into_iter())
        .map(|p: PathBuf| {
            tokio::task::spawn(async move { check_java_at_filepath(&p).await })
        })
        .buffer_unordered(64)
        .filter_map(async |x| x.ok().and_then(Result::ok))
        .collect()
        .await
}

// For example filepath 'path', attempt to resolve it and get a Java version at this path
// If no such path exists, or no such valid java at this path exists, returns None
//
// Uses a fingerprint-based cache to skip spawning a JVM process for known
// unchanged Java installations.
#[tracing::instrument]
pub async fn check_java_at_filepath(path: &Path) -> crate::Result<JavaVersion> {
    // Attempt to canonicalize the potential java filepath
    // If it fails, this path does not exist and None is returned (no Java here)
    let path = io::canonicalize(path)?;

    // Checks for existence of Java at this filepath
    // Adds JAVA_BIN to the end of the path if it is not already there
    let java = if path
        .file_name()
        .and_then(|x| x.to_str())
        .is_some_and(|x| x != JAVA_BIN)
    {
        path.join(JAVA_BIN)
    } else {
        path
    };

    if !java.exists() {
        return Err(JREError::NoExecutable(java).into());
    };

    // Try cache first - avoids spawning a JVM process for known installations
    if let Some(cached) = lookup_cache(&java).await {
        return Ok(cached);
    }

    let (_temp, file_path) =
        get_resource_file!(env "JAVA_JARS_DIR" / "theseus.jar")?;

    let output = Command::new(&java)
        .arg("-cp")
        .arg(file_path)
        .arg("com.modrinth.theseus.JavaInfo")
        .env_remove("_JAVA_OPTIONS")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut java_version = None;
    let mut java_arch = None;

    for line in stdout.lines() {
        let mut parts = line.split('=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();

        if key == "os.arch" {
            java_arch = Some(value);
        } else if key == "java.version" {
            java_version = Some(value);
        }
    }

    // Extract version info from it
    if let Some(arch) = java_arch
        && let Some(version) = java_version
    {
        if let Ok(version) = extract_java_version(version) {
            let path = java.to_string_lossy().to_string();
            let jv = JavaVersion {
                parsed_version: version,
                path,
                version: version.to_string(),
                architecture: arch.to_string(),
            };
            // Store in cache for future lookups
            store_cache(&java, &jv).await;
            return Ok(jv);
        }

        return Err(JREError::InvalidJREVersion(version.to_owned()).into());
    }

    Err(JREError::FailedJavaCheck(java).into())
}

pub fn extract_java_version(version: &str) -> Result<u32, JREError> {
    let mut split = version.split('.');

    let version = split.next().unwrap();
    let version = version.split_once('-').map_or(version, |(x, _)| x);
    let mut version = version.parse::<u32>()?;
    if version == 1 {
        version = split.next().map_or(Ok(1), |x| x.parse::<u32>())?;
    }

    Ok(version)
}

#[derive(thiserror::Error, Debug)]
pub enum JREError {
    #[error("Command error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Env error: {0}")]
    EnvError(#[from] env::VarError),

    #[error("No executable found at {0}")]
    NoExecutable(PathBuf),

    #[error("Could not check Java version at path {0}")]
    FailedJavaCheck(PathBuf),

    #[error("Invalid JRE version string: {0}")]
    InvalidJREVersion(String),

    #[error("Parsing error: {0}")]
    ParseError(#[from] std::num::ParseIntError),

    #[error("Join error: {0}")]
    JoinError(#[from] JoinError),

    #[error("No stored tag for Minecraft version {0}")]
    NoMinecraftVersionFound(String),

    #[error("Error getting launcher state")]
    StateError,
}
