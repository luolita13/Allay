<script setup lang="ts">
import {
	commonMessages,
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
</script>

<template>
	<div class="flex h-full">
		<!-- Sidebar -->
		<nav class="w-52 flex-shrink-0 overflow-y-auto border-r border-surface-5 bg-surface-1 p-3">
			<div class="mb-4 pl-2">
				<h2 class="text-lg font-extrabold text-contrast">Settings</h2>
			</div>
			<button
				v-for="(tab, i) in settingsTabs"
				:key="i"
				class="mb-0.5 flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm transition-colors"
				:class="
					activeTab === i
						? 'bg-brand/10 text-brand font-medium'
						: 'text-secondary hover:bg-surface-3 hover:text-contrast'
				"
				@click="activeTab = i"
			>
				<component :is="tab.icon" class="size-4 flex-shrink-0" />
				<span class="truncate">{{ formatMessage(tab.name) }}</span>
				<span
					v-if="tab.badge"
					class="ml-auto flex-shrink-0 rounded-full bg-surface-4 px-1.5 py-0.5 text-xs font-medium text-tertiary"
				>
					{{ formatMessage(tab.badge) }}
				</span>
			</button>
		</nav>

		<!-- Content -->
		<div class="flex-1 overflow-y-auto p-6">
			<div class="mx-auto max-w-2xl">
				<h2 class="mb-6 text-xl font-extrabold text-contrast">
					{{ formatMessage(settingsTabs[activeTab].name) }}
				</h2>
				<Suspense>
					<Transition name="tab-fade" mode="out-in">
						<component :is="TabContent" :key="activeTab" />
					</Transition>
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

<style>
.tab-fade-enter-active,
.tab-fade-leave-active {
	transition:
		opacity 0.2s ease,
		transform 0.2s ease;
}

.tab-fade-enter-from,
.tab-fade-leave-to {
	opacity: 0;
	transform: translateX(0.25rem);
}

@media (prefers-reduced-motion: reduce) {
	.tab-fade-enter-active,
	.tab-fade-leave-active {
		transition: none;
	}

	.tab-fade-enter-from,
	.tab-fade-leave-to {
		opacity: 1;
		transform: none;
	}
}
</style>
