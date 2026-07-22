<script setup>
import { BoxIcon, FolderOpenIcon, FolderSearchIcon, TrashIcon } from '@modrinth/assets'
import { ButtonStyled, Combobox, defineMessages, injectNotificationManager, Slider, StyledInput, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import ConfirmModalWrapper from '@/components/ui/modal/ConfirmModalWrapper.vue'
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
		defaultMessage: 'App directory',
	},
	appDirectoryDescription: {
		id: 'app.resource-management.app-directory.description',
		defaultMessage:
			'The directory where the launcher stores all of its files. Changes will be applied after restarting the launcher.',
	},
	purgeCacheConfirmTitle: {
		id: 'app.resource-management.purge-cache.confirm-title',
		defaultMessage: 'Are you sure you want to purge the cache?',
	},
	purgeCacheConfirmDescription: {
		id: 'app.resource-management.purge-cache.confirm-description',
		defaultMessage:
			'If you proceed, your entire cache will be purged. This may slow down the app temporarily.',
	},
	purgeCacheButton: {
		id: 'app.resource-management.purge-cache.button',
		defaultMessage: 'Purge cache',
	},
	appCacheTitle: {
		id: 'app.resource-management.app-cache.title',
		defaultMessage: 'App cache',
	},
	appCacheDescription: {
		id: 'app.resource-management.app-cache.description',
		defaultMessage:
			'The Modrinth app stores a cache of data to speed up loading. This can be purged to force the app to reload data. This may slow down the app temporarily.',
	},
	downloadSourcesTitle: {
		id: 'app.resource-management.download-sources.title',
		defaultMessage: 'Download sources',
	},
	downloadSourcesDescription: {
		id: 'app.resource-management.download-sources.description',
		defaultMessage:
			'Automatic mode chooses between official and mirror sources based on your local environment and recent connection quality.',
	},
	automaticSource: {
		id: 'app.resource-management.source.automatic',
		defaultMessage: 'Automatic (Recommended)',
	},
	officialSource: {
		id: 'app.resource-management.source.official',
		defaultMessage: 'Prefer official sources',
	},
	mirrorSource: {
		id: 'app.resource-management.source.mirror',
		defaultMessage: 'Prefer mirror sources',
	},
	minecraftMetadataSourceTitle: {
		id: 'app.resource-management.minecraft-metadata-source.title',
		defaultMessage: 'Minecraft metadata',
	},
	minecraftMetadataSourceDescription: {
		id: 'app.resource-management.minecraft-metadata-source.description',
		defaultMessage: 'Version manifests and metadata for Minecraft and supported mod loaders.',
	},
	minecraftFileSourceTitle: {
		id: 'app.resource-management.minecraft-file-source.title',
		defaultMessage: 'Minecraft files, loaders, and Java',
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
		defaultMessage: 'Maximum concurrent downloads',
	},
	maxConcurrentDownloadsDescription: {
		id: 'app.resource-management.max-concurrent-downloads.description',
		defaultMessage:
			'The maximum amount of files the launcher can download at the same time. Set this to a lower value if you have a poor internet connection. (app restart required to take effect)',
	},
	maxChunksPerFileTitle: {
		id: 'app.resource-management.max-chunks-per-file.title',
		defaultMessage: 'Maximum chunks per file download',
	},
	maxChunksPerFileDescription: {
		id: 'app.resource-management.max-chunks-per-file.description',
		defaultMessage:
			'For large files (>1MB), the launcher splits the download into multiple parallel chunks using HTTP Range requests. More chunks = faster downloads on high-bandwidth connections. (app restart required to take effect)',
	},
	maxConcurrentWritesTitle: {
		id: 'app.resource-management.max-concurrent-writes.title',
		defaultMessage: 'Maximum concurrent writes',
	},
	maxConcurrentWritesDescription: {
		id: 'app.resource-management.max-concurrent-writes.description',
		defaultMessage:
			'The maximum amount of files the launcher can write to the disk at once. Set this to a lower value if you are frequently getting I/O errors. (app restart required to take effect)',
	},
	dbBackupsTitle: {
		id: 'app.resource-management.db-backups.title',
		defaultMessage: 'App database backups',
	},
	dbBackupsButton: {
		id: 'app.resource-management.db-backups.button',
		defaultMessage: 'Open backups folder',
	},
	dbBackupsDescription: {
		id: 'app.resource-management.db-backups.description',
		defaultMessage:
			'Backups of important app data are stored here in case you need to recover them later.',
	},
	launcherLogsTitle: {
		id: 'app.resource-management.launcher-logs.title',
		defaultMessage: 'Launcher logs',
	},
	launcherLogsButton: {
		id: 'app.resource-management.launcher-logs.button',
		defaultMessage: 'Open logs folder',
	},
	launcherLogsDescription: {
		id: 'app.resource-management.launcher-logs.description',
		defaultMessage:
			'Session logs are stored here. If you encounter a bug or crash, please include the latest log file when reporting the issue.',
	},
	selectAppDirectory: {
		id: 'app.resource-management.select-app-directory',
		defaultMessage: 'Select a new app directory',
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
			<ConfirmModalWrapper
				ref="purgeCacheConfirmModal"
				:title="formatMessage(messages.purgeCacheConfirmTitle)"
				:description="formatMessage(messages.purgeCacheConfirmDescription)"
				:has-to-type="false"
				:proceed-label="formatMessage(messages.purgeCacheButton)"
				:show-ad-on-close="false"
				@proceed="purgeCache"
			/>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.appCacheTitle) }}</h2>
			<button id="purge-cache" class="btn min-w-max" @click="handlePurgeCacheClick">
				<TrashIcon />
				{{ formatMessage(messages.purgeCacheButton) }}
			</button>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.appCacheDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-3">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast mt-4">
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

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast mt-4">{{ formatMessage(messages.maxConcurrentDownloadsTitle) }}</h2>
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
			<h2 class="mt-0 m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.maxConcurrentWritesTitle) }}</h2>
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

		<div class="flex flex-col gap-2.5">
			<h2 class="mt-0 m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.dbBackupsTitle) }}</h2>
			<button id="open-db-backups-folder" class="btn min-w-max" @click="openDbBackupsFolder">
				<FolderOpenIcon />
				{{ formatMessage(messages.dbBackupsButton) }}
			</button>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.dbBackupsDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.launcherLogsTitle) }}</h2>
			<button id="open-launcher-logs-folder" class="btn min-w-max" @click="openLauncherLogsFolder">
				<FolderOpenIcon />
				{{ formatMessage(messages.launcherLogsButton) }}
			</button>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.launcherLogsDescription) }}
			</p>
		</div>
	</div>
</template>
