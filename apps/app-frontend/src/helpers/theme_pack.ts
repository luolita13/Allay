/**
 * Theme pack API wrappers.
 *
 * Wraps the `theme-pack` Tauri plugin so the frontend can install, uninstall,
 * list, and export theme packs.
 */
import { invoke } from '@tauri-apps/api/core'

export interface ThemePackManifest {
	manifest_version: number
	id: string
	name: string
	description: string | null
	author: string | null
	version: string | null
	background_image: string | null
	accent_color: string | null
	secondary_color: string | null
	background_blur: number | null
	background_opacity: number | null
	css_variables: Record<string, string> | null
	font_family: string | null
}

export interface InstalledThemePack {
	id: string
	name: string
	description: string | null
	author: string | null
	version: string | null
	dir: string
	background_image_path: string | null
	accent_color: string | null
	secondary_color: string | null
	background_blur: number | null
	background_opacity: number | null
	css_variables: Record<string, string> | null
	font_family: string | null
}

/**
 * Install a theme pack from a zip file path.
 */
export async function installFromPath(zipPath: string): Promise<ThemePackManifest> {
	return await invoke<ThemePackManifest>(
		'plugin:theme-pack|theme_pack_install_from_path',
		{ zipPath },
	)
}

/**
 * Uninstall a theme pack by id.
 */
export async function uninstall(themeId: string): Promise<void> {
	await invoke('plugin:theme-pack|theme_pack_uninstall', { themeId })
}

/**
 * List all installed theme packs.
 */
export async function listInstalled(): Promise<InstalledThemePack[]> {
	return await invoke<InstalledThemePack[]>(
		'plugin:theme-pack|theme_pack_list_installed',
	)
}

/**
 * Get a single installed theme pack by id.
 */
export async function getThemePack(
	themeId: string,
): Promise<InstalledThemePack | null> {
	return await invoke<InstalledThemePack | null>(
		'plugin:theme-pack|theme_pack_get',
		{ themeId },
	)
}

/**
 * Export an installed theme pack back into a zip file at `destPath`.
 */
export async function exportToZip(
	themeId: string,
	destPath: string,
): Promise<void> {
	await invoke('plugin:theme-pack|theme_pack_export_to_zip', {
		themeId,
		destPath,
	})
}

/**
 * Returns the absolute path to the themes directory.
 */
export async function getThemesDirPath(): Promise<string> {
	return await invoke<string>('plugin:theme-pack|theme_pack_get_themes_dir_path')
}
