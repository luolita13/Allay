//! Game watcher — Minecraft LAN broadcast listener and process monitoring.
//!
//! Listens for Minecraft's "Local world shared" broadcast on UDP 224.0.2.60:4445,
//! and detects when the Java process exits.

use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::types::FoundWorld;

/// Minecraft LAN discovery multicast address.
const MC_LAN_ADDR: &str = "224.0.2.60";
const MC_LAN_PORT: u16 = 4445;

/// Start listening for Minecraft LAN broadcasts.
/// Returns a receiver that yields `FoundWorld` when a broadcast is detected.
pub async fn start_broadcast_listener() -> Result<mpsc::UnboundedReceiver<FoundWorld>, String> {
    let socket = UdpSocket::bind("0.0.0.0:4445")
        .await
        .map_err(|e| format!("bind UDP 4445: {e}"))?;

    // Join multicast group.
    socket
        .join_multicast_v4(MC_LAN_ADDR.parse().unwrap(), "0.0.0.0".parse().unwrap())
        .map_err(|e| format!("join multicast: {e}"))?;

    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, _addr)) => {
                    if let Some(world) = parse_lan_broadcast(&buf[..len]) {
                        if tx.send(world).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("MC LAN broadcast recv error: {e}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    Ok(rx)
}

/// Parse a Minecraft LAN broadcast packet.
/// Format: `[MOTD]<motd>[/MOTD][AD]<port>[/AD]`
fn parse_lan_broadcast(data: &[u8]) -> Option<FoundWorld> {
    let text = String::from_utf8_lossy(data);

    // Extract MOTD.
    let motd = extract_between(&text, "[MOTD]", "[/MOTD]")?;
    // Extract AD (port).
    let ad = extract_between(&text, "[AD]", "[/AD]")?;
    let port: u16 = ad.trim().parse().ok()?;

    Some(FoundWorld {
        name: motd,
        port,
    })
}

fn extract_between(text: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = text.find(start)? + start.len();
    let end_idx = text[start_idx..].find(end)? + start_idx;
    Some(text[start_idx..end_idx].to_string())
}

/// Check if any Java process is running (Minecraft).
pub fn is_minecraft_running() -> bool {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    sys.processes()
        .values()
        .any(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name == "javaw" || name == "java"
        })
}

/// Wait for Minecraft to exit. Returns when no Java process is found.
pub async fn wait_for_minecraft_exit() {
    loop {
        if !is_minecraft_running() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lan_broadcast() {
        let data = "[MOTD]\u{00a7}dMy World[/MOTD][AD]25565[/AD]".as_bytes();
        let world = parse_lan_broadcast(data).unwrap();
        assert_eq!(world.port, 25565);
        assert!(world.name.contains("My World"));
    }

    #[test]
    fn test_parse_lan_broadcast_invalid() {
        assert!(parse_lan_broadcast(b"garbage").is_none());
    }
}
