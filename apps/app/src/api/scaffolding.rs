//! Scaffolding protocol — TCP binary application-layer protocol used to exchange
//! player profiles, host mod info, and Minecraft port between EasyTier peers.
//!
//! Ported from PCL-CE's `PCL.Core.Link.Scaffolding.*` (C#).
//!
//! ## Wire format
//!
//! Request frame:
//! ```text
//! +----------+------------------+-------------------+----------+
//! | type_len | type (ASCII)     | body_len (BE u32) | body     |
//! | 1 byte   | type_len bytes   | 4 bytes           | N bytes  |
//! +----------+------------------+-------------------+----------+
//! ```
//!
//! Response frame:
//! ```text
//! +--------+-------------------+----------+
//! | status | body_len (BE u32) | body     |
//! | 1 byte | 4 bytes           | N bytes  |
//! +--------+-------------------+----------+
//! ```
//!
//! Status: 0 = success, 32 = error (no body), 255 = error with message body.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::interval;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum PlayerKind {
    Host,
    Guest,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct PlayerProfile {
    pub name: String,
    pub machine_id: String,
    pub vendor: String,
    /// Only set by the server when responding to clients.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<PlayerKind>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct HostModInfo {
    pub modid: String,
    pub version: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

pub const STATUS_SUCCESS: u8 = 0;
pub const STATUS_ERROR: u8 = 32;
pub const STATUS_ERROR_WITH_MESSAGE: u8 = 255;

const REQ_PLAYER_PING: &str = "c:player_ping";
const REQ_SERVER_PORT: &str = "c:server_port";
const REQ_PLAYER_PROFILES_LIST: &str = "c:player_profiles_list";
const REQ_PING: &str = "c:ping";
const REQ_HOST_MODS: &str = "c:host_mods";
const REQ_PROTOCOLS: &str = "c:protocols";

const VENDOR: &str = "ModrinthApp";

// ---------------------------------------------------------------------------
// Protocol frame I/O
// ---------------------------------------------------------------------------

/// Write a request frame to the writer.
pub async fn write_request<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    request_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let type_bytes = request_type.as_bytes();
    if type_bytes.len() > 255 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Request type too long (max 255 bytes)",
        ));
    }
    writer.write_u8(type_bytes.len() as u8).await?;
    writer.write_all(type_bytes).await?;
    writer.write_u32(body.len() as u32).await?;
    writer.write_all(body).await?;
    writer.flush().await?;
    Ok(())
}

pub struct ResponseFrame {
    pub status: u8,
    pub body: Vec<u8>,
}

/// Read a response frame from the reader.
pub async fn read_response<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<ResponseFrame> {
    let status = reader.read_u8().await?;
    let body_len = reader.read_u32().await? as usize;
    let mut body = vec![0u8; body_len];
    if body_len > 0 {
        reader.read_exact(&mut body).await?;
    }
    Ok(ResponseFrame { status, body })
}

async fn write_response<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    response: &HandlerResponse,
) -> std::io::Result<()> {
    writer.write_u8(response.status).await?;
    writer.write_u32(response.body.len() as u32).await?;
    writer.write_all(&response.body).await?;
    writer.flush().await?;
    Ok(())
}

struct HandlerResponse {
    status: u8,
    body: Vec<u8>,
}

impl HandlerResponse {
    fn ok(body: Vec<u8>) -> Self {
        Self { status: STATUS_SUCCESS, body }
    }

    fn error() -> Self {
        Self { status: STATUS_ERROR, body: vec![] }
    }

    fn error_with_message(msg: impl Into<String>) -> Self {
        Self {
            status: STATUS_ERROR_WITH_MESSAGE,
            body: msg.into().into_bytes(),
        }
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct TrackedPlayer {
    profile: PlayerProfile,
    last_seen: Instant,
}

#[derive(Clone)]
struct ServerContext {
    players: Arc<RwLock<HashMap<String, TrackedPlayer>>>,
    mc_port: Arc<RwLock<u16>>,
    host_mods: Arc<RwLock<Vec<HostModInfo>>>,
}

pub struct ScaffoldingServer {
    pub port: u16,
    context: ServerContext,
    listener_task: Option<JoinHandle<()>>,
    cleanup_task: Option<JoinHandle<()>>,
}

impl ScaffoldingServer {
    /// Start a Scaffolding server bound to `0.0.0.0:port` (or random if port=0).
    /// The host is added as the first tracked player with `kind=HOST`.
    pub async fn start(
        port: u16,
        mc_port: u16,
        host_mods: Vec<HostModInfo>,
        host_profile: PlayerProfile,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        let actual_port = listener.local_addr()?.port();
        tracing::info!(
            "[scaffolding] Server listening on 0.0.0.0:{} (requested {})",
            actual_port,
            port
        );

        let mut host_profile = host_profile;
        host_profile.kind = Some(PlayerKind::Host);

        let mut players = HashMap::new();
        players.insert(
            host_profile.machine_id.clone(),
            TrackedPlayer {
                profile: host_profile.clone(),
                last_seen: Instant::now(),
            },
        );

        let context = ServerContext {
            players: Arc::new(RwLock::new(players)),
            mc_port: Arc::new(RwLock::new(mc_port)),
            host_mods: Arc::new(RwLock::new(host_mods)),
        };

        let ctx = context.clone();
        let listener_task = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, addr, ctx).await {
                                tracing::debug!("[scaffolding] Client handler ended: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("[scaffolding] Accept failed: {}", e);
                        break;
                    }
                }
            }
        });

        // Heartbeat cleanup: every 5s, remove players not seen in 10s (host is never removed).
        let players_ref = context.players.clone();
        let host_machine_id = context
            .players
            .read()
            .await
            .values()
            .find(|p| p.profile.kind == Some(PlayerKind::Host))
            .map(|p| p.profile.machine_id.clone());
        let cleanup_task = tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(5));
            tick.tick().await; // skip first immediate
            loop {
                tick.tick().await;
                let mut players = players_ref.write().await;
                let before = players.len();
                players.retain(|_id, p| {
                    p.profile.kind == Some(PlayerKind::Host)
                        || p.last_seen.elapsed() < Duration::from_secs(10)
                });
                let removed = before - players.len();
                if removed > 0 {
                    tracing::info!("[scaffolding] Cleaned up {} stale players", removed);
                }
                // Re-insert host if it was somehow removed.
                if let Some(ref host_id) = host_machine_id {
                    if !players.contains_key(host_id) {
                        // Host should never be missing, but log just in case.
                        tracing::warn!("[scaffolding] Host profile missing from player list!");
                    }
                }
            }
        });

        Ok(Self {
            port: actual_port,
            context,
            listener_task: Some(listener_task),
            cleanup_task: Some(cleanup_task),
        })
    }

    /// Update the Minecraft port (used by delayed port-forward mode).
    pub async fn update_mc_port(&self, port: u16) {
        let mut mc_port = self.context.mc_port.write().await;
        *mc_port = port;
        tracing::info!("[scaffolding] MC port updated to {}", port);
    }

    /// Update the host mod list.
    pub async fn update_host_mods(&self, mods: Vec<HostModInfo>) {
        let mut host_mods = self.context.host_mods.write().await;
        *host_mods = mods;
    }

    /// Get the current list of tracked players (host + guests).
    pub async fn get_players(&self) -> Vec<PlayerProfile> {
        let players = self.context.players.read().await;
        players.values().map(|p| p.profile.clone()).collect()
    }

    /// Get the current MC port.
    pub async fn get_mc_port(&self) -> u16 {
        *self.context.mc_port.read().await
    }

    /// Stop the server and clean up tasks.
    pub async fn stop(mut self) {
        if let Some(t) = self.listener_task.take() {
            t.abort();
        }
        if let Some(t) = self.cleanup_task.take() {
            t.abort();
        }
        tracing::info!("[scaffolding] Server stopped");
    }
}

async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    ctx: ServerContext,
) -> std::io::Result<()> {
    tracing::info!("[scaffolding] Client connected from {}", addr);
    loop {
        // Read request frame.
        let type_len = match stream.read_u8().await {
            Ok(b) => b as usize,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        };
        let mut type_buf = vec![0u8; type_len];
        stream.read_exact(&mut type_buf).await?;
        let request_type = String::from_utf8_lossy(&type_buf).to_string();
        let body_len = stream.read_u32().await? as usize;
        let mut body = vec![0u8; body_len];
        if body_len > 0 {
            stream.read_exact(&mut body).await?;
        }

        let response = handle_request(&request_type, &body, &ctx).await;
        if let Err(e) = write_response(&mut stream, &response).await {
            tracing::debug!("[scaffolding] Failed to write response: {}", e);
            break;
        }
    }
    tracing::info!("[scaffolding] Client disconnected: {}", addr);
    Ok(())
}

async fn handle_request(request_type: &str, body: &[u8], ctx: &ServerContext) -> HandlerResponse {
    tracing::debug!("[scaffolding] Handling request: {}", request_type);
    match request_type {
        REQ_PLAYER_PING => handle_player_ping(body, ctx).await,
        REQ_SERVER_PORT => handle_server_port(ctx).await,
        REQ_PLAYER_PROFILES_LIST => handle_player_profiles_list(ctx).await,
        REQ_PING => handle_ping(body),
        REQ_HOST_MODS => handle_host_mods(ctx).await,
        REQ_PROTOCOLS => handle_protocols(body),
        _ => HandlerResponse::error_with_message(format!(
            "Unknown request type: {}",
            request_type
        )),
    }
}

async fn handle_player_ping(body: &[u8], ctx: &ServerContext) -> HandlerResponse {
    let mut profile: PlayerProfile = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => {
            return HandlerResponse::error_with_message(format!(
                "Invalid PlayerProfile JSON: {}",
                e
            ));
        }
    };
    let machine_id = profile.machine_id.clone();
    profile.kind = Some(PlayerKind::Guest);

    let mut players = ctx.players.write().await;
    players.insert(
        machine_id.clone(),
        TrackedPlayer {
            profile: profile.clone(),
            last_seen: Instant::now(),
        },
    );
    tracing::info!(
        "[scaffolding] Player pinged: {} (machine_id={})",
        profile.name,
        machine_id
    );
    HandlerResponse::ok(vec![])
}

async fn handle_server_port(ctx: &ServerContext) -> HandlerResponse {
    let port = *ctx.mc_port.read().await;
    if port == 0 {
        // MC not yet open to LAN — tell client to wait.
        return HandlerResponse::error();
    }
    let mut body = Vec::with_capacity(2);
    body.push((port >> 8) as u8);
    body.push((port & 0xff) as u8);
    HandlerResponse::ok(body)
}

async fn handle_player_profiles_list(ctx: &ServerContext) -> HandlerResponse {
    let players = ctx.players.read().await;
    let list: Vec<PlayerProfile> = players.values().map(|p| p.profile.clone()).collect();
    match serde_json::to_vec(&list) {
        Ok(body) => HandlerResponse::ok(body),
        Err(e) => HandlerResponse::error_with_message(format!("Serialize error: {}", e)),
    }
}

fn handle_ping(body: &[u8]) -> HandlerResponse {
    // Echo request body back to client.
    HandlerResponse::ok(body.to_vec())
}

async fn handle_host_mods(ctx: &ServerContext) -> HandlerResponse {
    let mods = ctx.host_mods.read().await;
    match serde_json::to_vec(&*mods) {
        Ok(body) => HandlerResponse::ok(body),
        Err(e) => HandlerResponse::error_with_message(format!("Serialize error: {}", e)),
    }
}

fn handle_protocols(body: &[u8]) -> HandlerResponse {
    // Body: ASCII null-separated requested protocols.
    // Response: ASCII null-separated supported protocols (intersection).
    let request_str = String::from_utf8_lossy(body);
    let requested: Vec<&str> = request_str
        .split('\0')
        .filter(|s| !s.is_empty())
        .collect();
    let supported = [
        "player_ping",
        "server_port",
        "player_profiles_list",
        "ping",
        "host_mods",
        "protocols",
    ];
    let result: Vec<&str> = requested
        .into_iter()
        .filter(|r| supported.contains(r))
        .collect();
    let body = result.join("\0").into_bytes();
    HandlerResponse::ok(body)
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct ScaffoldingClient {
    stream: Arc<Mutex<TcpStream>>,
    heartbeat_task: Option<JoinHandle<()>>,
    players: Arc<RwLock<Vec<PlayerProfile>>>,
    last_heartbeat: Arc<RwLock<Option<Instant>>>,
    /// Notify when player list changes (for UI updates).
    pub on_players_changed: Arc<tokio::sync::Notify>,
}

impl ScaffoldingClient {
    /// Connect to a Scaffolding server. Sends initial `c:player_ping` and
    /// spawns a heartbeat task that pings every 5s.
    pub async fn connect(
        addr: SocketAddr,
        profile: PlayerProfile,
    ) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let stream = Arc::new(Mutex::new(stream));
        tracing::info!("[scaffolding] Client connected to {}", addr);

        let players = Arc::new(RwLock::new(Vec::new()));
        let last_heartbeat = Arc::new(RwLock::new(None));
        let on_players_changed = Arc::new(tokio::sync::Notify::new());

        // Send initial player_ping.
        {
            let mut s = stream.lock().await;
            let body = serde_json::to_vec(&profile).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;
            write_request(&mut *s, REQ_PLAYER_PING, &body).await?;
            let _ = read_response(&mut *s).await?;
        }

        // Spawn heartbeat loop: every 5s send player_ping + get_player_profiles_list.
        let stream_clone = stream.clone();
        let players_clone = players.clone();
        let last_heartbeat_clone = last_heartbeat.clone();
        let notify_clone = on_players_changed.clone();
        let profile_clone = profile.clone();

        let heartbeat_task = tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(5));
            tick.tick().await; // skip first immediate
            loop {
                tick.tick().await;
                let mut s = stream_clone.lock().await;

                // Send player_ping (refresh our presence).
                let mut success = false;
                if let Ok(body) = serde_json::to_vec(&profile_clone) {
                    if write_request(&mut *s, REQ_PLAYER_PING, &body).await.is_ok() {
                        if read_response(&mut *s).await.is_ok() {
                            success = true;
                        }
                    }
                }

                if success {
                    // Send get_player_profiles_list.
                    if write_request(&mut *s, REQ_PLAYER_PROFILES_LIST, &[]).await.is_ok() {
                        if let Ok(resp) = read_response(&mut *s).await {
                            if resp.status == STATUS_SUCCESS {
                                if let Ok(list) =
                                    serde_json::from_slice::<Vec<PlayerProfile>>(&resp.body)
                                {
                                    let mut players = players_clone.write().await;
                                    let changed = !profiles_equal(&*players, &list);
                                    *players = list;
                                    drop(players);
                                    if changed {
                                        notify_clone.notify_waiters();
                                    }
                                }
                            }
                        }
                    }

                    let mut hb = last_heartbeat_clone.write().await;
                    *hb = Some(Instant::now());
                } else {
                    tracing::warn!("[scaffolding] Heartbeat failed");
                    // Brief sleep to avoid hammering a dead connection.
                    drop(s);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        Ok(Self {
            stream,
            heartbeat_task: Some(heartbeat_task),
            players,
            last_heartbeat,
            on_players_changed,
        })
    }

    /// Get the latest cached player list (updated by heartbeat).
    pub async fn get_players(&self) -> Vec<PlayerProfile> {
        self.players.read().await.clone()
    }

    /// Query the server for the Minecraft port. Returns `None` if the host
    /// hasn't opened the world to LAN yet (status=32).
    pub async fn get_server_port(&self) -> std::io::Result<Option<u16>> {
        let mut s = self.stream.lock().await;
        write_request(&mut *s, REQ_SERVER_PORT, &[]).await?;
        let resp = read_response(&mut *s).await?;
        if resp.status == STATUS_ERROR || resp.body.is_empty() {
            return Ok(None);
        }
        if resp.body.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid server_port response (need 2 bytes)",
            ));
        }
        let port = ((resp.body[0] as u16) << 8) | (resp.body[1] as u16);
        Ok(if port == 0 { None } else { Some(port) })
    }

    /// Query the server for the host's mod list.
    pub async fn get_host_mods(&self) -> std::io::Result<Vec<HostModInfo>> {
        let mut s = self.stream.lock().await;
        write_request(&mut *s, REQ_HOST_MODS, &[]).await?;
        let resp = read_response(&mut *s).await?;
        if resp.status != STATUS_SUCCESS {
            return Ok(vec![]);
        }
        Ok(serde_json::from_slice(&resp.body).unwrap_or_default())
    }

    /// Negotiate supported protocols with the server.
    pub async fn check_protocols(
        &self,
        supported: Vec<String>,
    ) -> std::io::Result<Vec<String>> {
        let body = supported.join("\0").into_bytes();
        let mut s = self.stream.lock().await;
        write_request(&mut *s, REQ_PROTOCOLS, &body).await?;
        let resp = read_response(&mut *s).await?;
        let text = String::from_utf8_lossy(&resp.body);
        Ok(text
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    /// Protocol-level ping (echo).
    pub async fn ping(&self, payload: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut s = self.stream.lock().await;
        write_request(&mut *s, REQ_PING, payload).await?;
        let resp = read_response(&mut *s).await?;
        Ok(resp.body)
    }

    /// Update the player profile used by the heartbeat loop. The next
    /// heartbeat will use this profile for `c:player_ping`.
    pub async fn update_profile(&self, _profile: PlayerProfile) {
        // For simplicity, we don't currently support updating the profile mid-session.
        // Reconnect is required. This could be added later.
        tracing::warn!("[scaffolding] update_profile called but not implemented");
    }

    pub async fn last_heartbeat(&self) -> Option<Instant> {
        *self.last_heartbeat.read().await
    }

    /// Disconnect and stop the heartbeat task.
    pub async fn disconnect(mut self) {
        if let Some(t) = self.heartbeat_task.take() {
            t.abort();
        }
        tracing::info!("[scaffolding] Client disconnected");
    }
}

fn profiles_equal(a: &[PlayerProfile], b: &[PlayerProfile]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    // Order-insensitive comparison.
    let mut a_sorted: Vec<&str> = a.iter().map(|p| p.name.as_str()).collect();
    let mut b_sorted: Vec<&str> = b.iter().map(|p| p.name.as_str()).collect();
    a_sorted.sort_unstable();
    b_sorted.sort_unstable();
    a_sorted == b_sorted
}

// ---------------------------------------------------------------------------
// Helper: build a PlayerProfile from the local user
// ---------------------------------------------------------------------------

/// Build a PlayerProfile for the local user. The `machine_id` is derived from
/// the local hostname so it's stable across sessions.
pub fn build_local_profile(name: String) -> PlayerProfile {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let machine_id = match std::env::var("COMPUTERNAME") {
        Ok(name) => {
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        }
        Err(_) => uuid::Uuid::new_v4().to_string(),
    };

    PlayerProfile {
        name,
        machine_id,
        vendor: VENDOR.to_string(),
        kind: None,
    }
}
