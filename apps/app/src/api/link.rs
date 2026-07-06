use crate::api::Result;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use tokio::fs;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tauri::Manager;
use tauri_plugin_http::reqwest;

const LOBBY_CODE_PREFIX: &str = "ET-";
const HOST_VIRTUAL_IP: &str = "10.114.51.41";
const EASYTIER_VERSION: &str = "v2.2.4";
const EASYTIER_DOWNLOAD_URL: &str =
    "https://github.com/EasyTier/EasyTier/releases/download/v2.2.4/easytier-windows-x86_64-v2.2.4.zip";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LobbyPayload {
    #[serde(rename = "n")]
    network_name: String,
    #[serde(rename = "p")]
    password: String,
    #[serde(rename = "h")]
    host_virtual_ip: String,
    #[serde(rename = "m")]
    mc_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkRole {
    Idle,
    Host,
    Client,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LobbyStatus {
    pub state: LinkRole,
    pub lobby_code: Option<String>,
    pub network_name: Option<String>,
    pub virtual_ip: Option<String>,
    pub mc_port: Option<u16>,
    pub local_port: Option<u16>,
    pub peer_count: usize,
    pub last_refresh: Option<i64>,
    pub error: Option<String>,
}

struct ProxyHandle {
    task: JoinHandle<()>,
}

struct LinkState {
    role: LinkRole,
    lobby_code: Option<String>,
    payload: Option<LobbyPayload>,
    local_proxy_port: Option<u16>,
    easytier_rpc_port: Option<u16>,
    easytier: Option<Child>,
    proxy: Option<ProxyHandle>,
    peer_refresh: Option<JoinHandle<()>>,
    peer_count: usize,
    last_peer_refresh: Option<chrono::DateTime<chrono::Utc>>,
    error: Option<String>,
}

pub struct LinkManager {
    state: Arc<RwLock<LinkState>>,
}

impl Default for LinkManager {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(LinkState {
                role: LinkRole::Idle,
                lobby_code: None,
                payload: None,
                local_proxy_port: None,
                easytier_rpc_port: None,
                easytier: None,
                proxy: None,
                peer_refresh: None,
                peer_count: 0,
                last_peer_refresh: None,
                error: None,
            })),
        }
    }
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("link")
        .invoke_handler(tauri::generate_handler![
            link_create_lobby,
            link_join_lobby,
            link_leave_lobby,
            link_get_lobby_status,
        ])
        .setup(|app, _api| {
            app.manage(LinkManager::default());
            Ok(())
        })
        .build()
}

#[tauri::command]
pub async fn link_create_lobby(
    manager: tauri::State<'_, LinkManager>,
    network_name: String,
    password: String,
    mc_port: u16,
) -> Result<String> {
    link_leave_lobby(manager.clone()).await.ok();

    let payload = LobbyPayload {
        network_name,
        password,
        host_virtual_ip: HOST_VIRTUAL_IP.to_string(),
        mc_port,
    };

    let code = encode_lobby_code(&payload)?;

    {
        let mut state = manager.state.write().await;
        state.role = LinkRole::Host;
        state.lobby_code = Some(code.clone());
        state.payload = Some(payload);
        state.error = None;
    }

    match start_easytier_core(&manager.state, true, mc_port).await {
        Ok((child, rpc_port)) => {
            let mut state = manager.state.write().await;
            state.easytier = Some(child);
            state.easytier_rpc_port = Some(rpc_port);
            state.peer_refresh = Some(spawn_peer_refresh_loop(manager.state.clone(), rpc_port));
        }
        Err(e) => {
            let mut state = manager.state.write().await;
            state.error = Some(e.to_string());
            return Err(e.into());
        }
    }

    Ok(code)
}

#[tauri::command]
pub async fn link_join_lobby(
    manager: tauri::State<'_, LinkManager>,
    lobby_code: String,
) -> Result<u16> {
    link_leave_lobby(manager.clone()).await.ok();

    let payload = decode_lobby_code(&lobby_code)?;
    let target = format!("{}:{}", payload.host_virtual_ip, payload.mc_port);

    {
        let mut state = manager.state.write().await;
        state.role = LinkRole::Client;
        state.lobby_code = Some(lobby_code);
        state.payload = Some(payload);
        state.error = None;
    }

    match start_easytier_core(&manager.state, false, 0).await {
        Ok((child, rpc_port)) => {
            let mut state = manager.state.write().await;
            state.easytier = Some(child);
            state.easytier_rpc_port = Some(rpc_port);
            state.peer_refresh = Some(spawn_peer_refresh_loop(manager.state.clone(), rpc_port));
        }
        Err(e) => {
            let mut state = manager.state.write().await;
            state.error = Some(e.to_string());
            return Err(e.into());
        }
    }

    let local_port = start_tcp_proxy(manager.state.clone(), target).await?;
    {
        let mut state = manager.state.write().await;
        state.local_proxy_port = Some(local_port);
    }

    Ok(local_port)
}

#[tauri::command]
pub async fn link_leave_lobby(
    manager: tauri::State<'_, LinkManager>,
) -> Result<()> {
    let mut state = manager.state.write().await;

    if let Some(mut child) = state.easytier.take() {
        let _ = child.kill().await;
    }

    if let Some(proxy) = state.proxy.take() {
        proxy.task.abort();
    }

    if let Some(refresh) = state.peer_refresh.take() {
        refresh.abort();
    }

    state.role = LinkRole::Idle;
    state.lobby_code = None;
    state.payload = None;
    state.local_proxy_port = None;
    state.easytier_rpc_port = None;
    state.peer_count = 0;
    state.last_peer_refresh = None;
    state.error = None;

    Ok(())
}

#[tauri::command]
pub async fn link_get_lobby_status(
    manager: tauri::State<'_, LinkManager>,
) -> Result<LobbyStatus> {
    let state = manager.state.read().await;
    Ok(LobbyStatus {
        state: state.role,
        lobby_code: state.lobby_code.clone(),
        network_name: state.payload.as_ref().map(|p| p.network_name.clone()),
        virtual_ip: state.payload.as_ref().map(|p| p.host_virtual_ip.clone()),
        mc_port: state.payload.as_ref().map(|p| p.mc_port),
        local_port: state.local_proxy_port,
        peer_count: state.peer_count,
        last_refresh: state.last_peer_refresh.map(|dt| dt.timestamp_millis()),
        error: state.error.clone(),
    })
}

fn encode_lobby_code(payload: &LobbyPayload) -> theseus::Result<String> {
    let json = serde_json::to_string(payload)?;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json.as_bytes());
    Ok(format!("{LOBBY_CODE_PREFIX}{encoded}"))
}

fn decode_lobby_code(code: &str) -> theseus::Result<LobbyPayload> {
    let code = code.trim();
    let encoded = code.strip_prefix(LOBBY_CODE_PREFIX).ok_or_else(|| {
        theseus::ErrorKind::InputError("Invalid lobby code format".to_string()).as_error()
    })?;

    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid lobby code base64: {e}")).as_error()
    })?;

    let json = String::from_utf8(bytes).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid lobby code utf8: {e}")).as_error()
    })?;

    serde_json::from_str(&json).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid lobby code json: {e}")).as_error()
    })
}

async fn easytier_dir() -> theseus::Result<std::path::PathBuf> {
    let state = theseus::State::get().await?;
    Ok(state.directories.caches_dir().join("EasyTier"))
}

async fn ensure_easytier_binaries() -> theseus::Result<()> {
    let dir = easytier_dir().await?;
    let core = dir.join("easytier-core.exe");
    let cli = dir.join("easytier-cli.exe");

    if core.exists() && cli.exists() {
        return Ok(());
    }

    fs::create_dir_all(&dir).await.map_err(|e| {
        theseus::ErrorKind::FSError(format!("Failed to create EasyTier directory: {e}")).as_error()
    })?;

    let response = reqwest::get(EASYTIER_DOWNLOAD_URL).await.map_err(|e| {
        theseus::ErrorKind::OtherError(format!(
            "Failed to download EasyTier {EASYTIER_VERSION}: {e}"
        ))
        .as_error()
    })?;
    let response = response.error_for_status().map_err(|e| {
        theseus::ErrorKind::OtherError(format!(
            "Failed to download EasyTier {EASYTIER_VERSION}: {e}"
        ))
        .as_error()
    })?;
    let bytes = response.bytes().await.map_err(|e| {
        theseus::ErrorKind::OtherError(format!(
            "Failed to read EasyTier {EASYTIER_VERSION} download: {e}"
        ))
        .as_error()
    })?;

    extract_easytier_zip(&bytes, &dir).await?;

    if !core.exists() || !cli.exists() {
        return Err(theseus::ErrorKind::FSError(
            "EasyTier binaries missing after extraction".to_string(),
        )
        .as_error());
    }

    Ok(())
}

async fn extract_easytier_zip(bytes: &[u8], dir: &std::path::Path) -> theseus::Result<()> {
    let mut reader = async_zip::tokio::read::seek::ZipFileReader::with_tokio(Cursor::new(bytes.to_vec()))
        .await
        .map_err(|e| {
            theseus::ErrorKind::FSError(format!("Failed to read EasyTier zip: {e}")).as_error()
        })?;

    let entry_count = reader.file().entries().len();
    for i in 0..entry_count {
        let filename = reader
            .file()
            .entries()
            .get(i)
            .and_then(|e| e.filename().as_str().ok())
            .unwrap_or("")
            .to_string();
        let name = std::path::Path::new(&filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename)
            .to_string();

        if name != "easytier-core.exe" && name != "easytier-cli.exe" {
            continue;
        }

        let target = dir.join(&name);
        let mut data = vec![];
        let mut entry_reader = reader.reader_with_entry(i).await.map_err(|e| {
            theseus::ErrorKind::FSError(format!("Failed to open zip entry {name}: {e}")).as_error()
        })?;
        entry_reader
            .read_to_end_checked(&mut data)
            .await
            .map_err(|e| {
                theseus::ErrorKind::FSError(format!("Failed to extract {name}: {e}")).as_error()
            })?;

        fs::write(&target, data).await.map_err(|e| {
            theseus::ErrorKind::FSError(format!("Failed to write {name}: {e}")).as_error()
        })?;
    }

    Ok(())
}

async fn start_easytier_core(
    state: &RwLock<LinkState>,
    as_host: bool,
    mc_port: u16,
) -> theseus::Result<(Child, u16)> {
    ensure_easytier_binaries().await?;

    let (network_name, password) = {
        let state = state.read().await;
        let payload = state.payload.as_ref().ok_or_else(|| {
            theseus::ErrorKind::InputError("Lobby payload not set".to_string()).as_error()
        })?;
        (payload.network_name.clone(), payload.password.clone())
    };

    let dir = easytier_dir().await?;
    let core = dir.join("easytier-core.exe");

    let rpc_port = find_free_port().await?;

    let mut cmd = Command::new(&core);
    cmd.current_dir(&dir)
        .arg("--no-tun")
        .arg("--multi-thread")
        .arg("--enable-kcp-proxy")
        .arg("--enable-quic-proxy")
        .arg("--encryption-algorithm")
        .arg("aes-gcm")
        .arg("--compression")
        .arg("zstd")
        .arg("--network-name")
        .arg(&network_name)
        .arg("--network-secret")
        .arg(&password)
        .arg("--rpc-portal")
        .arg(rpc_port.to_string())
        .arg("--private-mode")
        .arg("true")
        .arg("-l")
        .arg("tcp://0.0.0.0:0")
        .arg("-l")
        .arg("udp://0.0.0.0:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if as_host {
        cmd.arg("-i")
            .arg(HOST_VIRTUAL_IP)
            .arg("--hostname")
            .arg(format!("scaffolding-mc-server-{mc_port}"));

        if mc_port > 0 {
            cmd.arg("--tcp-whitelist")
                .arg(mc_port.to_string())
                .arg("--udp-whitelist")
                .arg(mc_port.to_string());
        }
    } else {
        cmd.arg("--hostname")
            .arg(uuid::Uuid::new_v4().to_string())
            .arg("--tcp-whitelist")
            .arg("0")
            .arg("--udp-whitelist")
            .arg("0");
    }

    let child = cmd.spawn().map_err(|e| {
        theseus::ErrorKind::LauncherError(format!("Failed to start EasyTier: {e}")).as_error()
    })?;

    Ok((child, rpc_port))
}

async fn start_tcp_proxy(
    state: Arc<RwLock<LinkState>>,
    target: String,
) -> theseus::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;

    let state_for_task = state.clone();
    let task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((client, _)) => {
                    let target = target.clone();
                    let state = state_for_task.clone();
                    tokio::spawn(async move {
                        if let Err(e) = proxy_connection(client, &target, state).await {
                            tracing::warn!("Link TCP proxy connection failed: {e}");
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("Link TCP proxy accept failed: {e}");
                    break;
                }
            }
        }
    });

    {
        let mut state = state.write().await;
        state.proxy = Some(ProxyHandle { task });
    }

    Ok(local_addr.port())
}

async fn proxy_connection(
    client: TcpStream,
    target: &str,
    state: Arc<RwLock<LinkState>>,
) -> io::Result<()> {
    let target_addr: SocketAddr = target
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let target = TcpStream::connect(target_addr).await?;

    {
        let mut state = state.write().await;
        state.error = None;
    }

    let (mut client_read, mut client_write) = client.into_split();
    let (mut target_read, mut target_write) = target.into_split();

    let client_to_target = io::copy(&mut client_read, &mut target_write);
    let target_to_client = io::copy(&mut target_read, &mut client_write);

    let _ = tokio::select! {
        r = client_to_target => r,
        r = target_to_client => r,
    };

    Ok(())
}

fn spawn_peer_refresh_loop(state: Arc<RwLock<LinkState>>, rpc_port: u16) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            match refresh_peer_count(rpc_port).await {
                Ok(count) => {
                    let mut state = state.write().await;
                    state.peer_count = count;
                    state.last_peer_refresh = Some(chrono::Utc::now());
                    state.error = None;
                }
                Err(e) => {
                    let mut state = state.write().await;
                    state.error = Some(e.to_string());
                }
            }
        }
    })
}

async fn refresh_peer_count(rpc_port: u16) -> theseus::Result<usize> {
    let dir = easytier_dir().await?;
    let cli = dir.join("easytier-cli.exe");

    if !cli.exists() {
        return Ok(0);
    }

    let output = Command::new(&cli)
        .current_dir(&dir)
        .arg("--rpc-portal")
        .arg(format!("127.0.0.1:{rpc_port}"))
        .arg("-o")
        .arg("json")
        .arg("peer")
        .output()
        .await
        .map_err(|e| {
            theseus::ErrorKind::LauncherError(format!("Failed to run easytier-cli peer: {e}")).as_error()
        })?;

    if !output.status.success() {
        return Ok(0);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);

    let count = parsed.as_array().map(|arr| arr.len()).unwrap_or(0);

    Ok(count)
}

async fn find_free_port() -> theseus::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

// Provide a clone impl so link_leave_lobby can accept State<LinkManager> and be called internally.
impl Clone for LinkManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}
