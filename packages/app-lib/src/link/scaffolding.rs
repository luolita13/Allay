//! Scaffolding protocol — TCP-based communication between host and clients.
//!
//! Ported from PCL-CE's `ScaffoldingServer.cs` and `ScaffoldingClient.cs`.
//!
//! Frame format (request):
//!   [1 byte type_len][type_len bytes type_str][4 bytes body_len BE][body bytes]
//!
//! Frame format (response):
//!   [1 byte status][4 bytes body_len BE][body bytes]
//!
//! The server runs on the host's machine, listening on localhost.
//! Clients connect through EasyTier's port-forwarded local port.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

use super::types::*;

/// Maximum chat history to keep (ring buffer).
const MAX_CHAT_HISTORY: usize = 200;

/// Player heartbeat timeout (seconds).
const PLAYER_TIMEOUT_SECS: u64 = 10;

/// Heartbeat interval (seconds).
const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// Maximum consecutive heartbeat failures before declaring server disconnected.
const MAX_HEARTBEAT_FAILURES: u32 = 3;

// ---------------------------------------------------------------------------
// Frame serialization
// ---------------------------------------------------------------------------

/// Read a request frame from the stream.
async fn read_request(stream: &mut TcpStream) -> Result<(String, Vec<u8>), String> {
    // 1 byte type length
    let type_len = stream
        .read_u8()
        .await
        .map_err(|e| format!("read type_len: {e}"))? as usize;

    // type string
    let mut type_buf = vec![0u8; type_len];
    stream
        .read_exact(&mut type_buf)
        .await
        .map_err(|e| format!("read type: {e}"))?;
    let type_str = String::from_utf8_lossy(&type_buf).to_string();

    // 4 bytes body length (big endian)
    let body_len = stream
        .read_u32()
        .await
        .map_err(|e| format!("read body_len: {e}"))? as usize;

    // body
    let mut body = vec![0u8; body_len];
    if body_len > 0 {
        stream
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("read body: {e}"))?;
    }

    Ok((type_str, body))
}

/// Write a response frame to the stream.
async fn write_response(
    stream: &mut TcpStream,
    status: u8,
    body: &[u8],
) -> Result<(), String> {
    stream
        .write_u8(status)
        .await
        .map_err(|e| format!("write status: {e}"))?;
    stream
        .write_u32(body.len() as u32)
        .await
        .map_err(|e| format!("write body_len: {e}"))?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .await
            .map_err(|e| format!("write body: {e}"))?;
    }
    Ok(())
}

/// Write a request frame to the stream.
async fn write_request(
    stream: &mut TcpStream,
    type_str: &str,
    body: &[u8],
) -> Result<(), String> {
    let type_bytes = type_str.as_bytes();
    stream
        .write_u8(type_bytes.len() as u8)
        .await
        .map_err(|e| format!("write type_len: {e}"))?;
    stream
        .write_all(type_bytes)
        .await
        .map_err(|e| format!("write type: {e}"))?;
    stream
        .write_u32(body.len() as u32)
        .await
        .map_err(|e| format!("write body_len: {e}"))?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .await
            .map_err(|e| format!("write body: {e}"))?;
    }
    Ok(())
}

/// Read a response frame from the stream.
async fn read_response(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
    let status = stream
        .read_u8()
        .await
        .map_err(|e| format!("read status: {e}"))?;
    let body_len = stream
        .read_u32()
        .await
        .map_err(|e| format!("read body_len: {e}"))? as usize;
    let mut body = vec![0u8; body_len];
    if body_len > 0 {
        stream
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("read body: {e}"))?;
    }
    Ok((status, body))
}

// ---------------------------------------------------------------------------
// Tracked player (server-side)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TrackedPlayer {
    profile: PlayerProfile,
    last_seen: Instant,
}

/// Server-side shared state.
struct ServerState {
    players: RwLock<HashMap<String, TrackedPlayer>>,
    mc_port: RwLock<u16>,
    host_profile: PlayerProfile,
    host_mods: RwLock<Vec<HostModInfo>>,
    chat_history: RwLock<VecDeque<ChatMessage>>,
}

impl ServerState {
    fn new(host_profile: PlayerProfile, mc_port: u16) -> Self {
        let mut players = HashMap::new();
        let machine_id = host_profile.machine_id.clone();
        players.insert(
            machine_id,
            TrackedPlayer {
                profile: host_profile.clone(),
                last_seen: Instant::now(),
            },
        );
        Self {
            players: RwLock::new(players),
            mc_port: RwLock::new(mc_port),
            host_profile,
            host_mods: RwLock::new(Vec::new()),
            chat_history: RwLock::new(VecDeque::new()),
        }
    }

    fn update_player(&self, profile: PlayerProfile) {
        let mut players = self.players.write();
        players.insert(
            profile.machine_id.clone(),
            TrackedPlayer {
                profile,
                last_seen: Instant::now(),
            },
        );
    }

    fn get_player_list(&self) -> Vec<PlayerProfile> {
        let players = self.players.read();
        players.values().map(|p| p.profile.clone()).collect()
    }

    fn cleanup_stale_players(&self) {
        let mut players = self.players.write();
        let now = Instant::now();
        players.retain(|_, p| {
            now.duration_since(p.last_seen) < Duration::from_secs(PLAYER_TIMEOUT_SECS)
        });
        // Always keep the host.
        players.insert(
            self.host_profile.machine_id.clone(),
            TrackedPlayer {
                profile: self.host_profile.clone(),
                last_seen: Instant::now(),
            },
        );
    }

    fn add_chat_message(&self, msg: ChatMessage) {
        let mut history = self.chat_history.write();
        history.push_back(msg);
        while history.len() > MAX_CHAT_HISTORY {
            history.pop_front();
        }
    }

    fn get_messages_since(&self, since_ts: u64) -> Vec<ChatMessage> {
        let history = self.chat_history.read();
        history
            .iter()
            .filter(|m| m.timestamp >= since_ts)
            .cloned()
            .collect()
    }

    fn get_recent_messages(&self, limit: usize) -> Vec<ChatMessage> {
        let history = self.chat_history.read();
        let len = history.len();
        if len <= limit {
            history.iter().cloned().collect()
        } else {
            history.iter().skip(len - limit).cloned().collect()
        }
    }
}

// ---------------------------------------------------------------------------
// Scaffolding Server (runs on host)
// ---------------------------------------------------------------------------

/// The Scaffolding server, running on the host's machine.
pub struct ScaffoldingServer {
    state: Arc<ServerState>,
    shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    cleanup_shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    listen_port: u16,
}

impl ScaffoldingServer {
    /// Start a new scaffolding server on a random localhost port.
    pub async fn start(
        host_profile: PlayerProfile,
        mc_port: u16,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("bind scaffolding server: {e}"))?;
        let listen_port = listener.local_addr().map_err(|e| e.to_string())?.port();

        let state = Arc::new(ServerState::new(host_profile, mc_port));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let state_clone = state.clone();
        tokio::spawn(async move {
            let _ = Self::run(listener, state_clone, shutdown_rx).await;
        });

        // Spawn cleanup task with shutdown signal.
        let (cleanup_shutdown_tx, cleanup_shutdown_rx) = tokio::sync::oneshot::channel();
        let state_clone2 = state.clone();
        tokio::spawn(async move {
            tokio::pin! {
                let cleanup_fut = async {
                    loop {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        state_clone2.cleanup_stale_players();
                    }
                };
            }
            tokio::select! {
                _ = cleanup_fut => {},
                _ = cleanup_shutdown_rx => {},
            }
        });

        Ok(Self {
            state,
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
            cleanup_shutdown_tx: Mutex::new(Some(cleanup_shutdown_tx)),
            listen_port,
        })
}

    /// The port the server is listening on.
    pub fn port(&self) -> u16 {
        self.listen_port
    }

    /// Update the Minecraft port (for deferred forwarding).
    pub fn set_mc_port(&self, port: u16) {
        *self.state.mc_port.write() = port;
    }

    /// Update the host's mod list.
    pub fn set_host_mods(&self, mods: Vec<HostModInfo>) {
        *self.state.host_mods.write() = mods;
    }

    /// Get the current player list.
    pub fn get_players(&self) -> Vec<PlayerProfile> {
        self.state.get_player_list()
    }

    /// Add a chat message from the host (server-side, no TCP round-trip).
    pub fn add_chat_message(&self, sender_id: String, sender_name: String, content: String) -> ChatMessage {
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id,
            sender_name,
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        self.state.add_chat_message(msg.clone());
        msg
    }

    /// Get chat messages since the given timestamp (host-side).
    pub fn get_chat_messages_since(&self, since_ts: u64) -> Vec<ChatMessage> {
        self.state.get_messages_since(since_ts)
    }

    /// Stop the server.
    pub async fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(tx) = self.cleanup_shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
    }

    async fn run(
        listener: TcpListener,
        state: Arc<ServerState>,
        mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), String> {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    break;
                }
                accept_result = listener.accept() => {
                    let (mut stream, _addr) = accept_result.map_err(|e| e.to_string())?;
                    let state = state.clone();
                    tokio::spawn(async move {
                        let _ = Self::handle_connection(&mut stream, &state).await;
                    });
                }
            }
        }
        Ok(())
    }

    async fn handle_connection(
        stream: &mut TcpStream,
        state: &ServerState,
    ) -> Result<(), String> {
        let mut sender_id = String::new();
        let mut sender_name = String::new();

        loop {
            let (type_str, body) = match read_request(stream).await {
                Ok(frame) => frame,
                Err(_) => break, // Connection closed
            };

            // Update sender identity on player_ping
            if type_str == PROTOCOL_PLAYER_PING {
                if let Ok(profile) = serde_json::from_slice::<PlayerProfile>(&body) {
                    sender_id = profile.machine_id.clone();
                    sender_name = profile.name.clone();
                }
            }

            let (status, response_body) =
                Self::handle_request(&type_str, &body, state, &sender_id, &sender_name);

            if let Err(e) = write_response(stream, status, &response_body).await {
                tracing::debug!("scaffolding server write error: {e}");
                break;
            }
        }
        Ok(())
    }

    fn handle_request(
        type_str: &str,
        body: &[u8],
        state: &ServerState,
        sender_id: &str,
        sender_name: &str,
    ) -> (u8, Vec<u8>) {
        match type_str {
            PROTOCOL_PLAYER_PING => {
                // Body: JSON PlayerProfile
                if let Ok(profile) = serde_json::from_slice::<PlayerProfile>(body) {
                    state.update_player(profile);
                    let players = state.get_player_list();
                    let resp = serde_json::to_vec(&players).unwrap_or_default();
                    (0, resp)
                } else {
                    (1, b"invalid profile".to_vec())
                }
            }
            PROTOCOL_SERVER_PORT => {
                let port = *state.mc_port.read();
                let resp = serde_json::to_vec(&port).unwrap_or_default();
                (0, resp)
            }
            PROTOCOL_PLAYER_PROFILES_LIST => {
                let players = state.get_player_list();
                let resp = serde_json::to_vec(&players).unwrap_or_default();
                (0, resp)
            }
            PROTOCOL_PROTOCOLS => {
                let resp = serde_json::to_vec(SUPPORTED_PROTOCOLS).unwrap_or_default();
                (0, resp)
            }
            PROTOCOL_PING => {
                (0, Vec::new())
            }
            PROTOCOL_HOST_MODS => {
                let mods = state.host_mods.read().clone();
                let resp = serde_json::to_vec(&mods).unwrap_or_default();
                (0, resp)
            }
            PROTOCOL_CHAT_SEND => {
                if let Ok(req) = serde_json::from_slice::<ChatSendRequest>(body) {
                    if req.content.is_empty() || req.content.len() > 500 {
                        return (1, b"invalid message".to_vec());
                    }
                    let msg = ChatMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        sender_id: sender_id.to_string(),
                        sender_name: sender_name.to_string(),
                        content: req.content,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    };
                    state.add_chat_message(msg.clone());
                    let resp = serde_json::to_vec(&msg).unwrap_or_default();
                    (0, resp)
                } else {
                    (1, b"invalid chat_send request".to_vec())
                }
            }
            PROTOCOL_CHAT_POLL => {
                if let Ok(since_ts) = serde_json::from_slice::<u64>(body) {
                    let messages = state.get_messages_since(since_ts);
                    let resp = serde_json::to_vec(&messages).unwrap_or_default();
                    (0, resp)
                } else {
                    let messages = state.get_recent_messages(50);
                    let resp = serde_json::to_vec(&messages).unwrap_or_default();
                    (0, resp)
                }
            }
            _ => (1, b"unknown protocol".to_vec()),
        }
    }
}

// ---------------------------------------------------------------------------
// Scaffolding Client (runs on client)
// ---------------------------------------------------------------------------

/// Callback events from the scaffolding client.
#[derive(Debug, Clone)]
pub enum ScaffoldingEvent {
    /// Player list updated.
    PlayersUpdated(Vec<PlayerProfile>),
    /// Heartbeat latency updated.
    Heartbeat(u32),
    /// Server disconnected.
    ServerShutdown,
    /// Host's mod list received.
    HostMods(Vec<HostModInfo>),
    /// Server's MC port received.
    ServerPort(u16),
}

/// The Scaffolding client, running on the client's machine.
pub struct ScaffoldingClient {
    stream: Arc<Mutex<TcpStream>>,
    event_tx: mpsc::UnboundedSender<ScaffoldingEvent>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ScaffoldingClient {
    /// Start a new scaffolding client, connecting to the forwarded port.
    pub async fn start(
        forward_port: u16,
        profile: PlayerProfile,
    ) -> Result<(Self, mpsc::UnboundedReceiver<ScaffoldingEvent>), String> {
        let addr = format!("127.0.0.1:{forward_port}");
        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("connect to scaffolding server at {addr}: {e}"))?;

        let stream = Arc::new(Mutex::new(stream));
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Send initial player_ping as handshake.
        {
            let mut s = stream.lock().await;
            let body = serde_json::to_vec(&profile).map_err(|e| e.to_string())?;
            write_request(&mut s, PROTOCOL_PLAYER_PING, &body).await?;
            let (status, resp_body) = read_response(&mut s).await?;
            if status == 0 {
                if let Ok(players) = serde_json::from_slice::<Vec<PlayerProfile>>(&resp_body) {
                    let _ = event_tx.send(ScaffoldingEvent::PlayersUpdated(players));
                }
            }
        }

        let stream_clone = stream.clone();
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            let _ = Self::heartbeat_loop(
                stream_clone,
                profile,
                event_tx_clone,
                shutdown_rx,
            )
            .await;
        });

        Ok((
            Self {
                stream,
                event_tx,
                shutdown_tx: Mutex::new(Some(shutdown_tx)),
            },
            event_rx,
        ))
    }

    /// Get a clone of the internal TCP stream arc for making requests.
    pub fn stream(&self) -> Arc<Mutex<TcpStream>> {
        self.stream.clone()
    }

    /// Request the host's mod list.
    pub async fn request_host_mods(&self, stream: Arc<Mutex<TcpStream>>) -> Result<(), String> {
        let mut s = stream.lock().await;
        write_request(&mut s, PROTOCOL_HOST_MODS, &[]).await?;
        let (status, body) = read_response(&mut s).await?;
        if status == 0 {
            if let Ok(mods) = serde_json::from_slice::<Vec<HostModInfo>>(&body) {
                let _ = self.event_tx.send(ScaffoldingEvent::HostMods(mods));
            }
        }
        Ok(())
    }

    /// Request the server's MC port.
    pub async fn request_server_port(&self, stream: Arc<Mutex<TcpStream>>) -> Result<(), String> {
        let mut s = stream.lock().await;
        write_request(&mut s, PROTOCOL_SERVER_PORT, &[]).await?;
        let (status, body) = read_response(&mut s).await?;
        if status == 0 {
            if let Ok(port) = serde_json::from_slice::<u16>(&body) {
                let _ = self.event_tx.send(ScaffoldingEvent::ServerPort(port));
            }
        }
        Ok(())
    }

    /// Send a chat message to the room.
    pub async fn send_chat_message(
        &self,
        stream: Arc<Mutex<TcpStream>>,
        content: String,
    ) -> Result<ChatMessage, String> {
        let mut s = stream.lock().await;
        let req = ChatSendRequest { content };
        let body = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        write_request(&mut s, PROTOCOL_CHAT_SEND, &body).await?;
        let (status, resp_body) = read_response(&mut s).await?;
        if status == 0 {
            serde_json::from_slice::<ChatMessage>(&resp_body)
                .map_err(|e| format!("parse chat message: {e}"))
        } else {
            Err("chat send failed".to_string())
        }
    }

    /// Poll for new chat messages since the given timestamp.
    pub async fn poll_chat_messages(
        &self,
        stream: Arc<Mutex<TcpStream>>,
        since_ts: u64,
    ) -> Result<Vec<ChatMessage>, String> {
        let mut s = stream.lock().await;
        let body = serde_json::to_vec(&since_ts).map_err(|e| e.to_string())?;
        write_request(&mut s, PROTOCOL_CHAT_POLL, &body).await?;
        let (status, resp_body) = read_response(&mut s).await?;
        if status == 0 {
            serde_json::from_slice::<Vec<ChatMessage>>(&resp_body)
                .map_err(|e| format!("parse chat messages: {e}"))
        } else {
            Err("chat poll failed".to_string())
        }
    }

    /// Stop the client.
    pub async fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
    }

    async fn heartbeat_loop(
        stream: Arc<Mutex<TcpStream>>,
        profile: PlayerProfile,
        event_tx: mpsc::UnboundedSender<ScaffoldingEvent>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let mut failures = 0u32;
        let mut tick = 0u32;

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                _ = tokio::time::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)) => {}
            }

            let start = Instant::now();
            let mut s = stream.lock().await;

            // Every 3rd heartbeat, send a full player_ping to update the player list.
            // Otherwise, send a lightweight ping.
            tick += 1;
            let is_full_ping = tick % 3 == 0;

            if is_full_ping {
                let body = match serde_json::to_vec(&profile) {
                    Ok(b) => b,
                    Err(_) => continue,
                };

                if write_request(&mut s, PROTOCOL_PLAYER_PING, &body).await.is_err() {
                    failures += 1;
                    if failures >= MAX_HEARTBEAT_FAILURES {
                        let _ = event_tx.send(ScaffoldingEvent::ServerShutdown);
                        break;
                    }
                    continue;
                }

                match read_response(&mut s).await {
                    Ok((status, resp_body)) if status == 0 => {
                        failures = 0;
                        let latency = start.elapsed().as_millis() as u32;
                        let _ = event_tx.send(ScaffoldingEvent::Heartbeat(latency));

                        if let Ok(players) = serde_json::from_slice::<Vec<PlayerProfile>>(&resp_body) {
                            let _ = event_tx.send(ScaffoldingEvent::PlayersUpdated(players));
                        }
                    }
                    _ => {
                        failures += 1;
                        if failures >= MAX_HEARTBEAT_FAILURES {
                            let _ = event_tx.send(ScaffoldingEvent::ServerShutdown);
                            break;
                        }
                    }
                }
            } else {
                // Lightweight ping (empty body).
                if write_request(&mut s, PROTOCOL_PING, &[]).await.is_err() {
                    failures += 1;
                    if failures >= MAX_HEARTBEAT_FAILURES {
                        let _ = event_tx.send(ScaffoldingEvent::ServerShutdown);
                        break;
                    }
                    continue;
                }

                match read_response(&mut s).await {
                    Ok((status, _)) if status == 0 => {
                        failures = 0;
                        let latency = start.elapsed().as_millis() as u32;
                        let _ = event_tx.send(ScaffoldingEvent::Heartbeat(latency));
                    }
                    _ => {
                        failures += 1;
                        if failures >= MAX_HEARTBEAT_FAILURES {
                            let _ = event_tx.send(ScaffoldingEvent::ServerShutdown);
                            break;
                        }
                    }
                }
            }
        }
    }
}

// Re-export oneshot for convenience.
use tokio::sync::oneshot;
