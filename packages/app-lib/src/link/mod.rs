//! Link (online multiplayer) module.
//!
//! Provides EasyTier-based virtual networking and a Scaffolding protocol
//! for Minecraft multiplayer lobby management.
//!
//! Architecture:
//! - `types` — shared type definitions
//! - `lobby_code` — lobby code generation/parsing (Base34 + mod7 checksum)
//! - `scaffolding` — TCP-based protocol between host (server) and clients
//! - `manager` — EasyTier network lifecycle management
//! - `game_watcher` — Minecraft LAN broadcast listener + process monitoring
//! - `mod_compat` — mod list comparison between host and client

pub mod game_watcher;
pub mod lobby_code;
pub mod manager;
pub mod mod_compat;
pub mod scaffolding;
pub mod types;

pub use game_watcher::{is_minecraft_running, start_broadcast_listener, wait_for_minecraft_exit};
pub use lobby_code::{generate, parse};
pub use manager::LinkManager;
pub use mod_compat::{compare_mods, get_local_mods};
pub use scaffolding::{ScaffoldingClient, ScaffoldingEvent, ScaffoldingServer};
pub use types::*;

/// Returns the username of the currently active Minecraft account.
/// Falls back to an empty string if no account is logged in.
pub async fn get_active_username() -> String {
    if let Ok(state) = crate::State::get().await {
        if let Ok(Some(creds)) = crate::state::Credentials::get_active(&state.pool).await {
            return creds.offline_profile.name;
        }
    }
    String::new()
}
