<script setup lang="ts">
import { CheckIcon, ExternalIcon, PackageIcon, TrashIcon, UploadIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { open, save } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, ref } from 'vue'
import { exportToZip, getThemesDirPath } from '@/helpers/theme_pack'
import { openPath } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import type { InstalledThemePack } from '@/helpers/theme_pack'

const themeStore = useTheming()
const { formatMessage } = useVIntl()

const installing = ref(false)
const error = ref<string | null>(null)

const messages = defineMessages({
	sectionTitle: {
		id: 'app.appearance-settings.theme-pack.title',
		defaultMessage: 'Theme Packs',
	},
	sectionDescription: {
		id: 'app.appearance-settings.theme-pack.description',
		defaultMessage:
			'Install reusable theme packs to switch backgrounds, accent colors, and fonts in one click. Theme pack files are zip archives containing a modrinth-theme.json manifest.',
	},
	installButton: {
		id: 'app.appearance-settings.theme-pack.install',
		defaultMessage: 'Install Theme Pack',
	},
	openFolder: {
		id: 'app.appearance-settings.theme-pack.open-folder',
		defaultMessage: 'Open Themes Folder',
	},
	noPacks: {
		id: 'app.appearance-settings.theme-pack.empty',
		defaultMessage: 'No theme packs installed.',
	},
	activeBadge: {
		id: 'app.appearance-settings.theme-pack.active',
		defaultMessage: 'Active',
	},
	activateLabel: {
		id: 'app.appearance-settings.theme-pack.activate',
		defaultMessage: 'Activate',
	},
	deactivateLabel: {
		id: 'app.appearance-settings.theme-pack.deactivate',
		defaultMessage: 'Deactivate',
	},
	exportLabel: {
		id: 'app.appearance-settings.theme-pack.export',
		defaultMessage: 'Export',
	},
	deleteLabel: {
		id: 'app.appearance-settings.theme-pack.delete',
		defaultMessage: 'Delete',
	},
	byAuthor: {
		id: 'app.appearance-settings.theme-pack.by-author',
		defaultMessage: 'By: ',
	},
})

onMounted(async () => {
	await themeStore.refreshInstalledThemePacks()
	// Re-apply the active pack now that installed list is fresh.
	await themeStore.applyActiveThemePack()
})

const packs = computed<InstalledThemePack[]>(() => themeStore.installedThemePacks)
const activeId = computed<string | null>(() => themeStore.activeThemePackId)

async function pickAndInstall() {
	error.value = null
	const selected = await open({
		multiple: false,
		filters: [{ name: 'Theme pack (.zip)', extensions: ['zip'] }],
	})
	if (!selected || typeof selected !== 'string') return
	installing.value = true
	try {
		await themeStore.installThemePackFromPath(selected)
	} catch (e) {
		error.value = String(e)
	} finally {
		installing.value = false
	}
}

async function activate(pack: InstalledThemePack) {
	await themeStore.setActiveThemePack(pack.id)
}

async function deactivate() {
	await themeStore.setActiveThemePack(null)
}

async function remove(pack: InstalledThemePack) {
	if (activeId.value === pack.id) {
		await deactivate()
	}
	await themeStore.uninstallThemePackById(pack.id)
}

async function exportPack(pack: InstalledThemePack) {
	const dest = await save({
		defaultPath: `${pack.id}.zip`,
		filters: [{ name: 'Theme pack (.zip)', extensions: ['zip'] }],
	})
	if (!dest || typeof dest !== 'string') return
	await exportToZip(pack.id, dest)
}

async function openThemesFolder() {
	try {
		const dir = await getThemesDirPath()
		await openPath(dir)
	} catch (e) {
		error.value = String(e)
	}
}
</script>

<template>
	<div class="mt-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.sectionTitle) }}
		</h2>
		<p class="m-0 mt-1">{{ formatMessage(messages.sectionDescription) }}</p>

		<div class="mt-3 flex flex-wrap gap-2">
			<ButtonStyled :disabled="installing">
				<button type="button" @click="pickAndInstall">
					<UploadIcon />
					{{ formatMessage(messages.installButton) }}
				</button>
			</ButtonStyled>
			<ButtonStyled type="transparent">
				<button type="button" @click="openThemesFolder">
					<ExternalIcon />
					{{ formatMessage(messages.openFolder) }}
				</button>
			</ButtonStyled>
		</div>

		<div v-if="error" class="error-banner">{{ error }}</div>

		<div v-if="packs.length === 0" class="empty-state">
			<PackageIcon class="size-8" />
			<span>{{ formatMessage(messages.noPacks) }}</span>
		</div>

		<div v-else class="pack-list">
			<div
				v-for="pack in packs"
				:key="pack.id"
				class="pack-card"
				:class="{ active: pack.id === activeId }"
			>
				<div class="pack-header">
					<div class="pack-titles">
						<div class="pack-name">{{ pack.name }}</div>
						<div class="pack-meta">
							<span v-if="pack.author">
								{{ formatMessage(messages.byAuthor) }} {{ pack.author }}
							</span>
							<span v-if="pack.version">v{{ pack.version }}</span>
							<span class="pack-id">{{ pack.id }}</span>
						</div>
					</div>
					<span v-if="pack.id === activeId" class="active-badge">
						<CheckIcon class="size-3.5" />
						{{ formatMessage(messages.activeBadge) }}
					</span>
				</div>

				<div v-if="pack.description" class="pack-description">
					{{ pack.description }}
				</div>

				<div class="pack-tags">
					<span v-if="pack.accent_color" class="pack-tag" :style="{
						backgroundColor: pack.accent_color,
						color: '#fff',
					}">accent</span>
					<span v-if="pack.background_image_path" class="pack-tag">background</span>
					<span v-if="pack.font_family" class="pack-tag">font</span>
					<span
						v-if="pack.css_variables && Object.keys(pack.css_variables).length"
						class="pack-tag"
					>+{{ Object.keys(pack.css_variables).length }} vars</span>
				</div>

				<div class="pack-actions">
					<ButtonStyled
						v-if="pack.id !== activeId"
						color="brand"
						type="outlined"
					>
						<button type="button" @click="activate(pack)">
							{{ formatMessage(messages.activateLabel) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-else color="red" type="outlined">
						<button type="button" @click="deactivate">
							{{ formatMessage(messages.deactivateLabel) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button type="button" @click="exportPack(pack)">
							{{ formatMessage(messages.exportLabel) }}
						</button>
					</ButtonStyled>
					<ButtonStyled color="red" type="transparent">
						<button type="button" @click="remove(pack)">
							<TrashIcon />
							{{ formatMessage(messages.deleteLabel) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
		</div>
	</div>
</template>

<style scoped lang="scss">
.error-banner {
	margin-top: 0.75rem;
	background: var(--color-red-bg, rgba(239, 68, 68, 0.12));
	border: 1px solid var(--color-red);
	color: var(--color-red);
	border-radius: 0.5rem;
	padding: 0.5rem 0.75rem;
	font-size: 0.8125rem;
	word-break: break-word;
}

.empty-state {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	gap: 0.5rem;
	padding: 2rem 1rem;
	color: var(--color-text-secondary);
	margin-top: 0.75rem;
	border: 1px dashed var(--color-divider);
	border-radius: 0.75rem;
}

.pack-list {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
	gap: 0.75rem;
	margin-top: 0.75rem;
}

.pack-card {
	border: 1px solid var(--color-divider);
	border-radius: 0.75rem;
	padding: 0.875rem 1rem;
	background: var(--color-bg);
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	transition: border-color 0.15s, box-shadow 0.15s;
}

.pack-card.active {
	border-color: var(--color-brand);
	box-shadow: 0 0 0 1px var(--color-brand);
}

.pack-header {
	display: flex;
	justify-content: space-between;
	align-items: flex-start;
	gap: 0.5rem;
}

.pack-titles {
	min-width: 0;
}

.pack-name {
	font-weight: 600;
	color: var(--color-text);
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.pack-meta {
	display: flex;
	flex-wrap: wrap;
	gap: 0.5rem;
	font-size: 0.75rem;
	color: var(--color-text-secondary);
	margin-top: 0.125rem;
}

.pack-id {
	font-family: var(--mono-font, monospace);
	opacity: 0.7;
}

.active-badge {
	display: inline-flex;
	align-items: center;
	gap: 0.25rem;
	background: var(--color-brand);
	color: white;
	border-radius: 999px;
	padding: 0.125rem 0.5rem;
	font-size: 0.6875rem;
	font-weight: 600;
	flex-shrink: 0;
}

.pack-description {
	font-size: 0.8125rem;
	color: var(--color-text);
	line-height: 1.45;
}

.pack-tags {
	display: flex;
	flex-wrap: wrap;
	gap: 0.25rem;
}

.pack-tag {
	background: var(--color-bg);
	border: 1px solid var(--color-divider);
	border-radius: 0.25rem;
	padding: 0.0625rem 0.375rem;
	font-size: 0.6875rem;
	color: var(--color-text);
	text-transform: uppercase;
	letter-spacing: 0.04em;
	font-weight: 600;
}

.pack-actions {
	display: flex;
	flex-wrap: wrap;
	gap: 0.375rem;
	margin-top: 0.25rem;
}
</style>
