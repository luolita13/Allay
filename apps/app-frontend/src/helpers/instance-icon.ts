import { convertFileSrc } from '@tauri-apps/api/core'

import { getLoaderIcon } from './loader-icons'

export function getInstanceIconSrc(instance: { icon_path?: string | null; loader?: string }): string {
	if (instance.icon_path) return convertFileSrc(instance.icon_path)
	return getLoaderIcon(instance.loader ?? 'vanilla')
}
