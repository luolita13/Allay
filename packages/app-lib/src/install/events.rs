use super::control::JobGuard;
use super::model::{
    InstallJobEventKind, InstallJobSnapshot, InstallJobState,
    InstallPhaseDetails, InstallPhaseId, InstallProgress,
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
        let mut state = self.state.lock().await;
        // Use set_progress which automatically records PhaseStarted events
        state.set_progress(phase, progress, details);

        let record =
            store::update_state(self.job_id, &state, &app_state).await?;
        emit_install_job(&record.snapshot()).await
    }

    /// Record an event into the job state and persist + emit.
    pub async fn record_event(
        &self,
        kind: InstallJobEventKind,
    ) -> crate::Result<()> {
        let app_state = crate::State::get().await?;
        let mut state = self.state.lock().await;
        state.record_event(kind);

        let record =
            store::update_state(self.job_id, &state, &app_state).await?;
        emit_install_job(&record.snapshot()).await
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
