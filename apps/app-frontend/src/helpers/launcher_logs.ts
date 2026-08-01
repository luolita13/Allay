/**
 * Launcher session log API wrappers.
 *
 * Wraps the `launcher-logs` Tauri plugin so the frontend can browse, inspect,
 * and triage launcher session logs without leaving the app.
 */
import { invoke } from '@tauri-apps/api/core'

export interface LauncherSessionLogInfo {
	/** Filename, e.g. `session_20260802_153012_a1b2c3d4.log` */
	filename: string
	/** Session start timestamp (Unix seconds, UTC). Null if unparseable. */
	startedAt: number | null
	/** Session id parsed from filename, if present. */
	sessionId: string | null
	/** File size in bytes. */
	sizeBytes: number
	/** Last modification time (Unix seconds, UTC). */
	modifiedAt: number
	/** True if this is the most recently modified (active) session. */
	isCurrent: boolean
}

export interface LauncherSessionLogContent {
	filename: string
	/** Total file size on disk (bytes). */
	totalSizeBytes: number
	/** Number of bytes returned in `content`. */
	returnedBytes: number
	/** True when content was truncated (only the tail was returned). */
	truncated: boolean
	/** The log text. */
	content: string
}

/**
 * List all launcher session logs, newest first.
 */
export async function listLauncherLogSessions(): Promise<LauncherSessionLogInfo[]> {
	return await invoke<LauncherSessionLogInfo[]>(
		'plugin:launcher-logs|launcher_logs_list_sessions',
	)
}

/**
 * Read a specific session log by filename. If `maxBytes` is provided, returns
 * the tail of the file capped at that many bytes.
 */
export async function readLauncherLogSession(
	filename: string,
	maxBytes?: number,
): Promise<LauncherSessionLogContent> {
	return await invoke<LauncherSessionLogContent>(
		'plugin:launcher-logs|launcher_logs_read_session',
		{ filename, maxBytes: maxBytes ?? null },
	)
}

/**
 * Read the tail of the most recently modified (active) session log.
 */
export async function readLauncherLogCurrentTail(
	maxBytes: number,
): Promise<LauncherSessionLogContent> {
	return await invoke<LauncherSessionLogContent>(
		'plugin:launcher-logs|launcher_logs_read_current_tail',
		{ maxBytes },
	)
}

/**
 * Delete a specific session log by filename. The active session cannot be deleted.
 */
export async function deleteLauncherLogSession(filename: string): Promise<void> {
	await invoke<void>('plugin:launcher-logs|launcher_logs_delete_session', {
		filename,
	})
}

/**
 * Delete all session logs except the active one. Returns the number removed.
 */
export async function clearAllLauncherLogs(): Promise<number> {
	return await invoke<number>('plugin:launcher-logs|launcher_logs_clear_all')
}

/**
 * Get the absolute path to the launcher logs directory.
 */
export async function getLauncherLogsDirPath(): Promise<string | null> {
	return await invoke<string | null>(
		'plugin:launcher-logs|launcher_logs_get_dir_path',
	)
}

/**
 * Parsed log line for structured rendering.
 */
export interface ParsedLogLine {
	/** Original raw text (including timestamp). */
	raw: string
	/** Detected severity level (lowercase). */
	level: 'error' | 'warn' | 'info' | 'debug' | 'trace' | 'other'
	/** Timestamp portion, if extractable. */
	timestamp: string | null
	/** Span/module portion (between timestamp and level), if extractable. */
	span: string | null
	/** Message body. */
	message: string
}

const LEVEL_REGEX = /\b(TRACE|DEBUG|INFO|WARN(?:ING)?|ERROR)\b/

/**
 * Parse a single log line into structured fields for color coding.
 *
 * Handles the tracing_subscriber default format:
 *   `2026-08-02T10:30:00.123+08:00  INFO module: message`
 *   `2026-08-02T10:30:00.123+08:00  INFO module:span{...}: message`
 */
export function parseLogLine(line: string): ParsedLogLine {
	const trimmed = line.trimEnd()
	// Try to extract leading RFC3339 timestamp
	const tsMatch = /^(\d{4}-\d{2}-\d{2}T[\d:.+\-Z]+)\s+/.exec(trimmed)
	let rest = trimmed
	let timestamp: string | null = null
	if (tsMatch) {
		timestamp = tsMatch[1]
		rest = trimmed.slice(tsMatch[0].length)
	}

	// Find level token
	const levelMatch = LEVEL_REGEX.exec(rest)
	let level: ParsedLogLine['level'] = 'other'
	let span: string | null = null
	let message = rest
	if (levelMatch) {
		const lvl = levelMatch[1].toUpperCase()
		level =
			lvl === 'ERROR'
				? 'error'
				: lvl.startsWith('WARN')
					? 'warn'
					: lvl === 'INFO'
						? 'info'
						: lvl === 'DEBUG'
							? 'debug'
							: lvl === 'TRACE'
								? 'trace'
								: 'other'
		const after = rest.slice((levelMatch.index ?? 0) + levelMatch[0].length)
		const afterTrim = after.replace(/^[\s:]+/, '')
		// Split span/module from message at first ": " not inside braces
		const colonIdx = findTopLevelColon(afterTrim)
		if (colonIdx >= 0) {
			span = afterTrim.slice(0, colonIdx).trim()
			message = afterTrim.slice(colonIdx + 1).trim()
		} else {
			span = null
			message = afterTrim
		}
	}

	return {
		raw: trimmed,
		level,
		timestamp,
		span,
		message,
	}
}

/** Find the index of the first `: ` that is not inside `{}` braces. */
function findTopLevelColon(s: string): number {
	let depth = 0
	for (let i = 0; i < s.length - 1; i++) {
		const c = s[i]
		if (c === '{') depth++
		else if (c === '}') depth = Math.max(0, depth - 1)
		else if (c === ':' && s[i + 1] === ' ' && depth === 0) return i
	}
	return -1
}
