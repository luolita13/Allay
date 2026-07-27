<script setup lang="ts">
import {
	CheckIcon,
	ClipboardCopyIcon,
	DownloadIcon,
	EditIcon,
	ExternalIcon,
	FolderOpenIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import { Admonition, ButtonStyled, NewModal as Modal } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import type { CrashDiagnosisMatch, CrashDiagnosisResult } from '@/helpers/crash_diagnosis'
import {
	applyAutoFix,
	diagnoseLatestCrash,
	diagnoseRawLog,
	exportCrashReport,
} from '@/helpers/crash_diagnosis'
import { crash_diagnosed_listener } from '@/helpers/events'
import { openPath } from '@/helpers/utils'

const props = defineProps<{
	instanceId?: string
	instanceName?: string
}>()

defineEmits<{
	(e: 'proceed', action: 'fix' | 'close'): void
}>()

const modal = ref<InstanceType<typeof Modal> | null>(null)
const loading = ref(false)
const pastedLog = ref('')
const showPasteBox = ref(false)
const result = ref<CrashDiagnosisResult | null>(null)
const errorMessage = ref<string | null>(null)
const fixMessage = ref<string | null>(null)
const exporting = ref(false)

defineExpose({
	show: async (targetInstanceId?: string) => {
		modal.value?.show()
		fixMessage.value = null
		const id = targetInstanceId ?? props.instanceId
		if (id) {
			await loadDiagnosis(id)
		}
	},
	hide: () => modal.value?.hide(),
})

async function loadDiagnosis(instanceId: string) {
	loading.value = true
	errorMessage.value = null
	fixMessage.value = null
	try {
		result.value = await diagnoseLatestCrash(instanceId, 3600)
	} catch (e) {
		console.error('Failed to load crash diagnosis:', e)
		errorMessage.value = String(e)
	} finally {
		loading.value = false
	}
}

async function diagnosePasted() {
	if (!pastedLog.value.trim()) return
	loading.value = true
	errorMessage.value = null
	try {
		result.value = await diagnoseRawLog(pastedLog.value)
		showPasteBox.value = false
	} catch (e) {
		console.error('Failed to diagnose pasted log:', e)
		errorMessage.value = String(e)
	} finally {
		loading.value = false
	}
}

async function openCrashReportFile() {
	const selected = await open({
		multiple: false,
		filters: [{ name: 'Logs / Crash reports', extensions: ['txt', 'log', 'gz'] }],
	})
	if (selected && typeof selected === 'string') {
		loading.value = true
		try {
			const { readTextFile } = await import('@tauri-apps/plugin-fs')
			const text = await readTextFile(selected)
			result.value = await diagnoseRawLog(text)
		} catch (e) {
			console.error('Failed to diagnose crash report file:', e)
			errorMessage.value = String(e)
		} finally {
			loading.value = false
		}
	}
}

async function handleExportReport() {
	if (!props.instanceId) return
	exporting.value = true
	errorMessage.value = null
	try {
		const zipPath = await exportCrashReport(props.instanceId)
		// Open the folder containing the zip
		await openPath(zipPath.replace(/[^\\/]+$/, ''))
		fixMessage.value =
			'Crash report exported successfully! The ZIP file has been opened in your file explorer.'
	} catch (e) {
		console.error('Failed to export crash report:', e)
		errorMessage.value = String(e)
	} finally {
		exporting.value = false
	}
}

async function handleAutoFix(autoFix: CrashDiagnosisMatch['auto_fix']) {
	if (!props.instanceId || autoFix === 'none') return
	loading.value = true
	fixMessage.value = null
	errorMessage.value = null
	try {
		const msg = await applyAutoFix(props.instanceId, autoFix)
		fixMessage.value = msg
	} catch (e) {
		console.error('Failed to apply auto-fix:', e)
		errorMessage.value = String(e)
	} finally {
		loading.value = false
	}
}

function severityClass(sev: CrashDiagnosisMatch['severity']) {
	return {
		fatal: 'severity-fatal',
		warning: 'severity-warning',
		info: 'severity-info',
	}[sev]
}

function severityLabel(sev: CrashDiagnosisMatch['severity']) {
	return { fatal: 'Critical', warning: 'Warning', info: 'Info' }[sev]
}

function autoFixLabel(af: CrashDiagnosisMatch['auto_fix']): string | null {
	if (af === 'none') return null
	if (typeof af === 'object' && 'install_java' in af) return `Install Java ${af.install_java}`
	if (af === 'reinstall_instance') return 'Reinstall instance'
	if (af === 'clear_mods') return 'Open mods folder'
	if (af === 'update_graphics_driver') return 'Update graphics driver'
	if (af === 'increase_memory') return 'Increase memory'
	if (af === 'open_mods_folder') return 'Open mods folder'
	if (af === 'open_instance_settings') return 'Open instance settings'
	return null
}

function sourceLabel(source: string): string {
	const labels: Record<string, string> = {
		crash_report: 'Crash Report',
		minecraft_log: 'latest.log',
		hs_err_log: 'JVM Crash Log',
		launcher_output: 'Launcher Output',
		user_provided: 'Manual Input',
	}
	return labels[source] ?? source
}

async function copyReport() {
	if (!result.value) return
	const text = result.value.matched
		.map((m) => {
			let report = `## ${m.title} [${m.rule_id}]\n${m.description}\nFix: ${m.fix}`
			if (m.fragment) report += `\nFragment: ${m.fragment}`
			if (m.mod_files?.length) report += `\nMod files: ${m.mod_files.join(', ')}`
			return report
		})
		.join('\n\n')
	await navigator.clipboard.writeText(text)
}

// Listen for new crash diagnoses while the modal is closed
let unlisten: (() => void) | null = null
async function setupListener() {
	if (unlisten) return
	unlisten = await crash_diagnosed_listener((payload) => {
		if (!props.instanceId || payload.instance_id === props.instanceId) {
			result.value = payload
			modal.value?.show()
		}
	})
}

watch(
	() => props.instanceId,
	() => {
		setupListener()
	},
	{ immediate: true },
)

const hasAnyMatch = computed(() => (result.value?.matched.length ?? 0) > 0)
const hasSources = computed(() => (result.value?.sources?.length ?? 0) > 0)
</script>

<template>
	<Modal ref="modal" :closable="true" :noblur="false" class="crash-diagnosis-modal">
		<template #title>
			<div class="title-row">
				<span class="title-text">Minecraft crashed</span>
				<span v-if="instanceName" class="instance-chip" :title="instanceName">
					{{ instanceName }}
				</span>
			</div>
		</template>

		<div class="crash-diagnosis-body">
			<div v-if="loading" class="state-block">
				<SpinnerIcon class="animate-spin spinner-icon" />
				<p>Analyzing crash report…</p>
			</div>

			<Admonition
				v-else-if="errorMessage"
				type="critical"
				header="Failed to analyze crash report"
				dismissible
				@dismiss="errorMessage = null"
			>
				{{ errorMessage }}
			</Admonition>

			<template v-else-if="result && hasAnyMatch">
				<p class="lead">
					We detected <strong>{{ result.matched.length }}</strong>
					{{ result.matched.length === 1 ? 'issue' : 'issues' }} in the crash report. Review the
					findings below and follow the suggested fixes.
				</p>

				<div class="match-list">
					<div
						v-for="(m, idx) in result.matched"
						:key="m.rule_id + idx"
						class="match-card"
						:class="severityClass(m.severity)"
					>
						<div class="match-header">
							<span class="severity-pill">{{ severityLabel(m.severity) }}</span>
							<span class="match-title">{{ m.title }}</span>
							<span class="rule-id" :title="m.rule_id">{{ m.rule_id }}</span>
						</div>
						<p class="match-description">{{ m.description }}</p>
						<div v-if="m.fragment && m.fragment.length < 200" class="match-fragment">
							<span class="fragment-label">Captured:</span>
							<code>{{ m.fragment }}</code>
						</div>
						<div v-if="m.mod_files?.length" class="match-mod-files">
							<span class="mod-files-label">Suspected mod files:</span>
							<div class="mod-file-list">
								<code v-for="file in m.mod_files" :key="file" class="mod-file-chip">
									{{ file }}
								</code>
							</div>
						</div>
						<div class="match-fix">
							<CheckIcon class="fix-icon" />
							<span>{{ m.fix }}</span>
						</div>
						<div v-if="autoFixLabel(m.auto_fix)" class="match-autofix">
							<ButtonStyled color="brand" type="outlined">
								<button :disabled="loading" @click="handleAutoFix(m.auto_fix)">
									<DownloadIcon />
									{{ autoFixLabel(m.auto_fix) }}
								</button>
							</ButtonStyled>
						</div>
					</div>
				</div>

				<div v-if="hasSources" class="sources-bar">
					<span class="sources-label">Analyzed sources:</span>
					<span v-for="src in result.sources" :key="src" class="source-chip">
						{{ sourceLabel(src) }}
					</span>
				</div>

				<div v-if="fixMessage" class="fix-message">
					<CheckIcon class="fix-msg-icon" />
					<span>{{ fixMessage }}</span>
				</div>

				<div class="action-row">
					<ButtonStyled>
						<button @click="copyReport">
							<ClipboardCopyIcon />
							Copy diagnosis
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="instanceId" color="brand" type="outlined">
						<button :disabled="exporting" @click="handleExportReport">
							<DownloadIcon v-if="!exporting" />
							<SpinnerIcon v-else class="animate-spin" />
							{{ exporting ? 'Exporting…' : 'Export report' }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button @click="openCrashReportFile">
							<FolderOpenIcon />
							Analyze different file
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button @click="showPasteBox = !showPasteBox">
							<EditIcon />
							Paste log manually
						</button>
					</ButtonStyled>
				</div>
			</template>

			<div v-else-if="result" class="state-block">
				<p>
					No matching diagnosis rules were found for this crash. The launch may have failed for a
					reason outside our rule set (network issue, antivirus interference, etc.).
				</p>
				<div class="action-row">
					<ButtonStyled v-if="instanceId" color="brand" type="outlined">
						<button :disabled="exporting" @click="handleExportReport">
							<DownloadIcon v-if="!exporting" />
							<SpinnerIcon v-else class="animate-spin" />
							{{ exporting ? 'Exporting…' : 'Export report' }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button @click="openCrashReportFile">
							<FolderOpenIcon />
							Analyze different file
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button @click="showPasteBox = !showPasteBox">
							<EditIcon />
							Paste log manually
						</button>
					</ButtonStyled>
				</div>
			</div>

			<div v-else class="state-block">
				<p>No crash report was found for this instance yet.</p>
				<ButtonStyled type="transparent">
					<button @click="openCrashReportFile">
						<FolderOpenIcon />
						Select a crash report file
					</button>
				</ButtonStyled>
			</div>

			<div v-if="showPasteBox" class="paste-box">
				<textarea v-model="pastedLog" placeholder="Paste crash log contents here…" rows="8" />
				<ButtonStyled>
					<button :disabled="!pastedLog.trim()" @click="diagnosePasted">
						<ExternalIcon />
						Analyze
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Modal>
</template>

<style scoped lang="scss">
.title-row {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	flex-wrap: wrap;
}

.title-text {
	font-weight: 600;
}

.instance-chip {
	background: var(--color-bg);
	border: 1px solid var(--color-divider);
	color: var(--color-text);
	border-radius: 999px;
	padding: 0.125rem 0.625rem;
	font-size: 0.8125rem;
	max-width: 200px;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.crash-diagnosis-body {
	padding: 0.5rem 0;
	min-width: 520px;
	max-width: 720px;
}

.state-block {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	text-align: center;
	gap: 1rem;
	padding: 2rem 1rem;
	color: var(--color-text-secondary);
}

.spinner-icon {
	width: 1.75rem;
	height: 1.75rem;
	color: var(--color-brand);
}

.lead {
	margin: 0 0 1rem;
	color: var(--color-text);
}

.match-list {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	max-height: 50vh;
	overflow-y: auto;
	padding-right: 0.25rem;
}

.match-card {
	border: 1px solid var(--color-divider);
	border-left: 4px solid var(--color-divider);
	border-radius: 0.5rem;
	padding: 0.75rem 1rem;
	background: var(--color-bg);
}

.match-card.severity-fatal {
	border-left-color: var(--color-red);
}
.match-card.severity-warning {
	border-left-color: var(--color-orange);
}
.match-card.severity-info {
	border-left-color: var(--color-blue);
}

.match-header {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	margin-bottom: 0.375rem;
}

.severity-pill {
	font-size: 0.6875rem;
	font-weight: 700;
	letter-spacing: 0.04em;
	text-transform: uppercase;
	padding: 0.125rem 0.5rem;
	border-radius: 999px;
}

.severity-fatal .severity-pill {
	background: var(--color-red-bg, rgba(239, 68, 68, 0.12));
	color: var(--color-red);
}
.severity-warning .severity-pill {
	background: var(--color-orange-bg, rgba(249, 115, 22, 0.12));
	color: var(--color-orange);
}
.severity-info .severity-pill {
	background: var(--color-blue-bg, rgba(59, 130, 246, 0.12));
	color: var(--color-blue);
}

.match-title {
	font-weight: 600;
	color: var(--color-text);
	flex: 1;
	min-width: 0;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.rule-id {
	font-size: 0.6875rem;
	color: var(--color-text-secondary);
	font-family: var(--mono-font, monospace);
	opacity: 0.7;
}

.match-description {
	margin: 0 0 0.5rem;
	color: var(--color-text);
	line-height: 1.5;
	font-size: 0.875rem;
}

.match-fragment {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	margin: 0 0 0.5rem;
	font-size: 0.8125rem;
}

.fragment-label {
	color: var(--color-text-secondary);
}

.match-fragment code {
	background: var(--color-bg);
	border: 1px solid var(--color-divider);
	border-radius: 0.25rem;
	padding: 0.0625rem 0.375rem;
	font-family: var(--mono-font, monospace);
	font-size: 0.8125rem;
	color: var(--color-brand);
	word-break: break-all;
}

.match-mod-files {
	display: flex;
	flex-direction: column;
	gap: 0.375rem;
	margin: 0 0 0.5rem;
}

.mod-files-label {
	font-size: 0.8125rem;
	color: var(--color-text-secondary);
}

.mod-file-list {
	display: flex;
	flex-wrap: wrap;
	gap: 0.25rem;
}

.mod-file-chip {
	background: var(--color-brand-bg, rgba(27, 217, 106, 0.1));
	border: 1px solid var(--color-brand);
	border-radius: 0.25rem;
	padding: 0.125rem 0.5rem;
	font-family: var(--mono-font, monospace);
	font-size: 0.75rem;
	color: var(--color-brand);
}

.match-fix {
	display: flex;
	align-items: flex-start;
	gap: 0.5rem;
	font-size: 0.875rem;
	color: var(--color-text);
	background: var(--color-bg);
	border-radius: 0.375rem;
	padding: 0.5rem 0.625rem;
}

.fix-icon {
	width: 1rem;
	height: 1rem;
	color: var(--color-green);
	flex-shrink: 0;
	margin-top: 0.125rem;
}

.match-autofix {
	margin-top: 0.5rem;
	display: flex;
	gap: 0.5rem;
}

.sources-bar {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.375rem;
	margin-top: 0.75rem;
	padding: 0.5rem 0;
	border-top: 1px solid var(--color-divider);
}

.sources-label {
	font-size: 0.75rem;
	color: var(--color-text-secondary);
	margin-right: 0.25rem;
}

.source-chip {
	font-size: 0.6875rem;
	background: var(--color-surface-2);
	color: var(--color-text-secondary);
	padding: 0.125rem 0.5rem;
	border-radius: 999px;
}

.fix-message {
	display: flex;
	align-items: flex-start;
	gap: 0.5rem;
	margin-top: 0.75rem;
	padding: 0.625rem 0.75rem;
	background: var(--color-green-bg, rgba(34, 197, 94, 0.1));
	border: 1px solid var(--color-green);
	border-radius: 0.375rem;
	font-size: 0.875rem;
	color: var(--color-text);
}

.fix-msg-icon {
	width: 1rem;
	height: 1rem;
	color: var(--color-green);
	flex-shrink: 0;
	margin-top: 0.125rem;
}

.action-row {
	display: flex;
	flex-wrap: wrap;
	gap: 0.5rem;
	margin-top: 1rem;
}

.paste-box {
	margin-top: 1rem;
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
}

.paste-box textarea {
	width: 100%;
	resize: vertical;
	background: var(--color-bg);
	border: 1px solid var(--color-divider);
	border-radius: 0.5rem;
	padding: 0.625rem 0.75rem;
	font-family: var(--mono-font, monospace);
	font-size: 0.8125rem;
	color: var(--color-text);
}
</style>
