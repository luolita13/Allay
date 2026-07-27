use crate::State;
use crate::state::CachedEntry;
pub use daedalus::minecraft::VersionManifest;
pub use daedalus::modded::Manifest;
use daedalus::minecraft;
use reqwest::Method;

#[tracing::instrument]
pub async fn get_minecraft_versions() -> crate::Result<VersionManifest> {
    let state = State::get().await?;
    let mut minecraft_versions = CachedEntry::get_minecraft_manifest(
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::NoValueFor("minecraft versions".to_string())
    })?;

    // Try to fetch UVMC (Unlisted Versions of Minecraft) manifest for
    // April Fools and other unlisted versions
    if let Ok(uvmc_manifest) = crate::util::fetch::fetch_json::<minecraft::VersionManifest>(
        Method::GET,
        minecraft::UVMC_MANIFEST_URL,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await
    {
        let existing_ids: std::collections::HashSet<String> =
            minecraft_versions.versions.iter().map(|v| v.id.clone()).collect();

        for version in uvmc_manifest.versions {
            if existing_ids.contains(&version.id) {
                continue;
            }
            minecraft_versions.versions.push(version);
        }
    }

    // Classify known April Fools versions
    for version in &mut minecraft_versions.versions {
        if minecraft::is_april_fools_version(&version.id) {
            version.type_ = minecraft::VersionType::AprilFools;
        }
    }

    Ok(minecraft_versions)
}

// #[tracing::instrument]
pub async fn get_loader_versions(loader: &str) -> crate::Result<Manifest> {
    let state = State::get().await?;
    let cache_key =
        daedalus::modded::loader_manifest_metadata(loader).cache_key;
    let loaders = CachedEntry::get_loader_manifest(
        &cache_key,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::NoValueFor(format!("{loader} loader versions"))
    })?;

    Ok(loaders.manifest)
}
