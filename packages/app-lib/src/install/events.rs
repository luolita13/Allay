use super::control::JobGuard;
use super::model::{
    InstallJobEventKind, InstallJobSnapshot, InstallJobState,
    InstallJobStatus, InstallPhaseDetails, InstallPhaseId, InstallProgress,
};
use super::store;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct InstallProgressReporter {
    job_id: Uuid,
    state: Arc<Mutex<InstallJobState>>,
    guard: Arc<Mutex<Option<JobGuard>>>,
}

impl std::fmt::Debug for InstallProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstallProgressReporter")
            .field("job_id", &self.job_id)
            .finish_non_exhaustive()
    }
}

impl InstallProgressReporter {
    pub fn new(job_id: Uuid, state: InstallJobState) -> Self {
        Self {
            job_id,
            state: Arc::new(Mutex::new(state)),
            guard: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_guard(job_id: Uuid, state: InstallJobState, guard: JobGuard) -> Self {
        Self {
            job_id,
            state: Arc::new(Mutex::new(state)),
            guard: Arc::new(Mutex::new(Some(guard))),
        }
    }

    /// Check for cancellation/pause without emitting a progress update.
    /// Useful in throttled paths where we skip the full emit but still
    /// want to respond promptly to a cancel signal.
    pub async fn check_cancel(&self) -> crate::Result<()> {
        if let Some(guard) = self.guard.lock().await.as_mut() {
            guard.check().await?;
        }
        Ok(())
    }

    pub async fn update(
        &self,
        phase: InstallPhaseId,
        progress: Option<InstallProgress>,
        details: InstallPhaseDetails,
    ) -> crate::Result<()> {
        // Check for cancellation/pause on every progress update
        self.check_cancel().await?;

        let app_state = crate::State::get().await?;

        // Build snapshot from in-memory state while holding the lock briefly,
        // then emit it to the frontend immediately (before the DB write) so
        // progress / speed updates are visible in real-time.
        let snapshot = {
            let mut state = self.state.lock().await;
            state.set_progress(phase, progress, details);
            self.build_snapshot(&state).await
        };
        // Ignore emit errors — the Tauri event is best-effort for UI,
        // but the DB persist is what matters for correctness.
        let _ = emit_install_job(&snapshot).await;

        // Persist to DB after the emit — the DB write is critical for
        // durability but should not delay the frontend update.
        let state = self.state.lock().await;
        let _record =
            store::update_state(self.job_id, &state, &app_state).await?;
        Ok(())
    }

    /// Record an event into the job state and persist + emit.
    /// Emits the snapshot to the frontend BEFORE writing to the DB so
    /// per-file status updates appear in real-time during downloads.
    pub async fn record_event(
        &self,
        kind: InstallJobEventKind,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;

        // Build snapshot from in-memory state, emit immediately
        let snapshot = {
            let mut state = self.state.lock().await;
            state.record_event(kind);
            self.build_snapshot(&state).await
        };
        let _ = emit_install_job(&snapshot).await;

        // Persist to DB after emit
        let state = self.state.lock().await;
        let _record =
            store::update_state(self.job_id, &state, &app_state).await?;
        Ok(())
    }

    /// Build an InstallJobSnapshot from in-memory state without a DB read.
    /// This allows emitting progress events before the DB write completes.
    /// Timestamps and instance_id are approximate — the DB-backed record
    /// provides authoritative values for history/persistence.
    async fn build_snapshot(
        &self,
        state: &InstallJobState,
    ) -> InstallJobSnapshot {
        let now = chrono::Utc::now();

        InstallJobSnapshot {
            job_id: self.job_id,
            // Instance ID is not stored in state; the frontend will
            // use the value from the initial DB-backed snapshot.
            instance_id: None,
            instance_deleted: state.instance_deleted(),
            kind: state.request.kind(),
            // During progress updates the job is always running.
            status: InstallJobStatus::Running,
            provider: state.provider(),
            target: state.target.clone(),
            phase: state.progress.phase,
            progress: state.progress.progress.clone(),
            details: state.progress.details.clone(),
            display: state.display.clone(),
            error: state.error.clone(),
            rollback_error: state.rollback_error.clone(),
            created: now,
            modified: now,
            finished: None,
            summary: state.download_summary(),
            items: state.download_items(),
        }
    }
}

#[allow(unused_variables)]
pub async fn emit_install_job(
    snapshot: &InstallJobSnapshot,
) -> crate::Result<()> {
    #[cfg(feature = "tauri")]
    {
        use tauri::Emitter;

        let event_state = crate::EventState::get()?;
        event_state
            .app
            .emit("install_job", snapshot)
            .map_err(crate::event::EventError::from)?;
    }

    Ok(())
}
