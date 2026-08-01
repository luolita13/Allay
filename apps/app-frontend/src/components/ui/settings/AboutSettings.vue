<script setup lang="ts">
import { ClipboardCopyIcon, DownloadIcon, ExternalIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { save } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { platform as getOsPlatform, version as getOsVersion } from '@tauri-apps/plugin-os'
import { ref } from 'vue'
import { app_support_details } from '@/helpers/install'
import AllayAppLogo from '@/assets/allay_app.svg?component'
import LicenseModal from '@/components/ui/LicenseModal.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	appName: {
		id: 'app.about.app-name',
		defaultMessage: 'Allay',
	},
	craftedBy: {
		id: 'app.about.crafted-by',
		defaultMessage: 'Crafted By',
	},
	version: {
		id: 'app.about.version',
		defaultMessage: 'Version',
	},
	platform: {
		id: 'app.about.platform',
		defaultMessage: 'Platform',
	},
	license: {
		id: 'app.about.license',
		defaultMessage: 'License',
	},
	licenseValue: {
		id: 'app.about.license-value',
		defaultMessage: 'GPL-3.0',
	},
	viewSource: {
		id: 'app.about.view-source',
		defaultMessage: 'View Source',
	},
	reportIssue: {
		id: 'app.about.report-issue',
		defaultMessage: 'Report an Issue',
	},
	copyAppDiagnostics: {
		id: 'app.about.export-diagnostics',
		defaultMessage: 'Export diagnostics',
	},
	copyDiagnosticsSuccess: {
		id: 'app.about.export-diagnostics.success',
		defaultMessage: 'Diagnostics exported',
	},
	overrideAuthorized: {
		id: 'app.about.easter-egg.override-authorized',
		defaultMessage: 'System Override Authorized',
	},
	hallOfFameTitle: {
		id: 'app.about.easter-egg.hall-of-fame-title',
		defaultMessage: 'Hall of Fame',
	},
	hallOfFameDescription: {
		id: 'app.about.easter-egg.hall-of-fame-description',
		defaultMessage: 'The creators of this custom build:',
	},
})

const version = await getVersion()
const osPlatform = getOsPlatform()
const osVersion = getOsVersion()

const platformLabel =
	osPlatform === 'macos' ? 'macOS' : osPlatform.charAt(0).toUpperCase() + osPlatform.slice(1)

const clickCount = ref(0)
const showOverrideMessage = ref(false)
const showHallOfFame = ref(false)
const copyingDiagnostics = ref(false)
const diagnosticsExported = ref(false)

// Arcade-style high score table — replaces the AI-ish card grid
const highScores = [
	{ rank: '1ST', name: 'LUOLITA13', score: '999999', note: 'Lead Customizer' },
	{ rank: '2ND', name: 'MODRINTH TEAM', score: '750000', note: 'Original Authors' },
	{ rank: '3RD', name: 'THE STACK', score: '500000', note: 'Tauri + Vue + Rust' },
	{ rank: '4TH', name: 'YOU', score: '000000', note: 'The Player' },
]

const showLicense = ref(false)

function onLogoClick() {
	clickCount.value++
	console.log('[About Easter Egg] logo click:', clickCount.value)
	triggerEasterEgg()
}

function onLicenseClick() {
	showLicense.value = true
}

function onVersionClick() {
	clickCount.value++
	console.log('[About Easter Egg] version click:', clickCount.value)
	triggerEasterEgg()
}

function triggerEasterEgg() {
	if (clickCount.value === 10) {
		showOverrideMessage.value = true
		setTimeout(() => {
			showOverrideMessage.value = false
		}, 2500)
	}
	if (clickCount.value === 15) {
		showHallOfFame.value = true
	}
}

async function copyAppDiagnostics() {
	if (copyingDiagnostics.value) return
	copyingDiagnostics.value = true
	try {
		const text = await app_support_details()
		const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
		const filePath = await save({
			defaultPath: `allay-diagnostics-${timestamp}.txt`,
			filters: [{ name: 'Text', extensions: ['txt'] }],
		})
		if (filePath) {
			const { writeTextFile } = await import('@tauri-apps/plugin-fs')
			await writeTextFile(filePath, text)
			diagnosticsExported.value = true
			setTimeout(() => (diagnosticsExported.value = false), 2000)
		}
	} catch (error) {
		handleError(error)
	} finally {
		copyingDiagnostics.value = false
	}
}
</script>

<template>
	<div
		class="about-root"
	>
		<!-- Hero -->
		<div class="hero">
			<AllayAppLogo
				class="allay-logo"
				:class="{
					'egg-spin': clickCount >= 5 && clickCount < 15,
				}"
				title="Psst... try clicking me"
				@click="onLogoClick"
			/>
			<h1 class="app-name">{{ formatMessage(messages.appName) }}</h1>
		</div>

		<!-- Override message -->
		<Transition name="fade-pop">
			<div v-if="showOverrideMessage" class="override-message">
				{{ formatMessage(messages.overrideAuthorized) }}
			</div>
		</Transition>

		<!-- Metadata -->
		<dl class="metadata">
			<div class="meta-row">
				<dt>{{ formatMessage(messages.version) }}</dt>
				<dd class="version-value" title="Also clickable" @click="onVersionClick">{{ version }}</dd>
			</div>
			<div class="meta-row">
				<dt>{{ formatMessage(messages.platform) }}</dt>
				<dd>
					{{ platformLabel }}
					<span class="meta-sub">{{ osVersion }}</span>
				</dd>
			</div>
			<div class="meta-row">
				<dt>{{ formatMessage(messages.license) }}</dt>
				<dd class="license-clickable" role="button" tabindex="0" @click="onLicenseClick" @keydown.enter="onLicenseClick">
					{{ formatMessage(messages.licenseValue) }}
				</dd>
			</div>
		</dl>

		<!-- High Scores — arcade-style Hall of Fame -->
		<Transition name="fade-pop">
			<div v-if="showHallOfFame" class="arcade-cabinet">
				<div class="cabinet-screen">
					<div class="cabinet-header">
						<span class="blink">★</span>
						<span class="cabinet-title">HIGH SCORES</span>
						<span class="blink">★</span>
					</div>
					<table class="score-table">
						<tbody>
							<tr v-for="(entry, i) in highScores" :key="i">
								<td class="col-rank">{{ entry.rank }}</td>
								<td class="col-name">{{ entry.name }}</td>
								<td class="col-score">{{ entry.score }}</td>
							</tr>
						</tbody>
					</table>
					<div class="cabinet-footer">
						<span v-for="(entry, i) in highScores" :key="i" class="credit-line">
							<span class="credit-name">{{ entry.name }}</span>
							<span class="credit-note">{{ entry.note }}</span>
						</span>
					</div>
					<div class="insert-coin blink">INSERT COIN TO CONTINUE</div>
				</div>
			</div>
		</Transition>

		<!-- Links -->
		<div class="actions">
			<button class="action-link" @click="openUrl('https://github.com/luolita13/Allay')">
				<ExternalIcon class="size-4" />
				<span>{{ formatMessage(messages.viewSource) }}</span>
			</button>
			<button class="action-link" @click="openUrl('https://github.com/luolita13/Allay/issues')">
				<ExternalIcon class="size-4" />
				<span>{{ formatMessage(messages.reportIssue) }}</span>
			</button>
		</div>

		<!-- Diagnostics -->
		<ButtonStyled color="brand" size="large" :disabled="copyingDiagnostics">
			<button class="copy-diagnostics-btn" @click="copyAppDiagnostics">
				<DownloadIcon v-if="!diagnosticsExported" :class="{ 'animate-spin': copyingDiagnostics }" />
				<ClipboardCopyIcon v-else />
				{{ diagnosticsExported ? formatMessage(messages.copyDiagnosticsSuccess) : formatMessage(messages.copyAppDiagnostics) }}
			</button>
		</ButtonStyled>

		<!-- Footer -->
		<div class="footer">
			<p class="crafted">
				{{ formatMessage(messages.craftedBy) }}
				<button class="github-handle" @click="openUrl('https://github.com/luolita13')">
					github.com/luolita13
				</button>
			</p>
	</div>
</div>

	<LicenseModal v-if="showLicense" @close="showLicense = false" />
</template>

<style scoped>
.about-root {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 2.5rem;
	padding: 2rem 0 3rem;
	max-width: 28rem;
	margin: 0 auto;
	text-align: center;
	position: relative;
}

/* === Hero === */
.hero {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 1rem;
}

.allay-logo {
	width: 6rem;
	height: auto;
	filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.3));
	cursor: pointer;
	user-select: none;
}

.allay-logo.egg-spin {
	animation: spin-pulse 2s ease-in-out infinite;
}

@keyframes spin-pulse {
	0%, 100% { transform: rotate(0deg) scale(1); }
	50% { transform: rotate(180deg) scale(1.08); }
}

.app-name {
	margin: 0;
	font-size: 1.75rem;
	font-weight: 800;
	letter-spacing: -0.03em;
	color: var(--color-contrast);
}

/* === Metadata === */
.metadata {
	width: 100%;
	margin: 0;
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

.meta-row {
	display: flex;
	align-items: baseline;
	justify-content: space-between;
	gap: 1rem;
	padding: 0.6rem 0;
	border-bottom: 1px solid var(--color-button-border);
}

.meta-row:last-child {
	border-bottom: none;
}

.meta-row dt {
	font-size: 0.8rem;
	font-weight: 600;
	color: var(--color-tertiary);
	letter-spacing: 0.04em;
	text-transform: uppercase;
}

.meta-row dd {
	margin: 0;
	font-size: 0.95rem;
	font-weight: 500;
	color: var(--color-contrast);
}

.version-value {
	cursor: pointer;
	padding: 0.2rem 0.5rem;
	border-radius: 0.5rem;
	transition: background 0.15s ease;
}

.version-value:hover {
	background: color-mix(in srgb, var(--color-brand) 10%, transparent);
}

.license-clickable {
	cursor: pointer;
	padding: 0.2rem 0.5rem;
	border-radius: 0.5rem;
	color: var(--color-brand) !important;
	font-weight: 600 !important;
	transition: background 0.15s ease;
}

.license-clickable:hover {
	background: color-mix(in srgb, var(--color-brand) 10%, transparent);
	text-decoration: underline;
}

.meta-sub {
	margin-left: 0.5rem;
	font-size: 0.8rem;
	color: var(--color-tertiary);
	font-weight: 400;
}

.override-message {
	padding: 0.6rem 1.2rem;
	border-radius: 0.75rem;
	font-size: 0.85rem;
	font-weight: 700;
	letter-spacing: 0.04em;
	text-transform: uppercase;
	color: var(--color-brand);
	background: color-mix(in srgb, var(--color-brand) 12%, transparent);
	border: 1px solid color-mix(in srgb, var(--color-brand) 35%, transparent);
	box-shadow: 0 0 18px color-mix(in srgb, var(--color-brand) 25%, transparent);
}

/* === Arcade cabinet (Hall of Fame) === */
.arcade-cabinet {
	width: 100%;
	padding: 1rem;
	border-radius: 0.75rem;
	background: #0a0e0a;
	border: 2px solid var(--color-brand);
	box-shadow:
		0 0 0 4px #0a0e0a,
		0 0 0 6px color-mix(in srgb, var(--color-brand) 50%, transparent),
		0 0 24px color-mix(in srgb, var(--color-brand) 30%, transparent);
	position: relative;
	overflow: hidden;
}

/* CRT scanlines inside cabinet only */
.arcade-cabinet::before {
	content: '';
	position: absolute;
	inset: 0;
	background: repeating-linear-gradient(
		to bottom,
		transparent 0px,
		transparent 2px,
		rgba(0, 0, 0, 0.35) 3px,
		rgba(0, 0, 0, 0.35) 4px
	);
	pointer-events: none;
	z-index: 2;
}

.cabinet-screen {
	position: relative;
	z-index: 1;
	padding: 0.75rem 0.5rem;
	font-family: 'Courier New', 'Consolas', monospace;
	color: #4ade80;
	text-shadow:
		0 0 4px #4ade80,
		0 0 8px rgba(74, 222, 128, 0.5);
}

.cabinet-header {
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 0.75rem;
	font-size: 0.85rem;
	font-weight: 700;
	letter-spacing: 0.15em;
	margin-bottom: 0.75rem;
	padding-bottom: 0.5rem;
	border-bottom: 1px dashed rgba(74, 222, 128, 0.4);
}

.cabinet-title {
	color: #facc15;
	text-shadow:
		0 0 4px #facc15,
		0 0 8px rgba(250, 204, 21, 0.5);
}

.score-table {
	width: 100%;
	border-collapse: collapse;
	font-size: 0.8rem;
}

.score-table td {
	padding: 0.35rem 0.4rem;
	text-align: left;
}

.col-rank {
	width: 3rem;
	color: #facc15;
	font-weight: 700;
}

.col-name {
	color: #4ade80;
	letter-spacing: 0.05em;
	font-weight: 700;
}

.col-score {
	text-align: right;
	color: #f87171;
	font-weight: 700;
	letter-spacing: 0.1em;
}

.cabinet-footer {
	margin-top: 0.75rem;
	padding-top: 0.5rem;
	border-top: 1px dashed rgba(74, 222, 128, 0.4);
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
}

.credit-line {
	display: flex;
	justify-content: space-between;
	font-size: 0.65rem;
	color: rgba(74, 222, 128, 0.6);
	letter-spacing: 0.05em;
}

.credit-name {
	text-transform: uppercase;
}

.insert-coin {
	margin-top: 0.75rem;
	text-align: center;
	font-size: 0.7rem;
	letter-spacing: 0.2em;
	color: #facc15;
	text-shadow: 0 0 4px #facc15;
}

.blink {
	animation: blink 1s steps(2) infinite;
}

@keyframes blink {
	50% { opacity: 0.2; }
}

/* === Actions & Footer === */
.actions {
	display: flex;
	gap: 0.75rem;
	flex-wrap: wrap;
	justify-content: center;
}

.action-link {
	display: inline-flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.55rem 1.1rem;
	border-radius: 0.75rem;
	font-size: 0.85rem;
	font-weight: 600;
	color: var(--color-secondary);
	background: var(--color-button-bg);
	border: 1px solid var(--color-button-border);
	cursor: pointer;
	transition:
		color 0.2s ease,
		border-color 0.2s ease,
		transform 0.15s ease;
}

.action-link:hover {
	color: var(--color-contrast);
	border-color: var(--color-brand);
	transform: translateY(-1px);
}

.copy-diagnostics-btn {
	display: inline-flex;
	align-items: center;
	gap: 0.5rem;
	min-width: 16rem;
	justify-content: center;
}

.footer {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	margin-top: 0.5rem;
}

.crafted {
	margin: 0;
	font-size: 0.85rem;
	color: var(--color-secondary);
}

.github-handle {
	padding: 0;
	border: none;
	background: transparent;
	color: var(--color-brand);
	font-size: inherit;
	font-weight: 600;
	cursor: pointer;
}

.github-handle:hover {
	text-decoration: underline;
}

/* === Transitions === */
.fade-pop-enter-active,
.fade-pop-leave-active {
	transition:
		opacity 0.25s ease,
		transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.fade-pop-enter-from,
.fade-pop-leave-to {
	opacity: 0;
	transform: scale(0.92) translateY(0.5rem);
}
</style>
