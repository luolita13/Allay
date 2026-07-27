<script setup lang="ts">
import { ImagesIcon } from '@modrinth/assets'
import { defineMessages, HeadingLink, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { readDir, readFile, stat } from '@tauri-apps/plugin-fs'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import ImageViewer from '@/components/ui/ImageViewer.vue'
import { get_full_path, list } from '@/helpers/instance'
import { openPath } from '@/helpers/utils.js'
import { useTheming } from '@/store/theme'

dayjs.extend(relativeTime)

const { handleError } = injectNotificationManager()

const { formatMessage } = useVIntl()

const messages = defineMessages({
  title: { id: 'app.home.recent-screenshots.title', defaultMessage: 'Recent screenshots' },
})

interface Screenshot {
	name: string
	path: string
	url: string
	modified: number
	instanceId: string
	instanceName: string
}

const screenshots = ref<Screenshot[]>([])
const loading = ref(true)
const imageViewer = ref<InstanceType<typeof ImageViewer> | null>(null)

const themeStore = useTheming()

const MAX_SCREENSHOTS = 12
const SUPPORTED_EXTENSIONS = ['png', 'jpg', 'jpeg', 'bmp', 'webp', 'tiff']

const MIME_TYPES: Record<string, string> = {
	png: 'image/png',
	jpg: 'image/jpeg',
	jpeg: 'image/jpeg',
	bmp: 'image/bmp',
	webp: 'image/webp',
	tiff: 'image/tiff',
}

function normalizePath(p: string): string {
	return p.replace(/\\/g, '/')
}

function revokeScreenshotUrls() {
	for (const shot of screenshots.value) {
		URL.revokeObjectURL(shot.url)
	}
}

async function fileToObjectUrl(path: string, ext: string): Promise<string | null> {
	try {
		const bytes = await readFile(path)
		const blob = new Blob([bytes], { type: MIME_TYPES[ext] || 'image/png' })
		return URL.createObjectURL(blob)
	} catch (e) {
		console.warn('[HomeRecentScreenshots] failed to read screenshot:', path, e)
		return null
	}
}

async function fetchScreenshots() {
	loading.value = true
	try {
		const instances = await list().catch(handleError)
		if (!instances || instances.length === 0) {
			revokeScreenshotUrls()
			screenshots.value = []
			return
		}

		const all: Omit<Screenshot, 'url'>[] = []
		await Promise.all(
			instances.map(async (inst) => {
				try {
					const root = normalizePath(await get_full_path(inst.id))
					const folder = `${root}/screenshots`
					let entries
					try {
						entries = await readDir(folder)
					} catch {
						return
					}
					for (const entry of entries) {
						if (entry.isDirectory) continue
						const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
						if (!SUPPORTED_EXTENSIONS.includes(ext)) continue

						const absPath = `${folder}/${entry.name}`
						let mtime = 0
						try {
							const metadata = await stat(absPath)
							mtime = metadata.mtime ? Math.floor(metadata.mtime.getTime() / 1000) : 0
						} catch {
							continue
						}

						all.push({
							name: entry.name,
							path: absPath,
							modified: mtime,
							instanceId: inst.id,
							instanceName: inst.name,
						})
					}
				} catch {
					// Skip this instance
				}
			}),
		)

		all.sort((a, b) => b.modified - a.modified)
		const top = all.slice(0, MAX_SCREENSHOTS)

		const loaded = await Promise.all(
			top.map(async (shot) => {
				const ext = shot.name.split('.').pop()?.toLowerCase() ?? ''
				const url = await fileToObjectUrl(shot.path, ext)
				if (!url) return null
				return { ...shot, url } as Screenshot
			}),
		)

		revokeScreenshotUrls()
		screenshots.value = loaded.filter((s): s is Screenshot => s !== null)
	} finally {
		loading.value = false
	}
}

onUnmounted(revokeScreenshotUrls)

const hasScreenshots = computed(() => screenshots.value.length > 0)

function formatRelative(timestamp: number): string {
	if (!timestamp) return ''
	return dayjs.unix(timestamp).fromNow()
}

function openScreenshot(shot: Screenshot) {
	if (themeStore.useAppImageViewer) {
		const idx = screenshots.value.findIndex((s) => s.path === shot.path)
		imageViewer.value?.open(idx >= 0 ? idx : 0)
		return
	}
	openPath(shot.path).catch(handleError)
}

onMounted(fetchScreenshots)
</script>

<template>
	<div v-if="hasScreenshots" class="flex flex-col gap-2">
		<HeadingLink to="/library">
			<ImagesIcon class="inline mr-1" />
			{{ formatMessage(messages.title) }}
		</HeadingLink>
		<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
			<div
				v-for="shot in screenshots"
				:key="shot.path"
				class="screenshot-card group cursor-pointer"
				@click="openScreenshot(shot)"
				@contextmenu.prevent.stop
			>
				<div class="screenshot-thumb">
					<img
						:src="shot.url"
						:alt="shot.name"
						loading="lazy"
						class="w-full h-full object-cover"
					/>
				</div>
				<div class="screenshot-meta">
					<p class="m-0 text-xs font-medium text-contrast truncate">{{ shot.name }}</p>
					<p class="m-0 text-xs text-secondary truncate">
						{{ shot.instanceName }} · {{ formatRelative(shot.modified) }}
					</p>
				</div>
			</div>
		</div>
	</div>
	<ImageViewer ref="imageViewer" :images="screenshots.map(s => ({ path: s.path, name: s.name, url: s.url }))" />
</template>

<style scoped lang="scss">
.screenshot-card {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	padding: 0.4rem;
	border-radius: 0.5rem;
	background: var(--color-surface-2);
	transition: background 0.15s ease;

	&:hover {
		background: var(--color-surface-3);
	}
}

.screenshot-thumb {
	aspect-ratio: 16 / 9;
	overflow: hidden;
	border-radius: 0.375rem;
	background: var(--color-bg);
}

.screenshot-meta {
	display: flex;
	flex-direction: column;
	gap: 0.125rem;
}
</style>
