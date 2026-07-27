import { convertFileSrc } from '@tauri-apps/api/core'
import { readFile } from '@tauri-apps/plugin-fs'

const blobUrlCache = new Map<string, string>()
const fallbackCache = new Set<string>()

/**
 * Creates an object URL for a local file path by reading its binary contents.
 * This avoids Tauri's assetProtocol scope restrictions that can block arbitrary
 * local image paths (e.g. user-selected background images outside the app dir).
 *
 * The URL is cached per path. Call `revokePathUrl` to release it — do NOT
 * call `URL.revokeObjectURL` directly on the returned URL, as that would
 * invalidate cached entries for other consumers.
 *
 * If `readFile` fails (e.g. path outside fs:scope), falls back to
 * `convertFileSrc` which serves the file via Tauri's asset protocol.
 */
export async function createObjectUrlFromPath(path: string): Promise<string | null> {
	if (!path) return null

	const cached = blobUrlCache.get(path)
	if (cached) return cached

	// If we already know readFile fails for this path, go straight to fallback
	if (fallbackCache.has(path)) {
		return convertFileSrc(path)
	}

	try {
		const bytes = await readFile(path)
		const mime = guessMimeType(path)
		const blob = new Blob([bytes], { type: mime })
		const url = URL.createObjectURL(blob)
		blobUrlCache.set(path, url)
		return url
	} catch (error) {
		console.warn(
			`Failed to read image from ${path} via readFile, falling back to convertFileSrc:`,
			error,
		)
		fallbackCache.add(path)
		return convertFileSrc(path)
	}
}

/** Revokes the cached blob URL for a path and removes it from cache. */
export function revokePathUrl(path: string) {
	const url = blobUrlCache.get(path)
	if (url) {
		URL.revokeObjectURL(url)
		blobUrlCache.delete(path)
	}
}

function guessMimeType(path: string): string {
	const extension = path.split('.').pop()?.toLowerCase()
	switch (extension) {
		case 'png':
			return 'image/png'
		case 'jpg':
		case 'jpeg':
			return 'image/jpeg'
		case 'webp':
			return 'image/webp'
		case 'gif':
			return 'image/gif'
		case 'avif':
			return 'image/avif'
		case 'bmp':
			return 'image/bmp'
		default:
			return 'image/png'
	}
}

/**
 * Revokes a previously created object URL to free memory.
 */
export function revokeObjectUrl(url: string) {
	URL.revokeObjectURL(url)
}
