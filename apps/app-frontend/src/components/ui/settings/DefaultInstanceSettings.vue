<script setup lang="ts">
import {
	Combobox,
	defineMessages,
	injectNotificationManager,
	Slider,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import useMemorySlider from '@/composables/useMemorySlider'
import { get_max_memory } from '@/helpers/jre.js'
import { get, set } from '@/helpers/settings.ts'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	fullscreenTitle: {
		id: 'app.default-instance.fullscreen.title',
		defaultMessage: 'Fullscreen',
	},
	fullscreenDescription: {
		id: 'app.default-instance.fullscreen.description',
		defaultMessage: 'Overrides the options.txt file to run in fullscreen mode at launch.',
	},
	widthTitle: {
		id: 'app.default-instance.width.title',
		defaultMessage: 'Width',
	},
	widthDescription: {
		id: 'app.default-instance.width.description',
		defaultMessage: 'The width of the game window at launch.',
	},
	widthPlaceholder: {
		id: 'app.default-instance.width.placeholder',
		defaultMessage: 'Enter width...',
	},
	heightTitle: {
		id: 'app.default-instance.height.title',
		defaultMessage: 'Height',
	},
	heightDescription: {
		id: 'app.default-instance.height.description',
		defaultMessage: 'The height of the game window at launch.',
	},
	heightPlaceholder: {
		id: 'app.default-instance.height.placeholder',
		defaultMessage: 'Enter height...',
	},
	memoryAllocationTitle: {
		id: 'app.default-instance.memory-allocation.title',
		defaultMessage: 'Memory Allocation',
	},
	memoryAllocationDescription: {
		id: 'app.default-instance.memory-allocation.description',
		defaultMessage: 'Choose how memory is allocated for instances.',
	},
	memoryModeAuto: {
		id: 'app.default-instance.memory-mode.auto',
		defaultMessage: 'Auto',
	},
	memoryModeCustom: {
		id: 'app.default-instance.memory-mode.custom',
		defaultMessage: 'Custom',
	},
	memoryAllocated: {
		id: 'app.default-instance.memory-allocated',
		defaultMessage: 'Allocated Memory',
	},
	memoryWarning: {
		id: 'app.default-instance.memory-warning',
		defaultMessage:
			'You are allocating more than 75% of your system memory to Minecraft. This may cause system instability. Using Auto mode is recommended.',
	},
	memoryAutoHint: {
		id: 'app.default-instance.memory-auto-hint',
		defaultMessage:
			'Memory will be automatically calculated based on system RAM, instance type, and mod count.',
	},
	launchOptionsTitle: {
		id: 'app.default-instance.launch-options.title',
		defaultMessage: 'Launch Options',
	},
	processPriorityTitle: {
		id: 'app.default-instance.process-priority.title',
		defaultMessage: 'Process Priority',
	},
	processPriorityDescription: {
		id: 'app.default-instance.process-priority.description',
		defaultMessage: 'Sets the CPU priority for the game process.',
	},
	priorityNormal: {
		id: 'app.default-instance.priority.normal',
		defaultMessage: 'Normal',
	},
	priorityAboveNormal: {
		id: 'app.default-instance.priority.above-normal',
		defaultMessage: 'Above Normal',
	},
	priorityHigh: {
		id: 'app.default-instance.priority.high',
		defaultMessage: 'High',
	},
	priorityBelowNormal: {
		id: 'app.default-instance.priority.below-normal',
		defaultMessage: 'Below Normal',
	},
	priorityRealtime: {
		id: 'app.default-instance.priority.realtime',
		defaultMessage: 'Realtime',
	},
	ipProtocolTitle: {
		id: 'app.default-instance.ip-protocol.title',
		defaultMessage: 'IP Protocol Preference',
	},
	ipProtocolDescription: {
		id: 'app.default-instance.ip-protocol.description',
		defaultMessage: 'Preferred IP protocol stack for Java networking.',
	},
	ipDefault: {
		id: 'app.default-instance.ip.default',
		defaultMessage: 'Default',
	},
	ipPreferV4: {
		id: 'app.default-instance.ip.prefer-v4',
		defaultMessage: 'Prefer IPv4',
	},
	ipPreferV6: {
		id: 'app.default-instance.ip.prefer-v6',
		defaultMessage: 'Prefer IPv6',
	},
	windowTitleTitle: {
		id: 'app.default-instance.window-title.title',
		defaultMessage: 'Game Window Title',
	},
	windowTitlePlaceholder: {
		id: 'app.default-instance.window-title.placeholder',
		defaultMessage: 'e.g. {name} | Player: {user}',
	},
	windowTitleHint: {
		id: 'app.default-instance.window-title.hint',
		defaultMessage: 'Supports: {user} (username), {name} (instance name), {version} (game version)',
	},
	customInfoTitle: {
		id: 'app.default-instance.custom-info.title',
		defaultMessage: 'Custom Info',
	},
	customInfoPlaceholder: {
		id: 'app.default-instance.custom-info.placeholder',
		defaultMessage: 'e.g. MyLauncher',
	},
	customInfoHint: {
		id: 'app.default-instance.custom-info.hint',
		defaultMessage: 'Displayed in the bottom-left of the game and the F3 debug screen.',
	},
	minimizeLauncherTitle: {
		id: 'app.default-instance.minimize-launcher.title',
		defaultMessage: 'Launcher Visibility',
	},
	minimizeLauncherDescription: {
		id: 'app.default-instance.minimize-launcher.description',
		defaultMessage: 'Behavior of the launcher window when the Minecraft process starts.',
	},
	launcherVisibilityKeepOpen: {
		id: 'app.default-instance.launcher-visibility.keep-open',
		defaultMessage: 'Keep Open',
	},
	launcherVisibilityMinimize: {
		id: 'app.default-instance.launcher-visibility.minimize',
		defaultMessage: 'Minimize',
	},
	launcherVisibilityHideReopen: {
		id: 'app.default-instance.launcher-visibility.hide-reopen',
		defaultMessage: 'Hide, reopen on exit',
	},
	launcherVisibilityHideExit: {
		id: 'app.default-instance.launcher-visibility.hide-exit',
		defaultMessage: 'Hide, close on game exit',
	},
	launcherVisibilityExitImmediately: {
		id: 'app.default-instance.launcher-visibility.exit-immediately',
		defaultMessage: 'Exit immediately',
	},
})

const fetchSettings = await get()
const settings = ref(fetchSettings)

const { maxMemory, snapPoints } = (await useMemorySlider().catch(handleError)) as unknown as {
	maxMemory: number
	snapPoints: number[]
}

const systemMemoryMib = ref(0)
try {
	systemMemoryMib.value = Math.floor((await get_max_memory()) / 1024)
} catch {
	systemMemoryMib.value = maxMemory
}

const memoryWarning = computed(() => {
	if (systemMemoryMib.value === 0) return false
	return settings.value.memory.maximum > systemMemoryMib.value * 0.75
})

const isAutoMemory = computed(
	() => settings.value.memory_allocation_mode === 0,
)

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)
</script>

<template>
	<div class="flex flex-col gap-6">
		<!-- Display Options -->
		<div class="flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.fullscreenTitle) }}</h3>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.fullscreenDescription) }}
					</p>
				</div>
				<Toggle id="fullscreen" v-model="settings.force_fullscreen" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.widthTitle) }}</h3>
					<p class="m-0 leading-tight">{{ formatMessage(messages.widthDescription) }}</p>
				</div>
				<StyledInput
					id="width"
					v-model="settings.game_resolution[0]"
					:disabled="settings.force_fullscreen"
					autocomplete="off"
					type="number"
					:placeholder="formatMessage(messages.widthPlaceholder)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.heightTitle) }}</h3>
					<p class="m-0 leading-tight">{{ formatMessage(messages.heightDescription) }}</p>
				</div>
				<StyledInput
					id="height"
					v-model="settings.game_resolution[1]"
					:disabled="settings.force_fullscreen"
					autocomplete="off"
					type="number"
					:placeholder="formatMessage(messages.heightPlaceholder)"
				/>
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<!-- Memory -->
		<div class="flex flex-col gap-4">
			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.memoryAllocationTitle) }}</h3>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.memoryAllocationDescription) }}
					</p>
				</div>
				<Combobox
					id="memory-mode"
					:model-value="String(settings.memory_allocation_mode)"
					name="Memory allocation mode"
					class="max-w-40"
					:options="[
						{ value: '0', label: formatMessage(messages.memoryModeAuto) },
						{ value: '1', label: formatMessage(messages.memoryModeCustom) },
					]"
					@update:model-value="(v: string) => settings.memory_allocation_mode = Number(v)"
				/>
			</div>

			<div v-if="!isAutoMemory" class="flex flex-col gap-2.5">
				<div class="flex items-center justify-between">
					<h4 class="m-0 text-sm font-semibold text-contrast">{{ formatMessage(messages.memoryAllocated) }}</h4>
					<span class="text-sm text-secondary">{{ settings.memory.maximum }} MB</span>
				</div>
				<Slider
					id="max-memory"
					v-model="settings.memory.maximum"
					:min="512"
					:max="maxMemory"
					:step="64"
					:snap-points="snapPoints"
					:snap-range="512"
					unit="MB"
				/>
				<div
					v-if="memoryWarning"
					class="rounded-lg bg-yellow-bg px-3 py-2 text-sm text-yellow"
				>
					{{ formatMessage(messages.memoryWarning) }}
				</div>
			</div>
			<div v-else class="rounded-lg bg-bg px-3 py-2 text-sm text-secondary">
				{{ formatMessage(messages.memoryAutoHint) }}
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<!-- Launch Options -->
		<div class="flex flex-col gap-6">
			<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.launchOptionsTitle) }}</h3>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">{{ formatMessage(messages.processPriorityTitle) }}</h4>
					<p class="m-0 leading-tight">{{ formatMessage(messages.processPriorityDescription) }}</p>
				</div>
				<Combobox
					id="process-priority"
					:model-value="String(settings.process_priority)"
					name="Process priority"
					class="max-w-40"
					:options="[
						{ value: '1', label: formatMessage(messages.priorityNormal) },
						{ value: '0', label: formatMessage(messages.priorityAboveNormal) },
						{ value: '3', label: formatMessage(messages.priorityHigh) },
						{ value: '2', label: formatMessage(messages.priorityBelowNormal) },
						{ value: '4', label: formatMessage(messages.priorityRealtime) },
					]"
					@update:model-value="(v: string) => settings.process_priority = Number(v)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">{{ formatMessage(messages.ipProtocolTitle) }}</h4>
					<p class="m-0 leading-tight">{{ formatMessage(messages.ipProtocolDescription) }}</p>
				</div>
				<Combobox
					id="ip-stack"
					:model-value="String(settings.preferred_ip_stack)"
					name="IP stack"
					class="max-w-40"
					:options="[
						{ value: '1', label: formatMessage(messages.ipDefault) },
						{ value: '0', label: formatMessage(messages.ipPreferV4) },
						{ value: '2', label: formatMessage(messages.ipPreferV6) },
					]"
					@update:model-value="(v: string) => settings.preferred_ip_stack = Number(v)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">{{ formatMessage(messages.minimizeLauncherTitle) }}</h4>
					<p class="m-0 leading-tight">{{ formatMessage(messages.minimizeLauncherDescription) }}</p>
				</div>
				<Combobox
					id="launcher-visibility"
					:model-value="String(settings.launcher_visibility)"
					name="Launcher visibility dropdown"
					class="max-w-56"
					:options="[
						{ value: '5', label: formatMessage(messages.launcherVisibilityKeepOpen) },
						{ value: '4', label: formatMessage(messages.launcherVisibilityMinimize) },
						{ value: '3', label: formatMessage(messages.launcherVisibilityHideReopen) },
						{ value: '2', label: formatMessage(messages.launcherVisibilityHideExit) },
						{ value: '0', label: formatMessage(messages.launcherVisibilityExitImmediately) },
					]"
					@update:model-value="(v: string) => settings.launcher_visibility = Number(v)"
				/>
			</div>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">{{ formatMessage(messages.windowTitleTitle) }}</h4>
				<StyledInput
					id="window-title"
					v-model="settings.window_title"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.windowTitlePlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">
					{{ formatMessage(messages.windowTitleHint) }}
				</p>
			</div>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">{{ formatMessage(messages.customInfoTitle) }}</h4>
				<StyledInput
					id="custom-info"
					v-model="settings.custom_info"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.customInfoPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">
					{{ formatMessage(messages.customInfoHint) }}
				</p>
			</div>
		</div>
	</div>
</template>
