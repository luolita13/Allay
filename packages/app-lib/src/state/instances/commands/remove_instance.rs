use crate::install::{self, store as install_store};
use crate::state::State;
use crate::state::instances::adapters::sqlite::instance_rows;
use crate::util::io;

pub(crate) async fn remove_instance(
    instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;

    // Prevent deletion while the instance is still launching or running.
    // On Windows the game process locks files in the instance directory, so
    // removing it now would produce an OS error 32 (sharing violation) and
    // leave the UI in an inconsistent state.
    if state.process_manager.is_instance_running(&instance.id) {
        return Err(crate::ErrorKind::InputError(format!(
            "Cannot delete '{}' while Minecraft is still running. Please close the game and try again.",
            instance.name
        )).into());
    }

    // Cancel any active install/download jobs tied to this instance so the
    // Downloads page does not keep showing a running job after the instance
    // (and its files) are gone.
    match install_store::list_active_by_instance_id(&instance.id, state).await {
        Ok(jobs) => {
            for job in jobs {
                tracing::info!(
                    "Canceling active install job {} for deleted instance {}",
                    job.id,
                    instance.id
                );
                install::control::request_cancel(&job.id);
            }
        }
        Err(error) => {
            tracing::error!(
                "Failed to list active install jobs for instance {}: {error}",
                instance.id
            );
        }
    }

    instance_rows::delete_instance_by_id(&instance.id, &state.pool).await?;

    let path = state.directories.instances_dir().join(&instance.path);
    if path.exists() {
        io::remove_dir_all(&path).await?;
    }

    Ok(())
}
