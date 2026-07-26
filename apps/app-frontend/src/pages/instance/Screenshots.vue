<template>
	<div class="flex flex-col gap-4">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<StyledInput
				v-model="searchQuery"
				:icon="SearchIcon"
				type="text"
				autocomplete="off"
				:spellcheck="false"
				input-class="!h-10"
				wrapper-class="flex-1 min-w-0 max-w-md"
				clearable
				:placeholder="formatMessage(messages.searchPlaceholder, { count: screenshots.length })"
			/>
			<div class="flex gap-2">
				<ButtonStyled type="transparent" hover-color-fill="none">
					<button :disabled="loading" @click="refresh">
						<RefreshCwIcon :class="loading ? 'animate-spin' : ''" />
						{{ formatMessage(commonMessages.refreshButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled type="outlined">
					<button class="!h-10" @click="openFolder">
						<FolderOpenIcon class="size-5" />
						{{ formatMessage(messages.openFolder) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<ReadyTransition :pending="loading && screenshots.length === 0">
			<div v-if="filteredScreenshots.length > 0" class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
				<div
					v-for="shot in filteredScreenshots"
					:key="shot.path"
					class="screenshot-card group"
					@contextmenu.prevent.stop="(e) => onContextMenu(e, shot)"
				>
					<div class="screenshot-thumb">
						<img :src="shot.url" :alt="shot.name" loading="lazy" @click="openScreenshot(shot)" />
					</div>
					<div class="screenshot-meta">
						<span class="text-sm font-medium text-contrast truncate" :title="shot.name">{{ shot.name }}</span>
						<span class="text-xs text-secondary">{{ formatRelative(shot.modified) }}</span>
					</div>
				</div>
			</div>
			<EmptyState
				v-else-if="!loading"
				type="image"
				:heading="formatMessage(messages.noScreenshotsHeading)"
				:description="formatMessage(messages.noScreenshotsDescription)"
			>
				<template #actions>
					<ButtonStyled type="outlined">
						<button class="!h-10" @click="openFolder">
							<FolderOpenIcon class="size-5" />
							{{ formatMessage(messages.openFolder) }}
						</button>
					</ButtonStyled>
				</template>
			</EmptyState>
		</ReadyTransition>

		<ContextMenu ref="contextMenu" @option-clicked="handleContextOption">
			<template #open><ImageIcon /> {{ formatMessage(messages.open) }}</template>
			<template #open_folder><FolderOpenIcon /> {{ formatMessage(messages.openFolder) }}</template>
			<template #copy><ClipboardCopyIcon /> {{ formatMessage(messages.copyToClipboard) }}</template>
			<template #delete><TrashIcon /> {{ formatMessage(messages.delete) }}</template>
		</ContextMenu>

		<ConfirmModalWrapper
			ref="deleteConfirmModal"
			:title="formatMessage(messages.delete)"
			:description="formatMessage(messages.confirmDelete, { name: selected?.name ?? '' })"
			@proceed="doDeleteScreenshot"
		/>
		<ImageViewer ref="imageViewer" :images="filteredScreenshots.map(s => ({ path: s.path, name: s.name, url: s.url }))" />
	</div>
</template>

<script setup lang="ts">
import {
	ClipboardCopyIcon,
	FolderOpenIcon,
	ImageIcon,
	RefreshCwIcon,
	SearchIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	EmptyState,
	injectNotificationManager,
	ReadyTransition,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { readDir, remove, stat } from '@tauri-apps/plugin-fs'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, onMounted, ref, useTemplateRef } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import ConfirmModalWrapper from '@/components/ui/modal/ConfirmModalWrapper.vue'
import ImageViewer from '@/components/ui/ImageViewer.vue'
import { get_full_path } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { openPath } from '@/helpers/utils.js'
import { useTheming } from '@/store/theme'

dayjs.extend(relativeTime)

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()

const messages = defineMessages({
	searchPlaceholder: {
		id: 'app.instance.screenshots.search-placeholder',
		defaultMessage: 'Search {count} screenshots...',
	},
	openFolder: { id: 'app.instance.screenshots.open-folder', defaultMessage: 'Open folder' },
	noScreenshotsHeading: {
		id: 'app.instance.screenshots.none-heading',
		defaultMessage: 'No screenshots yet',
	},
	noScreenshotsDescription: {
		id: 'app.instance.screenshots.none-description',
		defaultMessage: 'Screenshots taken in-game will appear here',
	},
	open: { id: 'app.instance.screenshots.open', defaultMessage: 'Open' },
	copyToClipboard: { id: 'app.instance.screenshots.copy', defaultMessage: 'Copy to clipboard' },
	delete: { id: 'app.instance.screenshots.delete', defaultMessage: 'Delete' },
	confirmDelete: {
		id: 'app.instance.screenshots.confirm-delete',
		defaultMessage: 'Are you sure you want to delete "{name}"?',
	},
	deleted: { id: 'app.instance.screenshots.deleted', defaultMessage: 'Screenshot deleted' },
	copied: { id: 'app.instance.screenshots.copied', defaultMessage: 'Path copied to clipboard' },
})

interface Screenshot {
	name: string
	path: string
	url: string
	modified: number
}

const props = defineProps<{
	instance: GameInstance
	options: InstanceType<typeof ContextMenu> | null
	offline: boolean
	playing: boolean
	installed: boolean
	isServerInstance?: boolean
}>()

const instance = computed(() => props.instance)
const searchQuery = ref('')
const loading = ref(true)
const screenshots = ref<Screenshot[]>([])
const imageViewer = ref<InstanceType<typeof ImageViewer> | null>(null)
const themeStore = useTheming()
const contextMenu = ref<InstanceType<typeof ContextMenu>>()
const selected = ref<Screenshot | null>(null)
const deleteConfirmModal = useTemplateRef('deleteConfirmModal')

const SUPPORTED_EXTENSIONS = ['png', 'jpg', 'jpeg', 'bmp', 'webp', 'tiff']

function normalizePath(p: string): string {
	return p.replace(/\\/g, '/')
}

async function refresh() {
	loading.value = true
	try {
		const root = normalizePath(await get_full_path(instance.value.id))
		const folder = `${root}/screenshots`
		let entries
		try {
			entries = await readDir(folder)
		} catch {
			// Folder doesn't exist
			screenshots.value = []
			loading.value = false
			return
		}

		const results: Screenshot[] = []
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

			results.push({
				name: entry.name,
				path: absPath,
				url: convertFileSrc(absPath),
				modified: mtime,
			})
		}

		results.sort((a, b) => b.modified - a.modified)
		screenshots.value = results
	} catch (err) {
		handleError(err as Error)
	} finally {
		loading.value = false
	}
}

const filteredScreenshots = computed(() => {
	if (!searchQuery.value) return screenshots.value
	const q = searchQuery.value.toLowerCase()
	return screenshots.value.filter((s) => s.name.toLowerCase().includes(q))
})

function formatRelative(timestamp: number): string {
	if (!timestamp) return ''
	return dayjs.unix(timestamp).fromNow()
}

async function openFolder() {
	const root = normalizePath(await get_full_path(instance.value.id))
	await openPath(`${root}/screenshots`).catch(handleError)
}

async function openScreenshot(shot: Screenshot) {
	if (themeStore.useAppImageViewer) {
		const idx = screenshots.value.findIndex((s) => s.path === shot.path)
		imageViewer.value?.open(idx >= 0 ? idx : 0)
		return
	}
	await openPath(shot.path).catch(handleError)
}

async function copyPath(shot: Screenshot) {
	try {
		await navigator.clipboard.writeText(shot.path)
		addNotification({ type: 'success', title: formatMessage(messages.copied) })
	} catch (err) {
		handleError(err as Error)
	}
}

async function deleteScreenshot(shot: Screenshot) {
	selected.value = shot
	deleteConfirmModal.value?.show()
}

async function doDeleteScreenshot() {
	const shot = selected.value
	if (!shot) return
	try {
		await remove(shot.path)
		screenshots.value = screenshots.value.filter((s) => s.path !== shot.path)
		addNotification({ type: 'success', title: formatMessage(messages.deleted) })
	} catch (err) {
		handleError(err as Error)
	}
}

function onContextMenu(event: MouseEvent, shot: Screenshot) {
	selected.value = shot
	contextMenu.value?.showMenu(event, shot, [
		{ name: 'open', color: 'primary' },
		{ name: 'open_folder' },
		{ type: 'divider' },
		{ name: 'copy' },
		{ type: 'divider' },
		{ name: 'delete', color: 'danger' },
	])
}

function handleContextOption(args: { option: string; item: unknown }) {
	const shot = args.item as Screenshot
	switch (args.option) {
		case 'open':
			openScreenshot(shot)
			break
		case 'open_folder':
			openFolder()
			break
		case 'copy':
			copyPath(shot)
			break
		case 'delete':
			deleteScreenshot(shot)
			break
	}
}

onMounted(refresh)
</script>

<style scoped lang="scss">
.screenshot-card {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: 0.5rem;
	border-radius: 0.5rem;
	background: var(--color-surface-2);
	border: 1px solid var(--color-surface-5);
	transition: all 0.15s ease;
}

.screenshot-card:hover {
	background: var(--color-surface-3);
	border-color: var(--color-brand);
	transform: translateY(-1px);
}

.screenshot-thumb {
	aspect-ratio: 16 / 9;
	overflow: hidden;
	border-radius: 0.375rem;
	background: var(--color-surface-4);
	cursor: pointer;
}

.screenshot-thumb img {
	width: 100%;
	height: 100%;
	object-fit: cover;
	transition: transform 0.2s ease;
}

.screenshot-thumb:hover img {
	transform: scale(1.05);
}

.screenshot-meta {
	display: flex;
	flex-direction: column;
	gap: 0.125rem;
	min-width: 0;
}
</style>
