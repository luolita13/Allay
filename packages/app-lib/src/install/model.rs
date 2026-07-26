use crate::api::pack::import::ImportLauncherType;
use crate::api::pack::install_from::{CreatePackInstance, CreatePackLocation};
use crate::state::{
    InstanceInstallStage, InstanceLink, InstanceMetadata, ModLoader,
};
use chrono::{DateTime, Utc};
use modrinth_content_management::ResolveContentPlan;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub type InstallModpackPreview = CreatePackInstance;

// ---------------------------------------------------------------------------
// InstallJobState — core persistent state for an install job
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobState {
    pub schema_version: u32,
    pub request: InstallRequest,
    pub target: InstallTarget,
    pub cleanup: InstallCleanup,
    pub progress: InstallProgressState,
    pub paths: InstallJobPaths,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<InstallErrorContext>,
    #[serde(default)]
    pub events: Vec<InstallJobEvent>,
    #[serde(default)]
    pub display: Option<InstallJobDisplay>,
    pub rollback: Option<InstallRollbackState>,
    pub error: Option<InstallErrorView>,
    #[serde(default)]
    pub rollback_error: Option<InstallErrorView>,
}

impl InstallJobState {
    pub fn new(request: InstallRequest) -> Self {
        let target = request.target();
        let cleanup = request.cleanup();
        let kind = request.kind();
        let phase = InstallPhaseId::PreparingInstance;

        Self {
            schema_version: 1,
            request,
            target,
            cleanup,
            progress: InstallProgressState {
                phase,
                progress: None,
                details: InstallPhaseDetails::Empty,
                last_progress_at: None,
                last_progress_bytes: 0,
                speed_bytes_per_second: None,
            },
            paths: InstallJobPaths::default(),
            context: None,
            events: vec![InstallJobEvent {
                at: Utc::now(),
                kind: InstallJobEventKind::JobQueued { kind },
            }],
            display: None,
            rollback: None,
            error: None,
            rollback_error: None,
        }
    }

    pub fn record_event(&mut self, kind: InstallJobEventKind) {
        self.events.push(InstallJobEvent {
            at: Utc::now(),
            kind,
        });
    }

    pub fn set_context(&mut self, context: Option<InstallErrorContext>) {
        self.context = context;
    }

    pub fn set_progress(
        &mut self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) {
        if self.progress.phase != phase
            || matches!(&self.progress.details, InstallPhaseDetails::Empty)
                && !matches!(&details, InstallPhaseDetails::Empty)
        {
            self.record_event(InstallJobEventKind::PhaseStarted {
                phase,
                details: details.clone(),
            });
        }

        // Compute rolling-window speed from the delta between this update
        // and the previous one, rather than the global average since phase
        // start. This gives the user a real-time transfer rate that reflects
        // current network conditions, not a smoothed average.
        if let Some(ref new_progress) = progress {
            let now = Utc::now();
            let current_bytes = self
                .progress
                .progress
                .as_ref()
                .and_then(|p| p.secondary.as_ref())
                .map_or(new_progress.current, |sec| sec.current);

            if let Some(last_at) = self.progress.last_progress_at {
                let elapsed_ms = now
                    .signed_duration_since(last_at)
                    .num_milliseconds()
                    .max(1) as u64;
                let delta = current_bytes.saturating_sub(
                    self.progress.last_progress_bytes,
                );
                if delta > 0 && elapsed_ms > 0 {
                    self.progress.speed_bytes_per_second =
                        Some(delta.saturating_mul(1_000) / elapsed_ms);
                }
            }

            self.progress.last_progress_at = Some(now);
            self.progress.last_progress_bytes = current_bytes;
        }

        self.progress.phase = phase;
        self.progress.progress = progress;
        self.progress.details = details;
    }

    pub fn instance_deleted(&self) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.kind,
                InstallJobEventKind::TargetInstanceDeleted { .. }
            )
        })
    }

    pub fn provider(&self) -> InstallJobProvider {
        match &self.request {
            InstallRequest::CreateModpackInstance { location, .. }
            | InstallRequest::InstallPackToExistingInstance {
                location, ..
            } => match location {
                CreatePackLocation::FromVersionId { .. } => {
                    InstallJobProvider::Modrinth
                }
                CreatePackLocation::FromFile { .. } => {
                    InstallJobProvider::Local
                }
            },
            InstallRequest::CreateInstance { .. } => {
                InstallJobProvider::Minecraft
            }
            InstallRequest::InstallExistingInstance { .. } => {
                InstallJobProvider::Minecraft
            }
            InstallRequest::ImportInstance { .. }
            | InstallRequest::DuplicateInstance { .. } => {
                InstallJobProvider::Local
            }
            InstallRequest::InstallContent { .. } => {
                InstallJobProvider::Modrinth
            }
            InstallRequest::InstallCurseForgeFile { .. } => {
                InstallJobProvider::CurseForge
            }
        }
    }

    pub fn download_items(&self) -> Vec<DownloadItemSnapshot> {
        let mut items = Vec::<DownloadItemSnapshot>::new();
        for event in &self.events {
            match &event.kind {
                InstallJobEventKind::ContentFileDownloadAttempt {
                    path,
                    bytes_total,
                    attempt,
                    max_attempts,
                } => {
                    if let Some(item) =
                        items.iter_mut().find(|item| item.id == *path)
                    {
                        item.status = DownloadItemStatus::Downloading;
                        item.bytes_downloaded = 0;
                        item.bytes_total = *bytes_total;
                        item.attempt = Some(*attempt);
                        item.max_attempts = Some(*max_attempts);
                        item.error = None;
                    } else {
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: None,
                            version_id: None,
                            status: DownloadItemStatus::Downloading,
                            bytes_downloaded: 0,
                            bytes_total: *bytes_total,
                            attempt: Some(*attempt),
                            max_attempts: Some(*max_attempts),
                            error: None,
                            manual_url: None,
                        });
                    }
                }
                InstallJobEventKind::ContentFileCompleted { path, bytes } => {
                    if let Some(item) =
                        items.iter_mut().find(|item| item.id == *path)
                    {
                        item.status = DownloadItemStatus::Completed;
                        item.bytes_downloaded = *bytes;
                        item.bytes_total = Some(*bytes);
                        item.error = None;
                    } else {
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: None,
                            version_id: None,
                            status: DownloadItemStatus::Completed,
                            bytes_downloaded: *bytes,
                            bytes_total: Some(*bytes),
                            attempt: None,
                            max_attempts: None,
                            error: None,
                            manual_url: None,
                        });
                    }
                }
                InstallJobEventKind::ContentFileSkipped {
                    path,
                    reason,
                    project_id,
                    version_id,
                    manual_url,
                } => {
                    if let Some(item) =
                        items.iter_mut().find(|item| item.id == *path)
                    {
                        item.status = DownloadItemStatus::Skipped;
                        item.bytes_downloaded = 0;
                        item.project_id = project_id.clone();
                        item.version_id = version_id.clone();
                        item.error = Some(reason.clone());
                        item.manual_url = manual_url.clone();
                    } else {
                        items.push(DownloadItemSnapshot {
                            id: path.clone(),
                            name: path.clone(),
                            project_id: project_id.clone(),
                            version_id: version_id.clone(),
                            status: DownloadItemStatus::Skipped,
                            bytes_downloaded: 0,
                            bytes_total: None,
                            attempt: None,
                            max_attempts: None,
                            error: Some(reason.clone()),
                            manual_url: manual_url.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
        items
    }

    pub fn download_summary(&self) -> DownloadJobSummary {
        let mut summary = DownloadJobSummary::default();
        for event in &self.events {
            match &event.kind {
                InstallJobEventKind::ContentDownloadStarted {
                    files,
                    bytes,
                } => {
                    summary.files_total = Some(*files);
                    summary.bytes_total = *bytes;
                }
                InstallJobEventKind::ContentFileCompleted { bytes, .. } => {
                    summary.files_completed += 1;
                    summary.bytes_downloaded =
                        summary.bytes_downloaded.saturating_add(*bytes);
                }
                InstallJobEventKind::ContentFileSkipped { .. } => {
                    summary.files_completed += 1;
                }
                InstallJobEventKind::DownloadMetrics {
                    source,
                    fallback_count,
                } => {
                    summary.source = Some(source.clone());
                    summary.fallback_count =
                        summary.fallback_count.saturating_add(*fallback_count);
                }
                _ => {}
            }
        }
        if let Some(progress) = &self.progress.progress {
            if self.progress.phase == InstallPhaseId::DownloadingContent {
                summary.files_completed = progress.current;
                summary.files_total = Some(progress.total);
                if let Some(bytes) = &progress.secondary {
                    summary.bytes_downloaded = bytes.current;
                    summary.bytes_total = Some(bytes.total);
                }
            } else if self.progress.phase
                == InstallPhaseId::DownloadingMinecraft
                || self.progress.phase == InstallPhaseId::DownloadingPackFile
                || matches!(
                    &self.progress.details,
                    InstallPhaseDetails::Java {
                        step: InstallJavaStep::Downloading,
                        ..
                    }
                )
            {
                summary.bytes_downloaded = progress.current;
                summary.bytes_total = Some(progress.total);
            }
        }
        let actively_downloading = matches!(
            self.progress.phase,
            InstallPhaseId::DownloadingPackFile
                | InstallPhaseId::DownloadingContent
                | InstallPhaseId::DownloadingMinecraft
        ) || matches!(
            &self.progress.details,
            InstallPhaseDetails::Java {
                step: InstallJavaStep::Downloading,
                ..
            }
        );
        if actively_downloading && summary.bytes_downloaded > 0 {
            // Prefer the rolling-window instantaneous speed computed in
            // set_progress(); fall back to the phase-average only if no
            // speed sample has been recorded yet.
            summary.speed_bytes_per_second =
                self.progress.speed_bytes_per_second.or_else(|| {
                    let phase_started =
                        self.events.iter().rev().find_map(|event| {
                            matches!(
                                &event.kind,
                                InstallJobEventKind::PhaseStarted {
                                    phase, ..
                                } if *phase == self.progress.phase
                            )
                            .then_some(event.at)
                        });
                    phase_started.and_then(|started| {
                        let elapsed_ms = Utc::now()
                            .signed_duration_since(started)
                            .num_milliseconds()
                            .max(1) as u64;
                        let speed = summary
                            .bytes_downloaded
                            .saturating_mul(1_000)
                            .checked_div(elapsed_ms)
                            .unwrap_or(0);
                        (speed > 0).then_some(speed)
                    })
                });

            if let Some(speed) = summary.speed_bytes_per_second {
                summary.eta_seconds =
                    summary.bytes_total.and_then(|total| {
                        total
                            .saturating_sub(summary.bytes_downloaded)
                            .checked_add(speed - 1)
                            .and_then(|remaining| {
                                remaining.checked_div(speed)
                            })
                    });
            }
        }
        summary
    }
}

// ---------------------------------------------------------------------------
// InstallJobEvent / InstallJobEventKind — event sourcing types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobEvent {
    pub at: DateTime<Utc>,
    pub kind: InstallJobEventKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallInterruptReason {
    AppClosed,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallJobEventKind {
    JobQueued {
        kind: InstallJobKind,
    },
    JobStarted,
    JobSucceeded {
        instance_id: Option<String>,
    },
    JobCanceled {
        phase: InstallPhaseId,
    },
    PhaseStarted {
        phase: InstallPhaseId,
        details: InstallPhaseDetails,
    },
    ContentDownloadStarted {
        files: u64,
        bytes: Option<u64>,
    },
    ContentFileDownloadAttempt {
        path: String,
        bytes_total: Option<u64>,
        attempt: u32,
        max_attempts: u32,
    },
    ContentFileSkipped {
        path: String,
        reason: String,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(default)]
        version_id: Option<String>,
        #[serde(default)]
        manual_url: Option<String>,
    },
    ContentFileCompleted {
        path: String,
        bytes: u64,
    },
    DownloadMetrics {
        source: String,
        fallback_count: u64,
    },
    TargetInstanceDeleted {
        instance_id: String,
    },
    Interrupted {
        reason: InstallInterruptReason,
        phase: InstallPhaseId,
    },
    Failed {
        phase: InstallPhaseId,
        code: String,
        message: String,
    },
    RollbackStarted {
        cleanup: InstallCleanup,
    },
    RollbackCompleted,
    RollbackFailed {
        message: String,
    },
}

// ---------------------------------------------------------------------------
// InstallJobProvider — categorises the source of an install job
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobProvider {
    Modrinth,
    CurseForge,
    Minecraft,
    Java,
    Application,
    Local,
}

impl InstallJobProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::CurseForge => "curse_forge",
            Self::Minecraft => "minecraft",
            Self::Java => "java",
            Self::Application => "application",
            Self::Local => "local",
        }
    }
}

// ---------------------------------------------------------------------------
// DownloadItemStatus / DownloadItemSnapshot — per-file download tracking
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadItemStatus {
    Queued,
    Downloading,
    Verifying,
    Writing,
    WaitingForUser,
    Completed,
    Skipped,
    Failed,
    Canceled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadItemSnapshot {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    pub status: DownloadItemStatus,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    #[serde(default)]
    pub attempt: Option<u32>,
    #[serde(default)]
    pub max_attempts: Option<u32>,
    pub error: Option<String>,
    pub manual_url: Option<String>,
}

// ---------------------------------------------------------------------------
// DownloadJobSummary — aggregate download telemetry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DownloadJobSummary {
    pub files_completed: u64,
    pub files_total: Option<u64>,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub speed_bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub source: Option<String>,
    pub fallback_count: u64,
}

// ---------------------------------------------------------------------------
// InstallErrorContext — rich structured error context (no bon dependency)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallErrorContext {
    pub operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

// ---------------------------------------------------------------------------
// InstallApiErrorDetails — API-specific error details
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallApiErrorDetails {
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

// ---------------------------------------------------------------------------
// InstallRequest — what to install (code-specific variants preserved)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallRequest {
    CreateInstance {
        name: String,
        game_version: String,
        loader: ModLoader,
        loader_version: Option<String>,
        icon_path: Option<String>,
        link: InstanceLink,
    },
    CreateModpackInstance {
        location: CreatePackLocation,
        #[serde(default)]
        post_install_edit: Option<InstallPostInstallEdit>,
    },
    ImportInstance {
        launcher_type: ImportLauncherType,
        base_path: PathBuf,
        instance_folder: String,
    },
    DuplicateInstance {
        source_instance_id: String,
    },
    InstallExistingInstance {
        instance_id: String,
        force: bool,
    },
    InstallPackToExistingInstance {
        instance_id: String,
        location: CreatePackLocation,
        #[serde(default)]
        post_install_edit: Option<InstallPostInstallEdit>,
    },
    /// Install mods/resourcepacks/datapacks/shaders to an existing instance.
    /// Goes through the InstallJob system so that progress is reported to the
    /// ActionBar download popup (same as modpack installs).
    InstallContent {
        instance_id: String,
        plan: ResolveContentPlan,
    },
    /// Install a single CurseForge file (mod/resourcepack/shader/datapack/world)
    /// to an existing instance. Downloads directly from CurseForge CDN and
    /// places the file in the appropriate folder based on content_type.
    InstallCurseForgeFile {
        instance_id: String,
        mod_id: i64,
        file_id: i64,
        file_name: String,
        download_url: Option<String>,
        /// "mod" | "resourcepack" | "shader" | "datapack" | "world" | "modpack"
        content_type: String,
        /// Display title (project name) for the install popup
        title: String,
        /// Optional icon URL for the install popup
        icon_url: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstallPostInstallEdit {
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::double_option"
    )]
    pub icon_path: Option<Option<String>>,
    pub link: Option<InstanceLink>,
}

impl InstallRequest {
    pub fn kind(&self) -> InstallJobKind {
        match self {
            Self::CreateInstance { .. } => InstallJobKind::CreateInstance,
            Self::CreateModpackInstance { .. } => {
                InstallJobKind::CreateModpackInstance
            }
            Self::ImportInstance { .. } => InstallJobKind::ImportInstance,
            Self::DuplicateInstance { .. } => InstallJobKind::DuplicateInstance,
            Self::InstallExistingInstance { .. } => {
                InstallJobKind::InstallExistingInstance
            }
            Self::InstallPackToExistingInstance { .. } => {
                InstallJobKind::InstallPackToExistingInstance
            }
            Self::InstallContent { .. } => InstallJobKind::InstallContent,
            Self::InstallCurseForgeFile { .. } => {
                InstallJobKind::InstallCurseForgeFile
            }
        }
    }

    pub fn target(&self) -> InstallTarget {
        match self {
            Self::InstallExistingInstance { instance_id, .. }
            | Self::InstallPackToExistingInstance { instance_id, .. }
            | Self::InstallContent { instance_id, .. }
            | Self::InstallCurseForgeFile { instance_id, .. } => {
                InstallTarget::ExistingInstance {
                    instance_id: instance_id.clone(),
                }
            }
            _ => InstallTarget::NewInstance { instance_id: None },
        }
    }

    pub fn cleanup(&self) -> InstallCleanup {
        match self {
            Self::InstallExistingInstance { instance_id, .. }
            | Self::InstallPackToExistingInstance { instance_id, .. }
            | Self::InstallContent { instance_id, .. }
            | Self::InstallCurseForgeFile { instance_id, .. } => {
                InstallCleanup::RestoreExistingInstance {
                    instance_id: instance_id.clone(),
                }
            }
            _ => InstallCleanup::DeleteNewInstance { instance_id: None },
        }
    }
}

// ---------------------------------------------------------------------------
// InstallJobKind — categorises the type of install job
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobKind {
    CreateInstance,
    CreateModpackInstance,
    ImportInstance,
    DuplicateInstance,
    InstallExistingInstance,
    InstallPackToExistingInstance,
    InstallContent,
    InstallCurseForgeFile,
}

impl InstallJobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreateInstance => "create_instance",
            Self::CreateModpackInstance => "create_modpack_instance",
            Self::ImportInstance => "import_instance",
            Self::DuplicateInstance => "duplicate_instance",
            Self::InstallExistingInstance => "install_existing_instance",
            Self::InstallPackToExistingInstance => {
                "install_pack_to_existing_instance"
            }
            Self::InstallContent => "install_content",
            Self::InstallCurseForgeFile => "install_curseforge_file",
        }
    }

    pub fn from_stored_str(value: &str) -> Self {
        match value {
            "create_modpack_instance" => Self::CreateModpackInstance,
            "import_instance" => Self::ImportInstance,
            "duplicate_instance" => Self::DuplicateInstance,
            "install_existing_instance" => Self::InstallExistingInstance,
            "install_pack_to_existing_instance" => {
                Self::InstallPackToExistingInstance
            }
            "install_content" => Self::InstallContent,
            "install_curseforge_file" => Self::InstallCurseForgeFile,
            _ => Self::CreateInstance,
        }
    }
}

// ---------------------------------------------------------------------------
// InstallJobStatus — lifecycle status (Paused + Canceling + WaitingForUser)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJobStatus {
    Queued,
    Running,
    Canceling,
    WaitingForUser,
    Paused,
    Succeeded,
    Failed,
    Interrupted,
    Canceled,
}

impl InstallJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Canceling => "canceling",
            Self::WaitingForUser => "waiting_for_user",
            Self::Paused => "paused",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::Canceled => "canceled",
        }
    }

    pub fn from_stored_str(value: &str) -> Self {
        match value {
            "running" => Self::Running,
            "canceling" => Self::Canceling,
            "waiting_for_user" => Self::WaitingForUser,
            "paused" => Self::Paused,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "interrupted" => Self::Interrupted,
            "canceled" => Self::Canceled,
            _ => Self::Queued,
        }
    }

    pub fn is_finished(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Interrupted | Self::Canceled
        )
    }
}

// ---------------------------------------------------------------------------
// InstallTarget / InstallCleanup
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallTarget {
    NewInstance { instance_id: Option<String> },
    ExistingInstance { instance_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallCleanup {
    DeleteNewInstance { instance_id: Option<String> },
    RestoreExistingInstance { instance_id: String },
}

// ---------------------------------------------------------------------------
// InstallProgress / phase types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallProgressState {
    pub phase: InstallPhaseId,
    pub progress: Option<InstallProgress>,
    pub details: InstallPhaseDetails,
    /// Timestamp of the most recent progress update (for speed calculation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_progress_at: Option<DateTime<Utc>>,
    /// Bytes downloaded at the most recent progress update
    #[serde(default)]
    pub last_progress_bytes: u64,
    /// Rolling-window download speed (bytes/sec), computed from recent updates
    #[serde(default)]
    pub speed_bytes_per_second: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhaseId {
    PreparingInstance,
    ResolvingPack,
    DownloadingPackFile,
    ReadingPackManifest,
    DownloadingContent,
    ExtractingOverrides,
    ResolvingMinecraft,
    ResolvingLoader,
    PreparingJava,
    DownloadingMinecraft,
    RunningLoaderProcessors,
    Finalizing,
    RollingBack,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallProgress {
    pub current: u64,
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<InstallProgressSecondary>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallProgressSecondary {
    pub current: u64,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallJavaStep {
    Resolving,
    FetchingMetadata,
    Downloading,
    Extracting,
    Validating,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallPhaseDetails {
    Empty,
    Instance {
        name: String,
    },
    Minecraft {
        game_version: String,
        loader: ModLoader,
    },
    Java {
        major_version: u32,
        step: InstallJavaStep,
    },
    Modpack {
        project_id: Option<String>,
        version_id: Option<String>,
        title: Option<String>,
    },
    Import {
        launcher_type: ImportLauncherType,
        instance_folder: String,
    },
}

// ---------------------------------------------------------------------------
// InstallJobPaths / InstallJobDisplay / InstallRollbackState
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstallJobPaths {
    pub staging_dir: Option<PathBuf>,
    pub final_instance_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobDisplay {
    pub title: String,
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallRollbackState {
    pub instance: InstanceMetadata,
    pub install_stage: InstanceInstallStage,
}

// ---------------------------------------------------------------------------
// InstallErrorView — user-facing error with phase / api / context
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallErrorView {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<InstallPhaseId>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<InstallApiErrorDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<InstallErrorContext>,
}

impl InstallErrorView {
    pub fn from_error(
        code: &str,
        phase: InstallPhaseId,
        error: impl ToString,
    ) -> Self {
        Self {
            code: code.to_string(),
            phase: Some(phase),
            message: error.to_string(),
            api: None,
            context: None,
        }
    }

    pub fn from_message(
        code: &str,
        phase: InstallPhaseId,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_string(),
            phase: Some(phase),
            message: message.into(),
            api: None,
            context: None,
        }
    }
}

// ---------------------------------------------------------------------------
// InstallJobSnapshot — read-only view exposed to the frontend
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallJobSnapshot {
    pub job_id: Uuid,
    pub instance_id: Option<String>,
    pub instance_deleted: bool,
    pub kind: InstallJobKind,
    pub status: InstallJobStatus,
    pub provider: InstallJobProvider,
    pub target: InstallTarget,
    pub phase: InstallPhaseId,
    pub progress: Option<InstallProgress>,
    pub details: InstallPhaseDetails,
    pub display: Option<InstallJobDisplay>,
    pub error: Option<InstallErrorView>,
    pub rollback_error: Option<InstallErrorView>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub summary: DownloadJobSummary,
    pub items: Vec<DownloadItemSnapshot>,
}
