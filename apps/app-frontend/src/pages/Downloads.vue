<template>
	<div class="flex flex-col gap-3 p-6">
		<NavTabs
			:active-index="tab === 'active' ? 0 : 1"
			:links="downloadTabs"
			mode="local"
			@tab-click="selectTab"
		/>

		<div class="flex flex-wrap items-center gap-2">
			<ButtonStyled type="transparent" size="small" @click="refreshDownloads">
				<button :disabled="refreshing">
					<RefreshCwIcon :class="{ 'animate-spin': refreshing }" />
					{{ formatMessage(messages.refresh) }}
				</button>
			</ButtonStyled>
			<div class="flex-1"></div>
			<StyledInput
				v-model="query"
				:icon="SearchIcon"
				:placeholder="formatMessage(messages.search)"
				clearable
				wrapper-class="flex-1 min-w-0"
			/>
			<DropdownSelect
				v-model="provider"
				class="!w-44"
				name="download-provider"
				:options="providerOptions"
				:display-name="providerFilterLabel"
			/>
			<DropdownSelect
				v-if="tab === 'history'"
				v-model="historyStatus"
				class="!w-44"
				name="download-status"
				:options="historyStatusOptions"
				:display-name="historyStatusLabel"
			/>
			<ButtonStyled v-if="tab === 'history' && historyJobs.length" class="ml-auto" type="outlined">
				<button @click="clearHistoryModal?.show()">
					<TrashIcon />
					{{ formatMessage(messages.clearHistory) }}
				</button>
			</ButtonStyled>
		</div>

		<div
			v-if="visibleJobs.length || (tab === 'active' && legacyDownloads.length)"
			class="flex flex-col gap-3"
		>
			<Card
				v-for="bar in tab === 'active' ? legacyDownloads : []"
				:key="String(bar.loading_bar_uuid ?? bar.id)"
				class="!p-4"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex size-12 items-center justify-center rounded-xl bg-brand-highlight text-brand"
					>
						<DownloadIcon />
					</div>
					<div class="min-w-0 flex-grow">
						<div class="truncate font-semibold text-contrast">{{ bar.title || bar.message }}</div>
						<div class="truncate text-sm text-secondary">{{ bar.message }}</div>
					</div>
					<TagItem>
						<component :is="providerIcon(legacyProvider(bar))" />
						{{ providerLabel(legacyProvider(bar)) }}
					</TagItem>
					<Badge color="orange" :type="statusLabel('running')" />
				</div>
				<ProgressBar
					class="mt-4"
					full-width
					:progress="legacyPercent(bar)"
					:max="100"
					:label="formatMessage(messages.progress)"
					show-progress
				/>
			</Card>

			<Card v-for="job in visibleJobs" :key="job.job_id" class="!p-0">
				<div class="flex flex-wrap items-center gap-4 p-4">
					<img
						v-if="job.display?.icon"
						:src="displayIcon(job.display.icon)"
						alt=""
						class="size-12 rounded-xl object-cover"
					/>
					<div
						v-else
						class="flex size-12 items-center justify-center rounded-xl bg-brand-highlight text-brand"
					>
						<DownloadIcon />
					</div>
					<div class="min-w-48 flex-grow">
						<div class="flex flex-wrap items-center gap-2">
							<h2 class="m-0 truncate text-lg font-semibold text-contrast">
								{{ jobTitle(job) }}
							</h2>
							<Badge :color="statusColor(job.status)" :type="statusLabel(job.status)" />
						</div>
						<div class="mt-1 flex flex-wrap items-center gap-2 text-sm text-secondary">
							<component :is="providerIcon(job.provider)" class="size-3.5" />
							<span>{{ providerLabel(job.provider) }}</span>
							<BulletDivider />
							<span>{{ phaseLabel(job.phase) }}</span>
							<BulletDivider />
							<span>{{ formatDate(job.finished ?? job.modified) }}</span>
						</div>
					</div>
					<div class="flex flex-wrap items-center gap-2">
						<ButtonStyled v-if="canCancel(job)" color="red" type="outlined" size="small">
							<button :disabled="busy.has(job.job_id)" @click="cancel(job)">
								<XIcon />{{ formatMessage(messages.cancel) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="canRetry(job)" color="brand" size="small">
							<button :disabled="busy.has(job.job_id)" @click="retry(job)">
								<RefreshCwIcon />{{ formatMessage(messages.retry) }}
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="installJobInstanceId(job) && !job.instance_deleted"
							type="outlined"
							size="small"
						>
							<button
								@click="
									router.push(
										`/instance/${encodeURIComponent(installJobInstanceId(job)!)}`,
									)
								"
							>
								<ExternalIcon />{{ formatMessage(messages.openInstance) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="transparent" size="small">
							<button @click="toggleExpanded(job.job_id)">
								<ChevronDownIcon :class="expanded.has(job.job_id) ? 'rotate-180' : ''" />
								{{
									expanded.has(job.job_id)
										? formatMessage(messages.hideDetails)
										: formatMessage(messages.details)
								}}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="tab === 'history'" circular type="transparent" size="small">
							<button
								v-tooltip="formatMessage(messages.deleteRecord)"
								:aria-label="formatMessage(messages.deleteRecord)"
								:disabled="busy.has(job.job_id)"
								@click="remove(job)"
							>
								<TrashIcon />
							</button>
						</ButtonStyled>
					</div>
				</div>

				<div v-if="showProgress(job)" class="px-4 pb-4">
					<ProgressBar
						full-width
						:progress="jobPercent(job)"
						:max="100"
						:label="progressText(job)"
						:waiting="job.status === 'queued'"
						show-progress
					/>
				</div>

				<div
						v-if="expanded.has(job.job_id)"
						class="border-0 border-t border-solid border-divider p-4"
					>
						<Admonition
							v-if="job.error"
							class="mb-4"
							type="critical"
							:header="formatMessage(messages.errorDetails)"
						>
							{{ job.error.message }}
						</Admonition>

						<div v-if="job.items.length" class="flex flex-col gap-2">
							<Table
								:columns="itemColumns"
								:data="job.items"
								row-key="id"
								table-min-width="42rem"
								virtualized
								class="max-h-80 overflow-y-auto"
							>
								<template #cell-name="{ row }">
									<div class="min-w-0 py-2">
										<div class="truncate font-medium text-contrast">{{ row.name }}</div>
										<div
											v-if="row.project_id && row.version_id"
											class="truncate text-xs text-secondary"
										>
											{{
												formatMessage(messages.projectFile, {
													projectId: row.project_id,
													fileId: row.version_id,
												})
											}}
										</div>
										<div v-if="row.error" class="truncate text-xs text-red">
											{{ itemError(row) }}
										</div>
										<ButtonStyled v-if="row.manual_url" type="transparent" size="small">
											<button class="!px-0" @click.stop="openManualDownload(row)">
												<ExternalIcon />{{ formatMessage(messages.manualDownload) }}
											</button>
										</ButtonStyled>
									</div>
								</template>
								<template #cell-status="{ row }">
									<Badge :color="itemStatusColor(row.status)" :type="statusLabel(row.status)" />
								</template>
								<template #cell-progress="{ row }">
									<span>{{ itemProgress(row) }}</span>
								</template>
							</Table>
						</div>
						<EmptyState
							v-else
							type="no-documents"
							:heading="formatMessage(messages.noFileDetailsTitle)"
							:description="formatMessage(messages.noFileDetails)"
						/>
					</div>
			</Card>
		</div>

		<Card v-else>
			<EmptyState
				:type="query ? 'no-search-result' : 'no-tasks'"
				:heading="formatMessage(query ? messages.noResultsTitle : messages.emptyTitle)"
				:description="
					formatMessage(query ? messages.noResultsDescription : messages.emptyDescription)
				"
			/>
		</Card>
	</div>

	<ConfirmModal
		ref="clearHistoryModal"
		:danger="true"
		:markdown="false"
		:title="formatMessage(messages.clearHistoryTitle)"
		:description="formatMessage(messages.confirmClear)"
		:proceed-label="formatMessage(messages.clearHistory)"
		@proceed="clearHistory"
	/>
</template>

<script setup lang="ts">
import {
	ChevronDownIcon,
	ClockIcon,
	CurseForgeIcon,
	DownloadIcon,
	ExternalIcon,
	ModrinthIcon,
	RefreshCwIcon,
	SearchIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Badge,
	BulletDivider,
	ButtonStyled,
	Card,
	ConfirmModal,
	defineMessages,
	DropdownSelect,
	EmptyState,
	injectNotificationManager,
	NavTabs,
	ProgressBar,
	StyledInput,
	Table,
	type TableColumn,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
	import { openUrl } from '@tauri-apps/plugin-opener'
	import { computed, onMounted, ref } from 'vue'
	import { useRouter } from 'vue-router'

	import {
		installJobInstanceId,
		type InstallJobSnapshot,
		type InstallJobStatus,
		type InstallPhaseId,
	} from '@/helpers/install'
	import type { LoadingBar } from '@/helpers/state'
	import { injectDownloadManager } from '@/providers/download-manager'

type DownloadItem = InstallJobSnapshot['items'][number]

const manager = injectDownloadManager()
const router = useRouter()
const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

const tab = ref<'active' | 'history'>('active')
const query = ref('')
const provider = ref('all')
const historyStatus = ref('all')
const expanded = ref(new Set<string>())
const busy = ref(new Set<string>())
const refreshing = ref(false)
const clearHistoryModal = ref<InstanceType<typeof ConfirmModal>>()

const messages = defineMessages({
		inProgress: { id: 'app.downloads.in-progress', defaultMessage: 'In progress' },
		history: { id: 'app.downloads.history', defaultMessage: 'History' },
		refresh: { id: 'app.downloads.refresh', defaultMessage: 'Refresh' },
		search: { id: 'app.downloads.search', defaultMessage: 'Search downloads' },
	allSources: { id: 'app.downloads.all-sources', defaultMessage: 'All sources' },
	allStatuses: { id: 'app.downloads.all-statuses', defaultMessage: 'All statuses' },
	application: { id: 'app.downloads.application', defaultMessage: 'Application' },
	local: { id: 'app.downloads.local', defaultMessage: 'Local' },
	clearHistory: { id: 'app.downloads.clear-history', defaultMessage: 'Clear history' },
	clearHistoryTitle: {
		id: 'app.downloads.clear-history-title',
		defaultMessage: 'Clear download history?',
	},
	cancel: { id: 'app.downloads.cancel', defaultMessage: 'Cancel' },
	retry: { id: 'app.downloads.retry', defaultMessage: 'Retry' },
	openInstance: { id: 'app.downloads.open-instance', defaultMessage: 'Open instance' },
	details: { id: 'app.downloads.details', defaultMessage: 'Details' },
	hideDetails: { id: 'app.downloads.hide-details', defaultMessage: 'Hide details' },
	deleteRecord: { id: 'app.downloads.delete-record', defaultMessage: 'Delete record' },
	errorDetails: { id: 'app.downloads.error-details', defaultMessage: 'Download failed' },
	progress: { id: 'app.downloads.progress', defaultMessage: 'Progress' },
	noFileDetailsTitle: {
		id: 'app.downloads.no-file-details-title',
		defaultMessage: 'No file details',
	},
	noFileDetails: {
            id: 'app.downloads.no-file-details',
            defaultMessage: 'Files will appear here as they are downloaded.',
    },
	emptyTitle: { id: 'app.downloads.empty-title', defaultMessage: 'No downloads yet' },
	emptyDescription: {
		id: 'app.downloads.empty-description',
		defaultMessage: 'New downloads and installation progress will appear here.',
	},
	noResultsTitle: {
		id: 'app.downloads.no-results-title',
		defaultMessage: 'No matching downloads',
	},
	noResultsDescription: {
		id: 'app.downloads.no-results-description',
		defaultMessage: 'Try changing your search or filters.',
	},
	confirmClear: {
		id: 'app.downloads.confirm-clear',
		defaultMessage:
			'Completed, failed, interrupted, and canceled records will be permanently deleted.',
	},
	notAvailable: { id: 'app.downloads.not-available', defaultMessage: '\u2014' },
	itemName: { id: 'app.downloads.item-name', defaultMessage: 'File' },
	itemStatus: { id: 'app.downloads.item-status', defaultMessage: 'Status' },
	itemProgress: { id: 'app.downloads.item-progress', defaultMessage: 'Downloaded' },
	manualDownload: {
		id: 'app.curseforge.manual-downloads.open',
		defaultMessage: 'Open',
	},
	manualDownloadRequired: {
		id: 'app.downloads.manual-download-required',
		defaultMessage: 'CurseForge requires this file to be downloaded manually.',
	},
	projectFile: {
		id: 'app.curseforge.manual-downloads.project-file',
		defaultMessage: 'Project {projectId} \u00b7 File {fileId}',
	},
		curseforgeFileInstall: {
			id: 'app.downloads.curseforge-file-install',
			defaultMessage: 'CurseForge file install',
		},
	})

const statusMessages = defineMessages({
	queued: { id: 'app.downloads.status.queued', defaultMessage: 'Queued' },
	running: { id: 'app.downloads.status.running', defaultMessage: 'Running' },
	canceling: { id: 'app.downloads.status.canceling', defaultMessage: 'Canceling' },
	waiting_for_user: {
		id: 'app.downloads.status.waiting-for-user',
		defaultMessage: 'Action needed',
	},
	succeeded: { id: 'app.downloads.status.succeeded', defaultMessage: 'Completed' },
	failed: { id: 'app.downloads.status.failed', defaultMessage: 'Failed' },
	interrupted: { id: 'app.downloads.status.interrupted', defaultMessage: 'Interrupted' },
	canceled: { id: 'app.downloads.status.canceled', defaultMessage: 'Canceled' },
	completed: { id: 'app.downloads.item-status.completed', defaultMessage: 'Completed' },
	skipped: { id: 'app.downloads.item-status.skipped', defaultMessage: 'Skipped' },
	downloading: { id: 'app.downloads.item-status.downloading', defaultMessage: 'Downloading' },
	verifying: { id: 'app.downloads.item-status.verifying', defaultMessage: 'Verifying' },
	writing: { id: 'app.downloads.item-status.writing', defaultMessage: 'Writing' },
})

const phaseMessages = defineMessages({
	preparing_instance: {
		id: 'app.downloads.phase.preparing-instance',
		defaultMessage: 'Preparing instance',
	},
	resolving_pack: {
		id: 'app.downloads.phase.resolving-pack',
		defaultMessage: 'Resolving modpack',
	},
	downloading_pack_file: {
		id: 'app.downloads.phase.downloading-pack-file',
		defaultMessage: 'Downloading modpack',
	},
	reading_pack_manifest: {
		id: 'app.downloads.phase.reading-pack-manifest',
		defaultMessage: 'Reading manifest',
	},
	downloading_content: {
		id: 'app.downloads.phase.downloading-content',
		defaultMessage: 'Downloading content',
	},
	extracting_overrides: {
		id: 'app.downloads.phase.extracting-overrides',
		defaultMessage: 'Extracting overrides',
	},
	resolving_minecraft: {
		id: 'app.downloads.phase.resolving-minecraft',
		defaultMessage: 'Resolving Minecraft',
	},
	resolving_loader: {
		id: 'app.downloads.phase.resolving-loader',
		defaultMessage: 'Resolving loader',
	},
	preparing_java: {
		id: 'app.downloads.phase.preparing-java',
		defaultMessage: 'Preparing Java',
	},
	downloading_minecraft: {
		id: 'app.downloads.phase.downloading-minecraft',
		defaultMessage: 'Downloading Minecraft',
	},
	running_loader_processors: {
		id: 'app.downloads.phase.running-loader-processors',
		defaultMessage: 'Installing loader',
	},
	finalizing: { id: 'app.downloads.phase.finalizing', defaultMessage: 'Finalizing' },
	rolling_back: {
		id: 'app.downloads.phase.rolling-back',
		defaultMessage: 'Rolling back changes',
	},
})

const legacyDownloads = manager.legacyDownloads
const historyJobs = manager.historyJobs

const providerOptions = ['all', 'modrinth', 'curse_forge', 'minecraft', 'java', 'application', 'local']
const historyStatusOptions = ['all', 'succeeded', 'failed', 'interrupted', 'canceled']
const downloadTabs = computed(() => [
	{
		href: 'active',
		label: formatMessage(messages.inProgress),
		icon: DownloadIcon,
	},
	{ href: 'history', label: formatMessage(messages.history), icon: ClockIcon },
])
const itemColumns = computed<TableColumn[]>(() => [
	{ key: 'name', label: formatMessage(messages.itemName), width: '60%' },
	{ key: 'status', label: formatMessage(messages.itemStatus), width: '20%' },
	{ key: 'progress', label: formatMessage(messages.itemProgress), width: '20%', align: 'right' },
])
const sourceJobs = computed(() =>
	tab.value === 'active' ? manager.activeJobs.value : manager.historyJobs.value,
)
const visibleJobs = computed(() => {
	const normalized = query.value.trim().toLowerCase()
	return sourceJobs.value.filter((job) => {
		if (provider.value !== 'all' && job.provider !== provider.value) return false
		if (
			tab.value === 'history' &&
			historyStatus.value !== 'all' &&
			job.status !== historyStatus.value
		)
			return false
		return (
			!normalized ||
			jobTitle(job).toLowerCase().includes(normalized) ||
			job.job_id.includes(normalized)
		)
	})
})

function jobTitle(job: InstallJobSnapshot) {
			return (
				job.display?.title ||
				(job.details.type === 'instance' ? job.details.name : null) ||
				(job.details.type === 'modpack' ? job.details.title : null) ||
				(job.details.type === 'import' ? job.details.instance_folder : null) ||
				(job.kind === 'install_curseforge_file'
					? formatMessage(messages.curseforgeFileInstall)
					: null) ||
				job.job_id
			)
		}

function displayIcon(icon: string) {
	return /^(https?:|data:|blob:|asset:|tauri:)/.test(icon) ? icon : convertFileSrc(icon)
}

function providerLabel(value: InstallJobSnapshot['provider']) {
	return (
		{
			modrinth: 'Modrinth',
			curse_forge: 'CurseForge',
			minecraft: 'Minecraft',
			java: 'Java',
			application: formatMessage(messages.application),
			local: formatMessage(messages.local),
		} as Record<string, string>
	)[value]
}

function providerFilterLabel(value: string) {
	return value === 'all'
		? formatMessage(messages.allSources)
		: providerLabel(value as InstallJobSnapshot['provider'])
}

function providerIcon(value: InstallJobSnapshot['provider']) {
	return value === 'curse_forge' ? CurseForgeIcon : value === 'modrinth' ? ModrinthIcon : DownloadIcon
}

function legacyProvider(bar: LoadingBar): InstallJobSnapshot['provider'] {
	if (bar.bar_type?.type === 'pack_download') return 'curse_forge'
	if (bar.bar_type?.type === 'minecraft_download') return 'minecraft'
	if (bar.bar_type?.type === 'java_download') return 'java'
	if (bar.bar_type?.type === 'launcher_update') return 'application'
	return 'local'
}

function historyStatusLabel(value: string) {
	return value === 'all' ? formatMessage(messages.allStatuses) : statusLabel(value)
}

function statusLabel(status: string) {
	return status in statusMessages
		? formatMessage(statusMessages[status as keyof typeof statusMessages])
		: status
}

function phaseLabel(phase: InstallPhaseId) {
	return formatMessage(phaseMessages[phase])
}

function statusColor(status: InstallJobStatus): 'green' | 'red' | 'orange' | 'blue' | 'gray' {
	if (status === 'succeeded') return 'green'
	if (status === 'failed' || status === 'interrupted' || status === 'canceled') return 'red'
	if (status === 'running' || status === 'waiting_for_user' || status === 'canceling')
		return 'orange'
	return 'blue'
}

function itemStatusColor(
	status: DownloadItem['status'],
): 'green' | 'red' | 'orange' | 'blue' | 'gray' {
	if (status === 'completed') return 'green'
	if (status === 'failed' || status === 'canceled') return 'red'
	if (status === 'waiting_for_user') return 'orange'
	if (status === 'skipped') return 'gray'
	return 'blue'
}

function canCancel(job: InstallJobSnapshot) {
    return job.status === 'queued' || job.status === 'running' || job.status === 'paused'
}

function canRetry(job: InstallJobSnapshot) {
	return job.status === 'failed' || job.status === 'interrupted' || job.status === 'canceled'
}

function showProgress(job: InstallJobSnapshot) {
	return ['queued', 'running', 'canceling'].includes(job.status)
}

function jobPercent(job: InstallJobSnapshot) {
	const progress = job.progress?.secondary ?? job.progress
	if (!progress?.total) return job.status === 'succeeded' ? 100 : 0
	return Math.min(100, Math.max(0, Math.round((progress.current / progress.total) * 100)))
}

function progressText(job: InstallJobSnapshot) {
		const summary = job.summary ?? {}
		if (summary.bytes_total)
			return `${formatBytes(summary.bytes_downloaded)} / ${formatBytes(summary.bytes_total)}`
		if (summary.files_total) return `${summary.files_completed} / ${summary.files_total}`
		return phaseLabel(job.phase)
	}

function itemProgress(item: DownloadItem) {
	if (!item.bytes_total) return formatMessage(messages.notAvailable)
	return `${formatBytes(item.bytes_downloaded)} / ${formatBytes(item.bytes_total)}`
}

function itemError(item: DownloadItem) {
	if (item.error?.includes('requires manual download')) {
		return formatMessage(messages.manualDownloadRequired)
	}
	return item.error ?? ''
}

async function openManualDownload(item: DownloadItem) {
	if (item.manual_url) await openUrl(item.manual_url)
}

function legacyPercent(bar: LoadingBar) {
	if (!bar.total) return 0
	return Math.min(100, Math.max(0, Math.round(((bar.current ?? 0) / bar.total) * 100)))
}

function formatDate(value: string) {
	return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
		new Date(value),
	)
}

function selectTab(index: number) {
	tab.value = index === 0 ? 'active' : 'history'
}

function toggleExpanded(jobId: string) {
	const next = new Set(expanded.value)
	if (next.has(jobId)) {
		next.delete(jobId)
	} else {
		next.add(jobId)
	}
	expanded.value = next
}

async function withBusy(jobId: string, action: () => Promise<void>) {
	busy.value = new Set([...busy.value, jobId])
	try {
		await action()
	} catch (error) {
		handleError(error)
	} finally {
		const next = new Set(busy.value)
		next.delete(jobId)
		busy.value = next
	}
}

async function cancel(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.cancel(job.job_id))
}

async function retry(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.retry(job.job_id))
}

async function remove(job: InstallJobSnapshot) {
	await withBusy(job.job_id, () => manager.remove(job.job_id))
}

async function clearHistory() {
	try {
		await manager.clearHistory()
	} catch (error) {
		handleError(error)
	}
}

async function refreshDownloads() {
	if (refreshing.value) return
	refreshing.value = true
	try {
		await manager.refresh()
	} catch (error) {
		handleError(error)
	} finally {
		refreshing.value = false
	}
}

onMounted(() => {
		// Ensure the manager is started (no-op if already running from App.vue).
		// Also trigger a refresh to catch any jobs that started while we were away.
		void manager.refresh()
	})
</script>
