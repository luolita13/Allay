//! Type definitions for the link (online multiplayer) module.

use serde::{Deserialize, Serialize};

/// Current state of the link lobby.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyState {
    /// Not in any lobby.
    Idle,
    /// Initializing EasyTier.
    Initializing,
    /// EasyTier is running, waiting for further action.
    Initialized,
    /// Host: discovering local MC worlds.
    Discovering,
    /// Host: creating a lobby.
    Creating,
    /// Client: joining a lobby.
    Joining,
    /// Connected to a lobby.
    Connected,
    /// Leaving / cleaning up.
    Leaving,
    /// An error occurred.
    Error,
}

impl Default for LobbyState {
    fn default() -> Self {
        Self::Idle
    }
}

/// How a client is connected to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionWay {
    /// Direct local network connection.
    Local,
    /// Direct peer-to-peer connection.
    P2P,
    /// Relayed through a public server.
    Relay,
    /// Unknown / not yet determined.
    Unknown,
}

impl Default for ConnectionWay {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Network quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionQuality {
    /// Poor (>200ms or relay with high latency)
    Poor = 1,
    /// Fair (100-200ms)
    Fair = 2,
    /// Good (<100ms)
    Good = 3,
}

impl Default for ConnectionQuality {
    fn default() -> Self {
        Self::Good
    }
}

/// Connection info reported to the frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub way: ConnectionWay,
    pub quality: ConnectionQuality,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u32,
}

/// Whether this peer is the host or a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKind {
    Host,
    Client,
}

/// A player in the lobby.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    /// In-game username.
    pub name: String,
    /// Unique machine identifier.
    pub machine_id: String,
    /// Launcher vendor (always "Modrinth" for this app).
    pub vendor: String,
    /// Host or Client.
    pub kind: Option<PlayerKind>,
    /// Last known latency in ms.
    pub latency_ms: Option<u32>,
}

/// Information about a lobby, derived from the lobby code.
#[derive(Debug, Clone)]
pub struct LobbyInfo {
    /// Full lobby code, e.g. "U/XXXX-XXXX-XXXX-XXXX".
    pub full_code: String,
    /// EasyTier network name.
    pub network_name: String,
    /// EasyTier network secret.
    pub network_secret: String,
}

/// A locally discovered Minecraft world (via LAN broadcast).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundWorld {
    pub name: String,
    pub port: u16,
}

/// A mod entry from the host's instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostModInfo {
    pub mod_id: String,
    pub version: String,
    pub name: String,
}

/// A chat message in the lobby.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message ID (UUID).
    pub id: String,
    /// Sender's machine_id.
    pub sender_id: String,
    /// Sender's display name.
    pub sender_name: String,
    /// Message content (max 500 chars).
    pub content: String,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
}

/// Chat send request (client -> server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSendRequest {
    pub content: String,
}

/// Result of mod compatibility check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModCompatibilityResult {
    pub is_compatible: bool,
    /// Mods that exist locally but not on the host.
    pub local_only: Vec<HostModInfo>,
    /// Mods that exist on the host but not locally.
    pub host_only: Vec<HostModInfo>,
    /// Mods with mismatched versions.
    pub version_mismatch: Vec<(HostModInfo, HostModInfo)>,
}

/// Supported Scaffolding protocol request types.
pub(crate) const PROTOCOL_PLAYER_PING: &str = "c:player_ping";
pub(crate) const PROTOCOL_SERVER_PORT: &str = "c:server_port";
pub(crate) const PROTOCOL_PLAYER_PROFILES_LIST: &str = "c:player_profiles_list";
pub(crate) const PROTOCOL_PROTOCOLS: &str = "c:protocols";
pub(crate) const PROTOCOL_PING: &str = "c:ping";
pub(crate) const PROTOCOL_HOST_MODS: &str = "c:host_mods";
pub(crate) const PROTOCOL_CHAT_SEND: &str = "c:chat_send";
pub(crate) const PROTOCOL_CHAT_POLL: &str = "c:chat_poll";

/// All protocols this implementation supports.
pub const SUPPORTED_PROTOCOLS: &[&str] = &[
    PROTOCOL_PLAYER_PING,
    PROTOCOL_SERVER_PORT,
    PROTOCOL_PLAYER_PROFILES_LIST,
    PROTOCOL_PROTOCOLS,
    PROTOCOL_PING,
    PROTOCOL_HOST_MODS,
    PROTOCOL_CHAT_SEND,
    PROTOCOL_CHAT_POLL,
];
