<script setup lang="ts">
import {
	defineMessages,
	ProgressBar,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { settingsTabs } from '@/helpers/settings-tabs'
import { injectAppUpdateDownloadProgress } from '@/providers/download-progress'
import { useTheming } from '@/store/state'

const themeStore = useTheming()
const { formatMessage } = useVIntl()

const activeTab = ref(0)

const TabContent = computed(() => settingsTabs[activeTab.value]?.content ?? null)

const { progress, version: downloadingVersion } = injectAppUpdateDownloadProgress()

const messages = defineMessages({
	settingsTitle: {
		id: 'app.settings.title',
		defaultMessage: 'Settings',
	},
	downloading: {
		id: 'app.settings.downloading-version',
		defaultMessage: 'Downloading v{version}',
	},
})
</script>

<template>
	<div class="flex h-full">
		<!-- Sidebar -->
		<nav class="settings-sidebar w-72 flex-shrink-0 overflow-y-auto border-r border-surface-5 bg-surface-1/75 p-4">
			<div class="mb-5 pl-1">
				<h2 class="text-xl font-extrabold text-contrast">{{ formatMessage(messages.settingsTitle) }}</h2>
			</div>
			<button
				v-for="(tab, i) in settingsTabs"
				:key="i"
				class="settings-tab mb-1 flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm transition-colors"
				:class="
					activeTab === i
						? 'bg-brand/10 text-brand font-medium'
						: 'text-secondary hover:bg-surface-3 hover:text-contrast'
				"
				@click="activeTab = i"
			>
				<component :is="tab.icon" class="size-[18px] flex-shrink-0" />
				<span class="truncate">{{ formatMessage(tab.name) }}</span>
				<span
					v-if="tab.badge"
					class="ml-auto flex-shrink-0 rounded-full bg-surface-4 px-2 py-0.5 text-xs font-medium text-tertiary"
				>
					{{ formatMessage(tab.badge) }}
				</span>
			</button>
		</nav>

		<!-- Content -->
		<div class="flex-1 overflow-y-auto p-8">
			<div class="mx-auto max-w-2xl">
				<h2 class="mb-6 text-2xl font-extrabold text-contrast">
					{{ formatMessage(settingsTabs[activeTab].name) }}
				</h2>
				<Suspense>
					<component :is="TabContent" />
					<template #fallback>
						<div class="flex items-center justify-center py-12">
							<div class="size-6 animate-spin rounded-full border-2 border-brand border-t-transparent" />
						</div>
					</template>
				</Suspense>
			</div>

			<!-- Footer -->
			<div class="mx-auto mt-12 max-w-2xl">
					<div v-if="progress > 0 && progress < 1" class="border-t border-surface-5 pt-4">
						<p class="m-0 mb-2 text-sm text-secondary">
							Downloading v{{ downloadingVersion }}
						</p>
						<ProgressBar :progress="progress" />
					</div>
				</div>
		</div>
	</div>
</template>

<style scoped>
.settings-sidebar {
	font-size: 0.875rem;
}

.settings-tab {
	font-size: 0.9rem;
}
</style>
