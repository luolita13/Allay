/*
    tracing is set based on the environment variable RUST_LOG=xxx, depending on the amount of logs to show
        ERROR > WARN > INFO > DEBUG > TRACE
    eg. RUST_LOG=info will show info, warn, and error logs
        RUST_LOG="theseus=trace" will show *all* messages but from theseus only (and not dependencies using similar crates)
        RUST_LOG="theseus=trace" will show *all* messages but from theseus only (and not dependencies using similar crates)

    Error messages returned to Tauri will display as traced error logs if they return an error.
    This will also include an attached span trace if the error is from a tracing error, and the level is set to info, debug, or trace

    on unix:
        RUST_LOG="theseus=trace" {run command}

    The default is theseus=show, meaning only logs from theseus will be displayed, and at the info or higher level.

    Each production session log starts with a structured header block containing
    app version, OS, architecture, session id (uuid), key directories, and a
    brief system specs summary. This makes bug reports actionable without
    asking the user for environment details.

*/

// `fmt::Write` is needed for `writeln!(String, ...)` when building the
// session header; `io::Write` is needed for `File::write_all` and
// `File::flush`. Both are imported anonymously to avoid name conflicts.
// These are only needed in production (non-debug) builds where the file
// logger and session header are active.
#[cfg(not(debug_assertions))]
use std::fmt::Write as _;
#[cfg(not(debug_assertions))]
use std::io::Write as _;

// Handling for the live development logging
// This will log to the console, and will not log to a file
#[cfg(debug_assertions)]
pub fn start_logger(_app_identifier: &str) -> Option<()> {
    use tracing_subscriber::prelude::*;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("theseus=info,theseus_gui=info"));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .init();
    Some(())
}

// Handling for the live production logging
// This will log to a file in the logs directory, and will not show any logs in the console
#[cfg(not(debug_assertions))]
pub fn start_logger(app_identifier: &str) -> Option<()> {
    use crate::state::DirectoryInfo;
    use chrono::Local;
    use std::fs::OpenOptions;
    use tracing_subscriber::fmt::time::ChronoLocal;
    use tracing_subscriber::prelude::*;

    // Initialize and get logs directory path
    let logs_dir = if let Some(d) = DirectoryInfo::launcher_logs_dir_path(app_identifier) {
        d
    } else {
        eprintln!("Could not start logger");
        return None;
    };

    // Session id is embedded in the filename so it can be parsed back out by
    // the log viewer without reading the file body.
    let session_id = uuid::Uuid::new_v4().simple().to_string();
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_file_name = format!("session_{timestamp}_{session_id}.log");
    let log_file_path = logs_dir.join(&log_file_name);

    if let Err(err) = std::fs::create_dir_all(&logs_dir) {
        eprintln!("Could not create logs directory: {err}");
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Could not start open log file: {e}");
            return None;
        }
    };

    // Write structured session header BEFORE tracing takes over the file
    // handle. This block is plain text so it is easy to grep / paste into
    // bug reports. The trailing banner separates header from runtime logs.
    let header = build_session_header(app_identifier, &session_id, &timestamp.to_string());
    if let Err(e) = file.write_all(header.as_bytes()) {
        eprintln!("Failed to write log header: {e}");
    }
    // Flush so the header lands on disk even if the process crashes early.
    let _ = file.flush();

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("theseus=info"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file)
                .with_ansi(false) // disable ANSI escape codes
                .with_timer(ChronoLocal::rfc_3339()),
        )
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .init();

    Some(())
}

/// Build a structured, human-readable session header to prepend to every
/// production session log. Contains all the context a developer needs when
/// triaging a user-supplied log file.
#[cfg(not(debug_assertions))]
fn build_session_header(app_identifier: &str, session_id: &str, timestamp: &str) -> String {
    let mut out = String::new();
    let bar = "=".repeat(72);

    out.push_str(&bar);
    out.push('\n');
    out.push_str("Allay Launcher Session Log\n");
    out.push_str(&bar);
    out.push('\n');

    // --- Identity ---
    let version = env!("CARGO_PKG_VERSION");
    writeln!(out, "  session_id     : {session_id}").ok();
    writeln!(out, "  session_start  : {timestamp}").ok();
    writeln!(out, "  app_identifier : {app_identifier}").ok();
    writeln!(out, "  app_version    : {version}").ok();

    // --- OS / arch ---
    let os_kind = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let bitness = if cfg!(target_pointer_width = "64") { "64-bit" } else { "32-bit" };
    writeln!(out, "  os             : {os_kind} ({bitness})").ok();
    writeln!(out, "  arch           : {arch}").ok();

    // --- Hostname (best-effort, censored in production logs elsewhere) ---
    if let Some(host) = hostname() {
        writeln!(out, "  hostname       : {host}").ok();
    }

    // --- Key directories ---
    if let Some(d) = crate::state::DirectoryInfo::launcher_logs_dir_path(app_identifier) {
        writeln!(out, "  logs_dir       : {}", d.display()).ok();
    }
    if let Some(d) = crate::state::DirectoryInfo::initial_settings_dir_path(app_identifier) {
        writeln!(out, "  settings_dir   : {}", d.display()).ok();
    }
    if let Ok(cwd) = std::env::current_exe() {
        writeln!(out, "  executable     : {}", cwd.display()).ok();
    }
    let pid = std::process::id();
    writeln!(out, "  pid            : {pid}").ok();

    // --- System specs (best-effort, non-blocking on failure) ---
    if let Some(specs) = collect_system_specs() {
        writeln!(out, "  cpu            : {}", specs.cpu_brand).ok();
        writeln!(out, "  cpu_cores      : {}", specs.cpu_cores).ok();
        writeln!(
            out,
            "  memory_total   : {}",
            format_bytes(specs.memory_total_bytes)
        )
        .ok();
        if !specs.gpus.is_empty() {
            let gpu_list = specs.gpus.join("; ");
            writeln!(out, "  gpus           : {gpu_list}").ok();
        }
        if let Some(disk) = specs.main_disk {
            writeln!(
                out,
                "  main_disk      : {disk} ({} total / {} free)",
                format_bytes(specs.main_disk_total.unwrap_or(0)),
                format_bytes(specs.main_disk_free.unwrap_or(0))
            )
            .ok();
        }
    }

    // --- RUST_LOG filter in effect (helps explain why some logs are missing) ---
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "<unset: theseus=info>".to_string());
    writeln!(out, "  rust_log       : {rust_log}").ok();

    out.push_str(&bar);
    out.push('\n');
    out.push('\n');
    out
}

/// Best-effort hostname retrieval. Returns None on failure.
#[cfg(not(debug_assertions))]
fn hostname() -> Option<String> {
    // Try the `gethostname` crate-free approach via environment, falling back
    // to platform commands. We deliberately avoid pulling in a new crate.
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$env:COMPUTERNAME",
            ])
            .output()
            .ok()?;
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("hostname").output().ok()?;
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}

/// Lightweight system specs collected for the log header. We deliberately
/// reuse `sysinfo` (already a dependency) and keep this very short to avoid
/// slowing down startup.
#[cfg(not(debug_assertions))]
struct SystemSpecs {
    cpu_brand: String,
    cpu_cores: usize,
    memory_total_bytes: u64,
    gpus: Vec<String>,
    main_disk: Option<String>,
    main_disk_total: Option<u64>,
    main_disk_free: Option<u64>,
}

#[cfg(not(debug_assertions))]
fn collect_system_specs() -> Option<SystemSpecs> {
    use sysinfo::{MemoryRefreshKind, RefreshKind, System};

    // We only need CPU brand (static, no refresh required) + total memory.
    let mut system = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    system.refresh_memory();

    let cpu_brand = system
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();
    let cpu_cores = system.cpus().len();

    // Disk: pick the one that hosts the app data directory
    let (main_disk, main_disk_total, main_disk_free) = {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        let app_data_path = dirs::data_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                {
                    std::path::PathBuf::from("C:\\")
                }
                #[cfg(not(target_os = "windows"))]
                {
                    std::path::PathBuf::from("/")
                }
            });
        let md = disks.iter().find(|d| app_data_path.starts_with(d.mount_point()));
        if let Some(d) = md {
            (
                Some(d.mount_point().to_string_lossy().to_string()),
                Some(d.total_space()),
                Some(d.available_space()),
            )
        } else {
            (None, None, None)
        }
    };

    // GPU: best-effort, do not block startup. Reuses the same logic as
    // apps/app/src/api/system.rs but inlined here to avoid a circular dep.
    let gpus = collect_gpu_names();

    Some(SystemSpecs {
        cpu_brand,
        cpu_cores,
        memory_total_bytes: system.total_memory(),
        gpus,
        main_disk,
        main_disk_total,
        main_disk_free,
    })
}

#[cfg(not(debug_assertions))]
fn is_virtual_gpu(name: &str) -> bool {
    let lower = name.to_lowercase();
    const VIRTUAL_KEYWORDS: &[&str] = &[
        "basic render driver",
        "microsoft basic display",
        "microsoft display adapter",
        "virtual display",
        "virtual adapter",
        "virtual gpu",
        "gameviewer",
        "anydesk",
        "todesk display",
        "parsec virtual",
        "sunshine display",
        "remote desktop display",
        "mirror display",
        "indirect display",
        "ddu display",
        "spacedesk",
        "deskreen",
        "divid",
        "iweski",
        "usb display",
        "headless",
        "null display",
    ];
    VIRTUAL_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

#[cfg(not(debug_assertions))]
fn collect_gpu_names() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ])
            .output();
        if let Ok(o) = output {
            if o.status.success() {
                return String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .filter(|s| !is_virtual_gpu(s))
                    .collect();
            }
        }
        Vec::new()
    }
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("lspci").arg("-mm").output();
        if let Ok(o) = output {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                return stdout
                    .lines()
                    .filter(|l| {
                        let low = l.to_lowercase();
                        low.contains("vga compatible controller")
                            || low.contains("3d controller")
                            || low.contains("display controller")
                    })
                    .map(|l| {
                        let parts: Vec<&str> = l.split_whitespace().collect();
                        parts.last().copied().unwrap_or("Unknown GPU").to_string()
                    })
                    .collect();
            }
        }
        Vec::new()
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output();
        if let Ok(o) = output {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let Some(arr) = json.get("SPDisplaysDataType").and_then(|v| v.as_array()) {
                    return arr
                        .iter()
                        .filter_map(|g| g.get("sppci_model").and_then(|v| v.as_str()).map(String::from))
                        .collect();
                }
            }
        }
        Vec::new()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(not(debug_assertions))]
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", value, UNITS[unit_idx])
    }
}
