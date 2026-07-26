use crate::api::Result;
use theseus::prelude::crash_diagnosis::{
    AutoFixKind, CrashDiagnosisResult, CrashLogSource, CrashSeverity,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("crash-diagnosis")
        .invoke_handler(tauri::generate_handler![
            crash_diagnosis_diagnose_latest,
            crash_diagnosis_diagnose_raw_log,
            crash_diagnosis_list_rules,
            crash_diagnosis_list_severities,
            crash_diagnosis_list_auto_fix_kinds,
            crash_diagnosis_export_report,
            crash_diagnosis_apply_auto_fix,
            crash_diagnosis_cleanup_logs,
        ])
        .build()
}

/// Diagnose the most recent crash report (within `max_age_secs`) for the given instance.
///
/// `max_age_secs` is optional; pass `None` to consider any crash report regardless of age.
#[tauri::command]
pub async fn crash_diagnosis_diagnose_latest(
    instance_id: &str,
    max_age_secs: Option<u64>,
) -> Result<CrashDiagnosisResult> {
    Ok(theseus::prelude::crash_diagnosis::diagnose_latest_crash(
        instance_id,
        max_age_secs,
    )
    .await?)
}

/// Diagnose an arbitrary log string (e.g. user-pasted crash report).
#[tauri::command]
pub fn crash_diagnosis_diagnose_raw_log(log: String) -> Result<CrashDiagnosisResult> {
    Ok(theseus::prelude::crash_diagnosis::diagnose_raw_log(&log))
}

/// List all available rule ids - useful for diagnostic UI / debugging.
#[tauri::command]
pub fn crash_diagnosis_list_rules() -> Result<Vec<&'static str>> {
    Ok(theseus::prelude::crash_diagnosis::list_rule_ids())
}

/// Enumerate all possible severity values (for frontend enum generation).
#[tauri::command]
pub fn crash_diagnosis_list_severities() -> Result<Vec<CrashSeverity>> {
    Ok(vec![
        CrashSeverity::Fatal,
        CrashSeverity::Warning,
        CrashSeverity::Info,
    ])
}

/// Enumerate all possible auto-fix kinds (for frontend enum generation).
#[tauri::command]
pub fn crash_diagnosis_list_auto_fix_kinds() -> Result<Vec<AutoFixKind>> {
    Ok(vec![
        AutoFixKind::None,
        AutoFixKind::InstallJava(17),
        AutoFixKind::InstallJava(21),
        AutoFixKind::ReinstallInstance,
        AutoFixKind::ClearMods,
        AutoFixKind::UpdateGraphicsDriver,
        AutoFixKind::IncreaseMemory,
        AutoFixKind::OpenModsFolder,
        AutoFixKind::OpenInstanceSettings,
    ])
}

/// Export a crash report ZIP bundle for the given instance.
/// Returns the path to the generated ZIP file.
#[tauri::command]
pub async fn crash_diagnosis_export_report(
    instance_id: &str,
) -> Result<String> {
    Ok(theseus::prelude::crash_diagnosis::export_crash_report(
        instance_id,
    )
    .await?)
}

/// Apply an auto-fix action for the given instance.
/// Returns a user-readable message describing what was done.
#[tauri::command]
pub async fn crash_diagnosis_apply_auto_fix(
    instance_id: &str,
    auto_fix: AutoFixKind,
) -> Result<String> {
    Ok(theseus::prelude::crash_diagnosis::apply_auto_fix(
        instance_id,
        &auto_fix,
    )
    .await?)
}

/// Run log cleanup to prevent infinite log growth.
#[tauri::command]
pub async fn crash_diagnosis_cleanup_logs() -> Result<()> {
    theseus::prelude::crash_diagnosis::cleanup_old_logs().await;
    Ok(())
}
