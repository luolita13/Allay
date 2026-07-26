<script setup lang="ts">
import { ExternalIcon, ModrinthIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { platform as getOsPlatform, version as getOsVersion } from '@tauri-apps/plugin-os'
import { openUrl } from '@tauri-apps/plugin-opener'
import { onMounted, onUnmounted, ref } from 'vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	appName: {
		id: 'app.about.app-name',
		defaultMessage: 'Modrinth App',
	},
	edition: {
		id: 'app.about.edition',
		defaultMessage: 'Community Custom Edition',
	},
	craftedBy: {
		id: 'app.about.crafted-by',
		defaultMessage: 'Crafted by',
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
		defaultMessage: 'View source',
	},
	reportIssue: {
		id: 'app.about.report-issue',
		defaultMessage: 'Report an issue',
	},
	legalNotice: {
		id: 'app.about.legal-notice',
		defaultMessage:
			'This is an unofficial third-party customization of the open-source Modrinth App. Modrinth and its trademarks are the property of Rinth, Inc.',
	},
	overrideAuthorized: {
		id: 'app.about.easter-egg.override-authorized',
		defaultMessage: 'System override authorized',
	},
	hallOfFameTitle: {
		id: 'app.about.easter-egg.hall-of-fame-title',
		defaultMessage: 'Hall of Fame',
	},
	hallOfFameDescription: {
		id: 'app.about.easter-egg.hall-of-fame-description',
		defaultMessage: 'The architects behind this customized edition:',
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

// Konami state
const konamiActive = ref(false)
const shakeClass = ref('')
let shakeTimer: ReturnType<typeof setTimeout> | null = null

const konamiSequence = ['arrowup', 'arrowup', 'arrowdown', 'arrowdown', 'arrowleft', 'arrowright', 'arrowleft', 'arrowright', 'b', 'a']
const konamiBuffer = ref<string[]>([])

// Arcade-style high score table — replaces the AI-ish card grid
const highScores = [
	{ rank: '1ST', name: 'LUOLITA13', score: '999999', note: 'Lead Customizer' },
	{ rank: '2ND', name: 'MODRINTH TEAM', score: '750000', note: 'Original Authors' },
	{ rank: '3RD', name: 'THE STACK', score: '500000', note: 'Tauri + Vue + Rust' },
	{ rank: '4TH', name: 'YOU', score: '000000', note: 'The Player' },
]

function onLogoClick() {
	clickCount.value++
	console.log('[About Easter Egg] logo click:', clickCount.value)
	triggerEasterEgg()
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

function triggerShake(direction: 'x' | 'y') {
	if (shakeTimer) clearTimeout(shakeTimer)
	shakeClass.value = direction === 'x' ? 'shake-x' : 'shake-y'
	shakeTimer = setTimeout(() => {
		shakeClass.value = ''
	}, 400)
}

function onKeyDown(e: KeyboardEvent) {
	const key = e.key.toLowerCase()
	konamiBuffer.value.push(key)
	konamiBuffer.value = konamiBuffer.value.slice(-konamiSequence.length)

	console.log('[About Konami] buffer:', konamiBuffer.value.join(','))

	// Shake the UI on each arrow press during Konami input
	if (key === 'arrowleft' || key === 'arrowright') {
		triggerShake('x')
	} else if (key === 'arrowup' || key === 'arrowdown') {
		triggerShake('y')
	}

	if (konamiBuffer.value.join(',') === konamiSequence.join(',')) {
		console.log('[About Konami] activated')
		konamiActive.value = true
		konamiBuffer.value = []
		// Stay activated until user dismisses
	}
}

function dismissKonami() {
	konamiActive.value = false
}

onMounted(() => {
	window.addEventListener('keydown', onKeyDown)
})

onUnmounted(() => {
	window.removeEventListener('keydown', onKeyDown)
	if (shakeTimer) clearTimeout(shakeTimer)
})
</script>

<template>
	<div
		class="about-root"
		:class="{
			[shakeClass]: shakeClass,
		}"
	>
		<!-- Hero -->
		<div class="hero">
			<div
				class="logo-glow"
				:class="{
					'egg-spin': clickCount >= 5 && clickCount < 15,
					'egg-pulse': clickCount >= 5,
				}"
			>
				<ModrinthIcon class="logo-icon" title="Psst... try clicking me" @click="onLogoClick" />
			</div>
			<h1 class="app-name">{{ formatMessage(messages.appName) }}</h1>
			<div class="edition-badge">{{ formatMessage(messages.edition) }}</div>
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
				<dd>{{ formatMessage(messages.licenseValue) }}</dd>
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
			<button class="action-link" @click="openUrl('https://github.com/luolita13')">
				<ExternalIcon class="size-4" />
				<span>{{ formatMessage(messages.viewSource) }}</span>
			</button>
			<button class="action-link" @click="openUrl('https://github.com/luolita13/code/issues')">
				<ExternalIcon class="size-4" />
				<span>{{ formatMessage(messages.reportIssue) }}</span>
			</button>
		</div>

		<!-- Footer -->
		<div class="footer">
			<p class="crafted">
				{{ formatMessage(messages.craftedBy) }}
				<button class="github-handle" @click="openUrl('https://github.com/luolita13')">
					github.com/luolita13
				</button>
			</p>
			<p class="legal">{{ formatMessage(messages.legalNotice) }}</p>
		</div>

		<!-- Konami overlay — arcade "PLAYER ONE READY" -->
		<Transition name="konami-in">
			<div v-if="konamiActive" class="konami-overlay" @click="dismissKonami">
				<div class="konami-arcade">
					<!-- Pixel spaceship (SVG) -->
					<svg class="pixel-ship" viewBox="0 0 32 32" shape-rendering="crispEdges" aria-hidden="true">
						<g fill="currentColor">
							<rect x="14" y="4" width="4" height="2" />
							<rect x="12" y="6" width="8" height="2" />
							<rect x="10" y="8" width="12" height="2" />
							<rect x="8" y="10" width="16" height="4" />
							<rect x="4" y="14" width="24" height="4" />
							<rect x="2" y="18" width="28" height="2" />
							<rect x="6" y="20" width="4" height="2" />
							<rect x="22" y="20" width="4" height="2" />
							<rect x="14" y="20" width="4" height="4" />
						</g>
					</svg>

					<div class="arcade-line blink">PLAYER ONE</div>
					<div class="arcade-line blink" style="animation-delay: 0.3s">READY</div>
					<div class="arcade-cheat">30 LIVES — CHEAT ACTIVATED</div>
					<div class="arcade-hint">CLICK ANYWHERE TO EXIT</div>
				</div>
			</div>
		</Transition>
	</div>
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

/* === Shake animations on Konami arrow keys === */
@keyframes shake-x {
	0%, 100% { transform: translateX(0); }
	15% { transform: translateX(-10px); }
	30% { transform: translateX(8px); }
	45% { transform: translateX(-6px); }
	60% { transform: translateX(4px); }
	75% { transform: translateX(-2px); }
}

@keyframes shake-y {
	0%, 100% { transform: translateY(0); }
	15% { transform: translateY(-8px); }
	30% { transform: translateY(6px); }
	45% { transform: translateY(-4px); }
	60% { transform: translateY(3px); }
	75% { transform: translateY(-1px); }
}

.about-root.shake-x {
	animation: shake-x 0.4s ease-in-out;
}

.about-root.shake-y {
	animation: shake-y 0.4s ease-in-out;
}

/* === Hero === */
.hero {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 1rem;
}

.logo-glow {
	position: relative;
	display: flex;
	align-items: center;
	justify-content: center;
	width: 6rem;
	height: 6rem;
	border-radius: 50%;
	background: radial-gradient(
		circle at 30% 30%,
		color-mix(in srgb, var(--color-brand) 70%, white),
		var(--color-brand) 45%,
		transparent 72%
	);
}

.logo-glow.egg-spin {
	animation: spin-pulse 2s ease-in-out infinite;
}

.logo-glow.egg-pulse::after {
	content: '';
	position: absolute;
	inset: -30%;
	border-radius: 50%;
	background: radial-gradient(circle, var(--color-brand) 0%, transparent 60%);
	opacity: 0.3;
	filter: blur(12px);
	animation: neon-pulse 1.2s ease-in-out infinite;
	z-index: -1;
}

@keyframes spin-pulse {
	0%, 100% { transform: rotate(0deg) scale(1); }
	50% { transform: rotate(180deg) scale(1.08); }
}

@keyframes neon-pulse {
	0%, 100% { opacity: 0.2; transform: scale(0.95); }
	50% { opacity: 0.45; transform: scale(1.05); }
}

.logo-glow::before {
	content: '';
	position: absolute;
	inset: -20%;
	border-radius: 50%;
	background: radial-gradient(circle, var(--color-brand) 0%, transparent 65%);
	opacity: 0.18;
	filter: blur(18px);
	z-index: -1;
}

.logo-icon {
	width: 3rem;
	height: 3rem;
	color: white;
	filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.25));
	cursor: pointer;
	user-select: none;
}

.app-name {
	margin: 0;
	font-size: 1.75rem;
	font-weight: 800;
	letter-spacing: -0.03em;
	color: var(--color-contrast);
}

.edition-badge {
	padding: 0.35rem 0.85rem;
	border-radius: 999px;
	font-size: 0.7rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--color-brand);
	background: color-mix(in srgb, var(--color-brand) 12%, transparent);
	border: 1px solid color-mix(in srgb, var(--color-brand) 25%, transparent);
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

.legal {
	margin: 0;
	font-size: 0.7rem;
	line-height: 1.6;
	color: var(--color-tertiary);
	max-width: 22rem;
}

/* === Konami overlay === */
.konami-overlay {
	position: fixed;
	inset: 0;
	z-index: 1000;
	display: flex;
	align-items: center;
	justify-content: center;
	background: radial-gradient(circle at center, rgba(0, 0, 0, 0.85) 0%, rgba(0, 0, 0, 0.98) 100%);
	cursor: pointer;
	backdrop-filter: blur(2px);
}

.konami-arcade {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 1rem;
	padding: 2rem;
	font-family: 'Courier New', 'Consolas', monospace;
	color: #4ade80;
	text-shadow:
		0 0 6px #4ade80,
		0 0 12px rgba(74, 222, 128, 0.6);
	animation: arcade-flicker 3s infinite;
}

@keyframes arcade-flicker {
	0%, 100% { opacity: 1; }
	48% { opacity: 1; }
	50% { opacity: 0.85; }
	52% { opacity: 1; }
}

.pixel-ship {
	width: 4rem;
	height: 4rem;
	color: #facc15;
	filter:
		drop-shadow(0 0 6px #facc15)
		drop-shadow(0 0 12px rgba(250, 204, 21, 0.5));
	animation: ship-bob 1.2s ease-in-out infinite;
}

@keyframes ship-bob {
	0%, 100% { transform: translateY(0); }
	50% { transform: translateY(-6px); }
}

.arcade-line {
	font-size: 1.5rem;
	font-weight: 700;
	letter-spacing: 0.3em;
}

.arcade-cheat {
	font-size: 1rem;
	font-weight: 700;
	letter-spacing: 0.15em;
	color: #f87171;
	text-shadow:
		0 0 6px #f87171,
		0 0 12px rgba(248, 113, 113, 0.6);
	margin-top: 0.5rem;
}

.arcade-hint {
	margin-top: 1.5rem;
	font-size: 0.7rem;
	letter-spacing: 0.2em;
	color: rgba(74, 222, 128, 0.6);
	text-shadow: none;
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

.konami-in-enter-active {
	transition: opacity 0.2s ease;
}

.konami-in-leave-active {
	transition: opacity 0.3s ease;
}

.konami-in-enter-from,
.konami-in-leave-to {
	opacity: 0;
}
</style>
