import { invoke } from '@tauri-apps/api/core'

export interface CfSearchParams {
	projectType: string
	searchFilter?: string | null
	gameVersion?: string | null
	loader?: string | null
	sort?: string | null
	page?: number | null
	pageSize?: number | null
}

export interface CfSearchResult {
	search: string
	result: {
		hits: Record<string, unknown>[]
		offset: number
		limit: number
		total_hits: number
	}
}

export interface CfFile {
	id: number
	mod_id: number
	file_name: string
	display_name: string
	download_url?: string | null
	game_versions: string[]
	release_type: number // 1=Release, 2=Beta, 3=Alpha
	file_length: number
}

/**
 * Search CurseForge and return Modrinth V3-compatible results.
 * Hits include extra `_cf_mod_id`, `_cf_class_id`, etc. fields for
 * CurseForge-specific install flow.
 */
export async function cfSearch(params: CfSearchParams): Promise<CfSearchResult> {
	return await invoke<CfSearchResult>('plugin:curseforge|cf_search', {
		params: {
			projectType: params.projectType,
			searchFilter: params.searchFilter ?? null,
			gameVersion: params.gameVersion ?? null,
			loader: params.loader ?? null,
			sort: params.sort ?? null,
			page: params.page ?? null,
			pageSize: params.pageSize ?? null,
		},
	})
}

/**
 * Get files for a CurseForge mod, optionally filtered by game version and loader.
 */
export async function cfGetModFiles(
	modId: number,
	gameVersion?: string | null,
	loader?: string | null,
): Promise<CfFile[]> {
	return await invoke<CfFile[]>('plugin:curseforge|cf_get_mod_files', {
		modId,
		gameVersion: gameVersion ?? null,
		loader: loader ?? null,
	})
}

/**
 * Get the direct download URL for a CurseForge file.
 * Returns null if third-party download is not available.
 */
export async function cfGetFileDownloadUrl(
	modId: number,
	fileId: number,
): Promise<string | null> {
	return await invoke<string | null>('plugin:curseforge|cf_get_file_download_url', {
		modId,
		fileId,
	})
}

/**
 * Install a CurseForge file to an existing instance via the InstallJob system.
 * Displays download progress in the action bar like other installs.
 */
export async function cfInstallFile(
	instanceId: string,
	modId: number,
	fileId: number,
	fileName: string,
	downloadUrl: string | null,
	contentType: string,
	title: string,
	iconUrl: string | null,
): Promise<Record<string, unknown>> {
	return await invoke<Record<string, unknown>>('plugin:curseforge|cf_install_file', {
		instanceId,
		modId,
		fileId,
		fileName,
		downloadUrl,
		contentType,
		title,
		iconUrl,
	})
}
