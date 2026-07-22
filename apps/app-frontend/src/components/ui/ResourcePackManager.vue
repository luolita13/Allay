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
				:placeholder="formatMessage(messages.searchPlaceholder, { count: items.length })"
			/>
			<div class="flex gap-2">
				<ButtonStyled type="outlined">
					<button class="!h-10" @click="openFolder">
						<FolderOpenIcon class="size-5" />
						{{ formatMessage(messages.openFolder) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button class="!h-10 flex items-center gap-2" @click="addFromFile">
						<PlusIcon class="size-5" />
						{{ formatMessage(messages.addFromFile) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<ReadyTransition :pending="loading && items.length === 0">
			<div v-if="filteredItems.length > 0" class="flex flex-col gap-2">
				<div
					v-for="item in filteredItems"
					:key="item.path"
					class="resource-card"
					@contextmenu.prevent.stop="(e) => onContextMenu(e, item)"
				>
					<div class="flex items-center gap-3 flex-1 min-w-0">
						<div class="resource-icon">
							<PackIcon />
						</div>
						<div class="flex flex-col min-w-0">
							<span class="font-semibold text-contrast truncate" :title="item.name">{{ item.name }}</span>
							<span class="text-xs text-secondary">{{ formatRelative(item.modified) }}</span>
						</div>
					</div>
					<div class="flex items-center gap-1">
						<ButtonStyled type="transparent" circular>
							<button v-tooltip="formatMessage(messages.openFolder)" @click="openFolder">
								<FolderOpenIcon class="size-4" />
							</button>
						</ButtonStyled>
						<ButtonStyled type="transparent" circular color="red">
							<button v-tooltip="formatMessage(messages.delete)" @click="deleteItem(item)">
								<TrashIcon class="size-4" />
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>
			<EmptyState
				v-else-if="!loading"
				type="folder-open"
				:heading="formatMessage(messages.emptyHeading)"
				:description="formatMessage(messages.emptyDescription)"
			>
				<template #actions>
					<ButtonStyled color="brand">
						<button class="!h-10 flex items-center gap-2" @click="addFromFile">
							<PlusIcon class="size-5" />
							{{ formatMessage(messages.addFromFile) }}
						</button>
					</ButtonStyled>
				</template>
			</EmptyState>
		</ReadyTransition>

		<ContextMenu ref="contextMenu" @option-clicked="handleContextOption">
			<template #delete><TrashIcon /> {{ formatMessage(messages.delete) }}</template>
			<template #open_folder><FolderOpenIcon /> {{ formatMessage(messages.openFolder) }}</template>
		</ContextMenu>

		<ConfirmModalWrapper
			ref="deleteConfirmModal"
			:title="formatMessage(messages.delete)"
			:description="formatMessage(messages.confirmDelete, { name: selected?.name ?? '' })"
			@proceed="doDeleteItem"
		/>
	</div>
</template>

<script setup lang="ts">
import {
	FolderOpenIcon,
	PackageIcon as PackIcon,
	PlusIcon,
	SearchIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	EmptyState,
	injectNotificationManager,
	ReadyTransition,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { readDir, remove, stat } from '@tauri-apps/plugin-fs'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import ConfirmModalWrapper from '@/components/ui/modal/ConfirmModalWrapper.vue'
import { add_project_from_path, get_full_path } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { openPath } from '@/helpers/utils.js'

dayjs.extend(relativeTime)

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()

const messages = defineMessages({
	searchPlaceholder: {
		id: 'app.instance.resource-pack.search-placeholder',
		defaultMessage: 'Search {count} packs...',
	},
	openFolder: { id: 'app.instance.resource-pack.open-folder', defaultMessage: 'Open folder' },
	addFromFile: { id: 'app.instance.resource-pack.add-from-file', defaultMessage: 'Add from file' },
	delete: { id: 'app.instance.resource-pack.delete', defaultMessage: 'Delete' },
	emptyHeading: {
		id: 'app.instance.resource-pack.empty-heading',
		defaultMessage: 'No packs installed',
	},
	emptyDescription: {
		id: 'app.instance.resource-pack.empty-description',
		defaultMessage: 'Add a pack file to get started',
	},
	added: { id: 'app.instance.resource-pack.added', defaultMessage: 'Pack added' },
	deleted: { id: 'app.instance.resource-pack.deleted', defaultMessage: 'Pack deleted' },
	confirmDelete: {
		id: 'app.instance.resource-pack.confirm-delete',
		defaultMessage: 'Are you sure you want to delete "{name}"?',
	},
})

interface ResourceItem {
	name: string
	path: string
	modified: number
}

const props = defineProps<{
	instance: GameInstance
	folder: string
	projectType: string
}>()

const instance = computed(() => props.instance)
const searchQuery = ref('')
const loading = ref(true)
const items = ref<ResourceItem[]>([])
const contextMenu = ref<InstanceType<typeof ContextMenu>>()
const selected = ref<ResourceItem | null>(null)
const deleteConfirmModal = useTemplateRef('deleteConfirmModal')

const SUPPORTED_EXTENSIONS = ['zip', 'tar', 'gz', '7z']

function normalizePath(p: string): string {
	return p.replace(/\\/g, '/')
}

async function refresh() {
	loading.value = true
	try {
		const root = normalizePath(await get_full_path(instance.value.id))
		const folderPath = `${root}/${props.folder}`
		let entries
		try {
			entries = await readDir(folderPath)
		} catch {
			items.value = []
			loading.value = false
			return
		}

		const results: ResourceItem[] = []
		for (const entry of entries) {
			if (entry.isDirectory) continue
			const ext = entry.name.split('.').pop()?.toLowerCase() ?? ''
			if (!SUPPORTED_EXTENSIONS.includes(ext)) continue

			const absPath = `${folderPath}/${entry.name}`
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
				modified: mtime,
			})
		}

		results.sort((a, b) => b.modified - a.modified)
		items.value = results
	} catch (err) {
		handleError(err as Error)
	} finally {
		loading.value = false
	}
}

const filteredItems = computed(() => {
	if (!searchQuery.value) return items.value
	const q = searchQuery.value.toLowerCase()
	return items.value.filter((i) => i.name.toLowerCase().includes(q))
})

function formatRelative(timestamp: number): string {
	if (!timestamp) return ''
	return dayjs.unix(timestamp).fromNow()
}

async function openFolder() {
	const root = normalizePath(await get_full_path(instance.value.id))
	await openPath(`${root}/${props.folder}`).catch(handleError)
}

async function addFromFile() {
	try {
		const selected = await openDialog({
			multiple: false,
			filters: [{ name: 'Pack files', extensions: SUPPORTED_EXTENSIONS }],
		})
		if (Array.isArray(selected) ? selected.length === 0 : !selected) return
		const path = Array.isArray(selected) ? selected[0] : (selected as string)
		await add_project_from_path(instance.value.id, path, props.projectType as any)
		addNotification({ type: 'success', title: formatMessage(messages.added) })
		await refresh()
	} catch (err) {
		handleError(err as Error)
	}
}

async function deleteItem(item: ResourceItem) {
	selected.value = item
	deleteConfirmModal.value?.show()
}

async function doDeleteItem() {
	const item = selected.value
	if (!item) return
	try {
		await remove(item.path)
		items.value = items.value.filter((i) => i.path !== item.path)
		addNotification({ type: 'success', title: formatMessage(messages.deleted) })
	} catch (err) {
		handleError(err as Error)
	}
}

function onContextMenu(event: MouseEvent, item: ResourceItem) {
	selected.value = item
	contextMenu.value?.showMenu(event, item, [
		{ name: 'open_folder' },
		{ type: 'divider' },
		{ name: 'delete', color: 'danger' },
	])
}

function handleContextOption(args: { option: string; item: unknown }) {
	const item = args.item as ResourceItem
	switch (args.option) {
		case 'open_folder':
			openFolder()
			break
		case 'delete':
			deleteItem(item)
			break
	}
}

watch(() => instance.value.id, refresh)
onMounted(refresh)
</script>

<style scoped lang="scss">
.resource-card {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.75rem 1rem;
	border-radius: 0.5rem;
	background: var(--color-surface-2);
	border: 1px solid var(--color-surface-5);
	transition: all 0.15s ease;
}

.resource-card:hover {
	background: var(--color-surface-3);
	border-color: var(--color-brand);
}

.resource-icon {
	width: 2.5rem;
	height: 2.5rem;
	border-radius: 0.5rem;
	display: flex;
	align-items: center;
	justify-content: center;
	background: linear-gradient(135deg, var(--color-brand), var(--color-brand-highlight));
	color: white;
	flex-shrink: 0;
}

.resource-icon svg {
	width: 1.5rem;
	height: 1.5rem;
}
</style>
