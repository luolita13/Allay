<script setup lang="ts">
import {
	ModrinthIcon,
	SettingsIcon,
} from '@modrinth/assets'
import {
	defineMessage,
	defineMessages,
	ProgressBar,
	TabbedModal,
	useVIntl,
} from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { platform as getOsPlatform, version as getOsVersion } from '@tauri-apps/plugin-os'
import { ref, watch } from 'vue'

import { settingsTabs } from '@/helpers/settings-tabs'
import { get, set } from '@/helpers/settings.ts'
import { injectAppUpdateDownloadProgress } from '@/providers/download-progress.ts'
import { useTheming } from '@/store/state'

const themeStore = useTheming()

const { formatMessage } = useVIntl()

const devModeCounter = ref(0)

const developerModeEnabled = defineMessage({
	id: 'app.settings.developer-mode-enabled',
	defaultMessage: 'Developer mode enabled.',
})

const settingsTitle = defineMessage({
	id: 'app.settings.title',
	defaultMessage: 'Settings',
})

const tabs = settingsTabs

const modal = ref<InstanceType<typeof TabbedModal> | null>(null)

function show() {
	modal.value?.show()
}

defineExpose({ show })

const { progress, version: downloadingVersion } = injectAppUpdateDownloadProgress()

const version = await getVersion()
const osPlatform = getOsPlatform()
const osVersion = getOsVersion()
const settings = ref(await get())

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)

function devModeCount() {
	devModeCounter.value++
	if (devModeCounter.value > 5) {
		themeStore.devMode = !themeStore.devMode
		settings.value.developer_mode = !!themeStore.devMode
		devModeCounter.value = 0
	}
}

const messages = defineMessages({
	downloading: {
		id: 'app.settings.downloading',
		defaultMessage: 'Downloading v{version}',
	},
})
</script>
<template>
	<TabbedModal ref="modal" width="900px" :tabs="tabs">
		<template #title>
			<span class="flex items-center gap-2 text-lg font-extrabold text-contrast">
				<SettingsIcon /> {{ formatMessage(settingsTitle) }}
			</span>
		</template>
		<template #footer>
			<div class="mt-auto text-secondary text-sm">
				<div class="mb-3">
					<template v-if="progress > 0 && progress < 1">
						<p class="m-0 mb-2">
							{{ formatMessage(messages.downloading, { version: downloadingVersion }) }}
						</p>
						<ProgressBar :progress="progress" />
					</template>
				</div>
				<p v-if="themeStore.devMode" class="text-brand font-semibold m-0 mb-2">
					{{ formatMessage(developerModeEnabled) }}
				</p>
				<div class="flex items-center gap-3">
					<button
						class="p-0 m-0 bg-transparent border-none cursor-pointer button-animation"
						:class="{
							'text-brand': themeStore.devMode,
							'text-secondary': !themeStore.devMode,
						}"
						@click="devModeCount"
					>
						<ModrinthIcon class="w-6 h-6" />
					</button>
					<div class="max-w-[220px]">
					<p class="m-0">Modrinth App {{ version }} (Custom Edition)</p>
					<p class="m-0">
						<span v-if="osPlatform === 'macos'">macOS</span>
						<span v-else class="capitalize">{{ osPlatform }}</span>
						{{ osVersion }}
					</p>
				</div>
				</div>
			</div>
		</template>
	</TabbedModal>
</template>
