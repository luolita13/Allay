//! EasyTier network manager — wraps `EasyTierLauncher` to start/stop
//! network instances and configure port forwarding.
//!
//! Unlike PCL-CE which launches `easytier-core.exe` as an external process,
//! this module uses EasyTier as an in-process Rust crate, running the network
//! in a background thread managed by `EasyTierLauncher`.

use std::time::Duration;

use easytier::launcher::{EasyTierLauncher, NetworkConfig};
use easytier::proto::api::manage::NetworkingMethod;
use parking_lot::Mutex;
use uuid::Uuid;

use super::types::*;

/// Hardcoded fallback public relay nodes.
const FALLBACK_RELAY_NODES: &[&str] = &[
    "tcp://public.easytier.top:11010",
    "udp://public.easytier.top:11010",
    "tcp://easytier.1kay.net:11010",
    "udp://easytier.1kay.net:11010",
];

/// Check whether the current Windows process has administrator privileges.
/// Uses the `net session` command, which only succeeds when elevated.
#[cfg(windows)]
fn is_running_as_admin() -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "net", "session", ">nul", "2>&1"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
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
            if err.contains("Failed to create adapter") || err.contains("create adapter") {
                msg.push_str("\n\nOn Windows, creating a virtual network adapter requires the wintun driver and administrator privileges. Please:\n");
                msg.push_str("1. Make sure you launched Modrinth App as an administrator.\n");
                msg.push_str("2. Ensure wintun.dll is present next to the app executable.");
                #[cfg(windows)]
                if !is_running_as_admin() {
                    msg.push_str("\n\nCurrent process is NOT running with administrator rights.");
                }
            }
            return Err(msg);
        }

        *self.launcher.lock() = Some(launcher);
        Ok(())
    }

    /// Wait for the host's own EasyTier node to be ready.
    /// For the host, we just need the API service to be available
    /// and our own route entry to exist.
    async fn wait_ready_host(&self) -> Result<(), String> {
        for _ in 0..150 {
            let api_service_opt: Option<_> = {
                let launcher = self.launcher.lock();
                launcher.as_ref().and_then(|l| {
                    if l.running() {
                        l.get_api_service()
                    } else if let Some(err) = l.error_msg() {
                        tracing::error!("EasyTier failed to start: {err}");
                        None
                    } else {
                        None
                    }
                })
            };

            if let Some(api) = api_service_opt {
                use easytier::proto::api::instance::ListRouteRequest;
                use easytier::proto::rpc_types::controller::BaseController;
                let ctrl = BaseController::default();
                if let Ok(routes) = api
                    .get_peer_manage_service()
                    .list_route(ctrl, ListRouteRequest::default())
                    .await
                {
                    // Host is ready when we can see our own route.
                    if routes
                        .routes
                        .iter()
                        .any(|r| r.hostname.starts_with(HOST_HOSTNAME_PREFIX))
                    {
                        tracing::info!("EasyTier host is ready");
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tracing::warn!("EasyTier wait_ready_host timed out after 15s");
        Err("Timed out waiting for the virtual network to start.".to_string())
    }

    /// Wait for the client to find the host in the route list.
    async fn wait_ready_client(&self) -> Result<(), String> {
        for _ in 0..150 {
            let api_service_opt: Option<_> = {
                let launcher = self.launcher.lock();
                launcher.as_ref().and_then(|l| {
                    if l.running() {
                        l.get_api_service()
                    } else if let Some(err) = l.error_msg() {
                        tracing::error!("EasyTier failed to start: {err}");
                        None
                    } else {
                        None
                    }
                })
            };

            if let Some(api) = api_service_opt {
                use easytier::proto::api::instance::ListRouteRequest;
                use easytier::proto::rpc_types::controller::BaseController;
                let ctrl = BaseController::default();
                if let Ok(routes) = api
                    .get_peer_manage_service()
                    .list_route(ctrl, ListRouteRequest::default())
                    .await
                {
                    // Client is ready when we can see the host's route.
                    if routes
                        .routes
                        .iter()
                        .any(|r| r.hostname.starts_with(HOST_HOSTNAME_PREFIX))
                    {
                        tracing::info!("EasyTier client found host");
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tracing::warn!("EasyTier wait_ready_client timed out after 15s");
        Err("Could not find the host on the virtual network. Check the lobby code and make sure the host is online.".to_string())
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
    cfg.networking_method = Some(NetworkingMethod::PublicServer as i32);
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
    cfg
}
