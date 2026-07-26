/**
 * Crash diagnosis API wrappers.
 *
 * Wraps the `crash-diagnosis` Tauri plugin so the frontend can call into the
 * Rust rule-based crash analyzer.
 */
import { invoke } from '@tauri-apps/api/core'

export type CrashSeverity = 'fatal' | 'warning' | 'info'

export type CrashLogSource =
	| 'crash_report'
	| 'minecraft_log'
	| 'hs_err_log'
	| 'launcher_output'
	| 'user_provided'

export type AutoFixKind =
	| 'none'
	| { install_java: number }
	| 'reinstall_instance'
	| 'clear_mods'
	| 'update_graphics_driver'
	| 'increase_memory'
	| 'open_mods_folder'
	| 'open_instance_settings'

export interface CrashDiagnosisMatch {
	rule_id: string
	severity: CrashSeverity
	title: string
	description: string
	fix: string
	auto_fix: AutoFixKind
	fragment: string | null
	mod_files: string[] | null
}

export interface CrashDiagnosisResult {
	matched: CrashDiagnosisMatch[]
	scanned_bytes: number
	has_crash_report_header: boolean
	excerpt: string
	generated_at: number
	sources: CrashLogSource[]
}

/**
 * Diagnose the most recent crash report (within `maxAgeSecs` seconds) for the
 * given instance id.
 *
 * Pass `null` for `maxAgeSecs` to consider any crash report regardless of age.
 */
export async function diagnoseLatestCrash(
	instanceId: string,
	maxAgeSecs: number | null,
): Promise<CrashDiagnosisResult> {
	return await invoke<CrashDiagnosisResult>(
		'plugin:crash-diagnosis|crash_diagnosis_diagnose_latest',
		{ instanceId, maxAgeSecs },
	)
}

/**
 * Diagnose an arbitrary log string (e.g. user-pasted crash report).
 */
export async function diagnoseRawLog(log: string): Promise<CrashDiagnosisResult> {
	return await invoke<CrashDiagnosisResult>(
		'plugin:crash-diagnosis|crash_diagnosis_diagnose_raw_log',
		{ log },
	)
}

/**
 * List all available rule ids - useful for diagnostic UI / debugging.
 */
export async function listRules(): Promise<string[]> {
	return await invoke<string[]>('plugin:crash-diagnosis|crash_diagnosis_list_rules')
}

/**
 * Enumerate all possible severity values.
 */
export async function listSeverities(): Promise<CrashSeverity[]> {
	return await invoke<CrashSeverity[]>(
		'plugin:crash-diagnosis|crash_diagnosis_list_severities',
	)
}

/**
 * Enumerate all possible auto-fix kinds.
 */
export async function listAutoFixKinds(): Promise<AutoFixKind[]> {
	return await invoke<AutoFixKind[]>(
		'plugin:crash-diagnosis|crash_diagnosis_list_auto_fix_kinds',
	)
}

/**
 * Export a crash report ZIP bundle for the given instance.
 * Returns the path to the generated ZIP file.
 */
export async function exportCrashReport(instanceId: string): Promise<string> {
	return await invoke<string>(
		'plugin:crash-diagnosis|crash_diagnosis_export_report',
		{ instanceId },
	)
}

/**
 * Apply an auto-fix action for the given instance.
 * Returns a user-readable message describing what was done.
 */
export async function applyAutoFix(
	instanceId: string,
	autoFix: AutoFixKind,
): Promise<string> {
	return await invoke<string>(
		'plugin:crash-diagnosis|crash_diagnosis_apply_auto_fix',
		{ instanceId, autoFix },
	)
}

/**
 * Run log cleanup to prevent infinite log growth.
 */
export async function cleanupLogs(): Promise<void> {
	return await invoke<void>('plugin:crash-diagnosis|crash_diagnosis_cleanup_logs')
}
