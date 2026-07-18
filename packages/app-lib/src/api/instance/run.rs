use super::content::get_projects;
use crate::server_address::ServerAddress;
use crate::state::{
    Credentials, InstanceLink, MemoryAllocationMode, MemorySettings,
    ProcessMetadata, Settings, State,
};
use crate::util::fetch;
use crate::util::io::IOError;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum QuickPlayType {
    None,
    Singleplayer(String),
    Server(ServerAddress),
}

#[tracing::instrument]
pub async fn run(
    instance_id: &str,
    quick_play_type: QuickPlayType,
) -> crate::Result<ProcessMetadata> {
    let state = State::get().await?;
    let default_account = Credentials::get_default_credential(&state.pool)
        .await?
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError.as_error())?;

    run_credentials(instance_id, &default_account, quick_play_type).await
}

#[tracing::instrument(skip(credentials))]
async fn run_credentials(
    instance_id: &str,
    credentials: &Credentials,
    quick_play_type: QuickPlayType,
) -> crate::Result<ProcessMetadata> {
    let state = State::get().await?;
    let settings = Settings::get(&state.pool).await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to run a nonexistent instance {instance_id}!"
            ))
        })?;

    let pre_launch_hooks = context
        .launch_overrides
        .hooks
        .pre_launch
        .as_ref()
        .or(settings.hooks.pre_launch.as_ref())
        .filter(|hook_command| !hook_command.is_empty());
    if let Some(hook) = pre_launch_hooks {
        let mut cmd = shlex::split(hook)
            .ok_or_else(|| {
                crate::ErrorKind::LauncherError(format!(
                    "Invalid pre-launch command: {hook}",
                ))
            })?
            .into_iter();

        if let Some(command) = cmd.next() {
            let full_path = crate::util::io::canonicalize(
                state
                    .directories
                    .instances_dir()
                    .join(&context.instance.path),
            )?;

            if settings.pre_launch_wait {
                // Wait for the pre-launch command to complete
                let result = Command::new(command)
                    .args(cmd)
                    .current_dir(&full_path)
                    .spawn()
                    .map_err(|e| IOError::with_path(e, &full_path))?
                    .wait()
                    .await
                    .map_err(IOError::from)?;

                if !result.success() {
                    return Err(crate::ErrorKind::LauncherError(format!(
                        "Non-zero exit code for pre-launch hook: {}",
                        result.code().unwrap_or(-1)
                    ))
                    .as_error());
                }
            } else {
                // Don't wait for the pre-launch command; just spawn it
                Command::new(command)
                    .args(cmd)
                    .current_dir(&full_path)
                    .spawn()
                    .map_err(|e| IOError::with_path(e, &full_path))?;
            }
        }
    }

    let java_args = context
        .launch_overrides
        .extra_launch_args
        .clone()
        .unwrap_or_else(|| settings.extra_launch_args.clone());
    let wrapper = context
        .launch_overrides
        .hooks
        .wrapper
        .clone()
        .or(settings.hooks.wrapper.clone())
        .filter(|hook_command| !hook_command.is_empty());
    // Resolve effective memory allocation mode:
    // instance override (Auto/Custom) takes precedence; FollowGlobal or unset falls back to global.
    let effective_mode = match context.launch_overrides.memory_allocation_mode {
        Some(MemoryAllocationMode::FollowGlobal) | None => {
            settings.memory_allocation_mode
        }
        Some(mode) => mode,
    };

    let memory = match effective_mode {
        MemoryAllocationMode::Auto => MemorySettings {
            maximum: calculate_auto_memory(&state, &context)
                .unwrap_or(settings.memory.maximum),
        },
        MemoryAllocationMode::Custom | MemoryAllocationMode::FollowGlobal => {
            context.launch_overrides.memory.unwrap_or(settings.memory)
        }
    };
    let resolution = context
        .launch_overrides
        .game_resolution
        .unwrap_or(settings.game_resolution);
    let env_args = context
        .launch_overrides
        .custom_env_vars
        .clone()
        .unwrap_or_else(|| settings.custom_env_vars.clone());
    let post_exit_hook = context
        .launch_overrides
        .hooks
        .post_exit
        .clone()
        .or(settings.hooks.post_exit.clone())
        .filter(|hook_command| !hook_command.is_empty());

    let mut mc_set_options: Vec<(String, String)> = vec![];
    if let Some(fullscreen) = context.launch_overrides.force_fullscreen {
        mc_set_options.push(("fullscreen".to_string(), fullscreen.to_string()));
    } else if settings.force_fullscreen {
        mc_set_options.push(("fullscreen".to_string(), "true".to_string()));
    }

    if let Some(project_id) = server_play_project_id(&context.link)
        && !project_id.trim().is_empty()
    {
        let server_id = uuid::Uuid::new_v4().to_string();
        let join_result = fetch::INSECURE_REQWEST_CLIENT
			.post("https://sessionserver.mojang.com/session/minecraft/join")
			.json(&json!({
				"accessToken": &credentials.access_token,
				"selectedProfile": credentials.offline_profile.id.simple().to_string(),
				"serverId": &server_id,
			}))
			.timeout(Duration::from_secs(5))
			.send()
			.await;

        match join_result {
            Ok(resp) if resp.status().is_success() => {
                let result = fetch::post_json(
                    concat!(
                        env!("MODRINTH_API_BASE_URL"),
                        "analytics/minecraft-server-play"
                    ),
                    json!({
                        "project_id": project_id,
                        "username": &credentials.offline_profile.name,
                        "server_id": &server_id,
                    }),
                    &state.api_semaphore,
                    &state.pool,
                )
                .await;

                match result {
                    Ok(()) => {
                        info!(
                            "Tracked server play for '{project_id}' in analytics"
                        )
                    }
                    Err(err) => warn!("Failed to report server play: {err:?}"),
                }
            }
            Ok(resp) => warn!(
                "Failed to join Mojang session server: HTTP {}",
                resp.status()
            ),
            Err(err) => warn!("Failed to join Mojang session server: {err:?}"),
        }
    }

    crate::minecraft_skins::flush_pending_skin_change().await?;
    crate::launcher::launch_minecraft(
        &java_args,
        &env_args,
        &mc_set_options,
        &wrapper,
        &memory,
        &resolution,
        credentials,
        post_exit_hook,
        &context,
        quick_play_type,
        &settings,
    )
    .await
}

fn server_play_project_id(link: &InstanceLink) -> Option<&String> {
    match link {
        InstanceLink::ServerProject { project_id }
        | InstanceLink::ServerProjectModpack {
            server_project_id: project_id,
            ..
        } => Some(project_id),
        InstanceLink::Unmanaged
        | InstanceLink::ModrinthModpack { .. }
        | InstanceLink::ModrinthHosting { .. }
        | InstanceLink::ImportedModpack { .. }
        | InstanceLink::SharedInstance { .. } => None,
    }
}

pub async fn kill(instance_id: &str) -> crate::Result<()> {
    let state = State::get().await?;
    let processes =
        crate::api::process::get_by_instance_id(instance_id).await?;

    for process in processes {
        state.process_manager.kill(process.uuid).await?;
    }

    Ok(())
}

#[tracing::instrument]
pub async fn try_update_playtime_by_instance_id(
    instance_id: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to update playtime for nonexistent instance {instance_id}!"
            ))
        })?;
    let updated_recent_playtime = context.instance.recent_time_played;
    let res = if updated_recent_playtime > 0 {
        let modrinth_pack_version_id = match &context.link {
            InstanceLink::ModrinthModpack { version_id, .. }
            | InstanceLink::ServerProjectModpack {
                content_version_id: version_id,
                ..
            }
            | InstanceLink::ImportedModpack {
                version_id: Some(version_id),
                ..
            } => Some(version_id.clone()),
            InstanceLink::Unmanaged
            | InstanceLink::ServerProject { .. }
            | InstanceLink::ModrinthHosting { .. }
            | InstanceLink::ImportedModpack { .. }
            | InstanceLink::SharedInstance { .. } => None,
        };
        let playtime_update_json = json!({
            "seconds": updated_recent_playtime,
            "loader": context.applied_content_set.loader.as_str(),
            "game_version": &context.applied_content_set.game_version,
            "parent": modrinth_pack_version_id,
        });
        let mut hashmap: HashMap<String, serde_json::Value> = HashMap::new();

        for (_, project) in get_projects(instance_id, None).await? {
            if let Some(metadata) = project.metadata {
                hashmap
                    .insert(metadata.version_id, playtime_update_json.clone());
            }
        }

        fetch::post_json(
            concat!(env!("MODRINTH_API_BASE_URL"), "analytics/playtime"),
            serde_json::to_value(hashmap)?,
            &state.api_semaphore,
            &state.pool,
        )
        .await
    } else {
        Ok(())
    };

    if res.is_ok() {
        crate::state::instances::commands::mark_instance_playtime_submitted(
            &context.instance.id,
            updated_recent_playtime,
            &state.pool,
        )
        .await?;
    }

    res
}

/// Calculate automatic memory allocation based on available RAM, instance type, and mod count.
/// Port of PCL-CE's GetRam algorithm. Returns the allocation in MiB.
fn calculate_auto_memory(
    state: &State,
    context: &crate::state::InstanceLaunchContext,
) -> Option<u32> {
    const BYTES_PER_GIB: f64 = 1024.0 * 1024.0 * 1024.0;

    let ram_available_gb = {
        let bytes = crate::api::jre::available_memory_bytes();
        (bytes as f64 / BYTES_PER_GIB * 10.0).round() / 10.0
    };

    let instance_path = state
        .directories
        .instances_dir()
        .join(&context.instance.path);
    let mods_dir = instance_path.join("mods");

    let mod_count: u32 = std::fs::read_dir(&mods_dir)
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type()
                        .map(|t| t.is_file())
                        .unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0);

    let has_optifine = std::fs::read_dir(&mods_dir)
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type()
                        .map(|t| t.is_file())
                        .unwrap_or(false)
                })
                .any(|e| {
                    e.file_name()
                        .to_str()
                        .map(|name| name.to_lowercase().contains("optifine"))
                        .unwrap_or(false)
                })
        })
        .unwrap_or(false);

    let is_modded = !matches!(
        context.applied_content_set.loader,
        crate::state::ModLoader::Vanilla
    );

    let mod_count_f = mod_count as f64;
    let (ram_minimum_gb, ram_target1_gb, ram_target2_gb, ram_target3_gb) = if is_modded {
        (
            0.5 + mod_count_f / 150.0,
            1.5 + mod_count_f / 90.0,
            2.7 + mod_count_f / 50.0,
            4.5 + mod_count_f / 25.0,
        )
    } else if has_optifine {
        (0.5, 1.5, 3.0, 5.0)
    } else {
        (0.5, 1.5, 2.5, 4.0)
    };

    let mut ram_give_gb = 0.0;
    let mut ram_available_gb = ram_available_gb;

    let stages = [
        (ram_target1_gb, 1.0),
        (ram_target2_gb - ram_target1_gb, 0.7),
        (ram_target3_gb - ram_target2_gb, 0.4),
        (ram_target3_gb, 0.15),
    ];

    for (delta, ratio) in stages {
        ram_give_gb += (ram_available_gb * ratio).min(delta);
        ram_available_gb -= delta / ratio;
        if ram_available_gb < 0.1 {
            break;
        }
    }

    let ram_give_gb = (ram_give_gb.max(ram_minimum_gb) * 10.0).round() / 10.0;
    Some((ram_give_gb * 1024.0) as u32)
}
