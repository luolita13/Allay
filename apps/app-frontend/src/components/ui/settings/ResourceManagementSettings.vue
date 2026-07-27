<script setup>
import { BoxIcon, FolderOpenIcon, FolderSearchIcon, TrashIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	Slider,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import { purge_cache_types } from '@/helpers/cache.js'
import { get, set } from '@/helpers/settings.ts'
import { showAppDbBackupsFolder, showLauncherLogsFolder } from '@/helpers/utils.js'
import { useTheming } from '@/store/state'

const { handleError } = injectNotificationManager()
const themeStore = useTheming()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	appDirectoryTitle: {
		id: 'app.resource-management.app-directory.title',
		defaultMessage: 'App Directory',
	},
	appDirectoryDescription: {
		id: 'app.resource-management.app-directory.description',
		defaultMessage:
			'The directory where the launcher stores all files. Changes take effect after restarting the launcher.',
	},
	purgeCacheConfirmTitle: {
		id: 'app.resource-management.purge-cache.confirm-title',
		defaultMessage: 'Are you sure you want to clear the cache?',
	},
	purgeCacheConfirmDescription: {
		id: 'app.resource-management.purge-cache.confirm-description',
		defaultMessage:
			'If you continue, all caches will be cleared, which may cause the app to temporarily slow down.',
	},
	purgeCacheButton: {
		id: 'app.resource-management.purge-cache.button',
		defaultMessage: 'Clear Cache',
	},
	appCacheTitle: {
		id: 'app.resource-management.app-cache.title',
		defaultMessage: 'App Cache',
	},
	appCacheDescription: {
		id: 'app.resource-management.app-cache.description',
		defaultMessage:
			'Modrinth App caches data to speed up loading. Clearing the cache forces the app to reload data, which may cause temporary slowdowns.',
	},
	downloadSourcesTitle: {
		id: 'app.resource-management.download-sources.title',
		defaultMessage: 'Download Sources',
	},
	downloadSourcesDescription: {
		id: 'app.resource-management.download-sources.description',
		defaultMessage:
			'Auto mode automatically selects between official and mirror sources based on your local network conditions and recent connection quality.',
	},
	automaticSource: {
		id: 'app.resource-management.source.automatic',
		defaultMessage: 'Auto (Recommended)',
	},
	officialSource: {
		id: 'app.resource-management.source.official',
		defaultMessage: 'Prefer Official Source',
	},
	mirrorSource: {
		id: 'app.resource-management.source.mirror',
		defaultMessage: 'Prefer Mirror Source',
	},
	minecraftMetadataSourceTitle: {
		id: 'app.resource-management.minecraft-metadata-source.title',
		defaultMessage: 'Minecraft Metadata',
	},
	minecraftMetadataSourceDescription: {
		id: 'app.resource-management.minecraft-metadata-source.description',
		defaultMessage: 'Version manifests and metadata for Minecraft and supported mod loaders.',
	},
	minecraftFileSourceTitle: {
		id: 'app.resource-management.minecraft-file-source.title',
		defaultMessage: 'Minecraft Files, Loaders, and Java',
	},
	minecraftFileSourceDescription: {
		id: 'app.resource-management.minecraft-file-source.description',
		defaultMessage: 'Game files, assets, libraries, mod loaders, and Java runtimes.',
	},
	modrinthSourceTitle: {
			id: 'app.resource-management.modrinth-source.title',
			defaultMessage: 'Modrinth',
		},
		modrinthSourceDescription: {
			id: 'app.resource-management.modrinth-source.description',
			defaultMessage: 'Modrinth mod file downloads. Content browsing always uses the Modrinth API.',
		},
		curseforgeSourceTitle: {
			id: 'app.resource-management.curseforge-source.title',
			defaultMessage: 'CurseForge',
		},
		curseforgeSourceDescription: {
			id: 'app.resource-management.curseforge-source.description',
			defaultMessage: 'CurseForge API requests and file downloads.',
		},
	maxConcurrentDownloadsTitle: {
		id: 'app.resource-management.max-concurrent-downloads.title',
		defaultMessage: 'Max Concurrent Downloads',
	},
	maxConcurrentDownloadsDescription: {
		id: 'app.resource-management.max-concurrent-downloads.description',
		defaultMessage:
			'Maximum number of files the launcher can download simultaneously. Lower this value on poor network connections. (Requires app restart to take effect)',
	},
	maxChunksPerFileTitle: {
		id: 'app.resource-management.max-chunks-per-file.title',
		defaultMessage: 'Max Chunks per File',
	},
	maxChunksPerFileDescription: {
		id: 'app.resource-management.max-chunks-per-file.description',
		defaultMessage:
			'For files larger than 1 MB, the launcher splits downloads into multiple parallel chunks using HTTP Range requests. More chunks means faster downloads on high-bandwidth connections. (Requires app restart to take effect)',
	},
	maxConcurrentWritesTitle: {
		id: 'app.resource-management.max-concurrent-writes.title',
		defaultMessage: 'Max Concurrent Writes',
	},
	maxConcurrentWritesDescription: {
		id: 'app.resource-management.max-concurrent-writes.description',
		defaultMessage:
			'Maximum number of files the launcher can write to disk simultaneously. Lower this value if you frequently encounter I/O errors. (Requires app restart to take effect)',
	},
	dbBackupsTitle: {
		id: 'app.resource-management.db-backups.title',
		defaultMessage: 'App Database Backups',
	},
	dbBackupsButton: {
		id: 'app.resource-management.db-backups.button',
		defaultMessage: 'Open Backup Folder',
	},
	dbBackupsDescription: {
		id: 'app.resource-management.db-backups.description',
		defaultMessage:
			'Backups of important app data are stored here for easy recovery later.',
	},
	launcherLogsTitle: {
		id: 'app.resource-management.launcher-logs.title',
		defaultMessage: 'Launcher Logs',
	},
	launcherLogsButton: {
		id: 'app.resource-management.launcher-logs.button',
		defaultMessage: 'Open Logs Folder',
	},
	launcherLogsDescription: {
		id: 'app.resource-management.launcher-logs.description',
		defaultMessage:
			'Session logs are stored here. If you encounter a bug or crash, please attach the latest log file when reporting an issue.',
	},
	selectAppDirectory: {
		id: 'app.resource-management.select-app-directory',
		defaultMessage: 'Select new app directory',
	},
})

const settings = ref(await get())
const purgeCacheConfirmModal = ref(null)

const sourceOptions = computed(() => [
	{ value: '1', label: formatMessage(messages.automaticSource) },
	{ value: '2', label: formatMessage(messages.officialSource) },
	{ value: '0', label: formatMessage(messages.mirrorSource) },
])

const gameFileSource = computed({
	get: () => String(settings.value.game_file_source),
	set: (value) => {
		settings.value.game_file_source = Number(value)
	},
})

const versionListSource = computed({
	get: () => String(settings.value.version_list_source),
	set: (value) => {
		settings.value.version_list_source = Number(value)
	},
})

const modrinthSource = computed({
	get: () => String(settings.value.community_source),
	set: (value) => {
		settings.value.community_source = Number(value)
	},
})

const curseforgeSource = computed({
	get: () => String(settings.value.curseforge_source),
	set: (value) => {
		settings.value.curseforge_source = Number(value)
	},
})

watch(
	settings,
	async () => {
		const setSettings = JSON.parse(JSON.stringify(settings.value))

		if (!setSettings.custom_dir) {
			setSettings.custom_dir = null
		}

		await set(setSettings)
	},
	{ deep: true },
)

async function purgeCache() {
	await purge_cache_types([
		'project',
		'project_v3',
		'version',
		'user',
		'team',
		'organization',
		'file',
		'loader_manifest',
		'minecraft_manifest',
		'categories',
		'report_types',
		'loaders',
		'game_versions',
		'donation_platforms',
		'file_hash',
		'file_update',
		'search_results',
		'search_results_v3',
	]).catch(handleError)
}

function handlePurgeCacheClick() {
	if (themeStore.getFeatureFlag('skip_non_essential_warnings')) {
		void purgeCache()
		return
	}

	purgeCacheConfirmModal.value?.show()
}

async function openDbBackupsFolder() {
	await showAppDbBackupsFolder().catch(handleError)
}

async function openLauncherLogsFolder() {
	await showLauncherLogsFolder().catch(handleError)
}

async function findLauncherDir() {
	const newDir = await open({
		multiple: false,
		directory: true,
		title: formatMessage(messages.selectAppDirectory),
	})

	if (newDir) {
		settings.value.custom_dir = newDir
	}
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<!-- ===== Storage ===== -->
		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.appDirectoryTitle) }}</h2>
			<StyledInput
				id="appDir"
				v-model="settings.custom_dir"
				:icon="BoxIcon"
				type="text"
				wrapper-class="w-full"
			>
				<template #right>
					<ButtonStyled circular>
						<button class="ml-1.5" @click="findLauncherDir">
							<FolderSearchIcon />
						</button>
					</ButtonStyled>
				</template>
			</StyledInput>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.appDirectoryDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<ConfirmModal
				ref="purgeCacheConfirmModal"
				:title="formatMessage(messages.purgeCacheConfirmTitle)"
				:description="formatMessage(messages.purgeCacheConfirmDescription)"
				:has-to-type="false"
				:proceed-label="formatMessage(messages.purgeCacheButton)"
				@proceed="purgeCache"
			/>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.appCacheTitle) }}</h2>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.appCacheDescription) }}
			</p>
			<ButtonStyled>
				<button id="purge-cache" @click="handlePurgeCacheClick">
					<TrashIcon />
					{{ formatMessage(messages.purgeCacheButton) }}
				</button>
			</ButtonStyled>
		</div>

		<hr class="bg-button-border border-none h-[1px]" />

		<!-- ===== Downloads ===== -->
		<div class="flex flex-col gap-3">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.downloadSourcesTitle) }}
				</h2>
				<p class="m-0 leading-tight text-secondary">
					{{ formatMessage(messages.downloadSourcesDescription) }}
				</p>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-base font-semibold text-contrast">
						{{ formatMessage(messages.minecraftMetadataSourceTitle) }}
					</h3>
					<p class="m-0 leading-tight text-secondary">
						{{ formatMessage(messages.minecraftMetadataSourceDescription) }}
					</p>
				</div>
				<div class="w-48 shrink-0">
					<Combobox v-model="versionListSource" :options="sourceOptions" />
				</div>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-base font-semibold text-contrast">
						{{ formatMessage(messages.minecraftFileSourceTitle) }}
					</h3>
					<p class="m-0 leading-tight text-secondary">
						{{ formatMessage(messages.minecraftFileSourceDescription) }}
					</p>
				</div>
				<div class="w-48 shrink-0">
					<Combobox v-model="gameFileSource" :options="sourceOptions" />
				</div>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-base font-semibold text-contrast">
						{{ formatMessage(messages.modrinthSourceTitle) }}
					</h3>
					<p class="m-0 leading-tight text-secondary">
						{{ formatMessage(messages.modrinthSourceDescription) }}
					</p>
				</div>
				<div class="w-48 shrink-0">
					<Combobox v-model="modrinthSource" :options="sourceOptions" />
				</div>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-base font-semibold text-contrast">
						{{ formatMessage(messages.curseforgeSourceTitle) }}
					</h3>
					<p class="m-0 leading-tight text-secondary">
						{{ formatMessage(messages.curseforgeSourceDescription) }}
					</p>
				</div>
				<div class="w-48 shrink-0">
					<Combobox v-model="curseforgeSource" :options="sourceOptions" />
				</div>
			</div>
		</div>

		<hr class="bg-button-border border-none h-[1px]" />

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.maxConcurrentDownloadsTitle) }}</h2>
			<Slider
				id="max-downloads"
				v-model="settings.max_concurrent_downloads"
				:min="1"
				:max="30"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maxConcurrentDownloadsDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.maxChunksPerFileTitle) }}</h2>
			<Slider
				id="max-chunks"
				v-model="settings.max_chunks_per_file"
				:min="1"
				:max="32"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maxChunksPerFileDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.maxConcurrentWritesTitle) }}</h2>
			<Slider
				id="max-writes"
				v-model="settings.max_concurrent_writes"
				:min="1"
				:max="50"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maxConcurrentWritesDescription) }}
			</p>
		</div>

		<hr class="bg-button-border border-none h-[1px]" />

		<!-- ===== Maintenance ===== -->
		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.dbBackupsTitle) }}</h2>
			<ButtonStyled>
				<button id="open-db-backups-folder" @click="openDbBackupsFolder">
					<FolderOpenIcon />
					{{ formatMessage(messages.dbBackupsButton) }}
				</button>
			</ButtonStyled>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.dbBackupsDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.launcherLogsTitle) }}</h2>
			<ButtonStyled>
				<button id="open-launcher-logs-folder" @click="openLauncherLogsFolder">
					<FolderOpenIcon />
					{{ formatMessage(messages.launcherLogsButton) }}
				</button>
			</ButtonStyled>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.launcherLogsDescription) }}
			</p>
		</div>
	</div>
</template>
