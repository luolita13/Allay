//! EasyTier network manager — wraps `EasyTierLauncher` to start/stop
//! network instances and configure port forwarding.
//!
//! Unlike PCL-CE which launches `easytier-core.exe` as an external process,
//! this module uses EasyTier as an in-process Rust crate, running the network
//! in a background thread managed by `EasyTierLauncher`.

use std::time::Duration;

use easytier::launcher::{EasyTierLauncher, NetworkConfig};
use easytier::proto::api::manage::NetworkingMethod;
use easytier::proto::common::CompressionAlgoPb;
use parking_lot::Mutex;
use uuid::Uuid;

use super::types::*;

/// Hardcoded fallback public relay nodes.
///
/// The official `public.easytier.top` / `public.easytier.cn` nodes have been
/// offline since early 2026 (DNS returns NXDomain from the authoritative
/// servers), so we use community-maintained alternatives. All entries below
/// were verified reachable on 2026-07-29 from mainland China.
///
/// Using `Manual` networking mode lets EasyTier try every entry in `peer_urls`
/// until one succeeds, so listing multiple regions improves reliability.
const FALLBACK_RELAY_NODES: &[&str] = &[
    // 1. IP-based node (no DNS dependency, most reliable).
    //    Hosted in mainland China, confirmed reachable 2026-07-29.
    "tcp://39.108.52.138:11010",
    "udp://39.108.52.138:11010",
    // 2. Hong Kong dual-stack relay (lzw-723, May 2026).
    //    Resolves to 140.150.236.98.
    "tcp://et-hk.clickor.click:11010",
    "udp://et-hk.clickor.click:11010",
    // 3. US high-bandwidth relay (225284228a-droid, Jul 2026).
    //    Resolves to 38.147.105.194.
    "tcp://us01.225284.xyz:11010",
    "udp://us01.225284.xyz:11010",
];

/// Check whether the current Windows process has administrator privileges.
/// Uses the Windows token elevation API, which is more reliable than `net session`.
/// Currently unused because `no_tun = true` avoids the need for admin rights,
/// but kept for potential future use if TUN mode is re-enabled.
#[cfg(windows)]
#[allow(dead_code)]
fn is_running_as_admin() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::GetTokenInformation;
    use windows::Win32::Security::TokenElevation;
    use windows::Win32::Security::TOKEN_ELEVATION;
    use windows::Win32::Security::TOKEN_QUERY;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE(std::ptr::null_mut());
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut return_length = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        let _ = CloseHandle(token);

        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn is_running_as_admin() -> bool {
    false
}

/// Hostname prefix for the host's EasyTier instance.
/// Clients use this to identify the host in the peer list.
const HOST_HOSTNAME_PREFIX: &str = "scaffolding-mc-server-";

/// Virtual IP for the host (matches PCL-CE).
const HOST_VIRTUAL_IP: &str = "10.114.51.41";

/// The EasyTier network manager.
pub struct LinkManager {
    launcher: Mutex<Option<EasyTierLauncher>>,
    /// The port the scaffolding server listens on (host) or forwards to (client).
    scf_port: u16,
    /// The local port that forwards to the host's scaffolding server (client only).
    forward_port: u16,
    /// The Minecraft port (0 = deferred, not yet known).
    mc_port: Mutex<u16>,
    /// Whether this instance is the host.
    is_host: bool,
}

impl LinkManager {
    /// Create a new manager.
    pub fn new() -> Self {
        Self {
            launcher: Mutex::new(None),
            scf_port: 0,
            forward_port: 0,
            mc_port: Mutex::new(0),
            is_host: false,
        }
    }

    /// Start as host: create a new lobby with the given MC port.
    /// If `mc_port` is 0, the host will use deferred forwarding (MC port discovered later).
    pub async fn start_host(
        &mut self,
        lobby_info: &LobbyInfo,
        scf_port: u16,
        mc_port: u16,
    ) -> Result<(), String> {
        let hostname = format!("{}{}", HOST_HOSTNAME_PREFIX, scf_port);

        // Host: no port_forwards in NetworkConfig. Instead, the MC port
        // is exposed via EasyTier's whitelist (configured in the launcher args).
        // The Scaffolding server listens on scf_port and tells clients the MC port.
        let cfg = build_network_config(
            &lobby_info.network_name,
            &lobby_info.network_secret,
            &hostname,
            Some(HOST_VIRTUAL_IP.to_string()),
        );

        self.start_launcher(cfg)?;
        self.scf_port = scf_port;
        *self.mc_port.lock() = mc_port;
        self.is_host = true;

        // Wait for network to be ready (Host only needs its own route).
        self.wait_ready_host().await?;

        Ok(())
    }

    /// Start as client: join an existing lobby.
    /// `forward_port` is the local port that will forward to the host's scaffolding server.
    pub async fn start_client(
        &mut self,
        lobby_info: &LobbyInfo,
        forward_port: u16,
    ) -> Result<(), String> {
        let hostname = Uuid::new_v4().to_string();

        let cfg = build_network_config(
            &lobby_info.network_name,
            &lobby_info.network_secret,
            &hostname,
            None, // Use DHCP
        );

        self.start_launcher(cfg)?;
        self.forward_port = forward_port;
        self.is_host = false;

        // Wait until we can see the host in the route list.
        self.wait_ready_client().await?;

        Ok(())
    }

    /// Add a port forward (client side): forward local port to host's virtual IP + port.
    pub async fn add_port_forward(
        &mut self,
        local_port: u16,
        host_ip: &str,
        host_port: u16,
    ) -> Result<(), String> {
        // Extract the data plane (Arc<Socks5Server>) while holding the lock,
        // then release before any .await to keep the Future Send.
        let data_plane = {
            let launcher = self.launcher.lock();
            let launcher = launcher
                .as_ref()
                .ok_or("EasyTier not running")?;
            launcher.get_data_plane()
        };

        if let Some(dp) = data_plane {
            use easytier::common::config::PortForwardConfig;

            let cfg = PortForwardConfig {
                bind_addr: format!("0.0.0.0:{}", local_port).parse().unwrap(),
                dst_addr: format!("{}:{}", host_ip, host_port).parse().unwrap(),
                proto: "tcp".to_string(),
            };
            dp.add_port_forward(cfg)
                .await
                .map_err(|e| format!("add_port_forward: {e}"))?;
            return Ok(());
        }

        Err("Data plane not available, cannot add port forward".to_string())
    }

    /// Find the host's peer info in the network.
    /// Returns (host_virtual_ip, host_scf_port) if found.
    pub async fn find_host(&self) -> Result<(String, u16), String> {
        // Extract the Arc<dyn InstanceRpcService> while holding the lock,
        // then release the lock before any .await to keep the Future Send.
        let api_service = {
            let launcher = self.launcher.lock();
            let launcher = launcher
                .as_ref()
                .ok_or("EasyTier not running")?;
            launcher
                .get_api_service()
                .ok_or("API service not ready")?
        };

        use easytier::proto::rpc_types::controller::BaseController;
        use easytier::proto::api::instance::ListRouteRequest;

        let ctrl = BaseController::default();
        let routes = api_service
            .get_peer_manage_service()
            .list_route(ctrl, ListRouteRequest::default())
            .await
            .map_err(|e| format!("list_route: {e}"))?;

        for route in &routes.routes {
            // Check if this route's hostname matches the host pattern.
            if route.hostname.starts_with(HOST_HOSTNAME_PREFIX) {
                // Extract scf_port from hostname.
                let scf_port: u16 = route.hostname[HOST_HOSTNAME_PREFIX.len()..]
                    .parse()
                    .map_err(|_| "invalid host hostname format")?;

                // Get the peer's virtual IP.
                if let Some(addr) = &route.ipv4_addr {
                    let virtual_ip = addr.to_string();
                    if !virtual_ip.is_empty() {
                        return Ok((virtual_ip, scf_port));
                    }
                }
            }
        }

        Err("Host not found in route list".to_string())
    }

    /// Detect how the client is connected to the host.
    ///
    /// EasyTier's `Route` struct carries `peer_id` (the destination) and
    /// `next_hop_peer_id` (the next relay hop). When they are equal, the
    /// destination is reached directly — i.e. a P2P connection. When they
    /// differ, traffic is being forwarded through an intermediate relay node.
    ///
    /// We also cross-check `PeerInfo.directly_connected_conns`: even if the
    /// best route goes through a relay, a non-empty `directly_connected_conns`
    /// means a direct tunnel exists (EasyTier may prefer the relay route due
    /// to `latency_first`, but P2P is still available).
    ///
    /// Returns `(ConnectionWay, latency_ms)` where latency is the route's
    /// `path_latency`. On error (e.g. host not yet visible), returns
    /// `(Unknown, 0)`.
    pub async fn detect_connection_way(&self) -> (ConnectionWay, u32) {
        let api_service = {
            let launcher = self.launcher.lock();
            let Some(launcher) = launcher.as_ref() else {
                return (ConnectionWay::Unknown, 0);
            };
            let Some(api) = launcher.get_api_service() else {
                return (ConnectionWay::Unknown, 0);
            };
            api
        };

        use easytier::proto::api::instance::{ListPeerRequest, ListRouteRequest};
        use easytier::proto::rpc_types::controller::BaseController;
        let ctrl = BaseController::default();

        let routes = match api_service
            .get_peer_manage_service()
            .list_route(ctrl.clone(), ListRouteRequest::default())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("detect_connection_way: list_route error: {e}");
                return (ConnectionWay::Unknown, 0);
            }
        };

        let peers = match api_service
            .get_peer_manage_service()
            .list_peer(ctrl, ListPeerRequest::default())
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!("detect_connection_way: list_peer error: {e}");
                return (ConnectionWay::Unknown, 0);
            }
        };

        // Find the host's route by hostname prefix.
        for route in &routes.routes {
            if !route.hostname.starts_with(HOST_HOSTNAME_PREFIX) {
                continue;
            }

            let latency = route.path_latency.max(0) as u32;

            // P2P if the next hop IS the destination peer itself.
            if route.next_hop_peer_id == route.peer_id && route.peer_id != 0 {
                return (ConnectionWay::P2P, latency);
            }

            // Cross-check: does a direct tunnel to the host exist?
            if let Some(peer_info) = peers
                .peer_infos
                .iter()
                .find(|p| p.peer_id == route.peer_id)
            {
                if !peer_info.directly_connected_conns.is_empty() {
                    return (ConnectionWay::P2P, latency);
                }
            }

            // No direct path — traffic is relayed.
            return (ConnectionWay::Relay, latency);
        }

        // Host not visible in the route list yet.
        (ConnectionWay::Unknown, 0)
    }

    /// Update the MC port (for deferred forwarding on host).
    pub fn set_mc_port(&self, port: u16) {
        *self.mc_port.lock() = port;
    }

    /// Get the current MC port.
    pub fn mc_port(&self) -> u16 {
        *self.mc_port.lock()
    }

    /// Whether this instance is the host.
    pub fn is_host(&self) -> bool {
        self.is_host
    }

    /// Stop the EasyTier network.
    pub fn stop(&self) {
        let mut launcher = self.launcher.lock();
        if let Some(l) = launcher.take() {
            drop(l); // EasyTierLauncher's Drop stops the network
        }
    }

    /// Check if the network is running.
    pub fn is_running(&self) -> bool {
        let launcher = self.launcher.lock();
        launcher.as_ref().map(|l| l.running()).unwrap_or(false)
    }

    /// Get the last error message if any.
    pub fn error_msg(&self) -> Option<String> {
        let launcher = self.launcher.lock();
        launcher.as_ref().and_then(|l| l.error_msg())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn start_launcher(&self, cfg: NetworkConfig) -> Result<(), String> {
        let mut launcher = EasyTierLauncher::new();
        launcher.start(move || cfg.gen_config());

        // Check for immediate errors.
        std::thread::sleep(Duration::from_millis(500));

        if let Some(err) = launcher.error_msg() {
            let mut msg = format!("EasyTier start error: {err}");
            // With no_tun=true we should never hit the wintun adapter error,
            // but keep a helpful hint just in case.
            if err.contains("Failed to create adapter") || err.contains("create adapter") {
                msg.push_str("\n\nThis should not happen with TUN disabled. Please report this issue.");
            }
            return Err(msg);
        }

        *self.launcher.lock() = Some(launcher);
        Ok(())
    }

    /// Wait for the host's EasyTier node to be ready.
    ///
    /// The host only needs to be connected to at least one relay peer so that
    /// clients can discover it. It does NOT need to wait for its own route to
    /// appear in the route list — the host's own route may take a long time to
    /// propagate back, and waiting for it causes spurious 30-second timeouts.
    ///
    /// This mirrors PCL-CE's `CheckEasyTierStatusAsync`, which considers the
    /// network ready as soon as the peer list is non-empty.
    async fn wait_ready_host(&self) -> Result<(), String> {
        const MAX_RETRIES: usize = 300; // 30 seconds
        for attempt in 0..MAX_RETRIES {
            let (api_service_opt, running, error_opt, events) = {
                let launcher = self.launcher.lock();
                launcher.as_ref().map_or((None, false, None, Vec::new()), |l| {
                    (
                        l.get_api_service(),
                        l.running(),
                        l.error_msg(),
                        l.get_events(),
                    )
                })
            };

            if !running {
                if let Some(err) = error_opt {
                    return Err(format!("EasyTier failed to start: {err}"));
                }
                if attempt >= 10 {
                    tracing::debug!("EasyTier launcher not running yet, attempt {attempt}");
                }
            }

            if let Some(api) = api_service_opt {
                use easytier::proto::api::instance::ListPeerRequest;
                use easytier::proto::rpc_types::controller::BaseController;
                let ctrl = BaseController::default();
                match api
                    .get_peer_manage_service()
                    .list_peer(ctrl, ListPeerRequest::default())
                    .await
                {
                    Ok(peers) => {
                        // Host is ready as soon as it has connected to at least
                        // one relay peer. Clients discover the host through the
                        // relay; the host does not need to wait for its own route.
                        if !peers.peer_infos.is_empty() {
                            tracing::info!(
                                "EasyTier host is ready (connected to {} peer(s))",
                                peers.peer_infos.len()
                            );
                            return Ok(());
                        }
                        if attempt % 50 == 0 {
                            tracing::debug!(
                                "EasyTier host waiting for peers (attempt {attempt})",
                            );
                        }
                    }
                    Err(e) => {
                        if attempt % 50 == 0 {
                            tracing::debug!("EasyTier list_peer error (attempt {attempt}): {e}");
                        }
                    }
                }
            }

            // Surface important events so they appear in the session log.
            if attempt % 50 == 0 {
                Self::log_recent_events(&events);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let diagnostics = self.collect_diagnostics().await;
        tracing::warn!("EasyTier wait_ready_host timed out after 30s: {diagnostics}");
        Err(format!(
            "Timed out waiting for the virtual network to start.\n\nDiagnosis:\n{diagnostics}\n\n\
             Common causes:\n\
             • No reachable EasyTier relay from your current network.\n\
             • A firewall or antivirus is blocking outbound TCP/UDP on port 11010.\n\
             • The public relays are temporarily unavailable. Try again later."
        ))
    }

    /// Wait for the client to find the host in the route list.
    async fn wait_ready_client(&self) -> Result<(), String> {
        const MAX_RETRIES: usize = 300; // 30 seconds
        for attempt in 0..MAX_RETRIES {
            let (api_service_opt, running, error_opt, events) = {
                let launcher = self.launcher.lock();
                launcher.as_ref().map_or((None, false, None, Vec::new()), |l| {
                    (
                        l.get_api_service(),
                        l.running(),
                        l.error_msg(),
                        l.get_events(),
                    )
                })
            };

            if !running {
                if let Some(err) = error_opt {
                    return Err(format!("EasyTier failed to start: {err}"));
                }
            }

            if let Some(api) = api_service_opt {
                use easytier::proto::api::instance::ListRouteRequest;
                use easytier::proto::rpc_types::controller::BaseController;
                let ctrl = BaseController::default();
                match api
                    .get_peer_manage_service()
                    .list_route(ctrl, ListRouteRequest::default())
                    .await
                {
                    Ok(routes) => {
                        // Client is ready when we can see the host's route.
                        if routes
                            .routes
                            .iter()
                            .any(|r| r.hostname.starts_with(HOST_HOSTNAME_PREFIX))
                        {
                            tracing::info!("EasyTier client found host");
                            return Ok(());
                        }
                        if attempt % 50 == 0 {
                            tracing::debug!(
                                "EasyTier client waiting for host route (attempt {attempt}, routes: {})",
                                routes.routes.len()
                            );
                        }
                    }
                    Err(e) => {
                        if attempt % 50 == 0 {
                            tracing::debug!("EasyTier list_route error (attempt {attempt}): {e}");
                        }
                    }
                }
            }

            if attempt % 50 == 0 {
                Self::log_recent_events(&events);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let diagnostics = self.collect_diagnostics().await;
        tracing::warn!("EasyTier wait_ready_client timed out after 30s: {diagnostics}");
        Err(format!(
            "Could not find the host on the virtual network.\n\nDiagnosis:\n{diagnostics}\n\n\
             Common causes:\n\
             • The lobby code is incorrect or the host has left.\n\
             • The host cannot reach any EasyTier relay.\n\
             • A firewall is blocking EasyTier traffic on port 11010."
        ))
    }

    /// Emit the most recent EasyTier events to the tracing log so users can
    /// inspect them in the session log even when the UI only shows a timeout.
    fn log_recent_events(events: &[easytier::launcher::Event]) {
        if events.is_empty() {
            return;
        }
        let recent: Vec<String> = events
            .iter()
            .rev()
            .take(8)
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect();
        if !recent.is_empty() {
            tracing::info!("Recent EasyTier events:\n  - {}", recent.join("\n  - "));
        }
    }

    /// Collect a short diagnostic string when a wait loop times out.
    async fn collect_diagnostics(&self) -> String {
        let (running, error_opt, events) = {
            let launcher = self.launcher.lock();
            launcher.as_ref().map_or((false, None, Vec::new()), |l| {
                (l.running(), l.error_msg(), l.get_events())
            })
        };

        let mut lines = Vec::new();
        lines.push(format!("launcher running: {running}"));
        if let Some(err) = error_opt {
            lines.push(format!("launcher error: {err}"));
        }

        if running {
            let api_service_opt = {
                let launcher = self.launcher.lock();
                launcher.as_ref().and_then(|l| l.get_api_service())
            };
            if let Some(api) = api_service_opt {
                use easytier::proto::api::instance::{ListPeerRequest, ListRouteRequest};
                use easytier::proto::rpc_types::controller::BaseController;
                let ctrl = BaseController::default();
                if let Ok(peers) = api
                    .get_peer_manage_service()
                    .list_peer(ctrl.clone(), ListPeerRequest::default())
                    .await
                {
                    lines.push(format!("connected peers: {}", peers.peer_infos.len()));
                }
                if let Ok(routes) = api
                    .get_peer_manage_service()
                    .list_route(ctrl, ListRouteRequest::default())
                    .await
                {
                    lines.push(format!("known routes: {}", routes.routes.len()));
                    for r in routes.routes.iter().take(5) {
                        lines.push(format!(
                            "  route: {} -> {:?}",
                            r.hostname,
                            r.ipv4_addr.as_ref().map(|a| a.to_string())
                        ));
                    }
                }
            } else {
                lines.push("API service not available".to_string());
            }
        }

        if !events.is_empty() {
            lines.push("recent events:".to_string());
            for e in events.iter().rev().take(10) {
                if let Ok(s) = serde_json::to_string(e) {
                    lines.push(format!("  - {s}"));
                }
            }
        }

        lines.join("\n")
    }
}

impl Default for LinkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LinkManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Build a `NetworkConfig` for EasyTier.
///
/// Configuration is aligned with PCL-CE's `EasyTierEntity._BuildProcessAsync`:
/// P2P is the primary transport; relay nodes are used only for peer *discovery*,
/// not for data forwarding (`disable_relay_data = true`). This means if NAT
/// traversal fails (e.g. symmetric NAT on both sides), the connection fails
/// rather than silently degrading to a relayed path.
fn build_network_config(
    network_name: &str,
    network_secret: &str,
    hostname: &str,
    virtual_ipv4: Option<String>,
) -> NetworkConfig {
    let mut cfg = NetworkConfig::default();
    cfg.instance_id = Some(Uuid::new_v4().to_string());
    cfg.network_name = Some(network_name.to_string());
    cfg.network_secret = Some(network_secret.to_string());
    cfg.hostname = Some(hostname.to_string());
    cfg.dhcp = Some(virtual_ipv4.is_none());
    cfg.virtual_ipv4 = virtual_ipv4;
    cfg.network_length = Some(24);
    // Manual mode: try every relay in `peer_urls` for discovery.
    cfg.networking_method = Some(NetworkingMethod::Manual as i32);
    cfg.public_server_url = Some(
        FALLBACK_RELAY_NODES
            .first()
            .unwrap()
            .to_string(),
    );
    cfg.listener_urls = vec![
        "tcp://0.0.0.0:0".to_string(),
        "udp://0.0.0.0:0".to_string(),
    ];
    cfg.peer_urls = FALLBACK_RELAY_NODES.iter().map(|s| s.to_string()).collect();

    // --- P2P configuration (matches PCL-CE) ---
    cfg.latency_first = Some(true);
    cfg.multi_thread = Some(true);
    // Disable TUN: no admin rights or wintun.dll needed. MC connectivity is
    // handled by EasyTier's port-forward feature via the virtual IP.
    cfg.no_tun = Some(true);
    // KCP + QUIC proxies massively improve NAT traversal success rate.
    // They are off by default in EasyTier; PCL-CE enables them explicitly.
    cfg.enable_kcp_proxy = Some(true);
    cfg.enable_quic_proxy = Some(true);
    // Private mode: only allow peers in the same network to connect.
    cfg.enable_private_mode = Some(true);
    // P2P only — do NOT relay data through public nodes. Relay nodes are still
    // used for peer discovery (so clients can find the host), but actual game
    // traffic must go over a direct P2P tunnel. If NAT traversal fails the
    // connection fails, which is the intended behaviour.
    cfg.disable_relay_data = Some(true);
    // Encryption (matches PCL-CE's --encryption-algorithm aes-gcm).
    cfg.encryption_algorithm = Some("aes-gcm".to_string());
    // Compression (matches PCL-CE's --compression zstd). Reduces bandwidth on
    // the virtual tunnel; the `zstd` feature is enabled on the easytier dep.
    cfg.data_compress_algo = Some(CompressionAlgoPb::Zstd as i32);

    cfg
}
