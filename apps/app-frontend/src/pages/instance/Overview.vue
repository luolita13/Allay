<template>
	<div v-if="instance" class="flex flex-col gap-6">
		<ExportModal ref="exportModal" :instance="instance" />
		<ConfirmDeleteInstanceModal ref="deleteInstanceModal" @delete="doDeleteInstance" />
		<!-- Instance information card -->
		<Card>
			<template #header>
				<h2 class="text-xl font-bold text-contrast m-0">
					{{ formatMessage(messages.information) }}
				</h2>
			</template>
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 p-2">
				<div class="info-item">
					<span class="info-label">{{ formatMessage(messages.gameVersion) }}</span>
					<span class="info-value">{{ instance.game_version }}</span>
				</div>
				<div class="info-item">
					<span class="info-label">{{ formatMessage(messages.loader) }}</span>
					<span class="info-value capitalize">{{ instance.loader }}</span>
				</div>
				<div v-if="instance.loader_version" class="info-item">
					<span class="info-label">{{ formatMessage(messages.loaderVersion) }}</span>
					<span class="info-value truncate" :title="instance.loader_version">
						{{ instance.loader_version }}
					</span>
				</div>
				<div class="info-item">
					<span class="info-label">{{ formatMessage(messages.installStage) }}</span>
					<span class="info-value" :class="installStageClass">
						{{ formatInstallStage(instance.install_stage) }}
					</span>
				</div>
				<div class="info-item">
					<span class="info-label">{{ formatMessage(messages.created) }}</span>
					<span class="info-value">{{ formatDate(instance.created) }}</span>
				</div>
				<div v-if="instance.last_played" class="info-item">
					<span class="info-label">{{ formatMessage(messages.lastPlayed) }}</span>
					<span class="info-value">{{ formatDate(instance.last_played) }}</span>
				</div>
				<div class="info-item">
					<span class="info-label">{{ formatMessage(messages.playTime) }}</span>
					<span class="info-value">{{ formatPlayTime }}</span>
				</div>
				<div v-if="instance.link?.type" class="info-item">
					<span class="info-label">{{ formatMessage(messages.linkType) }}</span>
					<span class="info-value">{{ formatLinkType(instance.link.type) }}</span>
				</div>
				<div v-if="instance.update_channel" class="info-item">
					<span class="info-label">{{ formatMessage(messages.updateChannel) }}</span>
					<span class="info-value capitalize">{{ instance.update_channel }}</span>
				</div>
			</div>
		</Card>
	</div>
</template>

<script setup lang="ts">
import { Card, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, ref } from 'vue'

import type ContextMenu from '@/components/ui/ContextMenu.vue'
import ExportModal from '@/components/ui/ExportModal.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import { remove } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

dayjs.extend(relativeTime)

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	information: { id: 'app.instance.overview.information', defaultMessage: 'Information' },
	gameVersion: { id: 'app.instance.overview.game-version', defaultMessage: 'Game version' },
	loader: { id: 'app.instance.overview.loader', defaultMessage: 'Loader' },
	loaderVersion: { id: 'app.instance.overview.loader-version', defaultMessage: 'Loader version' },
	installStage: { id: 'app.instance.overview.install-stage', defaultMessage: 'Status' },
	created: { id: 'app.instance.overview.created', defaultMessage: 'Created' },
	lastPlayed: { id: 'app.instance.overview.last-played', defaultMessage: 'Last played' },
	playTime: { id: 'app.instance.overview.play-time', defaultMessage: 'Play time' },
	linkType: { id: 'app.instance.overview.link-type', defaultMessage: 'Source' },
	updateChannel: { id: 'app.instance.overview.update-channel', defaultMessage: 'Update channel' },
	installed: { id: 'app.instance.overview.status.installed', defaultMessage: 'Installed' },
	notInstalled: {
		id: 'app.instance.overview.status.not-installed',
		defaultMessage: 'Not installed',
	},
	installing: { id: 'app.instance.overview.status.installing', defaultMessage: 'Installing' },
	packInstalled: {
		id: 'app.instance.overview.status.pack-installed',
		defaultMessage: 'Modpack installed',
	},
	packInstalling: {
		id: 'app.instance.overview.status.pack-installing',
		defaultMessage: 'Modpack installing',
	},
	minecraftInstalling: {
		id: 'app.instance.overview.status.minecraft-installing',
		defaultMessage: 'Minecraft installing',
	},
	neverPlayed: { id: 'app.instance.overview.never-played', defaultMessage: 'Never played' },
	modrinthModpack: {
		id: 'app.instance.overview.link-type.modrinth-modpack',
		defaultMessage: 'Modrinth modpack',
	},
	serverProject: {
		id: 'app.instance.overview.link-type.server-project',
		defaultMessage: 'Server project',
	},
	serverModpack: {
		id: 'app.instance.overview.link-type.server-modpack',
		defaultMessage: 'Server modpack',
	},
	importedModpack: {
		id: 'app.instance.overview.link-type.imported-modpack',
		defaultMessage: 'Imported modpack',
	},
	hosting: { id: 'app.instance.overview.link-type.hosting', defaultMessage: 'Hosting' },
	sharedInstance: {
		id: 'app.instance.overview.link-type.shared-instance',
		defaultMessage: 'Shared instance',
	},
})

const props = defineProps<{
	instance: GameInstance
	options: InstanceType<typeof ContextMenu>
	offline: boolean
	playing: boolean
	installed: boolean
	isServerInstance?: boolean
	openSettings?: () => void
}>()

defineEmits<{
	(event: 'play' | 'stop'): void
}>()

const instance = computed(() => props.instance)
const exportModal = ref<InstanceType<typeof ExportModal>>()
const deleteInstanceModal = ref<InstanceType<typeof ConfirmDeleteInstanceModal>>()
const currentDeleteInstance = ref<string | null>(null)

async function doDeleteInstance() {
	if (currentDeleteInstance.value) {
		await remove(currentDeleteInstance.value).catch(handleError)
	}
}

const formatPlayTime = computed(() => {
	const total = instance.value
		? instance.value.recent_time_played + instance.value.submitted_time_played
		: 0
	if (total <= 0) return formatMessage(messages.neverPlayed)
	const hours = Math.floor(total / 3600)
	const minutes = Math.floor((total % 3600) / 60)
	if (hours >= 1) return `${hours}h ${minutes}m`
	return `${minutes}m`
})

const installStageClass = computed(() => {
	const stage = instance.value?.install_stage
	if (stage === 'installed') return 'text-green'
	if (stage === 'not_installed') return 'text-red'
	return 'text-orange'
})

function formatDate(date: Date | string): string {
	if (!date) return '—'
	return dayjs(date).format('YYYY-MM-DD HH:mm')
}

function formatInstallStage(stage: string): string {
	const map: Record<string, typeof messages.installed> = {
		installed: messages.installed,
		not_installed: messages.notInstalled,
		minecraft_installing: messages.minecraftInstalling,
		pack_installed: messages.packInstalled,
		pack_installing: messages.packInstalling,
	}
	return formatMessage(map[stage] ?? messages.installed)
}

function formatLinkType(type: string): string {
	const labels: Record<string, ReturnType<typeof formatMessage>> = {
		modrinth_modpack: formatMessage(messages.modrinthModpack),
		server_project: formatMessage(messages.serverProject),
		server_project_modpack: formatMessage(messages.serverModpack),
		imported_modpack: formatMessage(messages.importedModpack),
		moderation_hosting: formatMessage(messages.hosting),
		shared_instance: formatMessage(messages.sharedInstance),
	}
	return labels[type] ?? type
}
</script>

<style scoped lang="scss">
.info-item {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	padding: 0.5rem 0.75rem;
	border-radius: 0.5rem;
	background: var(--color-surface-2);
	border: 1px solid var(--color-surface-5);
}

.info-label {
	font-size: 0.75rem;
	color: var(--color-secondary);
	text-transform: uppercase;
	letter-spacing: 0.05em;
}

.info-value {
	font-size: 0.95rem;
	font-weight: 600;
	color: var(--color-contrast);
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}
</style>
