use crate::api::Result;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
use tauri::plugin::TauriPlugin;
use tauri::{Manager, State};

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("system")
        .invoke_handler(tauri::generate_handler![get_system_info])
        .setup(|app, _api| {
            app.manage(SystemMonitor::default());
            Ok(())
        })
        .build()
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub brand: String,
    pub core_count: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub mount_point: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub usage_percent: Option<f32>,
    pub memory_total_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub received_bytes_per_second: u64,
    pub transmitted_bytes_per_second: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disk: DiskInfo,
    pub gpus: Vec<GpuInfo>,
    pub network: NetworkInfo,
}

#[derive(Clone, Copy, Debug)]
struct NetworkSample {
    timestamp: Instant,
    received: u64,
    transmitted: u64,
}

pub struct SystemMonitor {
    system: Mutex<System>,
    last_network_sample: Mutex<Option<NetworkSample>>,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self {
            system: Mutex::new(System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            )),
            last_network_sample: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn get_system_info(monitor: State<SystemMonitor>) -> Result<SystemInfo> {
    let mut system = monitor.system.lock().map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to lock system monitor: {e}"
        )))
    })?;

    system.refresh_all();

    let cpu = system.cpus().first().map(|cpu| CpuInfo {
        usage_percent: cpu.cpu_usage(),
        brand: cpu.brand().to_string(),
        core_count: system.cpus().len(),
    });

    let memory = MemoryInfo {
        total_bytes: system.total_memory(),
        used_bytes: system.used_memory(),
        usage_percent: if system.total_memory() > 0 {
            (system.used_memory() as f32 / system.total_memory() as f32) * 100.0
        } else {
            0.0
        },
    };

    let mut disk = DiskInfo {
        total_bytes: 0,
        available_bytes: 0,
        usage_percent: 0.0,
        mount_point: String::new(),
    };

    let disks = Disks::new_with_refreshed_list();
    // Aggregate ALL physical disks to show total storage across the system.
    // This gives users a complete picture rather than just one drive.
    let mut total_space: u64 = 0;
    let mut available_space: u64 = 0;
    for d in disks.iter() {
        total_space += d.total_space();
        available_space += d.available_space();
    }
    disk.total_bytes = total_space;
    disk.available_bytes = available_space;
    disk.usage_percent = if total_space > 0 {
        ((total_space - available_space) as f64 / total_space as f64 * 100.0) as f32
    } else {
        0.0
    };
    disk.mount_point = "All disks".to_string();

    // GPU detection via WMI (Windows) or lspci (Linux) or system_profiler (macOS)
    let gpus = detect_gpus();

    let networks = Networks::new_with_refreshed_list();
    let received_total: u64 = networks.iter().map(|(_, n)| n.total_received()).sum();
    let transmitted_total: u64 = networks.iter().map(|(_, n)| n.total_transmitted()).sum();

    let mut last_sample = monitor.last_network_sample.lock().map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to lock network sample: {e}"
        )))
    })?;

    let now = Instant::now();
    let network = if let Some(sample) = *last_sample {
        let elapsed = now.duration_since(sample.timestamp).as_secs_f64().max(1e-6);
        let received = received_total.saturating_sub(sample.received) as f64 / elapsed;
        let transmitted = transmitted_total.saturating_sub(sample.transmitted) as f64 / elapsed;
        NetworkInfo {
            received_bytes_per_second: received as u64,
            transmitted_bytes_per_second: transmitted as u64,
        }
    } else {
        NetworkInfo {
            received_bytes_per_second: 0,
            transmitted_bytes_per_second: 0,
        }
    };

    *last_sample = Some(NetworkSample {
        timestamp: now,
        received: received_total,
        transmitted: transmitted_total,
    });

    Ok(SystemInfo {
        cpu: cpu.unwrap_or(CpuInfo {
            usage_percent: 0.0,
            brand: String::new(),
            core_count: 0,
        }),
        memory,
        disk,
        gpus,
        network,
    })
}

/// Detect GPU information using platform-specific methods.
/// On Windows: parses `dxdiag` output.
/// On Linux: parses `lspci` output.
/// On macOS: parses `system_profiler` output.
fn detect_gpus() -> Vec<GpuInfo> {
    #[cfg(target_os = "windows")]
    {
        detect_gpus_windows()
    }
    #[cfg(target_os = "linux")]
    {
        detect_gpus_linux()
    }
    #[cfg(target_os = "macos")]
    {
        detect_gpus_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

/// Returns true if the GPU name belongs to a virtual display adapter,
/// software renderer, or remote-desktop virtual GPU rather than real
/// hardware. These should be filtered out to avoid confusing users.
fn is_virtual_display_adapter(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Known virtual display adapters and software renderers
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

#[cfg(target_os = "windows")]
fn detect_gpus_windows() -> Vec<GpuInfo> {
    // Use PowerShell + Get-CimInstance to enumerate GPU devices.
    // This is more reliable than parsing dxdiag XML and doesn't require
    // writing temp files.
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object Name, AdapterCompatibility, AdapterRAM | ConvertTo-Json",
        ])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Vec::new();
    }

    // The output can be a single object or an array; normalize to array
    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let entries: Vec<&serde_json::Value> = match &json {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        single => vec![single],
    };

    let mut gpus = Vec::new();
    for entry in entries {
        let name = entry
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown GPU")
            .to_string();
        let vendor = entry
            .get("AdapterCompatibility")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let memory_total = entry
            .get("AdapterRAM")
            .and_then(|v| v.as_u64());

        // Skip virtual display adapters, software renderers, and remote
        // desktop virtual GPUs that are not real hardware.
        if is_virtual_display_adapter(&name) {
            continue;
        }

        gpus.push(GpuInfo {
            name,
            vendor,
            usage_percent: None,
            memory_total_bytes: memory_total,
            memory_used_bytes: None,
        });
    }

    gpus
}

#[cfg(target_os = "linux")]
fn detect_gpus_linux() -> Vec<GpuInfo> {
    let output = std::process::Command::new("lspci")
        .arg("-mm")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    for line in stdout.lines() {
        if !line.to_lowercase().contains("vga compatible controller")
            && !line.to_lowercase().contains("3d controller")
            && !line.to_lowercase().contains("display controller")
        {
            continue;
        }

        // Parse "Class" "Vendor" "Device" format
        let parts: Vec<&str> = line.split_whitespace().collect();
        let name = parts.last().unwrap_or(&"Unknown GPU").to_string();
        let vendor = parts.get(1).unwrap_or(&"Unknown").to_string();

        gpus.push(GpuInfo {
            name,
            vendor,
            usage_percent: None,
            memory_total_bytes: None,
            memory_used_bytes: None,
        });
    }

    gpus
}

#[cfg(target_os = "macos")]
fn detect_gpus_macos() -> Vec<GpuInfo> {
    let output = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut gpus = Vec::new();
    if let Some(arr) = json
        .get("SPDisplaysDataType")
        .and_then(|v| v.as_array())
    {
        for gpu in arr {
            let name = gpu
                .get("sppci_model")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown GPU")
                .to_string();
            let vendor = gpu
                .get("spdisplays_vendor")
                .and_then(|v| v.as_str())
                .unwrap_or("Apple")
                .to_string();
            let memory_total = gpu
                .get("spdisplays_vram")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok());

            gpus.push(GpuInfo {
                name,
                vendor,
                usage_percent: None,
                memory_total_bytes: memory_total,
                memory_used_bytes: None,
            });
        }
    }

    gpus
}
