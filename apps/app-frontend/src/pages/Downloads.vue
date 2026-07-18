<script setup lang="ts">
import {
	PauseIcon,
	PlayIcon,
	RefreshCwIcon,
	UpdatedIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	Badge,
	ButtonStyled,
	Card,
	ProgressBar,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { install_job_listener } from '@/helpers/events'
import {
	install_job_cancel,
	install_job_dismiss,
	install_job_list,
	install_job_pause,
	install_job_resume,
	install_job_retry,
	installJobInstanceId,
	isInstallJobFinished,
	type InstallJobSnapshot,
	type InstallJobStatus,
	type InstallPhaseId,
} from '@/helpers/install'
import { get_many as getInstances } from '@/helpers/instance'
import { useBreadcrumbs } from '@/store/breadcrumbs'

dayjs.extend(relativeTime)

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const breadcrumbs = useBreadcrumbs()

const messages = defineMessages({
	title: {
		id: 'app.downloads.title',
		defaultMessage: 'Downloads',
	},
	noDownloads: {
		id: 'app.downloads.no-downloads',
		defaultMessage: 'No downloads',
	},
	noDownloadsDescription: {
		id: 'app.downloads.no-downloads-description',
		defaultMessage: 'There are no active or recent downloads.',
	},
	refresh: {
		id: 'app.downloads.refresh',
		defaultMessage: 'Refresh',
	},
	tabActive: {
		id: 'app.downloads.tab.active',
		defaultMessage: 'Active',
	},
	tabFinished: {
		id: 'app.downloads.tab.finished',
		defaultMessage: 'Finished',
	},
	tabActiveCount: {
		id: 'app.downloads.tab.active-count',
		defaultMessage: 'Active ({count})',
	},
	tabFinishedCount: {
		id: 'app.downloads.tab.finished-count',
		defaultMessage: 'Finished ({count})',
	},
	cancel: {
		id: 'app.downloads.cancel',
		defaultMessage: 'Cancel',
	},
	pause: {
		id: 'app.downloads.pause',
		defaultMessage: 'Pause',
	},
	resume: {
		id: 'app.downloads.resume',
		defaultMessage: 'Resume',
	},
	retry: {
		id: 'app.downloads.retry',
		defaultMessage: 'Retry',
	},
	dismiss: {
		id: 'app.downloads.dismiss',
		defaultMessage: 'Dismiss',
	},
	created: {
		id: 'app.downloads.created',
		defaultMessage: 'Created',
	},
	updated: {
		id: 'app.downloads.updated',
		defaultMessage: 'Updated',
	},
	error: {
		id: 'app.downloads.error',
		defaultMessage: 'Error',
	},
	unknownJob: {
		id: 'app.downloads.unknown-job',
		defaultMessage: 'Install job',
	},
})

breadcrumbs.setRootContext({ name: formatMessage(messages.title), link: '/downloads' })

// 阶段描述的 i18n 消息映射
const phaseMessages = defineMessages({
	preparing_instance: {
		id: 'app.install.phase.preparing_instance',
		defaultMessage: 'Preparing instance',
	},
	resolving_pack: {
		id: 'app.install.phase.resolving_pack',
		defaultMessage: 'Resolving pack',
	},
	downloading_pack_file: {
		id: 'app.install.phase.downloading_pack_file',
		defaultMessage: 'Downloading pack',
	},
	reading_pack_manifest: {
		id: 'app.install.phase.reading_pack_manifest',
		defaultMessage: 'Reading manifest',
	},
	downloading_content: {
		id: 'app.install.phase.downloading_content',
		defaultMessage: 'Downloading content',
	},
	extracting_overrides: {
		id: 'app.install.phase.extracting_overrides',
		defaultMessage: 'Extracting files',
	},
	resolving_minecraft: {
		id: 'app.install.phase.resolving_minecraft',
		defaultMessage: 'Resolving Minecraft',
	},
	resolving_loader: {
		id: 'app.install.phase.resolving_loader',
		defaultMessage: 'Resolving loader',
	},
	preparing_java: {
		id: 'app.install.phase.preparing_java',
		defaultMessage: 'Preparing Java',
	},
	downloading_minecraft: {
		id: 'app.install.phase.downloading_minecraft',
		defaultMessage: 'Downloading Minecraft',
	},
	running_loader_processors: {
		id: 'app.install.phase.running_loader_processors',
		defaultMessage: 'Running processors',
	},
	finalizing: {
		id: 'app.install.phase.finalizing',
		defaultMessage: 'Finalizing',
	},
	rolling_back: {
		id: 'app.install.phase.rolling_back',
		defaultMessage: 'Rolling back',
	},
})

// 状态标签的 i18n 消息
const statusMessages = defineMessages({
	queued: {
		id: 'app.downloads.status.queued',
		defaultMessage: 'Queued',
	},
	running: {
		id: 'app.downloads.status.running',
		defaultMessage: 'Running',
	},
	paused: {
		id: 'app.downloads.status.paused',
		defaultMessage: 'Paused',
	},
	succeeded: {
		id: 'app.downloads.status.succeeded',
		defaultMessage: 'Succeeded',
	},
	failed: {
		id: 'app.downloads.status.failed',
		defaultMessage: 'Failed',
	},
	interrupted: {
		id: 'app.downloads.status.interrupted',
		defaultMessage: 'Interrupted',
	},
	canceled: {
		id: 'app.downloads.status.canceled',
		defaultMessage: 'Canceled',
	},
})

// 状态对应的 Badge 颜色和类型
const statusColorMap: Record<InstallJobStatus, string> = {
	queued: 'yellow',
	running: 'brand',
	paused: 'orange',
	succeeded: 'green',
	failed: 'red',
	interrupted: 'gray',
	canceled: 'gray',
}
const statusTypeMap: Record<InstallJobStatus, string> = {
	queued: 'queued',
	running: 'running',
	paused: 'paused',
	succeeded: 'succeeded',
	failed: 'failed',
	interrupted: 'interrupted',
	canceled: 'canceled',
}

const jobs = ref<InstallJobSnapshot[]>([])
const instanceNames = ref<Record<string, string>>({})
const loading = ref(false)
const route = useRoute()
const router = useRouter()

const activeTab = computed<'active' | 'finished'>({
	get() {
		return route.query.tab === 'finished' ? 'finished' : 'active'
	},
	set(value) {
		router.replace({ query: { ...route.query, tab: value } })
	},
})

function getJobTitle(job: InstallJobSnapshot): string {
	if (job.display?.title) return job.display.title
	if (job.details.type === 'instance') return job.details.name
	if (job.details.type === 'modpack' && job.details.title) return job.details.title
	const instanceId = installJobInstanceId(job)
	return (instanceId ? instanceNames.value[instanceId] : null) ?? formatMessage(messages.unknownJob)
}

function getDisplayIconUrl(icon: string | null | undefined): string | null {
	if (!icon) return null
	if (/^(https?:|data:|blob:|asset:|tauri:)/.test(icon)) return icon
	return convertFileSrc(icon)
}

function getJobIconUrl(job: InstallJobSnapshot): string | null {
	return getDisplayIconUrl(job.display?.icon)
}

function getPhaseLabel(phase: InstallPhaseId): string {
	const messageDescriptor = phaseMessages[phase]
	if (messageDescriptor) return formatMessage(messageDescriptor)
	return phase.replace(/_/g, ' ')
}

function getStatusLabel(status: InstallJobStatus): string {
	return formatMessage(statusMessages[status])
}

function getProgressFraction(job: InstallJobSnapshot): number | null {
	if (job.status === 'succeeded') return 1
	if (!job.progress || !job.progress.total || job.progress.total <= 0) return null
	return Math.max(0, Math.min(1, job.progress.current / job.progress.total))
}

function formatRelativeTime(dateStr: string): string {
	if (!dateStr) return '—'
	return dayjs(dateStr).fromNow()
}

const activeJobs = computed(() => {
	const active = jobs.value.filter((j) => !isInstallJobFinished(j.status))
	active.sort((a, b) => a.created.localeCompare(b.created))
	return active
})

const finishedJobs = computed(() => {
	const finished = jobs.value.filter((j) => isInstallJobFinished(j.status))
	finished.sort((a, b) => b.modified.localeCompare(a.modified))
	return finished
})

const currentJobs = computed(() => (activeTab.value === 'active' ? activeJobs.value : finishedJobs.value))

const hasCurrentJobs = computed(() => currentJobs.value.length > 0)

async function refreshMetadata() {
	const instanceIds = Array.from(
		new Set(
			jobs.value
				.map((job) => installJobInstanceId(job))
				.filter((id): id is string => !!id),
		),
	)

	if (instanceIds.length === 0) {
		instanceNames.value = {}
		return
	}

	const instances = await getInstances(instanceIds).catch((err) => {
		handleError(err)
		return []
	})

	instanceNames.value = Object.fromEntries(instances.map((inst) => [inst.id, inst.name]))
}

async function refresh() {
	loading.value = true
	try {
		const allJobs = await install_job_list(true).catch((err) => {
			handleError(err)
			return [] as InstallJobSnapshot[]
		})
		jobs.value = allJobs
		await refreshMetadata()
	} finally {
		loading.value = false
	}
}

// 操作处理
async function cancelJob(jobId: string) {
	try {
		await install_job_cancel(jobId)
	} catch (err: unknown) {
		handleError(err	)
	}
	await refresh()
}

async function pauseJob(jobId: string) {
	try {
		await install_job_pause(jobId)
	} catch (err: unknown) {
		handleError(err)
	}
	await refresh()
}

async function resumeJob(jobId: string) {
	try {
		await install_job_resume(jobId)
	} catch (err: unknown) {
		handleError(err)
	}
	await refresh()
}

async function retryJob(jobId: string) {
	try {
		await install_job_retry(jobId)
	} catch (err: unknown) {
		handleError(err)
	}
	await refresh()
}

async function dismissJob(jobId: string) {
	try {
		await install_job_dismiss(jobId)
	} catch (err: unknown) {
		handleError(err)
	}
	await refresh()
}

// 监听 install_job 事件，实时更新
const unlisten = await install_job_listener((updatedJob: InstallJobSnapshot) => {
	const existingIndex = jobs.value.findIndex((j) => j.job_id === updatedJob.job_id)
	if (existingIndex >= 0) {
		const existing = jobs.value[existingIndex]
		// 跳过过时的事件（modified 字段更早的更新）
		if (existing.modified.localeCompare(updatedJob.modified) > 0) return
		jobs.value = [...jobs.value.slice(0, existingIndex), updatedJob, ...jobs.value.slice(existingIndex + 1)]
	} else {
		jobs.value = [...jobs.value, updatedJob]
	}
	void refreshMetadata()
})

onUnmounted(() => {
	unlisten()
})

// 初始加载
await refresh()
</script>

<template>
	<div class="p-6 flex flex-col gap-4">
		<!-- 标题栏 -->
		<div class="flex items-center justify-between">
			<h1 class="m-0 text-2xl font-extrabold">
				{{ formatMessage(messages.title) }}
			</h1>
			<ButtonStyled color="brand">
				<button :disabled="loading" @click="refresh">
					<RefreshCwIcon :class="{ 'animate-spin': loading }" />
					{{ formatMessage(messages.refresh) }}
				</button>
			</ButtonStyled>
		</div>

		<!-- Tab switcher -->
		<div v-if="jobs.length > 0" class="tab-bar">
			<button
				class="tab-button"
				:class="{ 'tab-button--active': activeTab === 'active' }"
				@click="activeTab = 'active'"
			>
				{{ formatMessage(messages.tabActiveCount, { count: activeJobs.length }) }}
			</button>
			<button
				class="tab-button"
				:class="{ 'tab-button--active': activeTab === 'finished' }"
				@click="activeTab = 'finished'"
			>
				{{ formatMessage(messages.tabFinishedCount, { count: finishedJobs.length }) }}
			</button>
		</div>

		<!-- 任务列表 -->
		<template v-if="hasCurrentJobs">
			<Card v-for="job in currentJobs" :key="job.job_id" class="job-card">
				<div class="job-header">
					<div class="job-header-left">
						<Avatar
							v-if="getJobIconUrl(job)"
							:src="getJobIconUrl(job) ?? undefined"
							:alt="getJobTitle(job)"
							size="40px"
						/>
						<div v-else class="job-icon-placeholder">
							<Avatar size="40px" />
						</div>
						<div class="job-title-group">
							<span class="job-title">{{ getJobTitle(job) }}</span>
							<Badge :type="statusTypeMap[job.status]" :color="statusColorMap[job.status]">
								{{ getStatusLabel(job.status) }}
							</Badge>
						</div>
					</div>
				</div>

				<!-- 当前阶段：仅活跃任务显示 -->
				<div v-if="!isInstallJobFinished(job.status)" class="job-phase">
					{{ getPhaseLabel(job.phase) }}
				</div>

				<!-- 进度条：仅活跃任务显示 -->
				<div
					v-if="
						!isInstallJobFinished(job.status) &&
						(getProgressFraction(job) !== null || job.status === 'running' || job.status === 'paused')
					"
					class="job-progress"
				>
					<ProgressBar
						:progress="getProgressFraction(job) ?? 0"
						:show-progress="getProgressFraction(job) !== null"
					/>
					<span v-if="job.progress && job.progress.total > 0" class="job-progress-text">
						{{ Math.round((job.progress.current / job.progress.total) * 100) }}%
					</span>
				</div>

				<!-- 错误信息 -->
				<div v-if="job.error" class="job-error">
					<strong>{{ formatMessage(messages.error) }}:</strong> {{ job.error.message }}
				</div>

				<!-- 时间戳 -->
				<div class="job-timestamps">
					<span class="job-timestamp" :title="job.created">
						{{ formatMessage(messages.created) }} {{ formatRelativeTime(job.created) }}
					</span>
					<span class="job-timestamp" :title="job.modified">
						{{ formatMessage(messages.updated) }} {{ formatRelativeTime(job.modified) }}
					</span>
				</div>

				<!-- 操作按钮 -->
				<div class="job-actions">
					<!-- Queued: Cancel -->
					<template v-if="job.status === 'queued'">
						<ButtonStyled color="red" type="outlined">
							<button @click="cancelJob(job.job_id)">
								<XIcon />
								{{ formatMessage(messages.cancel) }}
							</button>
						</ButtonStyled>
					</template>

					<!-- Running: Pause, Cancel -->
					<template v-if="job.status === 'running'">
						<ButtonStyled color="orange" type="outlined">
							<button @click="pauseJob(job.job_id)">
								<PauseIcon />
								{{ formatMessage(messages.pause) }}
							</button>
						</ButtonStyled>
						<ButtonStyled color="red" type="outlined">
							<button @click="cancelJob(job.job_id)">
								<XIcon />
								{{ formatMessage(messages.cancel) }}
							</button>
						</ButtonStyled>
					</template>

					<!-- Paused: Resume, Cancel -->
					<template v-if="job.status === 'paused'">
						<ButtonStyled color="brand" type="outlined">
							<button @click="resumeJob(job.job_id)">
								<PlayIcon />
								{{ formatMessage(messages.resume) }}
							</button>
						</ButtonStyled>
						<ButtonStyled color="red" type="outlined">
							<button @click="cancelJob(job.job_id)">
								<XIcon />
								{{ formatMessage(messages.cancel) }}
							</button>
						</ButtonStyled>
					</template>

					<!-- Failed/Interrupted: Retry, Dismiss -->
					<template v-if="job.status === 'failed' || job.status === 'interrupted'">
						<ButtonStyled color="brand" type="outlined">
							<button @click="retryJob(job.job_id)">
								<UpdatedIcon />
								{{ formatMessage(messages.retry) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="outlined">
							<button @click="dismissJob(job.job_id)">
								<XIcon />
								{{ formatMessage(messages.dismiss) }}
							</button>
						</ButtonStyled>
					</template>

					<!-- Succeeded: Dismiss -->
					<template v-if="job.status === 'succeeded'">
						<ButtonStyled type="outlined">
							<button @click="dismissJob(job.job_id)">
								<XIcon />
								{{ formatMessage(messages.dismiss) }}
							</button>
						</ButtonStyled>
					</template>

					<!-- Canceled: Dismiss -->
					<template v-if="job.status === 'canceled'">
						<ButtonStyled type="outlined">
							<button @click="dismissJob(job.job_id)">
								<XIcon />
								{{ formatMessage(messages.dismiss) }}
							</button>
						</ButtonStyled>
					</template>
				</div>
			</Card>
		</template>

		<!-- 空状态 -->
		<div v-else class="empty-state">
			<h2 class="m-0 text-xl font-bold text-contrast">
				{{ formatMessage(messages.noDownloads) }}
			</h2>
			<p class="m-0 text-secondary">
				{{ formatMessage(messages.noDownloadsDescription) }}
			</p>
		</div>
	</div>
</template>

<style lang="scss" scoped>
.tab-bar {
	display: flex;
	gap: var(--gap-sm, 0.5rem);
	border-bottom: 2px solid var(--color-button-bg);
}

.tab-button {
	position: relative;
	padding: 0.75rem 1rem;
	background: transparent;
	border: none;
	color: var(--color-text);
	font-size: 0.9375rem;
	font-weight: 600;
	cursor: pointer;
	transition: color 0.15s ease;

	&:hover {
		color: var(--color-contrast);
	}

	&--active {
		color: var(--color-contrast);

		&::after {
			content: '';
			position: absolute;
			bottom: -2px;
			left: 0;
			right: 0;
			height: 2px;
			background: var(--color-brand);
			border-radius: 2px 2px 0 0;
		}
	}
}

.job-card {
	display: flex;
	flex-direction: column;
	gap: var(--gap-sm, 0.5rem);
}

.job-header {
	display: flex;
	align-items: center;
	justify-content: space-between;
}

.job-header-left {
	display: flex;
	align-items: center;
	gap: var(--gap-md, 0.75rem);
}

.job-title-group {
	display: flex;
	align-items: center;
	gap: var(--gap-sm, 0.5rem);
	flex-wrap: wrap;
}

.job-title {
	font-size: 1rem;
	font-weight: 700;
	color: var(--color-contrast);
}

.job-phase {
	font-size: 0.875rem;
	color: var(--color-secondary);
	font-weight: 500;
}

.job-progress {
	display: flex;
	align-items: center;
	gap: var(--gap-sm, 0.5rem);
}

.job-progress-text {
	font-size: 0.75rem;
	font-weight: 600;
	color: var(--color-secondary);
	white-space: nowrap;
}

.job-error {
	font-size: 0.875rem;
	color: var(--color-red);
	background: rgba(255, 0, 0, 0.06);
	border-radius: 0.375rem;
	padding: 0.5rem 0.75rem;
	border: 1px solid rgba(255, 0, 0, 0.15);
}

.job-timestamps {
	display: flex;
	gap: var(--gap-md, 0.75rem);
	flex-wrap: wrap;
}

.job-timestamp {
	font-size: 0.75rem;
	color: var(--color-secondary);
}

.job-actions {
	display: flex;
	gap: var(--gap-sm, 0.5rem);
	flex-wrap: wrap;
	padding-top: 0.25rem;
}

.empty-state {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	gap: var(--gap-sm, 0.5rem);
	padding: 3rem 1rem;
	text-align: center;
}
</style>
