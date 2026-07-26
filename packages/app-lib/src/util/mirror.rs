//! Mirror source URL rewriting for downloads
//!
//! Rewrites official Mojang/Modrinth/CurseForge URLs to mirror URLs
//! (BMCLAPI / MCIMirror) to improve download speeds in regions with poor
//! connectivity to official servers.
//!
//! Three strategies per source:
//! - `Mirror`: mirror URL first, fallback to official
//! - `Auto` (default): official URL first, fallback to mirror
//! - `Official`: official URL only, no fallback

use std::sync::atomic::{AtomicI32, Ordering};

use serde::{Deserialize, Serialize};

/// Download source strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "i32", from = "i32")]
pub enum DownloadSource {
    /// Prefer mirror source, fallback to official
    Mirror = 0,
    /// Auto - official first, fallback to mirror (default)
    Auto = 1,
    /// Official source only
    Official = 2,
}

impl DownloadSource {
    pub fn from_i32(val: i32) -> Self {
        match val {
            0 => Self::Mirror,
            1 => Self::Auto,
            2 => Self::Official,
            _ => Self::Auto,
        }
    }

    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

impl Default for DownloadSource {
    fn default() -> Self {
        Self::Auto
    }
}

impl From<i32> for DownloadSource {
    fn from(val: i32) -> Self {
        Self::from_i32(val)
    }
}

impl From<DownloadSource> for i32 {
    fn from(val: DownloadSource) -> Self {
        val.as_i32()
    }
}

/// Fetch strategy for a specific URL, combining both sources.
/// Determines which URL to try first, and what to fall back to.
#[derive(Debug, Clone)]
pub enum FetchStrategy {
    /// Use official URL only, no mirror fallback
    OfficialOnly,
    /// Try mirror URL first, fallback to official on failure
    MirrorFirst { mirror_url: String },
    /// Try official URL first, fallback to mirror on failure
    OfficialFirstWithFallback { mirror_url: String },
}

// Global cache for mirror settings (avoid DB query on every fetch)
static GAME_FILE_SOURCE: AtomicI32 = AtomicI32::new(1); // Default: Auto
static COMMUNITY_SOURCE: AtomicI32 = AtomicI32::new(1); // Default: Auto
static CURSEFORGE_SOURCE: AtomicI32 = AtomicI32::new(1); // Default: Auto
static VERSION_LIST_SOURCE: AtomicI32 = AtomicI32::new(1); // Default: Auto

/// Update global mirror settings cache.
/// Called from Settings::get / Settings::update to keep the cache in sync.
pub fn update_mirror_settings(
    game_source: i32,
    community_source: i32,
    curseforge_source: i32,
    version_list_source: i32,
) {
    GAME_FILE_SOURCE.store(game_source, Ordering::Relaxed);
    COMMUNITY_SOURCE.store(community_source, Ordering::Relaxed);
    CURSEFORGE_SOURCE.store(curseforge_source, Ordering::Relaxed);
    VERSION_LIST_SOURCE.store(version_list_source, Ordering::Relaxed);
}

/// Get current game file source setting
pub fn get_game_file_source() -> DownloadSource {
    DownloadSource::from_i32(GAME_FILE_SOURCE.load(Ordering::Relaxed))
}

/// Get current community source setting
pub fn get_community_source() -> DownloadSource {
    DownloadSource::from_i32(COMMUNITY_SOURCE.load(Ordering::Relaxed))
}

/// Get current CurseForge source setting
pub fn get_curseforge_source() -> DownloadSource {
    DownloadSource::from_i32(CURSEFORGE_SOURCE.load(Ordering::Relaxed))
}

/// Get current version list source setting
pub fn get_version_list_source() -> DownloadSource {
    DownloadSource::from_i32(VERSION_LIST_SOURCE.load(Ordering::Relaxed))
}

/// Determine the fetch strategy for a given URL based on current mirror settings.
///
/// Returns the appropriate strategy considering which source category the URL
/// belongs to (game files vs community resources vs version lists) and the user's
/// preference.
pub fn get_fetch_strategy(url: &str) -> FetchStrategy {
    // Check version list URLs (Mojang/Modrinth launcher-meta)
    if let Some(mirror_url) = rewrite_version_list(url) {
        return match get_version_list_source() {
            DownloadSource::Mirror => FetchStrategy::MirrorFirst { mirror_url },
            DownloadSource::Auto => {
                FetchStrategy::OfficialFirstWithFallback { mirror_url }
            }
            DownloadSource::Official => FetchStrategy::OfficialOnly,
        };
    }

    // Check game file URLs (Mojang/BMCLAPI)
    if let Some(mirror_url) = rewrite_bmclapi(url) {
        return match get_game_file_source() {
            DownloadSource::Mirror => FetchStrategy::MirrorFirst { mirror_url },
            DownloadSource::Auto => {
                FetchStrategy::OfficialFirstWithFallback { mirror_url }
            }
            DownloadSource::Official => FetchStrategy::OfficialOnly,
        };
    }

    // Check community URLs (Modrinth CDN / MCIMirror)
    if let Some(mirror_url) = rewrite_community_mirrors(url) {
        return match get_community_source() {
            DownloadSource::Mirror => FetchStrategy::MirrorFirst { mirror_url },
            DownloadSource::Auto => {
                FetchStrategy::OfficialFirstWithFallback { mirror_url }
            }
            DownloadSource::Official => FetchStrategy::OfficialOnly,
        };
    }

    // No mirror pattern matched - always use official
    FetchStrategy::OfficialOnly
}

/// Rewrite version list / metadata URLs to BMCLAPI mirror.
/// Returns Some(rewritten_url) if the URL matched, None otherwise.
///
/// This covers the Minecraft version manifest and Modrinth launcher-meta URLs.
/// Loader metadata manifests are Modrinth-specific processed formats and are not
/// directly available from BMCLAPI, so they are left as official-only.
fn rewrite_version_list(url: &str) -> Option<String> {
    // Mojang version manifest
    if url == "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json" {
        return Some(
            "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json".to_string(),
        );
    }
    if url == "https://launchermeta.mojang.com/mc/game/version_manifest.json" {
        return Some(
            "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json".to_string(),
        );
    }

    // Modrinth launcher-meta: Minecraft version manifest.
    // Modrinth's URL structure is minecraft/v{format}/manifest.json, which is
    // equivalent to BMCLAPI's /mc/game/version_manifest_v2.json.
    if url.starts_with("https://launcher-meta.modrinth.com/minecraft/")
        && url.ends_with("/manifest.json")
    {
        return Some(
            "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json".to_string(),
        );
    }

    None
}

/// Rewrite Mojang official URLs to BMCLAPI mirror.
/// Returns Some(rewritten_url) if the URL matched, None otherwise.
fn rewrite_bmclapi(url: &str) -> Option<String> {
    const REPLACEMENTS: &[(&str, &str)] = &[
        // Mojang version manifest / meta
        ("https://piston-meta.mojang.com/", "https://bmclapi2.bangbang93.com/"),
        // Mojang client JAR / data
        ("https://piston-data.mojang.com/", "https://bmclapi2.bangbang93.com/"),
        // Mojang launcher
        ("https://launcher.mojang.com/", "https://bmclapi2.bangbang93.com/"),
        // Mojang launcher meta
        ("https://launchermeta.mojang.com/", "https://bmclapi2.bangbang93.com/"),
        // Minecraft assets
        ("https://resources.download.minecraft.net/", "https://bmclapi2.bangbang93.com/assets/"),
        // Minecraft libraries
        ("https://libraries.minecraft.net/", "https://bmclapi2.bangbang93.com/maven/"),
    ];

    for (from, to) in REPLACEMENTS {
        if url.starts_with(from) {
            return Some(url.replacen(from, to, 1));
        }
    }
    None
}

/// Rewrite community-modded CDN URLs to the appropriate mirror.
/// Returns Some(rewritten_url) if the URL matched, None otherwise.
///
/// This function handles two classes of rewrites:
/// 1. **Mod file downloads** (Modrinth CDN, CurseForge CDN) → MCIMirror
/// 2. **Maven loader libraries** (launcher-meta.modrinth.com/maven/) → BMCLAPI
///
/// Note: Only CDN download URLs are mirrored by default here. Modrinth API
/// URLs are NOT mirrored because the mirror API may return incomplete/
/// incompatible data structures. CurseForge API mirroring is handled in
/// `crate::api::curseforge` because those requests require the `x-api-key`
/// header and need the full official URL for the fallback path.
fn rewrite_community_mirrors(url: &str) -> Option<String> {
    const REPLACEMENTS: &[(&str, &str)] = &[
        // Modrinth CDN (mod file downloads) → MCIMirror
        ("https://cdn.modrinth.com/", "https://mod.mcimirror.top/"),
        // CurseForge CDN (mod / modpack file downloads) → MCIMirror
        ("https://edge.forgecdn.net/", "https://mod.mcimirror.top/"),
        ("https://media.forgecdn.net/", "https://mod.mcimirror.top/"),
        // Modrinth launcher-meta maven: loader JAR libraries (Fabric / Forge /
        // NeoForge / Quilt) → BMCLAPI. daedalus_client rewrites every modded
        // loader library's `lib.url` to `launcher-meta.modrinth.com/maven/...`
        // at build time (see apps/daedalus_client/src/{fabric,forge,...}.rs).
        // BMCLAPI mirrors the exact same Maven repository layout under
        // `/maven/`, so the path can be reused as-is. Without this rule,
        // Auto-mode had no fallback URL and any download failure on the
        // Modrinth CDN would propagate as a hard error.
        ("https://launcher-meta.modrinth.com/maven/", "https://bmclapi2.bangbang93.com/maven/"),
    ];

    for (from, to) in REPLACEMENTS {
        if url.starts_with(from) {
            return Some(url.replacen(from, to, 1));
        }
    }
    None
}

/// Resolve a version list / metadata URL into primary and fallback URLs based on
/// the current `version_list_source` setting.
///
/// This is used for URLs that are not handled by the generic `get_fetch_strategy`
/// rewrite pipeline, such as Modrinth launcher-meta loader manifests which use a
/// Modrinth-specific processed format that BMCLAPI does not directly mirror.
pub fn resolve_version_list_url(url: &str) -> (String, Option<String>) {
    if let Some(mirror_url) = rewrite_version_list(url) {
        match get_version_list_source() {
            DownloadSource::Mirror => (mirror_url, Some(url.to_string())),
            DownloadSource::Auto => (url.to_string(), Some(mirror_url)),
            DownloadSource::Official => (url.to_string(), None),
        }
    } else {
        (url.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_version_list() {
        assert_eq!(
            rewrite_version_list("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json"),
            Some("https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json".to_string())
        );
        assert_eq!(
            rewrite_version_list("https://launcher-meta.modrinth.com/minecraft/v0/manifest.json"),
            Some("https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json".to_string())
        );
        // Loader manifests are not mirrored because BMCLAPI does not serve Modrinth's format
        assert_eq!(
            rewrite_version_list("https://launcher-meta.modrinth.com/forge/v0/manifest.json"),
            None
        );
    }

    #[test]
    fn test_rewrite_bmclapi() {
        assert_eq!(
            rewrite_bmclapi("https://piston-data.mojang.com/v1/objects/abc123/client.jar"),
            Some("https://bmclapi2.bangbang93.com/v1/objects/abc123/client.jar".to_string())
        );
        assert_eq!(
            rewrite_bmclapi("https://resources.download.minecraft.net/aa/bb/aabb.file"),
            Some("https://bmclapi2.bangbang93.com/assets/aa/bb/aabb.file".to_string())
        );
        assert_eq!(
            rewrite_bmclapi("https://libraries.minecraft.net/net/java/jinput.jar"),
            Some("https://bmclapi2.bangbang93.com/maven/net/java/jinput.jar".to_string())
        );
        assert_eq!(rewrite_bmclapi("https://example.com/file"), None);
    }

    #[test]
    fn test_rewrite_community_mirrors() {
        assert_eq!(
            rewrite_community_mirrors("https://cdn.modrinth.com/data/abc123/versions/v1/mod.jar"),
            Some("https://mod.mcimirror.top/data/abc123/versions/v1/mod.jar".to_string())
        );
        // API URLs should NOT be rewritten
        assert_eq!(rewrite_community_mirrors("https://api.modrinth.com/v2/project/sodium"), None);

        // Modrinth launcher-meta Maven (loader JAR libraries) MUST be mirrored
        // to BMCLAPI so Auto-mode fallback works when launcher-meta is unreachable.
        assert_eq!(
            rewrite_community_mirrors("https://launcher-meta.modrinth.com/maven/net/fabricmc/fabric-loader/0.14.21/fabric-loader-0.14.21.jar"),
            Some("https://bmclapi2.bangbang93.com/maven/net/fabricmc/fabric-loader/0.14.21/fabric-loader-0.14.21.jar".to_string())
        );
        // Other launcher-meta paths (manifests, etc.) should NOT be mirrored here
        // because BMCLAPI does not serve Modrinth's processed manifest format.
        assert_eq!(
            rewrite_community_mirrors("https://launcher-meta.modrinth.com/fabric/v0/manifest.json"),
            None
        );
        assert_eq!(
            rewrite_community_mirrors("https://launcher-meta.modrinth.com/minecraft/v0/manifest.json"),
            None
        );
    }

    // These tests mutate global Atomic settings, so they are grouped into a
    // single #[test] to avoid race conditions when running tests in parallel.
    #[test]
    fn test_fetch_strategy_scenarios() {
        // --- Auto (default): official first with mirror fallback ----------------
        update_mirror_settings(1, 1, 1, 1);

        // Game file (BMCLAPI rewrite)
        match get_fetch_strategy("https://piston-data.mojang.com/v1/objects/abc/client.jar") {
            FetchStrategy::OfficialFirstWithFallback { mirror_url } => {
                assert!(mirror_url.contains("bmclapi2.bangbang93.com"));
            }
            other => panic!("Expected OfficialFirstWithFallback, got {other:?}"),
        }

        // Community resource (MCIMirror rewrite)
        match get_fetch_strategy("https://cdn.modrinth.com/data/abc/mod.jar") {
            FetchStrategy::OfficialFirstWithFallback { mirror_url } => {
                assert!(mirror_url.contains("mod.mcimirror.top"));
            }
            other => panic!("Expected OfficialFirstWithFallback, got {other:?}"),
        }

        // Version list (BMCLAPI rewrite)
        match get_fetch_strategy("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json") {
            FetchStrategy::OfficialFirstWithFallback { mirror_url } => {
                assert!(mirror_url.contains("bmclapi2.bangbang93.com"));
            }
            other => panic!("Expected OfficialFirstWithFallback, got {other:?}"),
        }

        // Modrinth launcher-meta Maven (loader JAR libraries)
        match get_fetch_strategy(
            "https://launcher-meta.modrinth.com/maven/net/fabricmc/fabric-loader/0.14.21/fabric-loader-0.14.21.jar",
        ) {
            FetchStrategy::OfficialFirstWithFallback { mirror_url } => {
                assert!(
                    mirror_url.contains("bmclapi2.bangbang93.com"),
                    "expected BMCLAPI fallback URL, got {mirror_url}"
                );
            }
            other => panic!("Expected OfficialFirstWithFallback, got {other:?}"),
        }

        // --- Mirror mode: mirror first with official fallback -------------------
        update_mirror_settings(0, 0, 0, 0);

        match get_fetch_strategy("https://piston-data.mojang.com/v1/objects/abc/client.jar") {
            FetchStrategy::MirrorFirst { mirror_url } => {
                assert!(mirror_url.contains("bmclapi2.bangbang93.com"));
            }
            other => panic!("Expected MirrorFirst, got {other:?}"),
        }
        match get_fetch_strategy(
            "https://launcher-meta.modrinth.com/maven/net/fabricmc/fabric-loader/0.14.21/fabric-loader-0.14.21.jar",
        ) {
            FetchStrategy::MirrorFirst { mirror_url } => {
                assert!(mirror_url.contains("bmclapi2.bangbang93.com"));
            }
            other => panic!("Expected MirrorFirst, got {other:?}"),
        }

        // Even in Mirror mode, API / non-matching URLs stay official-only
        match get_fetch_strategy("https://api.modrinth.com/v2/project/sodium") {
            FetchStrategy::OfficialOnly => {}
            other => panic!("API URLs should always be OfficialOnly, got {other:?}"),
        }
        match get_fetch_strategy("https://example.com/file") {
            FetchStrategy::OfficialOnly => {}
            other => panic!("Non-matching URLs should be OfficialOnly, got {other:?}"),
        }

        // --- Official mode: no mirror fallback ----------------------------------
        update_mirror_settings(2, 2, 2, 2);

        match get_fetch_strategy("https://piston-data.mojang.com/v1/objects/abc/client.jar") {
            FetchStrategy::OfficialOnly => {}
            other => panic!("Expected OfficialOnly, got {other:?}"),
        }

        // --- resolve_version_list_url across modes ------------------------------
        update_mirror_settings(1, 1, 1, 1); // Auto
        let (primary, fallback) =
            resolve_version_list_url("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json");
        assert!(primary.contains("launchermeta.mojang.com"));
        assert!(fallback.unwrap().contains("bmclapi2.bangbang93.com"));

        update_mirror_settings(1, 1, 1, 0); // Mirror
        let (primary, fallback) =
            resolve_version_list_url("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json");
        assert!(primary.contains("bmclapi2.bangbang93.com"));
        assert!(fallback.unwrap().contains("launchermeta.mojang.com"));

        update_mirror_settings(1, 1, 1, 2); // Official
        let (primary, fallback) =
            resolve_version_list_url("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json");
        assert!(primary.contains("launchermeta.mojang.com"));
        assert!(fallback.is_none());
    }
}
