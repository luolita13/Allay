<script setup lang="ts">
import {
	ClipboardCopyIcon,
	DownloadIcon,
	FolderOpenIcon,
	RefreshCwIcon,
	SearchIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import { Admonition, ButtonStyled, NewModal as Modal } from '@modrinth/ui'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

import {
	type LauncherSessionLogInfo,
	type ParsedLogLine,
	clearAllLauncherLogs,
	deleteLauncherLogSession,
	listLauncherLogSessions,
	parseLogLine,
	readLauncherLogSession,
	readLauncherLogCurrentTail,
} from '@/helpers/launcher_logs'
import { showLauncherLogsFolder } from '@/helpers/utils'

const modal = ref<InstanceType<typeof Modal> | null>(null)

const sessions = ref<LauncherSessionLogInfo[]>([])
const selectedFilename = ref<string | null>(null)
const rawContent = ref<string>('')
const loading = ref(false)
const errorMessage = ref<string | null>(null)
const truncated = ref(false)
const totalSizeBytes = ref(0)

const searchQuery = ref('')
const levelFilters = ref({
	error: true,
	warn: true,
	info: true,
	debug: true,
	trace: true,
	other: true,
})

const autoRefresh = ref(true)
const autoScroll = ref(true)
let refreshTimer: ReturnType<typeof setInterval> | null = null
const REFRESH_INTERVAL_MS = 2000
const MAX_BYTES = 256 * 1024

defineExpose({
	show: async () => {
		modal.value?.show()
		errorMessage.value = null
		await refreshSessions()
		const current = sessions.value.find((s) => s.isCurrent)
		if (current) {
			await selectSession(current.filename)
		} else if (sessions.value.length > 0) {
			await selectSession(sessions.value[0].filename)
		}
	},
	hide: () => {
		stopAutoRefresh()
		modal.value?.hide()
	},
})

async function refreshSessions() {
	loading.value = true
	errorMessage.value = null
	try {
		sessions.value = await listLauncherLogSessions()
	} catch (e) {
		errorMessage.value = String(e)
	} finally {
		loading.value = false
	}
}

async function selectSession(filename: string) {
	if (selectedFilename.value === filename && rawContent.value) return
	selectedFilename.value = filename
	stopAutoRefresh()
	await loadSessionContent(filename)
	const info = sessions.value.find((s) => s.filename === filename)
	if (info?.isCurrent && autoRefresh.value) {
		startAutoRefresh()
	}
}

async function loadSessionContent(filename: string, tailOnly = true) {
	loading.value = true
	errorMessage.value = null
	try {
		const result = await readLauncherLogSession(filename, tailOnly ? MAX_BYTES : undefined)
		rawContent.value = result.content
		truncated.value = result.truncated
		totalSizeBytes.value = result.totalSizeBytes
		await nextTick()
		if (autoScroll.value) scrollToBottom()
	} catch (e) {
		errorMessage.value = String(e)
	} finally {
		loading.value = false
	}
}

async function loadFullSession() {
	if (!selectedFilename.value) return
	try {
		const result = await readLauncherLogSession(selectedFilename.value, undefined)
		rawContent.value = result.content
		truncated.value = false
		totalSizeBytes.value = result.totalSizeBytes
		await nextTick()
		if (autoScroll.value) scrollToBottom()
	} catch (e) {
		errorMessage.value = String(e)
	}
}

function startAutoRefresh() {
	stopAutoRefresh()
	refreshTimer = setInterval(async () => {
		if (!selectedFilename.value || loading.value) return
		try {
			const info = sessions.value.find((s) => s.filename === selectedFilename.value)
			if (!info?.isCurrent) {
				stopAutoRefresh()
				return
			}
			const result = await readLauncherLogCurrentTail(MAX_BYTES)
			if (result.content !== rawContent.value) {
				rawContent.value = result.content
				truncated.value = result.truncated
				totalSizeBytes.value = result.totalSizeBytes
				await nextTick()
				if (autoScroll.value) scrollToBottom()
			}
		} catch (e) {
			console.error('Auto-refresh failed:', e)
		}
	}, REFRESH_INTERVAL_MS)
}

function stopAutoRefresh() {
	if (refreshTimer) {
		clearInterval(refreshTimer)
		refreshTimer = null
	}
}

watch(autoRefresh, (on) => {
	const info = sessions.value.find((s) => s.filename === selectedFilename.value)
	if (on && info?.isCurrent) {
		startAutoRefresh()
	} else {
		stopAutoRefresh()
	}
})

onBeforeUnmount(() => {
	stopAutoRefresh()
})

const parsedLines = computed<ParsedLogLine[]>(() => {
	if (!rawContent.value) return []
	return rawContent.value.split('\n').map(parseLogLine)
})

const filteredLines = computed<ParsedLogLine[]>(() => {
	const q = searchQuery.value.trim().toLowerCase()
	return parsedLines.value.filter((line) => {
		if (!levelFilters.value[line.level]) return false
		if (q && !line.raw.toLowerCase().includes(q)) return false
		return true
	})
})

async function copyToClipboard() {
	try {
		await navigator.clipboard.writeText(rawContent.value)
	} catch (e) {
		errorMessage.value = `Failed to copy: ${e}`
	}
}

async function exportLog() {
	if (!selectedFilename.value) return
	try {
		const blob = new Blob([rawContent.value], { type: 'text/plain;charset=utf-8' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = selectedFilename.value
		document.body.appendChild(a)
		a.click()
		document.body.removeChild(a)
		URL.revokeObjectURL(url)
	} catch (e) {
		errorMessage.value = `Failed to export: ${e}`
	}
}

async function openFolder() {
	try {
		await showLauncherLogsFolder()
	} catch (e) {
		errorMessage.value = `Failed to open folder: ${e}`
	}
}

async function deleteCurrentSession() {
	if (!selectedFilename.value) return
	const filename = selectedFilename.value
	const info = sessions.value.find((s) => s.filename === filename)
	if (info?.isCurrent) {
		errorMessage.value = 'Cannot delete the active session log.'
		return
	}
	try {
		await deleteLauncherLogSession(filename)
		sessions.value = sessions.value.filter((s) => s.filename !== filename)
		if (sessions.value.length > 0) {
			await selectSession(sessions.value[0].filename)
		} else {
			selectedFilename.value = null
			rawContent.value = ''
		}
	} catch (e) {
		errorMessage.value = String(e)
	}
}

async function clearAllSessions() {
	try {
		await clearAllLauncherLogs()
		await refreshSessions()
		if (sessions.value.length > 0 && !selectedFilename.value) {
			await selectSession(sessions.value[0].filename)
		}
	} catch (e) {
		errorMessage.value = String(e)
	}
}

const logContainer = ref<HTMLElement | null>(null)
function scrollToBottom() {
	const el = logContainer.value
	if (el) el.scrollTop = el.scrollHeight
}

function onLogScroll() {
	if (!logContainer.value) return
	const el = logContainer.value
	const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40
	if (!atBottom && autoScroll.value) {
		autoScroll.value = false
	} else if (atBottom && !autoScroll.value) {
		autoScroll.value = true
	}
}

function formatBytes(bytes: number): string {
	if (bytes === 0) return '0 B'
	const units = ['B', 'KiB', 'MiB', 'GiB']
	let v = bytes
	let i = 0
	while (v >= 1024 && i < units.length - 1) {
		v /= 1024
		i++
	}
	return i === 0 ? `${v} ${units[i]}` : `${v.toFixed(1)} ${units[i]}`
}

function formatRelativeTime(unixSec: number): string {
	const now = Math.floor(Date.now() / 1000)
	const diff = now - unixSec
	if (diff < 60) return 'just now'
	if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
	if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
	return `${Math.floor(diff / 86400)}d ago`
}

function levelColor(level: ParsedLogLine['level']): string {
	switch (level) {
		case 'error':
			return '#ff5c5c'
		case 'warn':
			return '#f5a623'
		case 'info':
			return 'var(--text-contrast)'
		case 'debug':
			return 'var(--color-gray)'
		case 'trace':
			return 'var(--color-gray)'
		default:
			return 'var(--text-contrast)'
	}
}

const selectedInfo = computed<LauncherSessionLogInfo | null>(() => {
	if (!selectedFilename.value) return null
	return sessions.value.find((s) => s.filename === selectedFilename.value) ?? null
})

const levels = ['error', 'warn', 'info', 'debug', 'trace', 'other'] as const
</script>

<template>
	<Modal ref="modal" :header="`Launcher Logs`" :noblur="false">
		<div class="log-viewer">
			<!-- Top toolbar: session select + search + level filters -->
			<div class="toolbar">
				<select
					v-model="selectedFilename"
					class="session-select"
					:disabled="loading"
					@change="selectSession(($event.target as HTMLSelectElement).value)"
				>
					<option v-for="s in sessions" :key="s.filename" :value="s.filename">
						{{ s.isCurrent ? '● ' : '' }}{{ s.startedAt ? formatRelativeTime(s.startedAt) : s.filename }}
						({{ formatBytes(s.sizeBytes) }})
					</option>
				</select>

				<div class="search-box">
					<SearchIcon class="size-4 search-icon" />
					<input
						v-model="searchQuery"
						type="text"
						placeholder="Search..."
						class="search-input"
					/>
					<button v-if="searchQuery" class="clear-btn" @click="searchQuery = ''">
						<XIcon class="size-3" />
					</button>
				</div>

				<div class="level-toggles">
					<button
						v-for="lvl in levels"
						:key="lvl"
						class="level-toggle"
						:class="[
							`lvl-${lvl}`,
							{ active: levelFilters[lvl], inactive: !levelFilters[lvl] },
						]"
						@click="levelFilters[lvl] = !levelFilters[lvl]"
					>
						{{ lvl.toUpperCase() }}
					</button>
				</div>
			</div>

			<!-- Action bar -->
			<div class="action-bar">
				<div class="action-left">
					<ButtonStyled type="transparent">
						<button :disabled="loading" @click="refreshSessions">
							<RefreshCwIcon class="size-4" :class="{ spinning: loading }" />
							Refresh
						</button>
					</ButtonStyled>
					<ButtonStyled
						v-if="selectedInfo?.isCurrent"
						type="transparent"
					>
						<button :class="{ 'is-active': autoRefresh }" @click="autoRefresh = !autoRefresh">
							{{ autoRefresh ? '● Live' : '○ Paused' }}
						</button>
					</ButtonStyled>
				</div>
				<div class="action-right">
					<ButtonStyled type="transparent">
						<button @click="copyToClipboard">
							<ClipboardCopyIcon class="size-4" />
							Copy
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button @click="exportLog">
							<DownloadIcon class="size-4" />
							Export
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button @click="openFolder">
							<FolderOpenIcon class="size-4" />
							Folder
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="selectedFilename && !selectedInfo?.isCurrent" type="transparent" color="red">
						<button @click="deleteCurrentSession">
							<TrashIcon class="size-4" />
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="sessions.length > 1" type="transparent" color="red">
						<button @click="clearAllSessions">
							<TrashIcon class="size-4" />
							Clear old
						</button>
					</ButtonStyled>
				</div>
			</div>

			<!-- Truncation notice -->
			<div v-if="truncated" class="truncate-notice">
				Showing last {{ formatBytes(rawContent.length) }} of {{ formatBytes(totalSizeBytes) }}
				<button class="link-btn" @click="loadFullSession">Load full log</button>
			</div>

			<!-- Error message -->
			<Admonition v-if="errorMessage" type="danger" class="viewer-admonition">
				{{ errorMessage }}
			</Admonition>

			<!-- Log content -->
			<div ref="logContainer" class="log-content" @scroll="onLogScroll">
				<div v-if="loading && !rawContent" class="log-loading">
					<RefreshCwIcon class="size-5 spinning" />
					Loading...
				</div>
				<pre v-else-if="filteredLines.length === 0" class="log-empty">
No log lines match the current filters.</pre>
				<template v-else>
					<div
						v-for="(line, idx) in filteredLines"
						:key="idx"
						class="log-line"
						:style="{ color: levelColor(line.level) }"
					>
						<span class="log-timestamp">{{ line.timestamp ?? '' }}</span>
						<span class="log-level" :class="`lvl-${line.level}`">{{ line.level.toUpperCase() }}</span>
						<span v-if="line.span" class="log-span">{{ line.span }}:</span>
						<span class="log-message">{{ line.message }}</span>
					</div>
				</template>
			</div>

			<!-- Bottom info -->
			<div class="bottom-info">
				<span>{{ filteredLines.length }} / {{ parsedLines.length }} lines</span>
				<span v-if="selectedInfo?.isCurrent" class="live-dot">● LIVE</span>
			</div>
		</div>

		<template #footer>
			<ButtonStyled>
				<button @click="modal?.hide()">
					<XIcon />
					Close
				</button>
			</ButtonStyled>
		</template>
	</Modal>
</template>

<style scoped lang="scss">
.log-viewer {
	display: flex;
	flex-direction: column;
	height: 70vh;
	min-height: 400px;
	background: var(--color-bg);
	color: var(--text-contrast);
	border-radius: var(--radius-md);
	overflow: hidden;
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------
.toolbar {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.5rem 0.75rem;
	border-bottom: 1px solid var(--color-button-border);
	background: var(--color-raised-bg);
	flex-wrap: wrap;
}

.session-select {
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	color: var(--text-contrast);
	font-size: 0.8rem;
	padding: 0.3rem 0.5rem;
	outline: none;
	cursor: pointer;
	min-width: 180px;

	&:hover {
		border-color: var(--color-brand);
	}

	&:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
}

.search-box {
	position: relative;
	display: flex;
	align-items: center;
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	padding: 0.25rem 0.5rem;
	flex: 1;
	min-width: 150px;

	.search-icon {
		color: var(--color-gray);
		flex-shrink: 0;
	}

	&:focus-within {
		border-color: var(--color-brand);
	}
}

.search-input {
	border: none;
	background: transparent;
	color: var(--text-contrast);
	outline: none;
	font-size: 0.8rem;
	width: 100%;
	padding: 0 0.25rem;
}

.clear-btn {
	display: flex;
	align-items: center;
	justify-content: center;
	border: none;
	background: transparent;
	color: var(--color-gray);
	cursor: pointer;
	padding: 0.1rem;

	&:hover {
		color: var(--text-contrast);
	}
}

.level-toggles {
	display: flex;
	gap: 0.15rem;
}

.level-toggle {
	padding: 0.2rem 0.4rem;
	border: 1px solid transparent;
	border-radius: var(--radius-sm);
	background: transparent;
	font-size: 0.65rem;
	font-weight: 700;
	cursor: pointer;
	font-family: 'JetBrains Mono', monospace;
	letter-spacing: 0.03em;
	transition: all 0.15s ease;

	&.inactive {
		opacity: 0.3;
		text-decoration: line-through;
	}

	&.lvl-error {
		color: #ff5c5c;
	}
	&.lvl-warn {
		color: #f5a623;
	}
	&.lvl-info {
		color: #4a9eff;
	}
	&.lvl-debug {
		color: #a78bfa;
	}
	&.lvl-trace {
		color: #6b7280;
	}
	&.lvl-other {
		color: var(--color-gray);
	}

	&:hover {
		background: var(--color-button-bg);
	}
}

// ---------------------------------------------------------------------------
// Action bar
// ---------------------------------------------------------------------------
.action-bar {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.25rem 0.75rem;
	border-bottom: 1px solid var(--color-button-border);
	background: var(--color-raised-bg);
}

.action-left,
.action-right {
	display: flex;
	align-items: center;
	gap: 0.25rem;
}

.is-active {
	color: #4ade80 !important;
}

// ---------------------------------------------------------------------------
// Truncation notice
// ---------------------------------------------------------------------------
.truncate-notice {
	padding: 0.3rem 0.75rem;
	font-size: 0.7rem;
	color: #f5a623;
	background: #f5a6230d;
	border-bottom: 1px solid var(--color-button-border);
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.link-btn {
	background: transparent;
	border: none;
	color: var(--color-brand);
	cursor: pointer;
	font-size: 0.7rem;
	text-decoration: underline;
	padding: 0;
}

.viewer-admonition {
	margin: 0.5rem 0.75rem;
}

// ---------------------------------------------------------------------------
// Log content
// ---------------------------------------------------------------------------
.log-content {
	flex: 1;
	overflow: auto;
	padding: 0.5rem 0;
	font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
	font-size: 0.72rem;
	line-height: 1.5;
	background: var(--color-bg);
}

.log-loading,
.log-empty {
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 0.5rem;
	height: 100%;
	color: var(--color-gray);
	font-family: inherit;
	font-size: 0.85rem;
}

.log-line {
	display: flex;
	align-items: flex-start;
	padding: 0.05rem 0.75rem;
	white-space: pre-wrap;
	word-break: break-all;

	&:hover {
		background: var(--color-button-bg);
	}
}

.log-timestamp {
	flex-shrink: 0;
	color: var(--color-gray);
	opacity: 0.6;
	margin-right: 0.5rem;
	min-width: 0;
}

.log-level {
	flex-shrink: 0;
	font-weight: 700;
	margin-right: 0.5rem;
	opacity: 0.9;
	min-width: 3.2rem;
	text-align: left;

	&.lvl-error {
		color: #ff5c5c;
	}
	&.lvl-warn {
		color: #f5a623;
	}
	&.lvl-info {
		color: #4a9eff;
		opacity: 0.7;
	}
	&.lvl-debug {
		color: #a78bfa;
		opacity: 0.7;
	}
	&.lvl-trace {
		color: #6b7280;
		opacity: 0.6;
	}
	&.lvl-other {
		color: var(--color-gray);
	}
}

.log-span {
	flex-shrink: 0;
	color: var(--color-gray);
	opacity: 0.8;
	margin-right: 0.4rem;
	font-style: italic;
}

.log-message {
	flex: 1;
	white-space: pre-wrap;
	word-break: break-word;
	min-width: 0;
}

// ---------------------------------------------------------------------------
// Bottom info
// ---------------------------------------------------------------------------
.bottom-info {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.3rem 0.75rem;
	font-size: 0.7rem;
	color: var(--color-gray);
	border-top: 1px solid var(--color-button-border);
	background: var(--color-raised-bg);
}

.live-dot {
	color: #4ade80;
	font-family: 'JetBrains Mono', monospace;
	animation: livePulse 2s ease-in-out infinite;
}

@keyframes livePulse {
	0%,
	100% {
		opacity: 1;
	}
	50% {
		opacity: 0.5;
	}
}

// Spinning
.spinning {
	animation: spin 1s linear infinite;
}

@keyframes spin {
	from {
		transform: rotate(0deg);
	}
	to {
		transform: rotate(360deg);
	}
}
</style>
