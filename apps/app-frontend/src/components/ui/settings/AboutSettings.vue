<script setup lang="ts">
import { ExternalIcon, ModrinthIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { platform as getOsPlatform, version as getOsVersion } from '@tauri-apps/plugin-os'
import { openUrl } from '@tauri-apps/plugin-opener'

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
})

const version = await getVersion()
const osPlatform = getOsPlatform()
const osVersion = getOsVersion()

const platformLabel =
	osPlatform === 'macos' ? 'macOS' : osPlatform.charAt(0).toUpperCase() + osPlatform.slice(1)
</script>

<template>
	<div class="about-root">
		<!-- Hero -->
		<div class="hero">
			<div class="logo-glow">
				<ModrinthIcon class="logo-icon" />
			</div>
			<h1 class="app-name">{{ formatMessage(messages.appName) }}</h1>
			<div class="edition-badge">{{ formatMessage(messages.edition) }}</div>
		</div>

		<!-- Metadata -->
		<dl class="metadata">
			<div class="meta-row">
				<dt>{{ formatMessage(messages.version) }}</dt>
				<dd>{{ version }}</dd>
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

		<!-- Links -->
		<div class="actions">
			<button class="action-link" @click="openUrl('https://github.com/luolita13')">
				<ExternalIcon class="size-4" />
				<span>{{ formatMessage(messages.viewSource) }}</span>
			</button>
			<button class="action-link" @click="openUrl('https://github.com/luolita13/issues')">
				<ExternalIcon class="size-4" />
				<span>{{ formatMessage(messages.reportIssue) }}</span>
			</button>
		</div>

		<!-- Footer -->
		<div class="footer">
			<p class="crafted">
				{{ formatMessage(messages.craftedBy) }}
				<button
					class="github-handle"
					@click="openUrl('https://github.com/luolita13')"
				>
					github.com/luolita13
				</button>
			</p>
			<p class="legal">{{ formatMessage(messages.legalNotice) }}</p>
		</div>
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
}

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

.meta-sub {
	margin-left: 0.5rem;
	font-size: 0.8rem;
	color: var(--color-tertiary);
	font-weight: 400;
}

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
</style>
