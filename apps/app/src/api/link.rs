//! LAN Multiplayer via EasyTier virtual LAN, integrated with the Scaffolding
//! protocol (ported from PCL-CE) for player profile exchange, MC port
//! discovery, and host mod info.
//!
//! ## Architecture (mirrors PCL-CE)
//!
//! ### Host flow
//! 1. User launches a Minecraft instance and opens the world to LAN.
//! 2. `BroadcastListener` (UDP 224.0.2.60:4445 multicast) auto-detects the LAN port.
//!    - In delayed mode (mc_port=0), the host creates the lobby first, and the
//!      `GameWatcher` waits for the LAN broadcast to populate the MC port.
//! 3. `link_create_lobby` starts:
//!    - `ScaffoldingServer` on a random `scf_port` (TCP listener on 0.0.0.0)
//!    - EasyTier with `hostname=scaffolding-mc-server-{scf_port}`,
//!      `--tcp-whitelist={scf_port}`, `--udp-whitelist={scf_port}`,
//!      and (if mc_port>0) also `{mc_port}` whitelists.
//! 4. `GameWatcher` pings the MC port every 3s to detect process exit.
//!
//! ### Client flow
//! 1. `link_join_lobby` parses the lobby code (network_name, network_secret).
//! 2. EasyTier starts with `hostname={GUID}`, no whitelist.
//! 3. Wait for peer list, find host by hostname prefix `scaffolding-mc-server-`.
//! 4. Add EasyTier CLI port-forward: `127.0.0.1:local_scf_port → host_ip:scf_port`
//!    (4 entries: tcp/udp × IPv4/IPv6).
//! 5. `ScaffoldingClient` connects to `127.0.0.1:local_scf_port`, sends
//!    `c:player_ping` with local PlayerProfile, starts 5s heartbeat.
//! 6. Query `c:server_port`:
//!    - If >0: add port-forward `127.0.0.1:local_mc_port → host_ip:mc_port`,
//!      start `BroadcastLocal` sending `[MOTD]...[/MOTD][AD]{local_mc_port}[/AD]`
//!      to `127.0.0.1:4445` every 1.5s. MC client auto-discovers the server.
//!    - If 0: poll until host opens LAN.
//!
//! Lobby codes are PCL-CE compatible (`U/XXXX-XXXX-XXXX-XXXX`, 34-base, mod-7
//! checksum, `scaffolding-mc-` network name prefix).

use crate::api::scaffolding::{
    build_local_profile, HostModInfo, PlayerKind, PlayerProfile, ScaffoldingClient,
    ScaffoldingServer,
};
use crate::api::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::net::UdpSocket;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tauri::Manager;
use tauri_plugin_http::reqwest;

const EASYTIER_VERSION: &str = "v2.2.4";
const EASYTIER_DOWNLOAD_URL: &str =
    "https://github.com/EasyTier/EasyTier/releases/download/v2.2.4/easytier-windows-x86_64-v2.2.4.zip";
const HOST_VIRTUAL_IP: &str = "10.114.51.41";

/// Multicast endpoints used by Minecraft LAN discovery.
const LAN_MULTICAST_V4: &str = "224.0.2.60";
const LAN_MULTICAST_V6: &str = "ff75:230::60";
const LAN_MULTICAST_PORT: u16 = 4445;

/// 34-char alphabet used by PCL-CE lobby codes (no I/O, but I→1, O→0 on parse).
const LOBBY_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";
const LOBBY_BASE: u128 = 34;
const LOBBY_DATA_LEN: usize = 16;
const LOBBY_FULL_PREFIX: &str = "U/";
const LOBBY_NET_NAME_PREFIX: &str = "scaffolding-mc-";

/// Public relay nodes used as fallback when the uptime API is unreachable.
const FALLBACK_PUBLIC_NODES: &[&str] = &[
    "tcp://public.easytier.top:11010",
    "tcp://public2.easytier.cn:54321",
    "https://etnode.zkitefly.eu.org/node1",
    "https://etnode.zkitefly.eu.org/node2",
];

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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
    /// Host: the MC LAN port (0 if delayed mode and not yet detected).
    /// Client: the MC port reported by the host's Scaffolding server.
    pub mc_port: Option<u16>,
    /// Client: the local port that MC should connect to (via BroadcastLocal).
    pub local_port: Option<u16>,
    /// Scaffolding protocol port.
    pub scf_port: Option<u16>,
    pub peer_count: usize,
    pub player_count: usize,
    pub last_refresh: Option<i64>,
    pub error: Option<String>,
    pub initialized: bool,
    /// Host: whether GameWatcher has detected MC is running.
    /// Client: whether ScaffoldingClient is connected.
    pub scaffolding_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalWorld {
    pub motd: String,
    pub port: u16,
    pub last_seen: i64,
}

// ---------------------------------------------------------------------------
// LinkState
// ---------------------------------------------------------------------------

struct LinkState {
    role: LinkRole,
    lobby_code: Option<String>,
    network_name: Option<String>,
    network_secret: Option<String>,
    mc_port: Option<u16>,
    local_proxy_port: Option<u16>,
    scf_port: Option<u16>,
    local_scf_port: Option<u16>,
    easytier_rpc_port: Option<u16>,
    easytier: Option<Child>,
    peer_refresh: Option<JoinHandle<()>>,
    peer_count: usize,
    last_peer_refresh: Option<chrono::DateTime<chrono::Utc>>,
    error: Option<String>,
    easytier_ready: bool,
    /// Local user profile (used by Scaffolding heartbeat).
    local_profile: PlayerProfile,
    /// Host: Scaffolding server instance.
    scaffolding_server: Option<ScaffoldingServer>,
    /// Client: Scaffolding client instance.
    scaffolding_client: Option<ScaffoldingClient>,
    /// Client: BroadcastLocal task handle.
    broadcast_local: Option<JoinHandle<()>>,
    /// Host: GameWatcher task handle (monitors MC process / LAN port).
    game_watcher: Option<JoinHandle<()>>,
    /// Latest cached player list (synced from Scaffolding heartbeat).
    players: Vec<PlayerProfile>,
    /// Host: mod list to advertise via `c:host_mods`.
    host_mods: Vec<HostModInfo>,
}

impl Default for LinkState {
    fn default() -> Self {
        Self {
            role: LinkRole::Idle,
            lobby_code: None,
            network_name: None,
            network_secret: None,
            mc_port: None,
            local_proxy_port: None,
            scf_port: None,
            local_scf_port: None,
            easytier_rpc_port: None,
            easytier: None,
            peer_refresh: None,
            peer_count: 0,
            last_peer_refresh: None,
            error: None,
            easytier_ready: false,
            local_profile: build_local_profile("Player".to_string()),
            scaffolding_server: None,
            scaffolding_client: None,
            broadcast_local: None,
            game_watcher: None,
            players: Vec::new(),
            host_mods: Vec::new(),
        }
    }
}

pub struct LinkManager {
    state: Arc<RwLock<LinkState>>,
}

impl Clone for LinkManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl Default for LinkManager {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(LinkState::default())),
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin init
// ---------------------------------------------------------------------------

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("link")
        .invoke_handler(tauri::generate_handler![
            link_create_lobby,
            link_join_lobby,
            link_leave_lobby,
            link_get_lobby_status,
            link_check_easytier_ready,
            link_discover_local_worlds,
            link_get_players,
            link_get_host_mods,
            link_check_protocols,
            link_get_mc_port,
            link_set_host_mods,
            link_update_mc_port,
            link_set_local_player_name,
        ])
        .setup(|app, _api| {
            tracing::info!("[link] Plugin setup: managing LinkManager state");
            app.manage(LinkManager::default());
            tracing::info!("[link] Plugin setup complete");
            Ok(())
        })
        .build()
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn link_check_easytier_ready(manager: tauri::State<'_, LinkManager>) -> Result<bool> {
    let state = manager.state.read().await;
    Ok(state.easytier_ready)
}

/// Create a lobby. `mc_port=0` enables delayed mode (wait for MC to open LAN).
/// Returns the lobby code.
#[tauri::command]
pub async fn link_create_lobby(
    manager: tauri::State<'_, LinkManager>,
    mc_port: u16,
    player_name: String,
    host_mods: Vec<HostModInfo>,
) -> Result<String> {
    tracing::info!(
        "[link] link_create_lobby: mc_port={}, player_name={}",
        mc_port,
        player_name
    );
    link_leave_lobby(manager.clone()).await.ok();

    let (lobby_code, network_name, network_secret) = generate_lobby_code();
    tracing::info!(
        "[link] Generated lobby: code={}, network={}",
        lobby_code,
        network_name
    );

    // Build local player profile.
    let profile = build_local_profile(player_name);

    // Pick a free port for the Scaffolding protocol server.
    let scf_port = find_free_port().await?;
    tracing::info!("[link] Scaffolding server port: {}", scf_port);

    // Start Scaffolding server (binds 0.0.0.0:scf_port).
    let server = ScaffoldingServer::start(scf_port, mc_port, host_mods.clone(), profile.clone())
        .await
        .map_err(|e| {
            theseus::ErrorKind::LauncherError(format!(
                "Failed to start Scaffolding server: {e}"
            ))
            .as_error()
        })?;

    {
        let mut state = manager.state.write().await;
        state.role = LinkRole::Host;
        state.lobby_code = Some(lobby_code.clone());
        state.network_name = Some(network_name.clone());
        state.network_secret = Some(network_secret.clone());
        state.mc_port = if mc_port > 0 { Some(mc_port) } else { None };
        state.scf_port = Some(scf_port);
        state.local_profile = profile.clone();
        state.host_mods = host_mods;
        state.scaffolding_server = Some(server);
        state.error = None;
    }

    // Start EasyTier with hostname=scaffolding-mc-server-{scf_port}.
    match start_easytier_core(
        &manager.state,
        true,
        scf_port,
        mc_port,
        &network_name,
        &network_secret,
    )
    .await
    {
        Ok((child, rpc_port)) => {
            let mut state = manager.state.write().await;
            state.easytier = Some(child);
            state.easytier_rpc_port = Some(rpc_port);
            state.easytier_ready = true;
            state.peer_refresh = Some(spawn_peer_refresh_loop(manager.state.clone(), rpc_port));
            tracing::info!("[link] EasyTier host started, rpc_port={}", rpc_port);
        }
        Err(e) => {
            tracing::error!("[link] Failed to start EasyTier: {}", e);
            let mut state = manager.state.write().await;
            state.error = Some(e.to_string());
            return Err(e.into());
        }
    }

    // Start GameWatcher to monitor MC process / detect LAN port in delayed mode.
    let watcher = spawn_game_watcher(manager.state.clone(), scf_port, mc_port);
    {
        let mut state = manager.state.write().await;
        state.game_watcher = Some(watcher);
    }

    Ok(lobby_code)
}

/// Join a lobby. Returns nothing — UI should poll `link_get_lobby_status` /
/// `link_get_mc_port` / `link_get_players` to know when ready.
#[tauri::command]
pub async fn link_join_lobby(
    manager: tauri::State<'_, LinkManager>,
    lobby_code: String,
    player_name: String,
) -> Result<()> {
    tracing::info!("[link] link_join_lobby: code={}", lobby_code);
    link_leave_lobby(manager.clone()).await.ok();

    let (network_name, network_secret) = parse_lobby_code(&lobby_code)?;
    tracing::info!(
        "[link] Parsed lobby: network={}, secret_len={}",
        network_name,
        network_secret.len()
    );

    let profile = build_local_profile(player_name);

    {
        let mut state = manager.state.write().await;
        state.role = LinkRole::Client;
        state.lobby_code = Some(lobby_code.clone());
        state.network_name = Some(network_name.clone());
        state.network_secret = Some(network_secret.clone());
        state.local_profile = profile.clone();
        state.error = None;
    }

    // Start EasyTier client with hostname=GUID, no whitelist.
    match start_easytier_core(&manager.state, false, 0, 0, &network_name, &network_secret).await {
        Ok((child, rpc_port)) => {
            let mut state = manager.state.write().await;
            state.easytier = Some(child);
            state.easytier_rpc_port = Some(rpc_port);
            state.easytier_ready = true;
            state.peer_refresh = Some(spawn_peer_refresh_loop(manager.state.clone(), rpc_port));
            tracing::info!("[link] EasyTier client started, rpc_port={}", rpc_port);
        }
        Err(e) => {
            tracing::error!("[link] Failed to start EasyTier: {}", e);
            let mut state = manager.state.write().await;
            state.error = Some(e.to_string());
            return Err(e.into());
        }
    }

    // Wait for the host to appear in the peer list, get its virtual IP + scf_port.
    let (host_ip, scf_port) = wait_for_host(&manager.state).await?;
    tracing::info!(
        "[link] Host found at {}:{} (scf_port)",
        host_ip,
        scf_port
    );

    // Add EasyTier CLI port-forward: 127.0.0.1:local_scf_port → host_ip:scf_port.
    let local_scf_port = add_port_forward(&manager.state, &host_ip, scf_port).await?;
    {
        let mut state = manager.state.write().await;
        state.local_scf_port = Some(local_scf_port);
    }
    tracing::info!(
        "[link] Port-forward established: 127.0.0.1:{} → {}:{}",
        local_scf_port,
        host_ip,
        scf_port
    );

    // Connect ScaffoldingClient to 127.0.0.1:local_scf_port.
    let scf_addr: SocketAddr = format!("127.0.0.1:{}", local_scf_port).parse().map_err(|e| {
        theseus::ErrorKind::OtherError(format!("Invalid SocketAddr: {e}")).as_error()
    })?;
    let client = ScaffoldingClient::connect(scf_addr, profile).await.map_err(|e| {
        theseus::ErrorKind::LauncherError(format!("ScaffoldingClient connect failed: {e}"))
            .as_error()
    })?;
    {
        let mut state = manager.state.write().await;
        state.scaffolding_client = Some(client);
    }
    tracing::info!("[link] Scaffolding client connected");

    // Spawn a watcher that polls for MC port and starts BroadcastLocal when ready.
    let watcher = spawn_client_mc_watcher(manager.state.clone(), host_ip);
    {
        let mut state = manager.state.write().await;
        state.game_watcher = Some(watcher);
    }

    Ok(())
}

#[tauri::command]
pub async fn link_leave_lobby(manager: tauri::State<'_, LinkManager>) -> Result<()> {
    tracing::info!("[link] link_leave_lobby");
    let mut state = manager.state.write().await;

    if let Some(mut child) = state.easytier.take() {
        let _ = child.kill().await;
    }
    if let Some(refresh) = state.peer_refresh.take() {
        refresh.abort();
    }
    if let Some(watcher) = state.game_watcher.take() {
        watcher.abort();
    }
    if let Some(bcast) = state.broadcast_local.take() {
        bcast.abort();
    }
    if let Some(server) = state.scaffolding_server.take() {
        server.stop().await;
    }
    if let Some(client) = state.scaffolding_client.take() {
        client.disconnect().await;
    }

    *state = LinkState::default();
    Ok(())
}

#[tauri::command]
pub async fn link_get_lobby_status(manager: tauri::State<'_, LinkManager>) -> Result<LobbyStatus> {
    let state = manager.state.read().await;
    Ok(LobbyStatus {
        state: state.role,
        lobby_code: state.lobby_code.clone(),
        network_name: state.network_name.clone(),
        virtual_ip: Some(HOST_VIRTUAL_IP.to_string()),
        mc_port: state.mc_port,
        local_port: state.local_proxy_port,
        scf_port: state.scf_port,
        peer_count: state.peer_count,
        player_count: state.players.len(),
        last_refresh: state.last_peer_refresh.map(|dt| dt.timestamp_millis()),
        error: state.error.clone(),
        initialized: state.easytier_ready,
        scaffolding_ready: state.scaffolding_server.is_some() || state.scaffolding_client.is_some(),
    })
}

/// One-shot discovery of local MC LAN worlds (host side).
/// Listens on the multicast group for ~2 seconds and returns what was heard.
#[tauri::command]
pub async fn link_discover_local_worlds(
    _manager: tauri::State<'_, LinkManager>,
) -> Result<Vec<LocalWorld>> {
    Ok(discover_local_worlds(Duration::from_secs(2)).await)
}

/// Get the current player list (host + guests).
#[tauri::command]
pub async fn link_get_players(
    manager: tauri::State<'_, LinkManager>,
) -> Result<Vec<PlayerProfile>> {
    let mut state = manager.state.write().await;
    // Refresh from Scaffolding before returning.
    if let Some(ref server) = state.scaffolding_server {
        state.players = server.get_players().await;
    } else if let Some(ref client) = state.scaffolding_client {
        state.players = client.get_players().await;
    }
    Ok(state.players.clone())
}

/// Get the host's mod list (client side). Asks the server via `c:host_mods`.
#[tauri::command]
pub async fn link_get_host_mods(
    manager: tauri::State<'_, LinkManager>,
) -> Result<Vec<HostModInfo>> {
    let state = manager.state.read().await;
    if let Some(ref client) = state.scaffolding_client {
        match client.get_host_mods().await {
            Ok(mods) => return Ok(mods),
            Err(e) => {
                tracing::warn!("[link] get_host_mods failed: {}", e);
                return Ok(vec![]);
            }
        }
    }
    // Host: return own mod list.
    Ok(state.host_mods.clone())
}

/// Negotiate supported protocols with the server (client side).
#[tauri::command]
pub async fn link_check_protocols(
    manager: tauri::State<'_, LinkManager>,
    supported: Vec<String>,
) -> Result<Vec<String>> {
    let state = manager.state.read().await;
    if let Some(ref client) = state.scaffolding_client {
        return Ok(client.check_protocols(supported).await.unwrap_or_default());
    }
    Ok(vec![])
}

/// Get the MC port. Client: queries the host's Scaffolding server.
/// Host: returns the locally-set MC port.
#[tauri::command]
pub async fn link_get_mc_port(manager: tauri::State<'_, LinkManager>) -> Result<Option<u16>> {
    let state = manager.state.read().await;
    if state.role == LinkRole::Host {
        return Ok(state.mc_port);
    }
    if let Some(ref client) = state.scaffolding_client {
        return Ok(client.get_server_port().await.unwrap_or(None));
    }
    Ok(None)
}

/// Host: update the MC port (used by delayed mode + GameWatcher auto-detection).
#[tauri::command]
pub async fn link_update_mc_port(
    manager: tauri::State<'_, LinkManager>,
    port: u16,
) -> Result<()> {
    let mut state = manager.state.write().await;
    state.mc_port = Some(port);
    if let Some(ref server) = state.scaffolding_server {
        server.update_mc_port(port).await;
    }
    tracing::info!("[link] MC port updated to {}", port);
    Ok(())
}

/// Host: update the mod list advertised via `c:host_mods`.
#[tauri::command]
pub async fn link_set_host_mods(
    manager: tauri::State<'_, LinkManager>,
    mods: Vec<HostModInfo>,
) -> Result<()> {
    let mut state = manager.state.write().await;
    state.host_mods = mods.clone();
    if let Some(ref server) = state.scaffolding_server {
        server.update_host_mods(mods).await;
    }
    Ok(())
}

/// Update the local player's display name (used by Scaffolding heartbeat).
/// Note: takes effect on the next heartbeat.
#[tauri::command]
pub async fn link_set_local_player_name(
    manager: tauri::State<'_, LinkManager>,
    name: String,
) -> Result<()> {
    let mut state = manager.state.write().await;
    state.local_profile.name = name;
    Ok(())
}

// ---------------------------------------------------------------------------
// Lobby code generation / parsing (PCL-CE compatible)
// ---------------------------------------------------------------------------

fn generate_lobby_code() -> (String, String, String) {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);

    let mut value = u128::from_le_bytes(bytes);
    let remainder = value % 7;
    value -= remainder;

    let mut chars = [0u8; LOBBY_DATA_LEN];
    let mut v = value;
    for i in 0..LOBBY_DATA_LEN {
        chars[i] = LOBBY_ALPHABET[(v % LOBBY_BASE) as usize];
        v /= LOBBY_BASE;
    }

    let payload: String = (0..LOBBY_DATA_LEN)
        .map(|i| {
            let mut s = String::new();
            s.push(chars[i] as char);
            if i == 3 || i == 7 || i == 11 {
                s.push('-');
            }
            s
        })
        .collect();

    let full_code = format!("{}{}", LOBBY_FULL_PREFIX, payload);
    let payload_no_hyphen: String = payload.chars().filter(|c| *c != '-').collect();
    let network_name_payload = &payload_no_hyphen[..9];
    let network_secret_payload = &payload_no_hyphen[9..];
    let network_name = format!("{}{}", LOBBY_NET_NAME_PREFIX, network_name_payload);
    let network_secret = network_secret_payload.to_string();

    (full_code, network_name, network_secret)
}

fn parse_lobby_code(code: &str) -> theseus::Result<(String, String)> {
    let code = code.trim();
    if !code.starts_with(LOBBY_FULL_PREFIX) {
        return Err(theseus::ErrorKind::InputError(format!(
            "Invalid lobby code: must start with '{}'",
            LOBBY_FULL_PREFIX
        ))
        .as_error());
    }
    if code.len() != 21 {
        return Err(theseus::ErrorKind::InputError(format!(
            "Invalid lobby code: expected 21 characters, got {}",
            code.len()
        ))
        .as_error());
    }

    let payload = &code[LOBBY_FULL_PREFIX.len()..];
    for (i, c) in payload.chars().enumerate() {
        if i == 4 || i == 9 || i == 14 {
            if c != '-' {
                return Err(theseus::ErrorKind::InputError(format!(
                    "Invalid lobby code: expected '-' at position {}",
                    i
                ))
                .as_error());
            }
            continue;
        }
        if !c.is_ascii_alphanumeric() {
            return Err(theseus::ErrorKind::InputError(format!(
                "Invalid lobby code character: '{}'",
                c
            ))
            .as_error());
        }
    }

    let payload_no_hyphen: String = payload.chars().filter(|c| *c != '-').collect();
    let mut value: u128 = 0;
    for c in payload_no_hyphen.chars().rev() {
        let upper = c.to_ascii_uppercase();
        let digit = match upper {
            '0'..='9' => (upper as u8) - b'0',
            'A'..='H' => (upper as u8) - b'A' + 10,
            'J'..='N' => (upper as u8) - b'A' + 9,
            'P'..='Z' => (upper as u8) - b'A' + 9,
            'I' => 1,
            'O' => 0,
            _ => {
                return Err(theseus::ErrorKind::InputError(format!(
                    "Invalid lobby code character: '{}'",
                    c
                ))
                .as_error())
            }
        };
        value = value * LOBBY_BASE + digit as u128;
    }
    if value % 7 != 0 {
        return Err(theseus::ErrorKind::InputError(
            "Invalid lobby code: checksum verification failed".to_string(),
        )
        .as_error());
    }

    let network_name_payload = &payload_no_hyphen[..9];
    let network_secret_payload = &payload_no_hyphen[9..];
    let network_name = format!("{}{}", LOBBY_NET_NAME_PREFIX, network_name_payload);
    let network_secret = network_secret_payload.to_string();

    Ok((network_name, network_secret))
}

// ---------------------------------------------------------------------------
// EasyTier binary management
// ---------------------------------------------------------------------------

async fn easytier_dir() -> theseus::Result<std::path::PathBuf> {
    let state = theseus::State::get().await?;
    Ok(state.directories.caches_dir().join("EasyTier"))
}

async fn ensure_easytier_binaries() -> theseus::Result<()> {
    let dir = easytier_dir().await?;
    let core = dir.join("easytier-core.exe");
    let cli = dir.join("easytier-cli.exe");

    if core.exists() && cli.exists() {
        tracing::info!("[link] EasyTier binaries already present at {:?}", dir);
        return Ok(());
    }

    fs::create_dir_all(&dir).await.map_err(|e| {
        theseus::ErrorKind::FSError(format!("Failed to create EasyTier directory: {e}")).as_error()
    })?;

    let loading_bar = theseus::init_loading(
        theseus::LoadingBarType::EasyTierDownload {
            version: EASYTIER_VERSION.to_string(),
        },
        100.0,
        "Downloading EasyTier VPN core...",
    )
    .await?;

    tracing::info!("[link] Downloading EasyTier from {}", EASYTIER_DOWNLOAD_URL);
    theseus::emit_loading(&loading_bar, 0.0, Some("Connecting..."))?;

    let response = reqwest::get(EASYTIER_DOWNLOAD_URL).await.map_err(|e| {
        theseus::emit_loading(&loading_bar, 100.0, Some("Download failed")).ok();
        theseus::ErrorKind::OtherError(format!(
            "Failed to download EasyTier {EASYTIER_VERSION}: {e}. Check your network connection."
        ))
        .as_error()
    })?;
    let response = response.error_for_status().map_err(|e| {
        theseus::emit_loading(&loading_bar, 100.0, Some("Download failed")).ok();
        theseus::ErrorKind::OtherError(format!(
            "Failed to download EasyTier {EASYTIER_VERSION}: {e}"
        ))
        .as_error()
    })?;

    let total = response.content_length().unwrap_or(0);
    tracing::info!("[link] EasyTier zip size: {} bytes", total);

    use futures::StreamExt;
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::with_capacity(total as usize);
    let mut last_percent: f64 = 0.0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            theseus::emit_loading(&loading_bar, 100.0, Some("Download failed")).ok();
            theseus::ErrorKind::OtherError(format!("EasyTier download stream error: {e}")).as_error()
        })?;
        buffer.extend_from_slice(&chunk);
        if total > 0 {
            let percent = (buffer.len() as f64 / total as f64) * 80.0;
            if percent - last_percent >= 1.0 {
                theseus::emit_loading(
                    &loading_bar,
                    percent - last_percent,
                    Some(&format!("Downloading... {}%", (percent * 100.0 / 80.0) as u32)),
                )?;
                last_percent = percent;
            }
        }
    }

    theseus::emit_loading(&loading_bar, 5.0, Some("Extracting..."))?;
    tracing::info!("[link] Downloaded {} bytes, extracting", buffer.len());

    extract_easytier_zip(&buffer, &dir).await?;

    if !core.exists() || !cli.exists() {
        theseus::emit_loading(&loading_bar, 15.0, Some("Extraction failed"))?;
        return Err(theseus::ErrorKind::FSError(
            "EasyTier binaries missing after extraction".to_string(),
        )
        .as_error());
    }

    theseus::emit_loading(&loading_bar, 100.0, Some("Done"))?;
    tracing::info!("[link] EasyTier binaries ready at {:?}", dir);
    Ok(())
}

async fn extract_easytier_zip(bytes: &[u8], dir: &std::path::Path) -> theseus::Result<()> {
    let mut reader =
        async_zip::tokio::read::seek::ZipFileReader::with_tokio(std::io::Cursor::new(
            bytes.to_vec(),
        ))
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

        if name != "easytier-core.exe"
            && name != "easytier-cli.exe"
            && name != "Packet.dll"
            && name != "easytier-web.exe"
        {
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
        tracing::debug!("[link] Extracted {}", name);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// EasyTier process management
// ---------------------------------------------------------------------------

/// Start EasyTier. For host: hostname=`scaffolding-mc-server-{scf_port}`,
/// tcp/udp-whitelist={scf_port} (+ {mc_port} if > 0).
/// For client: hostname={GUID}, no whitelist.
async fn start_easytier_core(
    _state: &RwLock<LinkState>,
    as_host: bool,
    scf_port: u16,
    mc_port: u16,
    network_name: &str,
    network_secret: &str,
) -> theseus::Result<(Child, u16)> {
    tracing::info!(
        "[link] start_easytier_core: as_host={}, scf_port={}, mc_port={}, network={}",
        as_host,
        scf_port,
        mc_port,
        network_name
    );
    ensure_easytier_binaries().await?;

    let dir = easytier_dir().await?;
    let core = dir.join("easytier-core.exe");

    let rpc_port = find_free_port().await?;
    tracing::debug!("[link] RPC port: {}", rpc_port);

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
        .arg(network_name)
        .arg("--network-secret")
        .arg(network_secret)
        .arg("--rpc-portal")
        .arg(format!("127.0.0.1:{rpc_port}"))
        .arg("--private-mode")
        .arg("true")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let relays = get_public_nodes().await;
    for relay in &relays {
        cmd.arg("-p").arg(relay);
    }

    if as_host {
        // Critical: hostname uses the SCAFFOLDING port, not the MC port.
        // Clients find the host by matching this hostname prefix.
        cmd.arg("-i")
            .arg(HOST_VIRTUAL_IP)
            .arg("--hostname")
            .arg(format!("scaffolding-mc-server-{scf_port}"))
            .arg("--tcp-whitelist")
            .arg(scf_port.to_string())
            .arg("--udp-whitelist")
            .arg(scf_port.to_string());
        if mc_port > 0 {
            cmd.arg("--tcp-whitelist")
                .arg(mc_port.to_string())
                .arg("--udp-whitelist")
                .arg(mc_port.to_string());
        }
        cmd.arg("-l")
            .arg("tcp://0.0.0.0:0")
            .arg("-l")
            .arg("udp://0.0.0.0:0");
    } else {
        cmd.arg("--hostname")
            .arg(uuid::Uuid::new_v4().to_string())
            .arg("--tcp-whitelist")
            .arg("0")
            .arg("--udp-whitelist")
            .arg("0")
            .arg("-l")
            .arg("tcp://0.0.0.0:0")
            .arg("-l")
            .arg("udp://0.0.0.0:0");
    }

    tracing::debug!("[link] EasyTier args: {:?}", cmd.as_std());

    let child = cmd.spawn().map_err(|e| {
        theseus::ErrorKind::LauncherError(format!("Failed to start EasyTier: {e}")).as_error()
    })?;
    tracing::info!(
        "[link] EasyTier process started, PID: {}",
        child.id().unwrap_or(0)
    );

    Ok((child, rpc_port))
}

async fn get_public_nodes() -> Vec<String> {
    match try_fetch_public_nodes().await {
        Ok(nodes) => {
            if nodes.is_empty() {
                FALLBACK_PUBLIC_NODES.iter().map(|s| s.to_string()).collect()
            } else {
                nodes
            }
        }
        Err(e) => {
            tracing::warn!("[link] Failed to fetch public nodes: {}. Using fallback.", e);
            FALLBACK_PUBLIC_NODES.iter().map(|s| s.to_string()).collect()
        }
    }
}

async fn try_fetch_public_nodes() -> theseus::Result<Vec<String>> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct PublicNodeResponse {
        data: PublicNodeData,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PublicNodeData {
        items: Vec<PublicNodeItem>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PublicNodeItem {
        host: String,
        is_active: bool,
        is_allow_relay: bool,
    }

    let resp = reqwest::get("https://uptime.easytier.cn/api/nodes?page=1&per_page=50&is_active=true")
        .await
        .map_err(|e| {
            theseus::ErrorKind::OtherError(format!("Failed to fetch public nodes: {e}")).as_error()
        })?;

    let parsed: PublicNodeResponse = resp.json().await.map_err(|e| {
        theseus::ErrorKind::OtherError(format!("Failed to parse public nodes: {e}")).as_error()
    })?;

    let nodes: Vec<String> = parsed
        .data
        .items
        .into_iter()
        .filter(|n| n.is_active && n.is_allow_relay)
        .map(|n| n.host)
        .take(6)
        .collect();

    Ok(nodes)
}

// ---------------------------------------------------------------------------
// Peer discovery / host finding
// ---------------------------------------------------------------------------

/// Wait for the peer list to populate, then find the host's IP and scf_port.
async fn wait_for_host(state: &Arc<RwLock<LinkState>>) -> theseus::Result<(String, u16)> {
    let rpc_port = {
        let s = state.read().await;
        s.easytier_rpc_port
            .ok_or_else(|| theseus::ErrorKind::InputError("RPC port not set".to_string()).as_error())?
    };

    let dir = easytier_dir().await?;
    let cli = dir.join("easytier-cli.exe");

    let max_retries = 30u32;
    let mut retry = 0u32;
    loop {
        retry += 1;
        if retry > max_retries {
            return Err(theseus::ErrorKind::OtherError(
                "Timed out waiting for EasyTier peers".to_string(),
            )
            .as_error());
        }

        match query_peers(&cli, rpc_port).await {
            Ok(peers) => {
                if let Some(host) = find_host_in_peers(&peers) {
                    tracing::info!("[link] Found host: {:?}", host);
                    return Ok(host);
                }
            }
            Err(e) => {
                tracing::debug!("[link] Peer query failed (retry {}): {}", retry, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[derive(Debug, Deserialize)]
struct PeerInfo {
    hostname: Option<String>,
    ip: Option<String>,
}

async fn query_peers(cli: &std::path::Path, rpc_port: u16) -> theseus::Result<Vec<PeerInfo>> {
    let output = Command::new(cli)
        .arg("--rpc-portal")
        .arg(format!("127.0.0.1:{rpc_port}"))
        .arg("-o")
        .arg("json")
        .arg("peer")
        .output()
        .await
        .map_err(|e| {
            theseus::ErrorKind::LauncherError(format!("easytier-cli peer failed: {e}")).as_error()
        })?;

    if !output.status.success() {
        return Err(theseus::ErrorKind::OtherError(format!(
            "easytier-cli returned non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
        .as_error());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);

    let peers = parsed
        .get("peers")
        .and_then(|p| p.as_array())
        .or_else(|| parsed.as_array())
        .cloned()
        .unwrap_or_default();

    let result: Vec<PeerInfo> = peers
        .iter()
        .filter_map(|p| {
            Some(PeerInfo {
                hostname: p.get("hostname").and_then(|h| h.as_str()).map(|s| s.to_string()),
                ip: p.get("ip")
                    .or_else(|| p.get("ipv4"))
                    .and_then(|h| h.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect();

    Ok(result)
}

/// Find the host in the peer list by hostname prefix `scaffolding-mc-server-{scf_port}`.
fn find_host_in_peers(peers: &[PeerInfo]) -> Option<(String, u16)> {
    for peer in peers {
        if let Some(ref hostname) = peer.hostname {
            if let Some(rest) = hostname.strip_prefix("scaffolding-mc-server-") {
                if let Ok(port) = rest.parse::<u16>() {
                    if let Some(ref ip) = peer.ip {
                        return Some((ip.clone(), port));
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// EasyTier CLI port-forward (4 entries: tcp/udp × IPv4/IPv6)
// ---------------------------------------------------------------------------

/// Add a port-forward via `easytier-cli port-forward add`.
/// Picks a free local port, forwards to `target_ip:target_port`.
/// Returns the local port.
async fn add_port_forward(
    state: &Arc<RwLock<LinkState>>,
    target_ip: &str,
    target_port: u16,
) -> theseus::Result<u16> {
    let rpc_port = {
        let s = state.read().await;
        s.easytier_rpc_port
            .ok_or_else(|| theseus::ErrorKind::InputError("RPC port not set".to_string()).as_error())?
    };

    let dir = easytier_dir().await?;
    let cli = dir.join("easytier-cli.exe");
    let local_port = find_free_port().await?;
    let target_ip_v6 = if target_ip.contains(':') {
        target_ip.to_string()
    } else {
        target_ip.to_string()
    };

    // 4 CLI commands: tcp/udp × IPv4/IPv6.
    let commands = [
        format!("tcp 127.0.0.1:{local_port} {target_ip}:{target_port}"),
        format!("tcp [::1]:{local_port} [{target_ip_v6}]:{target_port}"),
        format!("udp 127.0.0.1:{local_port} {target_ip}:{target_port}"),
        format!("udp [::1]:{local_port} [{target_ip_v6}]:{target_port}"),
    ];

    for cmd_str in &commands {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }
        let result = Command::new(&cli)
            .arg("--rpc-portal")
            .arg(format!("127.0.0.1:{rpc_port}"))
            .arg("port-forward")
            .arg("add")
            .arg(parts[0]) // tcp/udp
            .arg(parts[1]) // local
            .arg(parts[2]) // target_ip (may have brackets)
            .arg(parts[3]) // target_port (combined)
            .output()
            .await;

        match result {
            Ok(out) => {
                if !out.status.success() {
                    tracing::warn!(
                        "[link] port-forward add failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
            }
            Err(e) => {
                tracing::warn!("[link] port-forward add spawn failed: {}", e);
            }
        }
    }

    tracing::info!(
        "[link] Port-forward added: 127.0.0.1:{} → {}:{}",
        local_port,
        target_ip,
        target_port
    );
    Ok(local_port)
}

// ---------------------------------------------------------------------------
// BroadcastListener — discover local MC LAN worlds
// ---------------------------------------------------------------------------

/// Listen on UDP 224.0.2.60:4445 (IPv4) and ff75:230::60:4445 (IPv6) for
/// Minecraft LAN broadcast packets. Returns discovered worlds.
async fn discover_local_worlds(timeout: Duration) -> Vec<LocalWorld> {
    let mut worlds: Vec<LocalWorld> = Vec::new();
    let v4_socket = UdpSocket::bind(("0.0.0.0", LAN_MULTICAST_PORT)).await.ok();
    let v6_socket = UdpSocket::bind(("::", LAN_MULTICAST_PORT)).await.ok();

    // Join multicast groups.
    if let Some(ref sock) = v4_socket {
        let multi_addr: std::net::Ipv4Addr = LAN_MULTICAST_V4.parse().unwrap();
        let _ = sock.join_multicast_v4(multi_addr, "0.0.0.0".parse().unwrap());
    }
    if let Some(ref sock) = v6_socket {
        let multi_addr: std::net::Ipv6Addr = LAN_MULTICAST_V6.parse().unwrap();
        let _ = sock.join_multicast_v6(&multi_addr, 0);
    }

    let deadline = tokio::time::Instant::now() + timeout;
    let mut buf = vec![0u8; 1024];

    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline - now;

        let v4_fut = async {
            if let Some(ref sock) = v4_socket {
                tokio::time::timeout(remaining, sock.recv_from(&mut buf)).await.ok()
            } else {
                None
            }
        };
        let v6_fut = async {
            if let Some(ref sock) = v6_socket {
                let mut buf6 = vec![0u8; 1024];
                tokio::time::timeout(remaining, sock.recv_from(&mut buf6))
                    .await
                    .ok()
                    .map(|r| (r, buf6))
            } else {
                None
            }
        };

        tokio::select! {
            result = v4_fut => {
                if let Some(Ok((len, _addr))) = result {
                    let msg = String::from_utf8_lossy(&buf[..len]).to_string();
                    if let Some(world) = parse_lan_broadcast(&msg) {
                        if !worlds.iter().any(|w| w.port == world.port) {
                            worlds.push(world);
                        }
                    }
                }
            }
            result = v6_fut => {
                if let Some((Ok((len, _addr)), _buf6)) = result {
                    let msg = String::from_utf8_lossy(&_buf6[..len]).to_string();
                    if let Some(world) = parse_lan_broadcast(&msg) {
                        if !worlds.iter().any(|w| w.port == world.port) {
                            worlds.push(world);
                        }
                    }
                }
            }
            _ = tokio::time::sleep(remaining) => break,
        }
    }

    worlds
}

fn parse_lan_broadcast(message: &str) -> Option<LocalWorld> {
    // Format: [MOTD]...[/MOTD][AD]port[/AD]
    let motd_start = message.find("[MOTD]").map(|i| i + 6);
    let motd_end = message.find("[/MOTD]")?;
    let ad_start = message.find("[AD]").map(|i| i + 4)?;
    let ad_end = message.find("[/AD]")?;

    let motd = if let Some(start) = motd_start {
        if start < motd_end {
            &message[start..motd_end]
        } else {
            "Minecraft Server"
        }
    } else {
        "Minecraft Server"
    };
    let port_str = &message[ad_start..ad_end];
    let port: u16 = port_str.trim().parse().ok()?;
    Some(LocalWorld {
        motd: motd.to_string(),
        port,
        last_seen: chrono::Utc::now().timestamp_millis(),
    })
}

// ---------------------------------------------------------------------------
// BroadcastLocal — make MC client auto-discover the proxy server
// ---------------------------------------------------------------------------

/// Start a background task that sends `[MOTD]Minecraft Server[/MOTD][AD]{port}[/AD]`
/// to `127.0.0.1:4445` every 1.5s. This makes the local MC client show the
/// proxied server in its multiplayer "Scanning for LAN worlds" list.
fn spawn_broadcast_local(local_port: u16, motd: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let sock = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("[link] BroadcastLocal socket bind failed: {}", e);
                return;
            }
        };
        let target: SocketAddr = format!("127.0.0.1:{LAN_MULTICAST_PORT}").parse().unwrap();
        let message = format!("[MOTD]{motd}[/MOTD][AD]{local_port}[/AD]");
        let bytes = message.as_bytes();

        let mut interval = tokio::time::interval(Duration::from_millis(1500));
        interval.tick().await; // skip first immediate
        loop {
            interval.tick().await;
            if let Err(e) = sock.send_to(bytes, target).await {
                tracing::debug!("[link] BroadcastLocal send failed: {}", e);
            }
        }
    })
}

// ---------------------------------------------------------------------------
// GameWatcher (host side)
// ---------------------------------------------------------------------------

/// Host-side watcher: in delayed mode (initial mc_port=0), listens for MC LAN
/// broadcasts to detect when the player opens their world to LAN. Once
/// detected, updates the ScaffoldingServer's mc_port so clients can connect.
fn spawn_game_watcher(
    state: Arc<RwLock<LinkState>>,
    _scf_port: u16,
    initial_mc_port: u16,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // If we already have a known mc_port, just monitor the MC process.
        if initial_mc_port > 0 {
            monitor_mc_process(state, initial_mc_port).await;
            return;
        }

        // Delayed mode: poll BroadcastListener until MC opens LAN.
        tracing::info!("[link] GameWatcher: delayed mode, waiting for MC LAN broadcast...");
        loop {
            let worlds = discover_local_worlds(Duration::from_secs(3)).await;
            if !worlds.is_empty() {
                let world = &worlds[0];
                tracing::info!(
                    "[link] GameWatcher: detected MC LAN world on port {} (motd={})",
                    world.port,
                    world.motd
                );
                let mut s = state.write().await;
                s.mc_port = Some(world.port);
                if let Some(ref server) = s.scaffolding_server {
                    server.update_mc_port(world.port).await;
                }
                drop(s);
                // Now monitor the MC process.
                monitor_mc_process(state, world.port).await;
                return;
            }
            // Check if we should stop (link was left).
            {
                let s = state.read().await;
                if s.role != LinkRole::Host {
                    return;
                }
            }
        }
    })
}

/// Periodically TCP-ping the MC port. If it stops responding, log a warning.
/// (We don't automatically kill the lobby — PCL leaves this to the user.)
async fn monitor_mc_process(state: Arc<RwLock<LinkState>>, mc_port: u16) {
    use tokio::net::TcpStream;
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    interval.tick().await; // skip first immediate
    loop {
        interval.tick().await;
        {
            let s = state.read().await;
            if s.role != LinkRole::Host {
                return;
            }
        }
        // Quick TCP connect check.
        let addr: SocketAddr = format!("127.0.0.1:{mc_port}").parse().unwrap();
        match tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(&addr)).await {
            Ok(Ok(_stream)) => {
                // MC is still alive.
            }
            Ok(Err(_)) | Err(_) => {
                tracing::warn!("[link] GameWatcher: MC port {} not responding", mc_port);
                // Don't auto-kill; just log. User can leave the lobby manually.
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Client-side MC watcher — polls Scaffolding for MC port, starts BroadcastLocal
// ---------------------------------------------------------------------------

fn spawn_client_mc_watcher(state: Arc<RwLock<LinkState>>, host_ip: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("[link] Client MC watcher: polling for host's MC port...");
        loop {
            // Check if link was left.
            {
                let s = state.read().await;
                if s.role != LinkRole::Client {
                    return;
                }
            }

            // Query Scaffolding for MC port.
            let mc_port = {
                let s = state.read().await;
                if let Some(ref client) = s.scaffolding_client {
                    client.get_server_port().await.unwrap_or(None)
                } else {
                    None
                }
            };

            if let Some(port) = mc_port {
                tracing::info!("[link] Client MC watcher: host's MC port is {}", port);
                // Add port-forward: 127.0.0.1:local_mc_port → host_ip:port.
                match add_port_forward(&state, &host_ip, port).await {
                    Ok(local_port) => {
                        let bcast = spawn_broadcast_local(local_port, "Minecraft Server".to_string());
                        let mut s = state.write().await;
                        s.local_proxy_port = Some(local_port);
                        s.broadcast_local = Some(bcast);
                        tracing::info!(
                            "[link] Client MC ready: local_port={} → {}:{}",
                            local_port,
                            host_ip,
                            port
                        );
                        return;
                    }
                    Err(e) => {
                        tracing::warn!("[link] Failed to add MC port-forward: {}", e);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    })
}

// ---------------------------------------------------------------------------
// Peer refresh loop
// ---------------------------------------------------------------------------

fn spawn_peer_refresh_loop(state: Arc<RwLock<LinkState>>, rpc_port: u16) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let dir = match easytier_dir().await {
                Ok(d) => d,
                Err(_) => continue,
            };
            let cli = dir.join("easytier-cli.exe");
            if !cli.exists() {
                continue;
            }

            match query_peers(&cli, rpc_port).await {
                Ok(peers) => {
                    let count = peers.len();
                    let mut state = state.write().await;
                    state.peer_count = count;
                    state.last_peer_refresh = Some(chrono::Utc::now());
                    state.error = None;
                }
                Err(e) => {
                    let mut state = state.write().await;
                    if state.peer_count == 0 {
                        state.error = Some(e.to_string());
                    }
                }
            }
        }
    })
}

async fn find_free_port() -> theseus::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

// Silence unused-import warnings for re-exported types used by the frontend.
#[allow(dead_code)]
fn _types_used_by_frontend() {
    let _ = PlayerKind::Host;
}
