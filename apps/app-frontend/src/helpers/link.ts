import { invoke } from '@tauri-apps/api/core'

export interface LobbyStatus {
	state: 'idle' | 'host' | 'client' | 'error'
	lobbyCode: string | null
	networkName: string | null
	virtualIp: string | null
	mcPort: number | null
	localPort: number | null
	peerCount: number
	lastRefresh: number | null
	error: string | null
}

export async function create_lobby(
	networkName: string,
	password: string,
	mcPort: number,
): Promise<string> {
	return await invoke('plugin:link|link_create_lobby', {
		networkName,
		password,
		mcPort,
	})
}

export async function join_lobby(lobbyCode: string): Promise<number> {
	return await invoke('plugin:link|link_join_lobby', { lobbyCode })
}

export async function leave_lobby(): Promise<void> {
	return await invoke('plugin:link|link_leave_lobby')
}

export async function get_lobby_status(): Promise<LobbyStatus> {
	return await invoke('plugin:link|link_get_lobby_status')
}
