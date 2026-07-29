//! Tauri plugin bindings for the link (online multiplayer) module.
//!
//! Exposes lobby creation/joining, player list, connection info,
//! mod compatibility checking, and game world discovery to the frontend.

use crate::api::{Result, TheseusSerializableError};
use parking_lot::RwLock;
use tauri::{Emitter, Manager, Runtime};
use theseus::link::{
    self, ChatMessage, ConnectionInfo, ConnectionQuality, ConnectionWay, FoundWorld, HostModInfo,
    LobbyState, ModCompatibilityResult, PlayerKind, PlayerProfile, ScaffoldingClient,
    ScaffoldingEvent, ScaffoldingServer,
};
use theseus::link::lobby_code;
use theseus::link::manager::LinkManager;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Global link state, managed by Tauri.
pub struct LinkGlobalState {
    pub state: RwLock<LinkInnerState>,
    pub manager: Mutex<LinkManager>,
    pub scf_server: Mutex<Option<ScaffoldingServer>>,
    pub scf_client: Mutex<Option<ScaffoldingClient>>,
}

/// Inner state that can be read/written without async.
#[derive(Default)]
pub struct LinkInnerState {
    pub lobby_state: LobbyState,
    pub lobby_code: Option<String>,
    pub username: Option<String>,
    pub machine_id: String,
    pub players: Vec<PlayerProfile>,
    pub is_host: bool,
    pub connection_info: ConnectionInfo,
    pub host_mods: Vec<HostModInfo>,
}

impl LinkGlobalState {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(LinkInnerState {
                machine_id: Uuid::new_v4().to_string(),
                ..Default::default()
            }),
            manager: Mutex::new(LinkManager::new()),
            scf_server: Mutex::new(None),
            scf_client: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Get the username of the currently active Minecraft account.
#[tauri::command]
pub async fn link_get_active_username() -> Result<String> {
    Ok(link::get_active_username().await)
}

/// Create a lobby as host. Returns the lobby code.
/// `mc_port` is the port of an already-opened Minecraft LAN world.
#[tauri::command]
pub async fn link_create_lobby<R: Runtime>(
    app: tauri::AppHandle<R>,
    mc_port: u16,
    username: String,
) -> Result<String> {
    let gs = app.state::<LinkGlobalState>();

    if mc_port == 0 {
        return Err(TheseusSerializableError::Theseus(
            theseus::Error::from(theseus::ErrorKind::InputError(
                "No Minecraft LAN world selected. Open a world to LAN in Minecraft first.".to_string(),
            )),
        ));
    }

    if username.is_empty() {
        return Err(TheseusSerializableError::Theseus(
            theseus::Error::from(theseus::ErrorKind::InputError(
                "No Minecraft account logged in.".to_string(),
            )),
        ));
    }

    // Generate lobby info.
    let lobby_info = link::generate();

    // Create host profile.
    let profile = PlayerProfile {
        name: username.clone(),
        machine_id: gs.state.read().machine_id.clone(),
        vendor: "Modrinth".to_string(),
        kind: Some(PlayerKind::Host),
        latency_ms: Some(0),
    };

    {
        let mut inner = gs.state.write();
        inner.lobby_state = LobbyState::Creating;
    }
    let _ = app.emit("link_state_changed", &LobbyState::Creating);

    // Start scaffolding server.
    let scf_server = ScaffoldingServer::start(profile.clone(), mc_port)
        .await
        .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;
    let scf_port = scf_server.port();

    // Start EasyTier as host.
    {
        let mut manager = gs.manager.lock().await;
        manager
            .start_host(&lobby_info, scf_port, mc_port)
            .await
            .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;
    }
    *gs.scf_server.lock().await = Some(scf_server);

    // Update state.
    {
        let mut inner = gs.state.write();
        inner.lobby_state = LobbyState::Connected;
        inner.lobby_code = Some(lobby_info.full_code.clone());
        inner.username = Some(username);
        inner.is_host = true;
        inner.players = vec![profile];
        inner.connection_info = ConnectionInfo {
            way: ConnectionWay::Local,
            quality: ConnectionQuality::Good,
            latency_ms: 0,
        };
    }
    let _ = app.emit("link_state_changed", &LobbyState::Connected);
    let _ = app.emit(
        "link_server_started",
        &serde_json::json!({ "lobby_code": lobby_info.full_code }),
    );

    Ok(lobby_info.full_code)
}

/// Join an existing lobby as client.
#[tauri::command]
pub async fn link_join_lobby<R: Runtime>(
    app: tauri::AppHandle<R>,
    lobby_code: String,
    username: String,
) -> Result<bool> {
    let gs = app.state::<LinkGlobalState>();

    if username.is_empty() {
        return Err(TheseusSerializableError::Theseus(
            theseus::Error::from(theseus::ErrorKind::InputError(
                "No Minecraft account logged in.".to_string(),
            )),
        ));
    }

    // Parse lobby code.
    let lobby_info = lobby_code::parse(&lobby_code)
        .map_err(|e| theseus::Error::from(theseus::ErrorKind::InputError(e)))?;

    {
        let mut inner = gs.state.write();
        inner.lobby_state = LobbyState::Joining;
    }
    let _ = app.emit("link_state_changed", &LobbyState::Joining);

    // Pick a single forward_port used for BOTH the EasyTier port forward
    // and the ScaffoldingClient connection. The EasyTier port forward maps
    // 0.0.0.0:{forward_port} -> {host_ip}:{host_scf_port}, and the
    // ScaffoldingClient connects to 127.0.0.1:{forward_port}.
    let forward_port = get_free_port().ok_or_else(|| {
        theseus::Error::from(theseus::ErrorKind::OtherError(
            "no free port available".to_string(),
        ))
    })?;

    // Start EasyTier as client and set up port forwarding.
    {
        let mut manager = gs.manager.lock().await;
        manager
            .start_client(&lobby_info, forward_port)
            .await
            .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;

        // Find the host in the peer list.
        let (host_ip, host_scf_port) = manager
            .find_host()
            .await
            .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;

        // Add port forward: local forward_port -> host_ip:host_scf_port.
        manager
            .add_port_forward(forward_port, &host_ip, host_scf_port)
            .await
            .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;
    }

    // Create client profile.
    let profile = PlayerProfile {
        name: username.clone(),
        machine_id: gs.state.read().machine_id.clone(),
        vendor: "Modrinth".to_string(),
        kind: Some(PlayerKind::Client),
        latency_ms: None,
    };

    // Connect scaffolding client to the SAME forwarded port.
    let (scf_client, mut event_rx) = ScaffoldingClient::start(forward_port, profile.clone())
        .await
        .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;

    // Request the MC port from the host via scaffolding protocol.
    let scf_stream = scf_client.stream();
    scf_client
        .request_server_port(scf_stream)
        .await
        .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;

    *gs.scf_client.lock().await = Some(scf_client);

    // Detect the initial connection way (P2P direct vs relay). EasyTier may
    // still be negotiating NAT traversal at this point, so the heartbeat loop
    // below will refresh it periodically.
    let (initial_way, route_latency) = {
        let manager = gs.manager.lock().await;
        manager.detect_connection_way().await
    };

    // Update state.
    {
        let mut inner = gs.state.write();
        inner.lobby_state = LobbyState::Connected;
        inner.lobby_code = Some(lobby_info.full_code);
        inner.username = Some(username);
        inner.is_host = false;
        inner.players = vec![profile];
        inner.connection_info.way = initial_way;
        if route_latency > 0 {
            inner.connection_info.latency_ms = route_latency;
        }
    }
    let _ = app.emit("link_state_changed", &LobbyState::Connected);

    // Spawn event listener for scaffolding client events.
    let app_clone = app.clone();
    tokio::spawn(async move {
        // Counter used to refresh the P2P/relay status every few heartbeats
        // instead of on every beat (the RPC round-trip is unnecessary that often).
        let mut heartbeat_count: u32 = 0;
        while let Some(event) = event_rx.recv().await {
            let gs = app_clone.state::<LinkGlobalState>();
            match event {
                ScaffoldingEvent::PlayersUpdated(players) => {
                    gs.state.write().players = players.clone();
                    let _ = app_clone.emit("link_players_changed", &players);
                }
                ScaffoldingEvent::Heartbeat(latency) => {
                    heartbeat_count = heartbeat_count.wrapping_add(1);
                    {
                        let mut inner = gs.state.write();
                        inner.connection_info.latency_ms = latency;
                        inner.connection_info.quality = if latency < 100 {
                            ConnectionQuality::Good
                        } else if latency < 200 {
                            ConnectionQuality::Fair
                        } else {
                            ConnectionQuality::Poor
                        };
                    }
                    // Refresh P2P/relay status every 5 heartbeats (~10s).
                    // NAT traversal can succeed mid-session, flipping relay→P2P.
                    if heartbeat_count % 5 == 0 {
                        let (way, _) = {
                            let manager = gs.manager.lock().await;
                            manager.detect_connection_way().await
                        };
                        let prev = gs.state.read().connection_info.way;
                        if way != prev {
                            tracing::info!(
                                "Connection way changed: {prev:?} -> {way:?}"
                            );
                            gs.state.write().connection_info.way = way;
                            let _ = app_clone.emit("link_connection_way_changed", &way);
                        }
                    }
                    let _ = app_clone.emit("link_heartbeat", &latency);
                }
                ScaffoldingEvent::ServerShutdown => {
                    gs.state.write().lobby_state = LobbyState::Error;
                    let _ = app_clone.emit("link_state_changed", &LobbyState::Error);
                    let _ = app_clone.emit("link_server_shutdown", &());
                }
                ScaffoldingEvent::HostMods(mods) => {
                    gs.state.write().host_mods = mods.clone();
                    let _ = app_clone.emit("link_mod_compat_result", &mods);
                }
                ScaffoldingEvent::ServerPort(port) => {
                    let _ = app_clone.emit("link_server_port", &port);
                }
            }
        }
    });

    Ok(true)
}

/// Leave the current lobby.
#[tauri::command]
pub async fn link_leave_lobby<R: Runtime>(app: tauri::AppHandle<R>) -> Result<()> {
    let gs = app.state::<LinkGlobalState>();

    {
        let mut inner = gs.state.write();
        inner.lobby_state = LobbyState::Leaving;
    }
    let _ = app.emit("link_state_changed", &LobbyState::Leaving);

    // Stop scaffolding server/client.
    if let Some(server) = gs.scf_server.lock().await.take() {
        server.stop().await;
    }
    if let Some(client) = gs.scf_client.lock().await.take() {
        client.stop().await;
    }

    // Stop EasyTier.
    {
        let manager = gs.manager.lock().await;
        manager.stop();
    }

    // Reset state.
    {
        let mut inner = gs.state.write();
        inner.lobby_state = LobbyState::Idle;
        inner.lobby_code = None;
        inner.username = None;
        inner.players = Vec::new();
        inner.is_host = false;
        inner.connection_info = ConnectionInfo::default();
        inner.host_mods = Vec::new();
    }
    let _ = app.emit("link_state_changed", &LobbyState::Idle);

    Ok(())
}

/// Get the current lobby state.
#[tauri::command]
pub async fn link_get_state<R: Runtime>(app: tauri::AppHandle<R>) -> Result<LobbyState> {
    let gs = app.state::<LinkGlobalState>();
    Ok(gs.state.read().lobby_state)
}

/// Get the current player list.
#[tauri::command]
pub async fn link_get_players<R: Runtime>(app: tauri::AppHandle<R>) -> Result<Vec<PlayerProfile>> {
    let gs = app.state::<LinkGlobalState>();

    // If host, get players from scaffolding server.
    if let Some(server) = gs.scf_server.lock().await.as_ref() {
        return Ok(server.get_players());
    }

    // Otherwise, return cached state.
    Ok(gs.state.read().players.clone())
}

/// Get the current lobby code.
#[tauri::command]
pub async fn link_get_lobby_code<R: Runtime>(app: tauri::AppHandle<R>) -> Result<Option<String>> {
    let gs = app.state::<LinkGlobalState>();
    Ok(gs.state.read().lobby_code.clone())
}

/// Get connection info.
#[tauri::command]
pub async fn link_get_connection_info<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<ConnectionInfo> {
    let gs = app.state::<LinkGlobalState>();
    Ok(gs.state.read().connection_info.clone())
}

/// Check if this instance is the host.
#[tauri::command]
pub async fn link_is_host<R: Runtime>(app: tauri::AppHandle<R>) -> Result<bool> {
    let gs = app.state::<LinkGlobalState>();
    Ok(gs.state.read().is_host)
}

/// Discover local Minecraft worlds via LAN broadcast.
#[tauri::command]
pub async fn link_discover_worlds() -> Result<Vec<FoundWorld>> {
    // This is a one-shot discovery: listen for 5 seconds and collect results.
    let mut rx = link::start_broadcast_listener()
        .await
        .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;

    let mut worlds = Vec::new();
    let mut seen_ports = std::collections::HashSet::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        tokio::select! {
            Some(world) = rx.recv() => {
                // Deduplicate by port (MC broadcasts every ~1.5s).
                if seen_ports.insert(world.port) {
                    worlds.push(world);
                }
            }
            _ = tokio::time::sleep_until(deadline) => break,
        }
    }
    Ok(worlds)
}

/// Check mod compatibility against the host's mod list.
#[tauri::command]
pub async fn link_check_mod_compat<R: Runtime>(
    app: tauri::AppHandle<R>,
    instance_path: String,
) -> Result<ModCompatibilityResult> {
    let gs = app.state::<LinkGlobalState>();

    // Get local mods.
    let mods_dir = std::path::Path::new(&instance_path).join("mods");
    let local_mods = link::get_local_mods(&mods_dir)
        .await
        .map_err(|e| theseus::Error::from(theseus::ErrorKind::OtherError(e)))?;

    // Get host mods from cached state.
    let host_mods = gs.state.read().host_mods.clone();

    // If we're the host, we're always compatible.
    if gs.state.read().is_host {
        return Ok(ModCompatibilityResult {
            is_compatible: true,
            local_only: Vec::new(),
            host_only: Vec::new(),
            version_mismatch: Vec::new(),
        });
    }

    // Compare.
    let result = link::compare_mods(&local_mods, &host_mods);
    Ok(result)
}

/// Get the host's mod list (for display).
#[tauri::command]
pub async fn link_get_host_mods<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<HostModInfo>> {
    let gs = app.state::<LinkGlobalState>();

    // If host, return our own mods.
    if gs.state.read().is_host {
        // TODO: get from instance
        return Ok(Vec::new());
    }

    // Otherwise, return cached host mods.
    Ok(gs.state.read().host_mods.clone())
}

/// Check if Minecraft is currently running.
#[tauri::command]
pub async fn link_is_minecraft_running() -> Result<bool> {
    Ok(link::is_minecraft_running())
}

/// Update the MC port on the host (for deferred forwarding).
#[tauri::command]
pub async fn link_update_mc_port<R: Runtime>(
    app: tauri::AppHandle<R>,
    port: u16,
) -> Result<()> {
    let gs = app.state::<LinkGlobalState>();

    // Update scaffolding server's MC port.
    if let Some(server) = gs.scf_server.lock().await.as_ref() {
        server.set_mc_port(port);
    }

    // Update manager's MC port.
    {
        let manager = gs.manager.lock().await;
        manager.set_mc_port(port);
    }

    let _ = app.emit("link_mc_port_updated", &port);
    Ok(())
}

/// Send a chat message to the room.
#[tauri::command]
pub async fn link_send_chat_message<R: Runtime>(
    app: tauri::AppHandle<R>,
    content: String,
) -> Result<ChatMessage> {
    if content.is_empty() || content.len() > 500 {
        return Err(TheseusSerializableError::Theseus(
            theseus::Error::from(theseus::ErrorKind::InputError(
                "Message must be 1-500 characters.".to_string(),
            )),
        ));
    }

    let gs = app.state::<LinkGlobalState>();

    // Client: send via scaffolding protocol
    if let Some(client) = gs.scf_client.lock().await.as_ref() {
        let stream = client.stream();
        let msg = client
            .send_chat_message(stream, content)
            .await
            .map_err(|e| {
                TheseusSerializableError::Theseus(theseus::Error::from(theseus::ErrorKind::OtherError(
                    e,
                )))
            })?;
        return Ok(msg);
    }

    // Host: add directly to server state
    if let Some(server) = gs.scf_server.lock().await.as_ref() {
        let inner = gs.state.read();
        let msg = server.add_chat_message(
            inner.machine_id.clone(),
            inner.username.clone().unwrap_or_default(),
            content,
        );
        return Ok(msg);
    }

    Err(TheseusSerializableError::Theseus(
        theseus::Error::from(theseus::ErrorKind::OtherError("Not connected".to_string())),
    ))
}

/// Poll for new chat messages (both host and client).
#[tauri::command]
pub async fn link_poll_chat_messages<R: Runtime>(
    app: tauri::AppHandle<R>,
    since_ts: u64,
) -> Result<Vec<ChatMessage>> {
    let gs = app.state::<LinkGlobalState>();

    // Client: poll via scaffolding protocol
    if let Some(client) = gs.scf_client.lock().await.as_ref() {
        let stream = client.stream();
        let messages = client
            .poll_chat_messages(stream, since_ts)
            .await
            .map_err(|e| {
                TheseusSerializableError::Theseus(theseus::Error::from(theseus::ErrorKind::OtherError(
                    e,
                )))
            })?;
        return Ok(messages);
    }

    // Host: return from server's local history
    if let Some(server) = gs.scf_server.lock().await.as_ref() {
        let messages = server.get_chat_messages_since(since_ts);
        return Ok(messages);
    }

    Err(TheseusSerializableError::Theseus(
        theseus::Error::from(theseus::ErrorKind::OtherError("Not connected".to_string())),
    ))
}

// ---------------------------------------------------------------------------
// Tauri plugin
// ---------------------------------------------------------------------------

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("link")
        .invoke_handler(tauri::generate_handler![
            link_get_active_username,
            link_create_lobby,
            link_join_lobby,
            link_leave_lobby,
            link_get_state,
            link_get_players,
            link_get_lobby_code,
            link_get_connection_info,
            link_is_host,
            link_discover_worlds,
            link_check_mod_compat,
            link_get_host_mods,
            link_is_minecraft_running,
            link_update_mc_port,
            link_send_chat_message,
            link_poll_chat_messages,
        ])
        .build()
}

/// Find a free TCP port on localhost.
fn get_free_port() -> Option<u16> {
    // Use the same approach as theseus: bind to port 0 and get the assigned port.
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|listener| listener.local_addr().ok())
        .map(|addr| addr.port())
}
