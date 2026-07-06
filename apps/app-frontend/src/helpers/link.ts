import { invoke } from '@tauri-apps/api/core'

export type LinkRole = 'idle' | 'host' | 'client'

export interface PlayerProfile {
	name: string
	machine_id: string
	vendor: string
	kind?: 'HOST' | 'GUEST'
}

export interface HostModInfo {
	modid: string
	version: string
	name: string
}

export interface LobbyStatus {
	state: LinkRole
	lobbyCode: string | null
	networkName: string | null
	virtualIp: string | null
	/** Host: MC LAN port (null if delayed mode and not yet detected).
	 *  Client: MC port reported by host's Scaffolding server. */
	mcPort: number | null
	/** Client: local port that MC should connect to (via BroadcastLocal). */
	localPort: number | null
	/** Scaffolding protocol port (host only). */
	scfPort: number | null
	peerCount: number
	playerCount: number
	lastRefresh: number | null
	error: string | null
	initialized: boolean
	/** True if Scaffolding server (host) or client (guest) is connected. */
	scaffoldingReady: boolean
}

export interface LocalWorld {
	motd: string
	port: number
	lastSeen: number
}

/** Create a lobby. `mcPort=0` enables delayed mode (wait for MC to open LAN).
 *  Returns the lobby code. */
export async function create_lobby(
	mcPort: number,
	playerName: string,
	hostMods: HostModInfo[] = [],
): Promise<string> {
	return await invoke('plugin:link|link_create_lobby', {
		mcPort,
		playerName,
		hostMods,
	})
}

/** Join a lobby. Poll `get_lobby_status` / `get_mc_port` / `get_players` to
 *  know when the connection is fully ready. */
export async function join_lobby(lobbyCode: string, playerName: string): Promise<void> {
	return await invoke('plugin:link|link_join_lobby', { lobbyCode, playerName })
}

export async function leave_lobby(): Promise<void> {
	return await invoke('plugin:link|link_leave_lobby')
}

export async function get_lobby_status(): Promise<LobbyStatus> {
	return await invoke('plugin:link|link_get_lobby_status')
}

export async function check_easytier_ready(): Promise<boolean> {
	return await invoke('plugin:link|link_check_easytier_ready')
}

/** One-shot discovery of local MC LAN worlds (host side). */
export async function discover_local_worlds(): Promise<LocalWorld[]> {
	return await invoke('plugin:link|link_discover_local_worlds')
}

/** Get the current player list (host + guests). */
export async function get_players(): Promise<PlayerProfile[]> {
	return await invoke('plugin:link|link_get_players')
}

/** Get the host's mod list (client side asks server; host returns own). */
export async function get_host_mods(): Promise<HostModInfo[]> {
	return await invoke('plugin:link|link_get_host_mods')
}

/** Negotiate supported protocols with the server (client side). */
export async function check_protocols(supported: string[]): Promise<string[]> {
	return await invoke('plugin:link|link_check_protocols', { supported })
}

/** Get the MC port. Client: queries the host's Scaffolding server.
 *  Host: returns the locally-set MC port. */
export async function get_mc_port(): Promise<number | null> {
	return await invoke('plugin:link|link_get_mc_port')
}

/** Host: update the MC port (delayed mode / GameWatcher auto-detection). */
export async function update_mc_port(port: number): Promise<void> {
	return await invoke('plugin:link|link_update_mc_port', { port })
}

/** Host: update the mod list advertised via `c:host_mods`. */
export async function set_host_mods(mods: HostModInfo[]): Promise<void> {
	return await invoke('plugin:link|link_set_host_mods', { mods })
}

/** Update the local player's display name (takes effect on next heartbeat). */
export async function set_local_player_name(name: string): Promise<void> {
	return await invoke('plugin:link|link_set_local_player_name', { name })
}
