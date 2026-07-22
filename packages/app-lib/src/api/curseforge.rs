//! CurseForge API integration
//!
//! Provides search, project info, and file listing for CurseForge mods.
//! CurseForge data is converted to Modrinth V3-compatible format so the
//! existing frontend components can render results without modification.
//!
//! API docs: https://docs.curseforge.com/
//! Base URL: https://api.curseforge.com/v1
//! Auth: x-api-key header
//!
//! Built-in API key: PolyMC's public key (allows read-only access).
//! Users can override via settings.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::state::{SearchResultV3, SearchResultsV3};
use crate::util::fetch::FetchSemaphore;

/// CurseForge API base URL (used for URL construction)
const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
/// Host portion of the CurseForge API, used for mirror rewriting.
const CURSEFORGE_API_HOST: &str = "https://api.curseforge.com";

/// Built-in public API key (PolyMC's key, same as PCL-CE / PrismLauncher).
/// Users can override this in settings.
pub const BUILT_IN_API_KEY: &str =
    "$2a$10$1Oqr2MX3O4n/ilhFGc597u8tfI3L2Hyr9/rtWDAMRjghSQV2QUuxq";

/// Minecraft game ID on CurseForge
const MINECRAFT_GAME_ID: i32 = 432;

/// CurseForge class IDs for Minecraft content types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfClassId {
    Mod = 6,
    Modpack = 4471,
    ResourcePack = 12,
    Shader = 6552,
    World = 17,
    DataPack = 5193,
}

impl CfClassId {
    /// Map a Modrinth project_type string to a CurseForge classId
    pub fn from_project_type(project_type: &str) -> Option<Self> {
        match project_type {
            "mod" => Some(Self::Mod),
            "modpack" => Some(Self::Modpack),
            "resourcepack" => Some(Self::ResourcePack),
            "shader" => Some(Self::Shader),
            "world" => Some(Self::World),
            "datapack" => Some(Self::DataPack),
            _ => None,
        }
    }

    /// Map this classId back to a Modrinth project_type string
    pub fn as_project_type(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Modpack => "modpack",
            Self::ResourcePack => "resourcepack",
            Self::Shader => "shader",
            Self::World => "world",
            Self::DataPack => "datapack",
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// CurseForge mod loader type enum
/// 0=Any, 1=Forge, 2=Cauldron, 3=LiteLoader, 4=Fabric, 5=Quilt, 6=NeoForge
#[derive(Debug, Clone, Copy)]
pub enum CfModLoaderType {
    Any = 0,
    Forge = 1,
    LiteLoader = 3,
    Fabric = 4,
    Quilt = 5,
    NeoForge = 6,
}

impl CfModLoaderType {
    /// Map a Modrinth loader name to a CurseForge ModLoaderType
    pub fn from_loader_name(loader: &str) -> Option<Self> {
        match loader {
            "forge" => Some(Self::Forge),
            "fabric" => Some(Self::Fabric),
            "quilt" => Some(Self::Quilt),
            "neoforge" => Some(Self::NeoForge),
            "liteloader" => Some(Self::LiteLoader),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// CurseForge search sort field
/// 1=Featured, 2=Popularity, 3=LastUpdated, 4=Name, 5=Author, 6=TotalDownloads
#[derive(Debug, Clone, Copy)]
pub enum CfSortField {
    Featured = 1,
    Popularity = 2,
    LastUpdated = 3,
    Name = 4,
    Author = 5,
    TotalDownloads = 6,
}

impl CfSortField {
    /// Map a Modrinth sort param to a CurseForge sort field
    /// Modrinth: "relevance", "downloads", "follows", "newest", "updated"
    pub fn from_modrinth_sort(sort: &str) -> Self {
        match sort {
            "downloads" => Self::TotalDownloads,
            "newest" | "updated" => Self::LastUpdated,
            _ => Self::Popularity,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Sort order for CurseForge search
#[derive(Debug, Clone, Copy)]
pub enum CfSortOrder {
    Ascending = 0,
    Descending = 1,
}

impl Default for CfSortOrder {
    fn default() -> Self {
        Self::Descending
    }
}

// ============================================================================
// CurseForge API response types (raw, before conversion)
// ============================================================================

#[derive(Debug, Deserialize)]
struct CfResponse<T> {
    data: T,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct CfPagination {
    #[serde(default)]
    index: u32,
    #[serde(default)]
    page_size: u32,
    #[serde(default)]
    result_count: u32,
    #[serde(default)]
    total_count: u32,
}

#[derive(Debug, Deserialize)]
struct CfSearchResult {
    #[serde(default)]
    pagination: CfPagination,
    #[serde(default)]
    mods: Vec<CfMod>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CfMod {
    pub id: i64,
    pub game_id: i32,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub download_count: u64,
    #[serde(default)]
    pub primary_category_id: i32,
    #[serde(default)]
    pub class_id: Option<i32>,
    #[serde(default)]
    pub categories: Vec<CfCategory>,
    #[serde(default)]
    pub authors: Vec<CfAuthor>,
    #[serde(default)]
    pub logo: Option<CfAsset>,
    #[serde(default)]
    pub screenshots: Vec<CfAsset>,
    #[serde(default)]
    pub links: CfLinks,
    #[serde(default)]
    pub date_created: String,
    #[serde(default)]
    pub date_modified: String,
    #[serde(default)]
    pub date_released: String,
    #[serde(default)]
    pub allow_mod_distribution: Option<bool>,
    #[serde(default)]
    pub game_popularity_rank: Option<f64>,
    #[serde(default)]
    pub is_available: Option<bool>,
    #[serde(default)]
    pub latest_files: Vec<CfFile>,
    #[serde(default)]
    pub latest_files_indexes: Vec<CfFileIndex>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CfCategory {
    pub id: i32,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub class_id: Option<i32>,
    #[serde(default)]
    pub parent_category_id: Option<i32>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CfAuthor {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CfAsset {
    pub id: i64,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CfLinks {
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfFile {
    pub id: i64,
    pub mod_id: i64,
    #[serde(default)]
    pub is_available: Option<bool>,
    #[serde(default)]
    pub display_name: String,
    pub file_name: String,
    #[serde(default)]
    pub release_type: i32, // 1=Release, 2=Beta, 3=Alpha
    #[serde(default)]
    pub file_status: i32,
    #[serde(default)]
    pub file_date: String,
    #[serde(default)]
    pub file_length: u64,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub sortable_game_versions: Vec<CfSortableGameVersion>,
    #[serde(default)]
    pub dependencies: Vec<CfFileDependency>,
    #[serde(default)]
    pub alternate_file_id: i64,
    #[serde(default)]
    pub is_server_pack: Option<bool>,
    #[serde(default)]
    pub file_fingerprint: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfFileIndex {
    pub game_version: String,
    #[serde(default)]
    pub file_id: Option<i64>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub release_type: Option<i32>,
    #[serde(default)]
    pub game_version_type_id: Option<i32>,
    #[serde(default)]
    pub mod_loader: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfSortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: String,
    pub game_version: String,
    #[serde(default)]
    pub game_version_release_date: Option<String>,
    #[serde(default)]
    pub game_version_type_id: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfFileDependency {
    pub mod_id: i64,
    #[serde(default)]
    pub file_id: Option<i64>,
    pub relation_type: i32, // 1=EmbeddedLibrary, 2=OptionalDependency, 3=RequiredDependency, 4=Tool, 5=Incompatible, 6=Include
}

#[derive(Debug, Deserialize)]
pub struct CfModFilesResult {
    #[serde(default)]
    pub pagination: CfPagination,
    pub data: Vec<CfFile>,
}

#[derive(Debug, Deserialize)]
struct CfDownloadUrlResult {
    #[serde(default)]
    data: Option<String>,
}

// ============================================================================
// Public request types
// ============================================================================

/// Search parameters for CurseForge search.
/// All fields are optional except class_id.
#[derive(Debug, Clone)]
pub struct CfSearchParams {
    pub class_id: CfClassId,
    pub search_filter: Option<String>,
    pub game_version: Option<String>,
    pub mod_loader_type: Option<CfModLoaderType>,
    pub sort_field: Option<CfSortField>,
    pub sort_order: Option<CfSortOrder>,
    pub page: Option<u32>,      // 0-indexed
    pub page_size: Option<u32>, // default 20, max 50
}

// ============================================================================
// HTTP helpers
// ============================================================================

/// Build a CurseForge API URL with query parameters.
fn build_url(endpoint: &str, params: &[(&str, &str)]) -> String {
    let base = format!("{}{}", CURSEFORGE_API_BASE, endpoint);
    if params.is_empty() {
        return base;
    }
    let query = params
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                urlencoding::encode(k),
                urlencoding::encode(v)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    format!("{}?{}", base, query)
}

/// Resolve the API key. Uses the built-in PolyMC public key.
fn resolve_api_key() -> &'static str {
    BUILT_IN_API_KEY
}

/// MCIMirror proxy base URL for CurseForge API.
const CURSEFORGE_MCIMIRROR_API_BASE: &str = "https://mod.mcimirror.top/curseforge";

/// Send a CurseForge API GET request and parse JSON response.
/// Adds the `x-api-key` header required by CurseForge.
/// Respects the `curseforge_source` mirror setting: when set to Mirror or Auto,
/// requests are routed through MCIMirror's CurseForge API proxy.
async fn cf_request<T: serde::de::DeserializeOwned>(
    url: &str,
    semaphore: &FetchSemaphore,
    _state: &crate::State,
) -> crate::Result<T> {
    let api_key = resolve_api_key();

    let (primary_url, fallback_url) =
        resolve_cf_api_urls(url, crate::util::mirror::get_curseforge_source().as_i32());

    tracing::debug!("CurseForge request: {url} (primary: {primary_url})");

    let client = &crate::util::fetch::INSECURE_REQWEST_CLIENT;
    let _permit = semaphore.0.acquire().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to acquire fetch semaphore: {e}"
        ))
        .as_error()
    })?;

    // Try primary URL first
    let response = match client
        .request(Method::GET, &primary_url)
        .header("x-api-key", api_key)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp,
        Ok(resp) => {
            let status = resp.status();
            // For non-success status, try fallback if available
            if let Some(fallback) = &fallback_url {
                tracing::info!(
                    "CurseForge primary returned {status}, trying fallback: {fallback}"
                );
                client
                    .request(Method::GET, fallback)
                    .header("x-api-key", api_key)
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .map_err(|e| {
                        crate::ErrorKind::OtherError(format!(
                            "CurseForge fallback request failed: {e}"
                        ))
                        .as_error()
                    })?
            } else {
                let body = resp.text().await.unwrap_or_default();
                return Err(crate::ErrorKind::OtherError(format!(
                    "CurseForge API returned {status}: {}",
                    body.chars().take(500).collect::<String>()
                ))
                .as_error());
            }
        }
        Err(primary_err) => {
            if let Some(fallback) = &fallback_url {
                tracing::info!(
                    "CurseForge primary request failed ({primary_err}), trying fallback: {fallback}"
                );
                client
                    .request(Method::GET, fallback)
                    .header("x-api-key", api_key)
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .map_err(|e| {
                        crate::ErrorKind::OtherError(format!(
                            "CurseForge fallback request failed: {e}"
                        ))
                        .as_error()
                    })?
            } else {
                return Err(crate::ErrorKind::OtherError(format!(
                    "CurseForge request failed: {primary_err}"
                ))
                .as_error());
            }
        }
    };

    let bytes = response.bytes().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to read CurseForge response: {e}"
        ))
        .as_error()
    })?;

    let value: T = serde_json::from_slice(&bytes).map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to parse CurseForge response: {e}\nBody: {}",
            String::from_utf8_lossy(&bytes)
                .chars()
                .take(500)
                .collect::<String>()
        ))
        .as_error()
    })?;

    Ok(value)
}

/// Resolve the actual URL(s) to use for a CurseForge API request based on the
/// curseforge_source setting.
///
/// Returns `(primary_url, optional_fallback_url)`.
fn resolve_cf_api_urls(
    original: &str,
    curseforge_source: i32,
) -> (String, Option<String>) {
    let mirror_url = original.replace(
        CURSEFORGE_API_HOST,
        CURSEFORGE_MCIMIRROR_API_BASE,
    );

    match curseforge_source {
        // Mirror first: try MCIMirror, fallback to official
        0 => (mirror_url, Some(original.to_string())),
        // Auto (default): try official first, fallback to mirror
        1 => (original.to_string(), Some(mirror_url)),
        // Official only
        _ => (original.to_string(), None),
    }
}

// ============================================================================
// Public API functions
// ============================================================================

/// Search CurseForge mods.
/// Returns results converted to Modrinth V3-compatible format.
#[tracing::instrument(skip(state))]
pub async fn search(
    params: CfSearchParams,
    state: &crate::State,
) -> crate::Result<SearchResultsV3> {
    let mut query_pairs: Vec<(&str, String)> = Vec::new();
    query_pairs.push(("gameId", MINECRAFT_GAME_ID.to_string()));
    query_pairs.push(("classId", params.class_id.as_i32().to_string()));

    if let Some(filter) = &params.search_filter {
        if !filter.is_empty() {
            query_pairs.push(("searchFilter", filter.clone()));
        }
    }
    if let Some(gv) = &params.game_version {
        query_pairs.push(("gameVersion", gv.clone()));
    }
    if let Some(loader) = params.mod_loader_type {
        query_pairs.push(("modLoaderType", loader.as_i32().to_string()));
    }
    if let Some(sort) = params.sort_field {
        query_pairs.push(("sortField", sort.as_i32().to_string()));
    }
    let order = params.sort_order.unwrap_or_default();
    query_pairs.push(("sortOrder", (order as i32).to_string()));

    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20).min(50);
    query_pairs.push(("index", (page * page_size).to_string()));
    query_pairs.push(("pageSize", page_size.to_string()));

    let str_pairs: Vec<(&str, &str)> = query_pairs
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let url = build_url("/mods/search", &str_pairs);

    let result: CfResponse<CfSearchResult> =
        cf_request(&url, &state.fetch_semaphore, state).await?;
    let search_result = result.data;

    // Build the original query string (for caching key)
    let search_key = str_pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");
    let search_key = format!("?{}", search_key);

    // Convert to Modrinth V3 format
    let hits = search_result
        .mods
        .into_iter()
        .map(|m| convert_mod_to_v3_hit(m, params.class_id))
        .collect::<Vec<_>>();

    let total_hits = search_result.pagination.total_count;
    let offset = search_result.pagination.index;

    Ok(SearchResultsV3 {
        search: search_key,
        result: SearchResultV3 {
            hits,
            offset,
            limit: page_size,
            total_hits,
        },
    })
}

/// Get a single CurseForge mod by ID.
#[tracing::instrument(skip(state))]
pub async fn get_mod(
    mod_id: i64,
    state: &crate::State,
) -> crate::Result<CfMod> {
    let url = build_url(&format!("/mods/{}", mod_id), &[]);
    let result: CfResponse<CfMod> =
        cf_request(&url, &state.fetch_semaphore, state).await?;
    Ok(result.data)
}

/// Get files for a CurseForge mod.
/// Optionally filter by game version and loader.
#[tracing::instrument(skip(state))]
pub async fn get_mod_files(
    mod_id: i64,
    game_version: Option<&str>,
    mod_loader_type: Option<CfModLoaderType>,
    state: &crate::State,
) -> crate::Result<Vec<CfFile>> {
    let mut query_pairs: Vec<(&str, String)> = Vec::new();
    if let Some(gv) = game_version {
        query_pairs.push(("gameVersion", gv.to_string()));
    }
    if let Some(loader) = mod_loader_type {
        query_pairs.push(("modLoaderType", loader.as_i32().to_string()));
    }
    let str_pairs: Vec<(&str, &str)> = query_pairs
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let url = build_url(&format!("/mods/{}/files", mod_id), &str_pairs);
    let result: CfResponse<CfModFilesResult> =
        cf_request(&url, &state.fetch_semaphore, state).await?;
    Ok(result.data.data)
}

/// Get a single CurseForge file by mod_id and file_id.
#[tracing::instrument(skip(state))]
pub async fn get_file(
    mod_id: i64,
    file_id: i64,
    state: &crate::State,
) -> crate::Result<CfFile> {
    let url = build_url(&format!("/mods/{}/files/{}", mod_id, file_id), &[]);
    let result: CfResponse<CfFile> =
        cf_request(&url, &state.fetch_semaphore, state).await?;
    Ok(result.data)
}

/// Get the direct download URL for a CurseForge file.
/// Uses the dedicated `/download-url` endpoint; if that returns no URL,
/// falls back to the `download_url` field from the file metadata.
/// Returns None if the file is not available for download
/// (some authors disallow third-party downloads).
#[tracing::instrument(skip(state))]
pub async fn get_file_download_url(
    mod_id: i64,
    file_id: i64,
    state: &crate::State,
) -> crate::Result<Option<String>> {
    // Prefer the dedicated download URL endpoint over the file metadata field.
    let url = build_url(
        &format!("/mods/{}/files/{}/download-url", mod_id, file_id),
        &[],
    );
    let result: CfResponse<CfDownloadUrlResult> =
        cf_request(&url, &state.fetch_semaphore, state).await?;
    if let Some(url) = result.data.data {
        return Ok(Some(url));
    }

    // Fallback to the file metadata's download_url if the endpoint returned null.
    let file = get_file(mod_id, file_id, state).await?;
    Ok(file.download_url)
}

/// Get CurseForge categories for a given class.
#[tracing::instrument(skip(state))]
pub async fn get_categories(
    class_id: Option<CfClassId>,
    state: &crate::State,
) -> crate::Result<Vec<CfCategory>> {
    let mut query_pairs: Vec<(&str, String)> = Vec::new();
    query_pairs.push(("gameId", MINECRAFT_GAME_ID.to_string()));
    if let Some(class_id) = class_id {
        query_pairs.push(("classId", class_id.as_i32().to_string()));
    }
    let str_pairs: Vec<(&str, &str)> = query_pairs
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let url = build_url("/categories", &str_pairs);
    let result: CfResponse<Vec<CfCategory>> =
        cf_request(&url, &state.fetch_semaphore, state).await?;
    Ok(result.data)
}

// ============================================================================
// Conversion: CurseForge → Modrinth V3
// ============================================================================

/// Names that appear in CurseForge game_versions but are not actual
/// Minecraft versions (loaders or Java versions). Used to filter them out
/// when building the version list.
const NON_VERSION_ENTRIES: &[&str] = &[
    "Fabric",
    "Forge",
    "Quilt",
    "NeoForge",
    "LiteLoader",
    "Java 8",
    "Java 9",
    "Java 10",
    "Java 11",
    "Java 12",
    "Java 13",
    "Java 14",
    "Java 15",
    "Java 16",
    "Java 17",
    "Java 18",
    "Java 19",
    "Java 20",
    "Java 21",
];

/// Convert a CurseForge mod to a Modrinth V3 search hit (serde_json::Value).
///
/// Field mapping:
/// - project_id ← id (as string)
/// - project_types ← [class_id → project_type]
/// - slug ← slug
/// - name ← name
/// - summary ← summary
/// - author ← authors[0].name
/// - downloads ← download_count
/// - icon_url ← logo.thumbnail_url
/// - categories ← categories[].name
/// - date_created ← date_created
/// - date_modified ← date_modified
/// - loaders ← derived from latest_files_indexes
/// - gallery ← screenshots[].url
fn convert_mod_to_v3_hit(
    cf_mod: CfMod,
    class_id: CfClassId,
) -> serde_json::Value {
    let project_type = class_id.as_project_type();

    let author = cf_mod
        .authors
        .first()
        .map(|a| a.name.clone())
        .unwrap_or_default();

    let author_id = cf_mod
        .authors
        .first()
        .map(|a| a.id.to_string())
        .unwrap_or_default();

    let icon_url = cf_mod
        .logo
        .as_ref()
        .and_then(|l| l.thumbnail_url.clone())
        .or_else(|| cf_mod.logo.as_ref().and_then(|l| l.url.clone()));

    let categories: Vec<String> =
        cf_mod.categories.iter().map(|c| c.name.clone()).collect();

    let display_categories: Vec<String> =
        categories.iter().take(3).cloned().collect();

    let gallery: Vec<String> = cf_mod
        .screenshots
        .iter()
        .filter_map(|s| s.url.clone())
        .collect();

    let featured_gallery = gallery.first().cloned();

    // Derive loaders from latest_files_indexes.
    // CurseForge's latest_files_indexes contains entries with
    // gameVersion like "Fabric", "Forge", "Quilt", "NeoForge"
    let mut loaders_set = std::collections::HashSet::new();
    for idx in &cf_mod.latest_files_indexes {
        match idx.game_version.as_str() {
            "Fabric" => {
                loaders_set.insert("fabric");
            }
            "Forge" => {
                loaders_set.insert("forge");
            }
            "Quilt" => {
                loaders_set.insert("quilt");
            }
            "NeoForge" => {
                loaders_set.insert("neoforge");
            }
            "LiteLoader" => {
                loaders_set.insert("liteloader");
            }
            _ => {}
        }
    }
    let loaders: Vec<String> =
        loaders_set.into_iter().map(|s| s.to_string()).collect();

    // Build versions list from game_versions of latest_files
    let mut versions_set = std::collections::HashSet::new();
    for file in &cf_mod.latest_files {
        for gv in &file.game_versions {
            if !NON_VERSION_ENTRIES.contains(&gv.as_str()) {
                versions_set.insert(gv.clone());
            }
        }
    }
    let versions: Vec<String> = versions_set.into_iter().collect();

    let latest_version_id = cf_mod
        .latest_files
        .first()
        .map(|f| f.id.to_string())
        .unwrap_or_default();

    serde_json::json!({
        "project_id": cf_mod.id.to_string(),
        "version_id": latest_version_id,
        "project_types": [project_type],
        "slug": cf_mod.slug,
        "author": author,
        "author_id": author_id,
        "organization": null,
        "organization_id": null,
        "name": cf_mod.name,
        "summary": cf_mod.summary,
        "categories": categories,
        "display_categories": display_categories,
        "downloads": cf_mod.download_count,
        "follows": 0u64,
        "icon_url": icon_url,
        "date_created": cf_mod.date_created,
        "date_modified": cf_mod.date_modified,
        "license": "",
        "gallery": gallery,
        "featured_gallery": featured_gallery,
        "color": null,
        "loaders": loaders,
        "versions": versions,
        // CurseForge-specific extras for the frontend
        "_cf_mod_id": cf_mod.id,
        "_cf_class_id": class_id.as_i32(),
        "_cf_page_url": cf_mod.links.website_url,
        "_cf_download_count": cf_mod.download_count,
    })
}

/// Convert a CurseForge file to a Modrinth-version-compatible JSON value.
/// Used by the frontend version picker.
pub fn convert_file_to_version(cf_file: &CfFile) -> serde_json::Value {
    let release_type = match cf_file.release_type {
        1 => "release",
        2 => "beta",
        3 => "alpha",
        _ => "release",
    };

    // Filter out loader names and Java versions from game_versions
    let game_versions: Vec<String> = cf_file
        .game_versions
        .iter()
        .filter(|gv| !NON_VERSION_ENTRIES.contains(&gv.as_str()))
        .cloned()
        .collect();

    // Extract loaders from game_versions
    let loaders: Vec<String> = cf_file
        .game_versions
        .iter()
        .filter_map(|gv| match gv.as_str() {
            "Fabric" => Some("fabric"),
            "Forge" => Some("forge"),
            "Quilt" => Some("quilt"),
            "NeoForge" => Some("neoforge"),
            "LiteLoader" => Some("liteloader"),
            _ => None,
        })
        .map(|s| s.to_string())
        .collect();

    let primary_file_url = cf_file.download_url.clone().unwrap_or_default();

    serde_json::json!({
        "id": cf_file.id.to_string(),
        "project_id": cf_file.mod_id.to_string(),
        "author_id": "",
        "featured": false,
        "name": cf_file.display_name,
        "version_number": cf_file.file_name,
        "changelog": null,
        "changelog_url": null,
        "date_published": cf_file.file_date,
        "downloads": 0,
        "version_type": release_type,
        "status": "listed",
        "files": [{
            "hashes": [],
            "url": primary_file_url,
            "filename": cf_file.file_name,
            "primary": true,
            "size": cf_file.file_length,
            "file_type": null
        }],
        "game_versions": game_versions,
        "loaders": loaders,
        "dependencies": cf_file.dependencies.iter().map(|d| {
            serde_json::json!({
                "version_id": d.file_id.map(|i| i.to_string()),
                "project_id": d.mod_id.to_string(),
                "dependency_type": match d.relation_type {
                    3 => "required",
                    2 => "optional",
                    1 => "embedded",
                    6 => "optional",
                    _ => "optional"
                }
            })
        }).collect::<Vec<_>>(),
        "_cf_file_id": cf_file.id,
        "_cf_mod_id": cf_file.mod_id,
        "_cf_download_url": cf_file.download_url,
        "_cf_file_length": cf_file.file_length,
    })
}
