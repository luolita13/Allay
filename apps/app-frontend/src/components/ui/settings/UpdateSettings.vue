<script setup lang="ts">
import { ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'
import DOMPurify from 'dompurify'
import MarkdownIt from 'markdown-it'
import { computed, onMounted, ref } from 'vue'

import { areUpdatesEnabled } from '@/helpers/utils.js'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	currentVersion: {
		id: 'app.updates.current-version',
		defaultMessage: 'Current version',
	},
	latestVersion: {
		id: 'app.updates.latest-version',
		defaultMessage: 'Latest version',
	},
	checkForUpdates: {
		id: 'app.updates.check-for-updates',
		defaultMessage: 'Check for updates',
	},
	checking: {
		id: 'app.updates.checking',
		defaultMessage: 'Checking for updates…',
	},
	upToDate: {
		id: 'app.updates.up-to-date',
		defaultMessage: 'You are on the latest version.',
	},
	updateAvailable: {
		id: 'app.updates.update-available',
		defaultMessage: 'Version {version} is available.',
	},
	checkFailed: {
		id: 'app.updates.check-failed',
		defaultMessage: 'Failed to check for updates.',
	},
	updatesDisabled: {
		id: 'app.updates.updates-disabled',
		defaultMessage: 'Updates are only available in release builds.',
	},
	updateNow: {
		id: 'app.updates.update-now',
		defaultMessage: 'Download & install',
	},
	releaseNotes: {
		id: 'app.updates.release-notes',
		defaultMessage: 'Release notes',
	},
	loadingChangelog: {
		id: 'app.updates.loading-changelog',
		defaultMessage: 'Loading release notes…',
	},
	changelogError: {
		id: 'app.updates.changelog-error',
		defaultMessage: 'Failed to load release notes.',
	},
	changelogErrorDetail: {
		id: 'app.updates.changelog-error-detail',
		defaultMessage: 'Reason: {error}',
	},
	viewOnGithub: {
		id: 'app.updates.view-on-github',
		defaultMessage: 'View on GitHub',
	},
	publishedOn: {
		id: 'app.updates.published-on',
		defaultMessage: 'Published on {date}',
	},
	preRelease: {
		id: 'app.updates.prerelease',
		defaultMessage: 'Pre-release',
	},
})

type UpdateStatus = 'idle' | 'disabled' | 'checking' | 'up-to-date' | 'available' | 'error'

interface GitHubRelease {
	id: number
	tag_name: string
	name: string
	body: string
	html_url: string
	published_at: string
	prerelease: boolean
	draft: boolean
}

const currentVersion = ref('')
const updateStatus = ref<UpdateStatus>('idle')
const updateVersion = ref('')

const changelog = ref<GitHubRelease[]>([])
const changelogLoading = ref(false)
const changelogError = ref(false)
const changelogErrorMessage = ref('')
const expandedReleaseId = ref<number | null>(null)

const latestRelease = computed(() => changelog.value[0] ?? null)

const md = new MarkdownIt({
	html: false,
	linkify: true,
	breaks: false,
	typographer: true,
})

function renderMarkdown(body: string): string {
	if (!body) return ''
	const html = md.render(body)
	return DOMPurify.sanitize(html, {
		USE_PROFILES: { html: true },
		ALLOWED_ATTR: ['href', 'title', 'target', 'rel', 'src', 'alt'],
	})
}

const renderedBodies = computed<Record<number, string>>(() => {
	const map: Record<number, string> = {}
	for (const release of changelog.value) {
		map[release.id] = renderMarkdown(release.body)
	}
	return map
})

onMounted(async () => {
	currentVersion.value = await getVersion()
	await loadChangelog()
	const enabled = await areUpdatesEnabled()
	if (!enabled) {
		updateStatus.value = 'disabled'
	}
})

async function checkForUpdates() {
	updateStatus.value = 'checking'
	try {
		const update = (await invoke('plugin:updater|check')) as { version?: string } | null
		if (update) {
			updateStatus.value = 'available'
			updateVersion.value = update.version ?? ''
		} else {
			updateStatus.value = 'up-to-date'
		}
	} catch {
		updateStatus.value = 'error'
	}
}

async function loadChangelog() {
	changelogLoading.value = true
	changelogError.value = false
	changelogErrorMessage.value = ''
	try {
		const response = await tauriFetch(
			'https://api.github.com/repos/luolita13/Allay/releases?per_page=15',
			{
				method: 'GET',
				headers: {
					Accept: 'application/vnd.github+json',
					'X-GitHub-Api-Version': '2022-11-28',
					'User-Agent': 'Allay-App',
				},
			},
		)
		if (!response.ok) throw new Error(`HTTP ${response.status} ${response.statusText}`)
		const data = (await response.json()) as GitHubRelease[]
		changelog.value = data.filter((r) => !r.draft)
		if (latestRelease.value) {
			updateVersion.value = latestRelease.value.tag_name
		}
	} catch (err) {
		const message = err instanceof Error ? err.message : String(err)
		console.error('[UpdateSettings] Failed to fetch release notes:', err)
		changelogErrorMessage.value = message
		changelogError.value = true
	} finally {
		changelogLoading.value = false
	}
}

function toggleRelease(id: number) {
	expandedReleaseId.value = expandedReleaseId.value === id ? null : id
}

function formatDate(iso: string): string {
	try {
		return new Date(iso).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
		})
	} catch {
		return iso
	}
}
</script>

<template>
	<div class="flex flex-col gap-5">
		<!-- Status card -->
		<section class="flex items-center justify-between gap-4 rounded-xl bg-button-bg border border-button-border p-4">
			<div class="flex flex-col gap-[0.15rem]">
				<p class="m-0 text-[0.75rem] font-semibold uppercase tracking-[0.05em] text-tertiary">
					{{ formatMessage(messages.currentVersion) }}
				</p>
				<p class="m-0 text-[1.15rem] font-bold text-contrast tabular-nums">
					v{{ currentVersion || '—' }}
				</p>
			</div>
			<ButtonStyled>
				<button
					:disabled="updateStatus === 'checking' || updateStatus === 'disabled'"
					@click="checkForUpdates"
				>
					<RefreshCwIcon
						class="size-4"
						:class="{ 'animate-spin': updateStatus === 'checking' }"
					/>
					{{ formatMessage(messages.checkForUpdates) }}
				</button>
			</ButtonStyled>
		</section>

		<!-- Status message -->
		<p
			class="m-0 text-[0.85rem] flex items-center gap-3 flex-wrap"
			:class="{
				'text-tertiary': updateStatus === 'disabled',
				'text-secondary': updateStatus === 'checking',
				'text-green-400': updateStatus === 'up-to-date',
				'text-brand': updateStatus === 'available',
				'text-red-400': updateStatus === 'error',
			}"
		>
			<template v-if="updateStatus === 'disabled'">
				{{ formatMessage(messages.updatesDisabled) }}
			</template>
			<template v-else-if="updateStatus === 'checking'">
				{{ formatMessage(messages.checking) }}
			</template>
			<template v-else-if="updateStatus === 'up-to-date'">
				{{ formatMessage(messages.upToDate) }}
			</template>
			<template v-else-if="updateStatus === 'available'">
				{{ formatMessage(messages.updateAvailable, { version: updateVersion }) }}
				<ButtonStyled type="outlined">
					<button @click="openUrl('https://github.com/luolita13/Allay/releases/latest')">
						{{ formatMessage(messages.updateNow) }}
						<ExternalIcon class="size-3.5" />
					</button>
				</ButtonStyled>
			</template>
			<template v-else-if="updateStatus === 'error'">
				{{ formatMessage(messages.checkFailed) }}
			</template>
		</p>

		<!-- Changelog -->
		<section class="flex flex-col gap-3">
			<div class="flex items-center justify-between">
				<h3 class="m-0 text-[0.95rem] font-bold text-contrast tracking-[0.01em]">
					{{ formatMessage(messages.releaseNotes) }}
				</h3>
				<ButtonStyled type="transparent">
					<button
						:disabled="changelogLoading"
						title="Refresh"
						@click="loadChangelog"
					>
						<RefreshCwIcon
							class="size-3.5"
							:class="{ 'animate-spin': changelogLoading }"
						/>
					</button>
				</ButtonStyled>
			</div>

			<div
				v-if="changelogLoading && changelog.length === 0"
				class="p-5 text-center text-[0.85rem] text-tertiary border border-dashed border-button-border rounded-xl"
			>
				{{ formatMessage(messages.loadingChangelog) }}
			</div>
			<div
				v-else-if="changelogError"
				class="p-5 text-center text-[0.85rem] text-red-400 border border-dashed border-red-400/40 rounded-xl"
			>
				<div class="font-semibold mb-1">{{ formatMessage(messages.changelogError) }}</div>
				<div v-if="changelogErrorMessage" class="opacity-75 text-[0.78rem] font-mono">
					{{ formatMessage(messages.changelogErrorDetail, { error: changelogErrorMessage }) }}
				</div>
			</div>
			<ul v-else class="m-0 p-0 list-none flex flex-col gap-[0.4rem]">
				<li
					v-for="release in changelog"
					:key="release.id"
					class="rounded-xl bg-button-bg border border-button-border overflow-hidden"
				>
					<button
						class="w-full flex items-center justify-between gap-3 py-[0.65rem] px-[0.9rem] bg-transparent border-none text-left text-contrast text-[0.85rem] cursor-pointer transition-colors hover:bg-surface-3"
						@click="toggleRelease(release.id)"
					>
						<div class="flex flex-col gap-[0.1rem] min-w-0">
							<div class="flex items-center gap-2">
								<span class="font-bold tabular-nums">{{ release.tag_name }}</span>
								<span
									v-if="release.prerelease"
									class="text-[0.65rem] px-[0.45rem] py-[0.1rem] rounded-full bg-yellow-300/18 text-yellow-300 border border-yellow-300/35 font-semibold tracking-[0.04em] uppercase"
								>
									{{ formatMessage(messages.preRelease) }}
								</span>
							</div>
							<span class="text-[0.72rem] text-tertiary">
								{{ formatMessage(messages.publishedOn, { date: formatDate(release.published_at) }) }}
							</span>
						</div>
						<span
							class="text-[1.4rem] text-tertiary transition-transform duration-200 inline-block"
							:class="{ 'rotate-90': expandedReleaseId === release.id }"
						>&#8250;</span>
					</button>
					<div
						v-if="expandedReleaseId === release.id"
						class="px-[0.9rem] pb-[0.85rem] flex flex-col gap-[0.6rem] border-t border-button-border"
					>
						<div
							v-if="release.body"
							class="markdown-body mt-[0.6rem] p-3 rounded-lg bg-bg text-[0.78rem] leading-relaxed text-secondary max-h-72 overflow-y-auto"
							v-html="renderedBodies[release.id] || release.body"
						/>
						<p v-else class="mt-[0.6rem] text-[0.8rem] text-tertiary italic">No description provided.</p>
						<ButtonStyled type="outlined">
							<button @click="openUrl(release.html_url)">
								<ExternalIcon class="size-3.5" />
								{{ formatMessage(messages.viewOnGithub) }}
							</button>
						</ButtonStyled>
					</div>
				</li>
			</ul>
		</section>
	</div>
</template>

<style>
/* Unscoped so v-html content picks up these styles */
.markdown-body {
	font-size: 0.8rem;
	line-height: 1.6;
	word-wrap: break-word;
	color: var(--color-secondary, #b3b3b3);
}

.markdown-body > *:first-child {
	margin-top: 0 !important;
}
.markdown-body > *:last-child {
	margin-bottom: 0 !important;
}

.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
	margin: 1em 0 0.5em;
	font-weight: 700;
	line-height: 1.25;
	color: var(--color-contrast, #f5f5f5);
}

.markdown-body h1 {
	font-size: 1.4em;
	border-bottom: 1px solid var(--color-button-border, rgba(255, 255, 255, 0.1));
	padding-bottom: 0.2em;
}
.markdown-body h2 {
	font-size: 1.2em;
	border-bottom: 1px solid var(--color-button-border, rgba(255, 255, 255, 0.1));
	padding-bottom: 0.2em;
}
.markdown-body h3 {
	font-size: 1.05em;
}
.markdown-body h4 {
	font-size: 0.95em;
}
.markdown-body h5 {
	font-size: 0.9em;
}
.markdown-body h6 {
	font-size: 0.85em;
	color: var(--color-tertiary, #888);
}

.markdown-body p {
	margin: 0.6em 0;
}

.markdown-body a {
	color: var(--color-brand, #4f88ff);
	text-decoration: none;
	font-weight: 500;
}
.markdown-body a:hover {
	text-decoration: underline;
}

.markdown-body ul,
.markdown-body ol {
	margin: 0.6em 0;
	padding-left: 1.5em;
}

.markdown-body li {
	margin: 0.2em 0;
}

.markdown-body li + li {
	margin-top: 0.2em;
}

.markdown-body li > p {
	margin: 0.2em 0;
}

.markdown-body blockquote {
	margin: 0.6em 0;
	padding: 0 0.8em;
	color: var(--color-tertiary, #888);
	border-left: 0.2em solid var(--color-button-border, rgba(255, 255, 255, 0.15));
}

.markdown-body code {
	padding: 0.15em 0.35em;
	font-size: 0.85em;
	border-radius: 4px;
	background: rgba(255, 255, 255, 0.08);
	font-family:
		'SF Mono', 'Menlo', 'Consolas', 'Liberation Mono', 'Courier New', monospace;
}

.markdown-body pre {
	padding: 0.8em;
	border-radius: 6px;
	background: rgba(0, 0, 0, 0.25);
	overflow-x: auto;
	font-size: 0.78em;
	line-height: 1.5;
}

.markdown-body pre code {
	padding: 0;
	background: transparent;
	font-size: inherit;
}

.markdown-body hr {
	height: 0;
	margin: 1.2em 0;
	border: 0;
	border-top: 1px solid var(--color-button-border, rgba(255, 255, 255, 0.1));
}

.markdown-body img {
	max-width: 100%;
	height: auto;
	border-radius: 4px;
}

.markdown-body table {
	display: block;
	width: max-content;
	max-width: 100%;
	overflow: auto;
	border-collapse: collapse;
	margin: 0.6em 0;
}

.markdown-body table th,
.markdown-body table td {
	padding: 0.4em 0.7em;
	border: 1px solid var(--color-button-border, rgba(255, 255, 255, 0.1));
}

.markdown-body table tr {
	background: transparent;
	border-top: 1px solid var(--color-button-border, rgba(255, 255, 255, 0.1));
}

.markdown-body table tr:nth-child(2n) {
	background: rgba(255, 255, 255, 0.025);
}
</style>