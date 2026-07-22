import { createContext } from '@modrinth/ui'
import { computed, type ComputedRef, type Ref, ref } from 'vue'

import { install_job_listener, loading_listener } from '@/helpers/events'
import {
	download_history_clear,
	download_job_cancel,
	download_job_delete,
	download_job_list,
	download_job_retry,
	install_job_list,
	type InstallJobSnapshot,
} from '@/helpers/install'
import type { LoadingBar } from '@/helpers/state'
import { progress_bars_list } from '@/helpers/state'

const activeStatuses = new Set(['queued', 'running', 'paused', 'canceling', 'waiting_for_user'])
const downloadBarTypes = new Set([
	'java_download',
	'pack_file_download',
	'pack_download',
	'minecraft_download',
	'instance_update',
	'launcher_update',
])

export interface DownloadManager {
	jobs: Ref<InstallJobSnapshot[]>
	legacyDownloads: Ref<LoadingBar[]>
	activeJobs: ComputedRef<InstallJobSnapshot[]>
	historyJobs: ComputedRef<InstallJobSnapshot[]>
	activeCount: ComputedRef<number>
	queuedCount: ComputedRef<number>
	start: () => Promise<void>
	refresh: () => Promise<void>
	cancel: (jobId: string) => Promise<void>
	retry: (jobId: string) => Promise<void>
	remove: (jobId: string) => Promise<void>
	clearHistory: () => Promise<void>
	dispose: () => void
}

export function createDownloadManager(handleError: (error: unknown) => void): DownloadManager {
	const jobs = ref<InstallJobSnapshot[]>([])
	const legacyDownloads = ref<LoadingBar[]>([])
	let started = false
	let disposed = false
	let unlistenJobs: (() => void) | null = null
	let unlistenLoading: (() => void) | null = null
	let pollInterval: ReturnType<typeof setInterval> | null = null
	let installJobListFailedLogged = false

	function setJob(job: InstallJobSnapshot) {
		const current = jobs.value.find((candidate) => candidate.job_id === job.job_id)
		if (current && current.modified.localeCompare(job.modified) > 0) return
		jobs.value = [job, ...jobs.value.filter((candidate) => candidate.job_id !== job.job_id)].sort(
			(a, b) => b.created.localeCompare(a.created),
		)
	}

	async function refresh() {
		let jobsList: InstallJobSnapshot[] = []

		// Try the new paginated API first; fall back to the legacy list if it
		// isn't available (e.g. backend binary hasn't been rebuilt yet).
		const page = await download_job_list({ limit: 250 }).catch((error) => {
			console.warn('[download-manager] download_job_list failed:', error)
			return null
		})

		if (page) {
			console.log('[download-manager] download_job_list returned', page.jobs.length, 'jobs')
			jobsList = page.jobs
		} else {
			console.warn('[download-manager] falling back to install_job_list')
			const fallbackJobs = await install_job_list(true).catch((error) => {
				console.error('[download-manager] install_job_list failed:', error)
				if (!installJobListFailedLogged) {
					installJobListFailedLogged = true
					handleError(error)
				}
				return []
			})
			console.log('[download-manager] install_job_list returned', fallbackJobs.length, 'jobs')
			jobsList = fallbackJobs
		}

		if (!disposed) {
			jobs.value = jobsList
		}
	}

	async function refreshLegacyDownloads() {
		const bars = await progress_bars_list().catch((error) => {
			console.warn('[download-manager] progress_bars_list failed:', error)
			return {}
		})
		const values = Object.values(bars)
		console.log('[download-manager] progress bars:', values.length, values.map((b) => b.bar_type?.type))
		legacyDownloads.value = values
			.filter((bar) => downloadBarTypes.has(bar.bar_type?.type ?? ''))
			.map((bar) => ({
				...bar,
				title: bar.title ?? bar.bar_type?.pack_name ?? bar.bar_type?.instance_name ?? bar.message,
			}))
	}

	async function start() {
		if (started || disposed) return
		started = true
		console.log('[download-manager] starting')
		await refresh()
		await refreshLegacyDownloads()
		unlistenJobs = await install_job_listener((job: InstallJobSnapshot) => {
			console.log('[download-manager] install_job event:', job.job_id, job.status, job.phase)
			setJob(job)
		})
		unlistenLoading = await loading_listener(() => void refreshLegacyDownloads())
		pollInterval = setInterval(() => {
			if (!disposed) {
				void refresh()
				void refreshLegacyDownloads()
			}
		}, 3000)
		console.log('[download-manager] listeners attached, polling every 3s')
	}

	async function cancel(jobId: string) {
		setJob(await download_job_cancel(jobId))
	}

	async function retry(jobId: string) {
		setJob(await download_job_retry(jobId))
	}

	async function remove(jobId: string) {
		await download_job_delete(jobId)
		jobs.value = jobs.value.filter((job) => job.job_id !== jobId)
	}

	async function clearHistory() {
		await download_history_clear()
		jobs.value = jobs.value.filter((job) => activeStatuses.has(job.status))
	}

	const activeJobs = computed(() => jobs.value.filter((job) => activeStatuses.has(job.status)))
	const historyJobs = computed(() => jobs.value.filter((job) => !activeStatuses.has(job.status)))

	return {
		jobs,
		legacyDownloads,
		activeJobs,
		historyJobs,
		activeCount: computed(() => activeJobs.value.length + legacyDownloads.value.length),
		queuedCount: computed(() => jobs.value.filter((job) => job.status === 'queued').length),
		start,
		refresh,
		cancel,
		retry,
		remove,
		clearHistory,
		dispose() {
			disposed = true
			unlistenJobs?.()
			unlistenLoading?.()
			if (pollInterval) {
				clearInterval(pollInterval)
				pollInterval = null
			}
		},
	}
}

export const [injectDownloadManager, provideDownloadManager] = createContext<DownloadManager>(
	'root',
	'downloadManager',
)
