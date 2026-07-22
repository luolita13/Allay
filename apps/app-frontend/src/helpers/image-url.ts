import { readFile } from '@tauri-apps/plugin-fs'

const blobUrlCache = new Map<string, string>()

/**
 * Creates an object URL for a local file path by reading its binary contents.
 * This avoids Tauri's assetProtocol scope restrictions that can block arbitrary
 * local image paths (e.g. user-selected background images outside the app dir).
 */
export async function createObjectUrlFromPath(path: string): Promise<string | null> {
	if (!path) return null

	const cached = blobUrlCache.get(path)
	if (cached) return cached

	try {
		const bytes = await readFile(path)
		const mime = guessMimeType(path)
		const blob = new Blob([bytes], { type: mime })
		const url = URL.createObjectURL(blob)
		blobUrlCache.set(path, url)
		return url
	} catch (error) {
		console.warn(`Failed to load image from ${path}:`, error)
		return null
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
