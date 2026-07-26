use super::control::{self, JobGuard};
use super::events::{InstallProgressReporter, emit_install_job};
use super::model::{
    InstallCleanup, InstallErrorView, InstallJobDisplay, InstallJobEvent,
    InstallJobEventKind, InstallJobSnapshot,
    InstallJobState, InstallJobStatus, InstallPhaseDetails, InstallPhaseId,
    InstallPostInstallEdit, InstallRequest, InstallRollbackState,
    InstallTarget,
};
use super::{recovery, store};
use crate::ErrorKind;
use chrono::Utc;
use crate::api::pack::install_from::{
    CreatePackLocation, generate_pack_from_file,
    generate_pack_from_version_id_with_reporter, get_instance_from_pack,
};
use crate::api::pack::install_mrpack::install_zipped_mrpack_files_with_reporter;
use crate::event::InstancePayloadType;
use crate::event::emit::emit_instance;
use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::{
    ContentSourceKind, InstanceInstallStage, InstanceLink, ModLoader, State,
};
use crate::util::fetch::DownloadReason;
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

pub async fn create_instance(
    name: String,
    game_version: String,
    loader: ModLoader,
    loader_version: Option<String>,
    icon_path: Option<String>,
    link: InstanceLink,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::CreateInstance {
        name,
        game_version,
        loader,
        loader_version,
        icon_path,
        link,
    })
    .await
}

pub async fn create_modpack_instance(
    location: CreatePackLocation,
    post_install_edit: Option<InstallPostInstallEdit>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::CreateModpackInstance {
        location,
        post_install_edit,
    })
    .await
}

pub async fn import_instance(
    launcher_type: crate::api::pack::import::ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::ImportInstance {
        launcher_type,
        base_path,
        instance_folder,
    })
    .await
}

pub async fn duplicate_instance(
    source_instance_id: String,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::DuplicateInstance { source_instance_id }).await
}

pub async fn install_existing_instance(
    instance_id: String,
    force: bool,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallExistingInstance { instance_id, force }).await
}

pub async fn install_pack_to_existing_instance(
    instance_id: String,
    location: CreatePackLocation,
    post_install_edit: Option<InstallPostInstallEdit>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallPackToExistingInstance {
        instance_id,
        location,
        post_install_edit,
    })
    .await
}

/// Install content (mods/resourcepacks/datapacks/shaders) to an existing instance.
/// Goes through the InstallJob system for progress reporting.
pub async fn install_content_to_instance(
    instance_id: String,
    plan: modrinth_content_management::ResolveContentPlan,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallContent {
        instance_id,
        plan,
    })
    .await
}

/// Install a single CurseForge file to an existing instance.
/// Goes through the InstallJob system so the download popup shows progress.
pub async fn install_curseforge_file(
    instance_id: String,
    mod_id: i64,
    file_id: i64,
    file_name: String,
    download_url: Option<String>,
    content_type: String,
    title: String,
    icon_url: Option<String>,
) -> crate::Result<InstallJobSnapshot> {
    start(InstallRequest::InstallCurseForgeFile {
        instance_id,
        mod_id,
        file_id,
        file_name,
        download_url,
        content_type,
        title,
        icon_url,
    })
    .await
}

pub async fn list_jobs(
    include_finished: bool,
) -> crate::Result<Vec<InstallJobSnapshot>> {
    let state = State::get().await?;
    Ok(store::list(include_finished, &state)
        .await?
        .into_iter()
        .map(|job| job.snapshot())
        .collect())
}

pub async fn get_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    Ok(store::get_required(job_id, &state).await?.snapshot())
}

pub async fn retry_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let mut job = store::get_required(job_id, &state).await?;

    if !matches!(
        job.status,
        InstallJobStatus::Failed | InstallJobStatus::Interrupted
    ) {
        return Err(crate::ErrorKind::InputError(
            "Only failed or interrupted install jobs can be retried"
                .to_string(),
        )
        .into());
    }

    job.state.target = job.state.request.target();
    job.state.cleanup = job.state.request.cleanup();
    job.state.rollback = None;
    job.state.error = None;
    job.state.progress.phase = InstallPhaseId::PreparingInstance;
    job.state.progress.progress = None;
    job.state.progress.details = InstallPhaseDetails::Empty;
    job.state.events = vec![InstallJobEvent {
        at: Utc::now(),
        kind: InstallJobEventKind::JobQueued {
            kind: job.state.request.kind(),
        },
    }];
    job.state.context = None;
    job.state.rollback_error = None;
    prepare_initial_instance(&mut job.state, &state).await?;

    let record = store::update_status(
        job_id,
        InstallJobStatus::Queued,
        &job.state,
        &state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await?;
    spawn_job(job_id);

    Ok(record.snapshot())
}

pub async fn cancel_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;

    match job.status {
        InstallJobStatus::Queued => {
            // Queued job: immediately mark as canceled
            let mut job_state = job.state.clone();
            job_state.error = Some(InstallErrorView {
                code: "canceled".to_string(),
                phase: None,
                message: "Install was canceled".to_string(),
                api: None,
                context: None,
            });
            recovery::apply_cleanup(&job_state, &state).await?;
            clear_deleted_new_instance_id(&mut job_state);
            let record = store::update_status(
                job_id,
                InstallJobStatus::Canceled,
                &job_state,
                &state,
            )
            .await?;
            emit_install_job(&record.snapshot()).await?;
            control::unregister_job(&job_id);
            Ok(record.snapshot())
        }
        InstallJobStatus::Running | InstallJobStatus::Paused => {
            // Running/Paused job: trigger cancellation token.
            // The running task will detect this and gracefully stop,
            // then update its own status to Canceled.
            if control::request_cancel(&job_id) {
                // Also resume if paused so it can process the cancellation
                if job.status == InstallJobStatus::Paused {
                    control::request_resume(&job_id);
                }
                // Return the current snapshot (status will be updated by the runner)
                Ok(job.snapshot())
            } else {
                // No control handle: the job is stale (e.g. app crashed while running).
                // Mark it as canceled immediately.
                let mut job_state = job.state.clone();
                job_state.error = Some(InstallErrorView {
                    code: "canceled".to_string(),
                    phase: None,
                    message: "Install was canceled".to_string(),
                    api: None,
                    context: None,
                });
                clear_deleted_new_instance_id(&mut job_state);
                let record = store::update_status(
                    job_id,
                    InstallJobStatus::Canceled,
                    &job_state,
                    &state,
                )
                .await?;
                emit_install_job(&record.snapshot()).await?;
                Ok(record.snapshot())
            }
        }
        _ => Err(crate::ErrorKind::InputError(
            "Only queued, running, or paused install jobs can be canceled"
                .to_string(),
        )
        .into()),
    }
}

pub async fn pause_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;

    if job.status != InstallJobStatus::Running {
        return Err(crate::ErrorKind::InputError(
            "Only running install jobs can be paused".to_string(),
        )
        .into());
    }

    // If the control handle is missing, the job is stale (e.g. app crashed).
    // Mark it as paused in the database anyway so the user can retry/cancel.
    control::request_pause(&job_id);

    let record = store::update_status(
        job_id,
        InstallJobStatus::Paused,
        &job.state,
        &state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await?;

    Ok(record.snapshot())
}

pub async fn resume_job(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;

    if job.status != InstallJobStatus::Paused {
        return Err(crate::ErrorKind::InputError(
            "Only paused install jobs can be resumed".to_string(),
        )
        .into());
    }

    // If the control handle is missing, the job is stale. Update the database
    // so the UI doesn't get stuck, but it cannot actually resume.
    control::request_resume(&job_id);

    let record = store::update_status(
        job_id,
        InstallJobStatus::Running,
        &job.state,
        &state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await?;

    Ok(record.snapshot())
}

pub async fn dismiss_job(job_id: Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    store::dismiss(job_id, &state).await
}

async fn start(request: InstallRequest) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let id = Uuid::new_v4();
    let mut job_state = InstallJobState::new(request);
    prepare_initial_instance(&mut job_state, &state).await?;
    let record =
        store::insert(id, &job_state, InstallJobStatus::Queued, &state).await?;
    emit_install_job(&record.snapshot()).await?;
    spawn_job(id);
    Ok(record.snapshot())
}

async fn prepare_initial_instance(
    job_state: &mut InstallJobState,
    state: &State,
) -> crate::Result<()> {
    match job_state.request.clone() {
        InstallRequest::CreateInstance {
            name,
            game_version,
            loader,
            loader_version,
            icon_path,
            link,
        } => {
            let metadata = crate::api::instance::create(
                name,
                game_version,
                loader,
                loader_version,
                icon_path,
                link,
            )
            .await?;
            set_display(
                job_state,
                metadata.instance.name,
                metadata.instance.icon_path,
            );
            set_instance_id(job_state, metadata.instance.id);
        }
        InstallRequest::CreateModpackInstance {
            location,
            post_install_edit,
        } => {
            let preview = get_instance_from_pack(location).await?;
            let name = post_install_edit
                .as_ref()
                .and_then(|edit| edit.name.clone())
                .unwrap_or_else(|| preview.name.clone());
            let icon_path = match post_install_edit
                .as_ref()
                .and_then(|edit| edit.icon_path.as_ref())
            {
                Some(icon_path) => icon_path.clone(),
                None => preview
                    .icon
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| preview.icon_url.clone()),
            };
            let link = post_install_edit
                .as_ref()
                .and_then(|edit| edit.link.clone())
                .or_else(|| preview.link.clone())
                .unwrap_or(InstanceLink::Unmanaged);
            let metadata = crate::api::instance::create(
                name,
                preview.game_version,
                preview.modloader,
                preview.loader_version,
                icon_path,
                link,
            )
            .await?;
            set_display(
                job_state,
                metadata.instance.name,
                metadata.instance.icon_path,
            );
            set_instance_id(job_state, metadata.instance.id);
        }
        InstallRequest::ImportInstance {
            instance_folder, ..
        } => {
            let metadata = crate::api::instance::create(
                instance_folder,
                "1.19.4".to_string(),
                ModLoader::Vanilla,
                Some("latest".to_string()),
                None,
                InstanceLink::Unmanaged,
            )
            .await?;
            set_display(
                job_state,
                metadata.instance.name,
                metadata.instance.icon_path,
            );
            set_instance_id(job_state, metadata.instance.id);
        }
        InstallRequest::DuplicateInstance { source_instance_id } => {
            let metadata =
                crate::state::get_instance(&source_instance_id, &state.pool)
                    .await?
                    .ok_or_else(|| {
                        crate::ErrorKind::InputError(
                            "Unknown instance".to_string(),
                        )
                    })?;
            let created = crate::api::instance::create(
                metadata.instance.name,
                metadata.applied_content_set.game_version,
                metadata.applied_content_set.loader,
                metadata.applied_content_set.loader_version,
                metadata.instance.icon_path,
                metadata.link,
            )
            .await?;
            set_display(
                job_state,
                created.instance.name,
                created.instance.icon_path,
            );
            set_instance_id(job_state, created.instance.id);
        }
        InstallRequest::InstallExistingInstance { instance_id, .. }
        | InstallRequest::InstallPackToExistingInstance {
            instance_id, ..
        }
        | InstallRequest::InstallContent { instance_id, .. }
        | InstallRequest::InstallCurseForgeFile { instance_id, .. } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
        }
    }

    Ok(())
}

fn spawn_job(job_id: Uuid) {
    // Register control handles before spawning
    let _control = control::register_job(job_id);
    tokio::spawn(async move {
        if let Err(error) = run_job(job_id).await {
            tracing::error!(
                "Install job {job_id} failed to update state: {error}"
            );
        }
    });
}

async fn run_job(job_id: Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;

    if job.status != InstallJobStatus::Queued {
        return Ok(());
    }

    let mut job_state = job.state.clone();
    job_state.record_event(InstallJobEventKind::JobStarted);
    let record = store::update_status(
        job_id,
        InstallJobStatus::Running,
        &job_state,
        &state,
    )
    .await?;
    emit_install_job(&record.snapshot()).await?;

    // Create a guard for cancellation/pause checking
    let mut guard = control::get_control(&job_id)
        .map(|c| JobGuard::new(&c))
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Install job control handle not found".to_string(),
            )
        })?;

    // Run the request while also listening for cancellation.
    // This ensures that even if a network I/O step blocks for a long time
    // without hitting a guard.check() point, the job still responds to cancel.
    let cancel_token = control::get_control(&job_id)
        .map(|c| c.cancel.clone());
    let result = if let Some(token) = cancel_token {
        tokio::select! {
            res = run_request(job_id, &mut job_state, &state, &mut guard) => res,
            _ = token.cancelled() => {
                Err(crate::ErrorKind::InputError(
                    "Install was canceled".to_string(),
                ).into())
            }
        }
    } else {
        run_request(job_id, &mut job_state, &state, &mut guard).await
    };

    // Always unregister control when done
    control::unregister_job(&job_id);

    match result {
        Ok(instance_id) => {
            // Reload the latest job_state from the database so we preserve
            // any events (ContentFileCompleted, etc.) that the
            // InstallProgressReporter persisted during the install.
            if let Ok(latest) = store::get_required(job_id, &state).await {
                job_state = latest.state.clone();
            }
            if let Some(instance_id) = instance_id {
                set_instance_id(&mut job_state, instance_id);
            }
            job_state.progress.phase = InstallPhaseId::Finalizing;
            job_state.progress.progress = None;
            job_state.progress.details = InstallPhaseDetails::Empty;
            job_state.error = None;
            job_state.record_event(InstallJobEventKind::JobSucceeded {
                instance_id: current_instance_id(&job_state),
            });
            let record = store::update_status(
                job_id,
                InstallJobStatus::Succeeded,
                &job_state,
                &state,
            )
            .await?;
            emit_install_job(&record.snapshot()).await?;
        }
        Err(error) => {
            // Check if the error is due to cancellation
            let is_canceled = control::get_control(&job_id)
                .map_or(false, |c| c.cancel.is_cancelled())
                || matches!(
                    error.raw.as_ref(),
                    crate::ErrorKind::InputError(msg) if msg == "Install was canceled"
                );
            // Reload the latest job_state from the database. The
            // InstallProgressReporter holds its own clone of job_state and
            // persists events (ContentFileDownloadAttempt, ContentFileCompleted,
            // etc.) directly to the database via store::update_state. The local
            // job_state variable in run_job does NOT see those events. If we
            // were to write our local job_state back to the database without
            // reloading, we would overwrite and lose all the file download
            // events that were recorded during the install.
            if let Ok(latest) = store::get_required(job_id, &state).await {
                job_state = latest.state.clone();
            }
            if is_canceled {
                job_state.progress.phase = InstallPhaseId::RollingBack;
                job_state.progress.progress = None;
                job_state.progress.details = InstallPhaseDetails::Empty;
                job_state.error = Some(InstallErrorView {
                    code: "canceled".to_string(),
                    phase: None,
                    message: "Install was canceled".to_string(),
                    api: None,
                    context: None,
                });
                job_state.record_event(InstallJobEventKind::JobCanceled {
                    phase: job_state.progress.phase,
                });
                recovery::apply_cleanup(&job_state, &state).await?;
                clear_deleted_new_instance_id(&mut job_state);
                let record = store::update_status(
                    job_id,
                    InstallJobStatus::Canceled,
                    &job_state,
                    &state,
                )
                .await?;
                emit_install_job(&record.snapshot()).await?;
            } else {
                job_state.progress.phase = InstallPhaseId::RollingBack;
                job_state.progress.progress = None;
                job_state.progress.details = InstallPhaseDetails::Empty;
                job_state.error = Some(install_error_view(&error, job_state.progress.phase));
                job_state.record_event(InstallJobEventKind::Failed {
                    phase: job_state.progress.phase,
                    code: "install_error".to_string(),
                    message: error.to_string(),
                });
                recovery::apply_cleanup(&job_state, &state).await?;
                clear_deleted_new_instance_id(&mut job_state);
                let record = store::update_status(
                    job_id,
                    InstallJobStatus::Failed,
                    &job_state,
                    &state,
                )
                .await?;
                emit_install_job(&record.snapshot()).await?;
                return Err(error);
            }
        }
    }

    Ok(())
}

async fn run_request(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
    guard: &mut JobGuard,
) -> crate::Result<Option<String>> {
    match job_state.request.clone() {
        InstallRequest::CreateInstance {
            name,
            game_version,
            loader,
            loader_version: _,
            icon_path: _,
            link: _,
        } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingInstance,
                InstallPhaseDetails::Instance { name: name.clone() },
            )
            .await?;
            guard.check().await?;
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingMinecraft,
                InstallPhaseDetails::Minecraft {
                    game_version,
                    loader,
                },
            )
            .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            guard.check().await?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                false,
                Some(InstallProgressReporter::with_guard(job_id, job_state.clone(), guard.clone())),
            )
            .await?;
            guard.check().await?;
            Ok(Some(instance_id))
        }
        InstallRequest::CreateModpackInstance {
            location,
            post_install_edit,
        } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::ResolvingPack,
                modpack_details(&location),
            )
            .await?;
            install_pack(
                job_id,
                job_state,
                location,
                instance_id.clone(),
                DownloadReason::Modpack,
                guard.clone(),
            )
            .await?;
            apply_post_install_edit(&instance_id, post_install_edit).await?;
            Ok(Some(instance_id))
        }
        InstallRequest::ImportInstance {
            launcher_type,
            base_path,
            instance_folder,
        } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingInstance,
                InstallPhaseDetails::Import {
                    launcher_type,
                    instance_folder: instance_folder.clone(),
                },
            )
            .await?;
            crate::api::pack::import::import_instance_with_reporter(
                &instance_id,
                launcher_type,
                base_path,
                instance_folder,
                InstallProgressReporter::with_guard(job_id, job_state.clone(), guard.clone()),
            )
            .await?;
            Ok(Some(instance_id))
        }
        InstallRequest::DuplicateInstance { source_instance_id } => {
            let Some(instance_id) = current_instance_id(job_state) else {
                return Err(crate::ErrorKind::InputError(
                    "Install job is missing its instance id".to_string(),
                )
                .into());
            };
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::PreparingInstance,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let state = State::get().await?;
            crate::api::pack::import::copy_dotminecraft_with_reporter(
                &instance_id,
                crate::api::instance::get_full_path(&source_instance_id)
                    .await?,
                &state.io_semaphore,
                InstallProgressReporter::with_guard(job_id, job_state.clone(), guard.clone()),
                InstallPhaseDetails::Empty,
            )
            .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                false,
                Some(InstallProgressReporter::with_guard(job_id, job_state.clone(), guard.clone())),
            )
            .await?;
            emit_instance(&instance_id, InstancePayloadType::Edited).await?;
            Ok(Some(instance_id))
        }
        InstallRequest::InstallExistingInstance { instance_id, force } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingMinecraft,
                InstallPhaseDetails::Empty,
            )
            .await?;
            let context =
                crate::state::instances::commands::get_instance_launch_context(
                    &instance_id,
                    &state.pool,
                )
                .await?
                .ok_or_else(|| {
                    crate::ErrorKind::InputError("Unknown instance".to_string())
                })?;
            crate::launcher::install_minecraft_with_reporter(
                &context,
                force,
                Some(InstallProgressReporter::with_guard(job_id, job_state.clone(), guard.clone())),
            )
            .await?;
            Ok(Some(instance_id))
        }
        InstallRequest::InstallPackToExistingInstance {
            instance_id,
            location,
            post_install_edit,
        } => {
            prepare_existing_rollback(job_state, state, &instance_id).await?;
            let disabled_project_ids = remove_existing_pack_content(
                job_id,
                job_state,
                state,
                &instance_id,
                &guard,
            )
            .await?;
            install_pack(
                job_id,
                job_state,
                location,
                instance_id.clone(),
                DownloadReason::Modpack,
                guard.clone(),
            )
            .await?;
            restore_disabled_projects(
                &instance_id,
                disabled_project_ids,
                state,
            )
            .await?;
            apply_post_install_edit(&instance_id, post_install_edit).await?;
            Ok(Some(instance_id))
        }
        InstallRequest::InstallContent { instance_id, plan } => {
            use crate::install::model::{
                InstallPhaseDetails, InstallPhaseId,
            };

            // Collect project ids for completion/failure events
            let project_ids = {
                let mut ids =
                    Vec::with_capacity(plan.dependencies.len() + 1);
                ids.push(plan.primary.project_id.clone());
                ids.extend(
                    plan.dependencies
                        .iter()
                        .map(|d| d.project_id.clone()),
                );
                ids
            };

            // Initial progress report
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingContent,
                InstallPhaseDetails::Modpack {
                    project_id: Some(plan.primary.project_id.clone()),
                    version_id: Some(plan.primary.version_id.clone()),
                    title: None,
                },
            )
            .await?;
            guard.check().await?;

            let reporter = InstallProgressReporter::with_guard(
                job_id,
                job_state.clone(),
                guard.clone(),
            );
            let install_result = crate::state::instances::commands::install_resolved_content_plan_with_reporter(
                &instance_id,
                &plan,
                state,
                &reporter,
            )
            .await;

            match install_result {
                Ok(()) => {
                    // Emit ContentInstallFinished so frontend removes placeholders
                    if let Err(e) = crate::event::emit::emit_instance(
                        &instance_id,
                        crate::event::InstancePayloadType::ContentInstallFinished {
                            project_ids: project_ids.clone(),
                        },
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to emit content install finished event: {e}"
                        );
                    }
                    if let Err(e) = crate::event::emit::emit_instance(
                        &instance_id,
                        crate::event::InstancePayloadType::Edited,
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to emit instance edited event after content install: {e}"
                        );
                    }
                    Ok(Some(instance_id))
                }
                Err(error) => {
                    // Emit ContentInstallFailed so frontend can clean up placeholders
                    if let Err(e) = crate::event::emit::emit_instance(
                        &instance_id,
                        crate::event::InstancePayloadType::ContentInstallFailed {
                            project_ids,
                            message: error.to_string(),
                        },
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to emit content install failed event: {e}"
                        );
                    }
                    Err(error)
                }
            }
        }
        InstallRequest::InstallCurseForgeFile {
            instance_id,
            mod_id,
            file_id,
            file_name,
            download_url,
            content_type,
            title,
            icon_url,
        } => {
            use crate::install::model::{InstallPhaseDetails, InstallPhaseId};
            use std::io::Read;

            // The "project id" we use for frontend compatibility is the
            // stringified CurseForge mod id (prefixed to avoid collision
            // with Modrinth IDs).
            let project_ids = vec![format!("cf-{}", mod_id)];

            // Set display info for the install popup
            job_state.display = Some(
                crate::install::model::InstallJobDisplay {
                    title: title.clone(),
                    icon: icon_url.clone(),
                },
            );

            // Initial progress report
            update_progress(
                job_id,
                job_state,
                state,
                InstallPhaseId::DownloadingContent,
                InstallPhaseDetails::Modpack {
                    project_id: Some(format!("cf-{}", mod_id)),
                    version_id: Some(format!("cf-{}", file_id)),
                    title: Some(title.clone()),
                },
            )
            .await?;
            guard.check().await?;

            // Resolve download URL: use provided URL or fetch it from CF API
            let download_url = match download_url {
                Some(url) if !url.is_empty() => url,
                _ => {
                    crate::api::curseforge::get_file_download_url(
                        mod_id,
                        file_id,
                        state,
                    )
                    .await?
                    .ok_or_else(|| {
                        crate::ErrorKind::OtherError(format!(
                            "CurseForge file {} has no download URL available",
                            file_id
                        ))
                        .as_error()
                    })?
                }
            };

            tracing::info!(
                "Downloading CurseForge file: {} ({})",
                file_name,
                download_url
            );

            // Download the file bytes using fetch_chunked for mirror support
            // and parallel chunked download on large files. CurseForge does
            // not expose SHA1 hashes in its API, so we pass None for sha1.
            let settings = crate::state::Settings::get(&state.pool).await?;
            let max_chunks = settings.max_chunks_per_file;
            let bytes = crate::util::fetch::fetch_chunked(
                &download_url,
                None,
                None,
                None,
                &state.fetch_semaphore,
                &state.pool,
                max_chunks,
                None,
            )
            .await?;

            guard.check().await?;

            // Get the instance filesystem path
            let instance_path = crate::api::instance::get_full_path(
                &instance_id,
            )
            .await?;

            // Determine target subfolder based on content_type
            let subfolder = match content_type.as_str() {
                "mod" => "mods",
                "resourcepack" => "resourcepacks",
                "shader" => "shaderpacks",
                "datapack" => "datapacks",
                "world" => "saves",
                "modpack" => "mods",
                _ => "mods",
            };

            let target_dir = instance_path.join(subfolder);
            tokio::fs::create_dir_all(&target_dir).await.map_err(|e| {
                crate::ErrorKind::OtherError(format!(
                    "Failed to create {}: {e}",
                    target_dir.display()
                ))
                .as_error()
            })?;

            let target_path = target_dir.join(&file_name);

            // For worlds, extract the zip into saves/<world_name>/
            if content_type == "world" {
                let world_name = file_name
                    .trim_end_matches(".zip")
                    .trim_end_matches(".mcworld");
                let world_dir = target_dir.join(world_name);
                tokio::fs::create_dir_all(&world_dir).await.map_err(|e| {
                    crate::ErrorKind::OtherError(format!(
                        "Failed to create world dir: {e}"
                    ))
                    .as_error()
                })?;

                let cursor = std::io::Cursor::new(bytes.to_vec());
                let mut archive =
                    zip::ZipArchive::new(cursor).map_err(|e| {
                        crate::ErrorKind::OtherError(format!(
                            "Failed to read world zip: {e}"
                        ))
                        .as_error()
                    })?;

                for i in 0..archive.len() {
                    let mut entry = archive
                        .by_index(i)
                        .map_err(|e| {
                            crate::ErrorKind::OtherError(format!(
                                "Zip entry {i}: {e}"
                            ))
                            .as_error()
                        })?;
                    let out_path = match entry.enclosed_name() {
                        Some(p) => world_dir.join(p),
                        None => continue,
                    };
                    if entry.is_dir() {
                        tokio::fs::create_dir_all(&out_path)
                            .await
                            .map_err(|e| {
                                crate::ErrorKind::OtherError(format!(
                                    "mkdir: {e}"
                                ))
                                .as_error()
                            })?;
                    } else {
                        if let Some(parent) = out_path.parent() {
                            tokio::fs::create_dir_all(parent)
                                .await
                                .map_err(|e| {
                                    crate::ErrorKind::OtherError(format!(
                                        "mkdir: {e}"
                                    ))
                                    .as_error()
                                })?;
                        }
                        let mut buf =
                            Vec::with_capacity(entry.size() as usize);
                        entry.read_to_end(&mut buf).map_err(|e| {
                            crate::ErrorKind::OtherError(format!(
                                "read zip entry: {e}"
                            ))
                            .as_error()
                        })?;
                        tokio::fs::write(&out_path, &buf)
                            .await
                            .map_err(|e| {
                                crate::ErrorKind::OtherError(format!(
                                    "write {}: {e}",
                                    out_path.display()
                                ))
                                .as_error()
                            })?;
                    }
                }
                tracing::info!(
                    "Extracted CurseForge world to {}",
                    world_dir.display()
                );
            } else {
                // Just write the file directly
                tokio::fs::write(&target_path, &bytes)
                    .await
                    .map_err(|e| {
                        crate::ErrorKind::OtherError(format!(
                            "Failed to write {}: {e}",
                            target_path.display()
                        ))
                        .as_error()
                    })?;
                tracing::info!(
                    "Wrote CurseForge file to {}",
                    target_path.display()
                );
            }

            // Emit ContentInstallFinished so frontend removes placeholders
            if let Err(e) = crate::event::emit::emit_instance(
                &instance_id,
                crate::event::InstancePayloadType::ContentInstallFinished {
                    project_ids: project_ids.clone(),
                },
            )
            .await
            {
                tracing::error!(
                    "Failed to emit CF content install finished event: {e}"
                );
            }
            if let Err(e) = crate::event::emit::emit_instance(
                &instance_id,
                crate::event::InstancePayloadType::Edited,
            )
            .await
            {
                tracing::error!(
                    "Failed to emit instance edited event after CF install: {e}"
                );
            }

            Ok(Some(instance_id))
        }
    }
}

async fn apply_post_install_edit(
    instance_id: &str,
    edit: Option<InstallPostInstallEdit>,
) -> crate::Result<()> {
    let Some(edit) = edit else {
        return Ok(());
    };

    if edit.name.is_none() && edit.icon_path.is_none() && edit.link.is_none() {
        return Ok(());
    }

    crate::api::instance::edit(
        instance_id,
        crate::state::instances::commands::EditInstance {
            name: edit.name,
            icon_path: edit.icon_path,
            link: edit.link,
            ..Default::default()
        },
    )
    .await?;
    emit_instance(instance_id, InstancePayloadType::Edited).await?;

    Ok(())
}

async fn remove_existing_pack_content(
    job_id: Uuid,
    job_state: &InstallJobState,
    state: &State,
    instance_id: &str,
    guard: &JobGuard,
) -> crate::Result<HashSet<String>> {
    let metadata = crate::state::instances::commands::get_instance_metadata(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    let (project_id, version_id) = match &metadata.link {
        InstanceLink::ModrinthModpack {
            project_id,
            version_id,
        } => (project_id.clone(), version_id.clone()),
        InstanceLink::ServerProjectModpack {
            content_project_id,
            content_version_id,
            ..
        } => (content_project_id.clone(), content_version_id.clone()),
        InstanceLink::ImportedModpack { .. } => {
            remove_existing_imported_pack_content(
                instance_id,
                &metadata,
                state,
            )
            .await?;
            return Ok(HashSet::new());
        }
        _ => return Ok(HashSet::new()),
    };

    let disabled_project_ids =
        crate::state::instances::commands::list_project_files(
            instance_id,
            state,
        )
        .await?
        .into_iter()
        .filter_map(|file| (!file.enabled).then_some(file.project_id?))
        .collect::<HashSet<_>>();
    let reporter = InstallProgressReporter::with_guard(job_id, job_state.clone(), guard.clone());
    let old_pack = generate_pack_from_version_id_with_reporter(
        project_id.clone(),
        version_id.clone(),
        metadata.instance.name.clone(),
        None,
        instance_id.to_string(),
        DownloadReason::Update,
        reporter,
    )
    .await?;

    crate::api::pack::install_mrpack::remove_all_related_files(
        instance_id.to_string(),
        old_pack.file,
    )
    .await?;

    Ok(disabled_project_ids)
}

async fn remove_existing_imported_pack_content(
    instance_id: &str,
    metadata: &crate::state::InstanceMetadata,
    state: &State,
) -> crate::Result<()> {
    let entries = content_rows::get_content_entries(
        &metadata.applied_content_set.id,
        &state.pool,
    )
    .await?;
    let files = content_rows::get_instance_files(instance_id, &state.pool)
        .await?
        .into_iter()
        .map(|file| (file.id.clone(), file))
        .collect::<std::collections::HashMap<_, _>>();
    let base = state
        .directories
        .instances_dir()
        .join(&metadata.instance.path);

    let mut removed_file_ids = HashSet::new();
    for entry in entries {
        if !matches!(
            entry.source_kind,
            ContentSourceKind::ImportedModpack
                | ContentSourceKind::ModrinthModpack
        ) {
            continue;
        }

        let Some(file_id) = entry.file_id else {
            continue;
        };
        if !removed_file_ids.insert(file_id.clone()) {
            continue;
        }

        let Some(file) = files.get(&file_id) else {
            continue;
        };
        crate::util::io::remove_file(base.join(&file.relative_path)).await?;
        content_rows::remove_content_entries_for_file(
            &metadata.applied_content_set.id,
            &file.id,
            &state.pool,
        )
        .await?;
        content_rows::remove_instance_file_by_relative_path(
            instance_id,
            &file.relative_path,
            &state.pool,
        )
        .await?;
    }

    Ok(())
}

async fn restore_disabled_projects(
    instance_id: &str,
    disabled_project_ids: HashSet<String>,
    state: &State,
) -> crate::Result<()> {
    if disabled_project_ids.is_empty() {
        return Ok(());
    }

    for file in crate::state::instances::commands::list_project_files(
        instance_id,
        state,
    )
    .await?
    {
        if file.enabled
            && let Some(project_id) = &file.project_id
            && disabled_project_ids.contains(project_id)
        {
            crate::state::instances::commands::toggle_disable_project(
                instance_id,
                &file.relative_path,
                Some(false),
                state,
            )
            .await?;
        }
    }

    Ok(())
}

async fn install_pack(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    location: CreatePackLocation,
    instance_id: String,
    reason: DownloadReason,
    guard: JobGuard,
) -> crate::Result<()> {
    let reporter = InstallProgressReporter::with_guard(job_id, job_state.clone(), guard);
    reporter
        .update(
            InstallPhaseId::DownloadingPackFile,
            None,
            modpack_details(&location),
        )
        .await?;

    let create_pack = match location {
        CreatePackLocation::FromVersionId {
            project_id,
            version_id,
            title,
            icon_url,
        } => {
            generate_pack_from_version_id_with_reporter(
                project_id,
                version_id,
                title,
                icon_url,
                instance_id.clone(),
                reason,
                reporter.clone(),
            )
            .await?
        }
        CreatePackLocation::FromFile { path } => {
            generate_pack_from_file(path, instance_id.clone()).await?
        }
    };

    install_zipped_mrpack_files_with_reporter(
        create_pack,
        false,
        reason,
        reporter,
    )
    .await?;

    Ok(())
}

async fn prepare_existing_rollback(
    job_state: &mut InstallJobState,
    state: &State,
    instance_id: &str,
) -> crate::Result<()> {
    if job_state.rollback.is_some() {
        return Ok(());
    }

    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unknown instance {instance_id}"
            ))
        })?;
    let install_stage = instance.instance.install_stage;
    set_display(
        job_state,
        instance.instance.name.clone(),
        instance.instance.icon_path.clone(),
    );
    job_state.rollback = Some(InstallRollbackState {
        instance,
        install_stage,
    });
    job_state.cleanup = InstallCleanup::RestoreExistingInstance {
        instance_id: instance_id.to_string(),
    };

    crate::state::instances::commands::set_instance_install_stage(
        instance_id,
        InstanceInstallStage::MinecraftInstalling,
        &state.pool,
    )
    .await?;
    emit_instance(instance_id, InstancePayloadType::Edited).await?;

    Ok(())
}

async fn update_progress(
    job_id: Uuid,
    job_state: &mut InstallJobState,
    state: &State,
    phase: InstallPhaseId,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    job_state.set_progress(phase, None, details);
    let record = store::update_state(job_id, job_state, state).await?;
    emit_install_job(&record.snapshot()).await?;
    Ok(())
}

fn set_instance_id(job_state: &mut InstallJobState, instance_id: String) {
    job_state.target = match &job_state.target {
        InstallTarget::ExistingInstance { .. } => {
            InstallTarget::ExistingInstance {
                instance_id: instance_id.clone(),
            }
        }
        InstallTarget::NewInstance { .. } => InstallTarget::NewInstance {
            instance_id: Some(instance_id.clone()),
        },
    };
    job_state.cleanup = match &job_state.cleanup {
        InstallCleanup::RestoreExistingInstance { .. } => {
            InstallCleanup::RestoreExistingInstance { instance_id }
        }
        InstallCleanup::DeleteNewInstance { .. } => {
            InstallCleanup::DeleteNewInstance {
                instance_id: Some(instance_id),
            }
        }
    };
}

fn clear_deleted_new_instance_id(job_state: &mut InstallJobState) {
    if matches!(job_state.cleanup, InstallCleanup::DeleteNewInstance { .. }) {
        job_state.target = InstallTarget::NewInstance { instance_id: None };
        job_state.cleanup =
            InstallCleanup::DeleteNewInstance { instance_id: None };
    }
}

fn set_display(
    job_state: &mut InstallJobState,
    title: String,
    icon: Option<String>,
) {
    job_state.display = Some(InstallJobDisplay { title, icon });
}

fn install_error_view(error: &crate::Error, phase: InstallPhaseId) -> InstallErrorView {
    match error.raw.as_ref() {
        ErrorKind::FetchError(_)
        | ErrorKind::ApiIsDownError(_)
        | ErrorKind::WSError(_)
        | ErrorKind::WSClosedError(_) => InstallErrorView::from_message(
            "network_error",
            phase,
            "network_error",
        ),
        _ => InstallErrorView::from_message(
            "unknown_error",
            phase,
            "unknown_error",
        ),
    }
}

fn current_instance_id(job_state: &InstallJobState) -> Option<String> {
    match &job_state.target {
        InstallTarget::NewInstance { instance_id } => instance_id.clone(),
        InstallTarget::ExistingInstance { instance_id } => {
            Some(instance_id.clone())
        }
    }
}

fn modpack_details(location: &CreatePackLocation) -> InstallPhaseDetails {
    match location {
        CreatePackLocation::FromVersionId {
            project_id,
            version_id,
            title,
            ..
        } => InstallPhaseDetails::Modpack {
            project_id: Some(project_id.clone()),
            version_id: Some(version_id.clone()),
            title: Some(title.clone()),
        },
        CreatePackLocation::FromFile { .. } => InstallPhaseDetails::Modpack {
            project_id: None,
            version_id: None,
            title: None,
        },
    }
}

pub async fn job_support_details(job_id: Uuid) -> crate::Result<String> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;
    super::diagnostics::build_job_support_details(&job, &state).await
}

pub async fn retry_job_as_new(job_id: Uuid) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let job = store::get_required(job_id, &state).await?;

    if !matches!(
        job.status,
        InstallJobStatus::Failed | InstallJobStatus::Interrupted | InstallJobStatus::Canceled
    ) {
        return Err(crate::ErrorKind::InputError(
            "Only failed, interrupted, or canceled install jobs can be retried"
                .to_string(),
        )
        .into());
    }

    // Start a new job with the same request
    start(job.state.request.clone()).await
}

pub async fn clear_job_history() -> crate::Result<u64> {
    let state = State::get().await?;
    store::clear_finished(&state).await
}
