//! Launcher session log API.
//!
//! Exposes the on-disk launcher session log files (written by
//! `theseus::logger::start_logger`) to the frontend so users can browse,
//! inspect, and triage launcher logs without leaving the app.

use crate::api::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use theseus::{ErrorKind, State};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("launcher-logs")
        .invoke_handler(tauri::generate_handler![
            launcher_logs_list_sessions,
            launcher_logs_read_session,
            launcher_logs_read_current_tail,
            launcher_logs_delete_session,
            launcher_logs_clear_all,
            launcher_logs_get_dir_path,
        ])
        .build()
}

/// Metadata for a single launcher session log file.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSessionLogInfo {
    /// Filename, e.g. `session_20260802_153012_a1b2c3d4.log`
    pub filename: String,
    /// Session start timestamp parsed from the filename (Unix seconds, UTC).
    /// `None` if the filename could not be parsed.
    pub started_at: Option<u64>,
    /// Session id parsed from the filename, if present.
    pub session_id: Option<String>,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last modification time (Unix seconds, UTC).
    pub modified_at: u64,
    /// True if this is the most recently modified session log (i.e. likely
    /// the active session).
    pub is_current: bool,
}

/// Content of a session log, with optional truncation.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSessionLogContent {
    pub filename: String,
    /// Total file size on disk (bytes).
    pub total_size_bytes: u64,
    /// Number of bytes returned in `content`. May be less than
    /// `total_size_bytes` when truncated.
    pub returned_bytes: usize,
    /// True when the returned content is the tail of the file (i.e. the
    /// beginning was truncated to fit `max_bytes`).
    pub truncated: bool,
    /// The log text. UTF-8 lossy.
    pub content: String,
}

/// Resolve the launcher logs directory from the global State.
async fn logs_dir() -> Result<PathBuf> {
    let state = State::get()
        .await
        .map_err(|e| ErrorKind::OtherError(format!("State not available: {e}")).as_error())?;
    let dir = state
        .directories
        .launcher_logs_dir()
        .ok_or_else(|| {
            ErrorKind::FSError("Could not resolve launcher logs directory".to_string())
                .as_error()
        })?;
    Ok(dir)
}

/// List all launcher session logs, newest first.
#[tauri::command]
pub async fn launcher_logs_list_sessions() -> Result<Vec<LauncherSessionLogInfo>> {
    let dir = logs_dir().await?;

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
    let read = std::fs::read_dir(&dir)?;
    for entry in read.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("session_") || !name.ends_with(".log") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let size = meta.len();
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push((p, size, mtime));
    }

    // Sort by mtime descending (newest first)
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    let newest_mtime = entries.first().map(|(_, _, t)| *t);

    let infos = entries
        .into_iter()
        .map(|(path, size, mtime)| {
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let (started_at, session_id) = parse_session_filename(&filename);
            let modified_at = mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let is_current = newest_mtime.map(|n| n == mtime).unwrap_or(false);
            LauncherSessionLogInfo {
                filename,
                started_at,
                session_id,
                size_bytes: size,
                modified_at,
                is_current,
            }
        })
        .collect();

    Ok(infos)
}

/// Read a specific session log by filename. If `max_bytes` is provided, the
/// returned content is the *tail* of the file, capped at that many bytes.
#[tauri::command]
pub async fn launcher_logs_read_session(
    filename: String,
    max_bytes: Option<u64>,
) -> Result<LauncherSessionLogContent> {
    let dir = logs_dir().await?;
    validate_filename(&filename)?;

    let path = dir.join(&filename);
    let total_size = std::fs::metadata(&path)?.len();

    let bytes = if let Some(max) = max_bytes {
        read_tail(&path, max as usize)?
    } else {
        std::fs::read(&path)?
    };

    let returned_bytes = bytes.len();
    let truncated = (returned_bytes as u64) < total_size;
    let content = String::from_utf8_lossy(&bytes).into_owned();

    Ok(LauncherSessionLogContent {
        filename,
        total_size_bytes: total_size,
        returned_bytes,
        truncated,
        content,
    })
}

/// Read the tail of the most recently modified session log (the active
/// session). Useful for a live "current session" view in the UI.
#[tauri::command]
pub async fn launcher_logs_read_current_tail(max_bytes: u64) -> Result<LauncherSessionLogContent> {
    let dir = logs_dir().await?;

    if !dir.exists() {
        return Err(ErrorKind::FSError(
            "Launcher logs directory does not exist".to_string(),
        )
        .as_error()
        .into());
    }

    // Find the newest session_*.log file
    let mut newest: Option<(PathBuf, SystemTime)> = None;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("session_") || !name.ends_with(".log") {
            continue;
        }
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        match &newest {
            None => newest = Some((p, mtime)),
            Some((_, cur)) if mtime > *cur => newest = Some((p, mtime)),
            _ => {}
        }
    }

    let (path, _) = newest.ok_or_else(|| {
        ErrorKind::OtherError("No launcher session log found".to_string()).as_error()
    })?;

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let total_size = std::fs::metadata(&path)?.len();
    let bytes = read_tail(&path, max_bytes as usize)?;
    let returned_bytes = bytes.len();
    let truncated = (returned_bytes as u64) < total_size;
    let content = String::from_utf8_lossy(&bytes).into_owned();

    Ok(LauncherSessionLogContent {
        filename,
        total_size_bytes: total_size,
        returned_bytes,
        truncated,
        content,
    })
}

/// Delete a specific session log by filename. The current (active) session
/// log cannot be deleted.
#[tauri::command]
pub async fn launcher_logs_delete_session(filename: String) -> Result<()> {
    let dir = logs_dir().await?;
    validate_filename(&filename)?;

    let path = dir.join(&filename);

    // Find newest file to protect the active session
    let newest_name = find_newest_session_filename(&dir)?;

    if newest_name.as_deref() == Some(filename.as_str()) {
        return Err(ErrorKind::InputError(
            "Cannot delete the active session log".to_string(),
        )
        .as_error()
        .into());
    }

    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Delete all session logs *except* the most recently modified (active) one.
/// Returns the number of files removed.
#[tauri::command]
pub async fn launcher_logs_clear_all() -> Result<u32> {
    let dir = logs_dir().await?;

    if !dir.exists() {
        return Ok(0);
    }

    let newest_name = find_newest_session_filename(&dir)?;

    let mut removed: u32 = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("session_") || !name.ends_with(".log") {
            continue;
        }
        if newest_name.as_deref() == Some(name.as_str()) {
            continue;
        }
        if std::fs::remove_file(&p).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Return the absolute path to the launcher logs directory, for "Open folder"
/// affordances in the UI.
#[tauri::command]
pub async fn launcher_logs_get_dir_path() -> Result<Option<String>> {
    let state = State::get().await?;
    Ok(state
        .directories
        .launcher_logs_dir()
        .map(|p| p.to_string_lossy().to_string()))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate that a filename is a bare session log filename (no path traversal).
fn validate_filename(filename: &str) -> Result<()> {
    if filename.contains(std::path::MAIN_SEPARATOR)
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return Err(ErrorKind::InputError("Invalid log filename".to_string()).as_error().into());
    }
    if !filename.starts_with("session_") || !filename.ends_with(".log") {
        return Err(ErrorKind::InputError("Filename is not a session log".to_string()).as_error().into());
    }
    Ok(())
}

/// Find the filename of the most recently modified session log in the dir.
fn find_newest_session_filename(dir: &std::path::Path) -> Result<Option<String>> {
    let mut newest_name: Option<String> = None;
    let mut newest_mtime = SystemTime::UNIX_EPOCH;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("session_") || !name.ends_with(".log") {
            continue;
        }
        if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
            if m > newest_mtime {
                newest_mtime = m;
                newest_name = Some(name);
            }
        }
    }
    Ok(newest_name)
}

/// Parse a session log filename of the form
/// `session_YYYYMMDD_HHMMSS_<session_id>.log` (new format) or
/// `session_YYYYMMDD_HHMMSS.log` (legacy format).
///
/// Returns `(started_at_unix_seconds, session_id)`.
fn parse_session_filename(filename: &str) -> (Option<u64>, Option<String>) {
    // Strip "session_" prefix and ".log" suffix
    let stem = filename
        .strip_prefix("session_")
        .and_then(|s| s.strip_suffix(".log"))
        .unwrap_or(filename);

    let parts: Vec<&str> = stem.splitn(3, '_').collect();
    if parts.len() < 2 {
        return (None, None);
    }

    let date = parts[0]; // YYYYMMDD
    let time = parts[1]; // HHMMSS

    // Parse YYYYMMDD_HHMMSS into Unix timestamp (local time, since logger
    // writes local time). Use chrono for parsing.
    use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    let parsed_date = NaiveDate::parse_from_str(date, "%Y%m%d").ok();
    let parsed_time = NaiveTime::parse_from_str(time, "%H%M%S").ok();
    let started_at = match (parsed_date, parsed_time) {
        (Some(d), Some(t)) => {
            let ndt = NaiveDateTime::new(d, t);
            Local
                .from_local_datetime(&ndt)
                .single()
                .map(|dt| dt.timestamp() as u64)
        }
        _ => None,
    };

    let session_id = if parts.len() == 3 {
        let sid = parts[2];
        if sid.is_empty() {
            None
        } else {
            Some(sid.to_string())
        }
    } else {
        None
    };

    (started_at, session_id)
}

/// Read the last `max_bytes` bytes of a file. If the file is smaller, returns
/// the whole file. Aligns to the start of the next newline to avoid cutting
/// a log line in half.
fn read_tail(path: &std::path::Path, max_bytes: usize) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path)?;
    let total = file.metadata()?.len();

    if total <= max_bytes as u64 {
        let mut buf = Vec::with_capacity(total as usize);
        file.read_to_end(&mut buf)?;
        return Ok(buf);
    }

    // Seek to (total - max_bytes), then advance to the next newline to start
    // on a clean line boundary (avoids cutting a log line in half).
    let start = total - max_bytes as u64;
    file.seek(SeekFrom::Start(start))?;

    let mut buf = vec![0u8; max_bytes];
    let n = file.read(&mut buf)?;
    buf.truncate(n);

    // Skip forward to the first newline so we start at a line boundary.
    // (The first "line" after a tail cut is usually partial.)
    if let Some(idx) = buf.iter().position(|&b| b == b'\n') {
        buf.drain(0..=idx);
    }

    Ok(buf)
}
