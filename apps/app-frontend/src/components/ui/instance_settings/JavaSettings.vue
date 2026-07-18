<script setup lang="ts">
import {
	CheckCircleIcon,
	CoffeeIcon,
	FolderSearchIcon,
	RefreshCwIcon,
	SearchIcon,
	SpinnerIcon,
	XCircleIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	defineMessages,
	injectNotificationManager,
	Slider,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, readonly, ref, watch } from 'vue'

import JavaDetectionModal from '@/components/ui/JavaDetectionModal.vue'
import useJavaTest from '@/composables/useJavaTest'
import useMemorySlider from '@/composables/useMemorySlider'
import { edit, get_optimal_jre_key } from '@/helpers/instance'
import { get } from '@/helpers/settings.ts'
import { injectInstanceSettings } from '@/providers/instance-settings'

import type { AppSettings } from '../../../helpers/types'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const { instance } = injectInstanceSettings()

const globalSettings = (await get().catch(handleError)) as unknown as AppSettings

const optimalJava = readonly(await get_optimal_jre_key(instance.value.id).catch(handleError))

const overrideJavaInstall = ref(!!instance.value.java_path)
const javaPath = ref(instance.value.java_path ?? optimalJava?.path ?? '')

const activePath = computed(() =>
	overrideJavaInstall.value ? javaPath.value : (optimalJava?.path ?? ''),
)

watch(overrideJavaInstall, (enabled) => {
	if (enabled && !javaPath.value) {
		javaPath.value = optimalJava?.path ?? ''
	}
})

const { testingJava, javaTestResult, testJavaInstallationDebounced, testJavaInstallation } =
	useJavaTest()

const hoveringTest = ref(false)
let hasInitialized = false

watch(
	activePath,
	(newPath) => {
		if (newPath && optimalJava?.parsed_version) {
			if (!hasInitialized) {
				testJavaInstallation(newPath, optimalJava?.parsed_version, false)
				hasInitialized = true
			} else {
				testJavaInstallationDebounced(newPath, optimalJava?.parsed_version)
			}
		}
	},
	{ immediate: true },
)

const javaDetectionModal = ref<{ show: (version: number, current: object) => void } | null>(null)

async function handleBrowseJava() {
	const result = await open({ multiple: false })
	if (result) {
		javaPath.value = result
	}
}

function handleDetectJava() {
	javaDetectionModal.value?.show(optimalJava?.parsed_version, { path: javaPath.value })
}

const overrideJavaArgs = ref((instance.value.extra_launch_args?.length ?? 0) > 0)
const javaArgs = ref(
	(instance.value.extra_launch_args ?? globalSettings.extra_launch_args).join(' '),
)

const overrideEnvVars = ref((instance.value.custom_env_vars?.length ?? 0) > 0)
const envVars = ref(
	(instance.value.custom_env_vars ?? globalSettings.custom_env_vars)
		.map((x) => x.join('='))
		.join(' '),
)

const memoryAllocationMode = ref(String(instance.value.memory_allocation_mode ?? 2))
const memory = ref(instance.value.memory ?? globalSettings.memory)
const isCustomMemory = computed(() => memoryAllocationMode.value === '1')
const { maxMemory, snapPoints } = (await useMemorySlider().catch(handleError)) as unknown as {
	maxMemory: number
	snapPoints: number[]
}

const editInstanceObject = computed(() => {
	const mode = Number(memoryAllocationMode.value)
	return {
		java_path:
			overrideJavaInstall.value && javaPath.value
				? javaPath.value.replace('java.exe', 'javaw.exe')
				: null,
		extra_launch_args: overrideJavaArgs.value
			? javaArgs.value.trim().split(/\s+/).filter(Boolean)
			: null,
		custom_env_vars: overrideEnvVars.value
			? envVars.value
					.trim()
					.split(/\s+/)
					.filter(Boolean)
					.map((x) => x.split('=').filter(Boolean))
			: null,
		memory: mode === 1 ? memory.value : null,
		memory_allocation_mode: mode,
	}
})

watch(
	[
		overrideJavaInstall,
		javaPath,
		overrideJavaArgs,
		javaArgs,
		overrideEnvVars,
		envVars,
		memoryAllocationMode,
		memory,
	],
	async () => {
		await edit(instance.value.id, editInstanceObject.value)
	},
	{ deep: true },
)

const messages = defineMessages({
	javaInstallation: {
		id: 'instance.settings.tabs.java.java-installation',
		defaultMessage: 'Java installation',
	},
	customJavaInstallation: {
		id: 'instance.settings.tabs.java.custom-java-installation',
		defaultMessage: 'Custom Java installation',
	},
	javaPathPlaceholder: {
		id: 'instance.settings.tabs.java.java-path-placeholder',
		defaultMessage: '/path/to/java',
	},
	javaMemory: {
		id: 'instance.settings.tabs.java.java-memory',
		defaultMessage: 'Memory allocated',
	},
	memoryModeFollowGlobal: {
		id: 'instance.settings.tabs.java.memory-mode-follow-global',
		defaultMessage: 'Follow global',
	},
	memoryModeAuto: {
		id: 'instance.settings.tabs.java.memory-mode-auto',
		defaultMessage: 'Auto',
	},
	memoryModeCustom: {
		id: 'instance.settings.tabs.java.memory-mode-custom',
		defaultMessage: 'Custom',
	},
	memoryModeHint: {
		id: 'instance.settings.tabs.java.memory-mode-hint',
		defaultMessage:
			'Follow global uses the global memory setting; Auto calculates memory based on system RAM, instance type, and mod count.',
	},
	javaArguments: {
		id: 'instance.settings.tabs.java.java-arguments',
		defaultMessage: 'Java arguments',
	},
	customJavaArguments: {
		id: 'instance.settings.tabs.java.custom-java-arguments',
		defaultMessage: 'Custom Java arguments',
	},
	enterJavaArguments: {
		id: 'instance.settings.tabs.java.enter-java-arguments',
		defaultMessage: 'Enter Java arguments...',
	},
	javaEnvironmentVariables: {
		id: 'instance.settings.tabs.java.environment-variables',
		defaultMessage: 'Environment variables',
	},
	customEnvironmentVariables: {
		id: 'instance.settings.tabs.java.custom-environment-variables',
		defaultMessage: 'Custom environment variables',
	},
	enterEnvironmentVariables: {
		id: 'instance.settings.tabs.java.enter-environment-variables',
		defaultMessage: 'Enter environmental variables...',
	},
	hooks: {
		id: 'instance.settings.tabs.java.hooks',
		defaultMessage: 'Hooks',
	},
})
</script>

<template>
	<div>
		<JavaDetectionModal ref="javaDetectionModal" @submit="(val) => (javaPath = val.path)" />
		<h2 class="m-0 mb-2 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaInstallation) }}
		</h2>
		<Checkbox
			v-model="overrideJavaInstall"
			:label="formatMessage(messages.customJavaInstallation)"
			class="mb-2"
		/>
		<div class="flex gap-4 p-4 bg-bg rounded-2xl">
			<div class="flex gap-3 items-start flex-1 min-w-0">
				<div
					class="w-10 h-10 flex items-center justify-center rounded-full bg-button-bg border-solid border-[1px] border-button-border p-2 mt-1 shrink-0 [&_svg]:h-full [&_svg]:w-full"
				>
					<CoffeeIcon />
				</div>
				<div class="flex flex-col gap-2 flex-1 min-w-0">
					<span class="font-semibold leading-none mt-2"
						>Java {{ optimalJava?.parsed_version }}</span
					>
					<div class="flex gap-2 items-center">
						<StyledInput
							:model-value="activePath"
							:disabled="!overrideJavaInstall"
							autocomplete="off"
							:placeholder="formatMessage(messages.javaPathPlaceholder)"
							wrapper-class="flex-1 min-w-0"
							@update:model-value="(val) => (javaPath = String(val))"
						/>
						<ButtonStyled
							:color="
								!hoveringTest && !testingJava
									? javaTestResult === true
										? 'green'
										: 'red'
									: 'standard'
							"
							color-fill="text"
						>
							<button
								:disabled="!overrideJavaInstall || testingJava"
								@click="testJavaInstallation(activePath, optimalJava?.parsed_version, true)"
								@mouseenter="overrideJavaInstall && (hoveringTest = true)"
								@mouseleave="hoveringTest = false"
							>
								<SpinnerIcon v-if="testingJava" class="animate-spin h-4 w-4" />
								<CheckCircleIcon
									v-else-if="javaTestResult === true && !hoveringTest"
									class="h-4 w-4"
								/>
								<XCircleIcon v-else-if="javaTestResult !== true && !hoveringTest" class="h-4 w-4" />
								<RefreshCwIcon v-else-if="overrideJavaInstall" class="h-4 w-4" />
							</button>
						</ButtonStyled>
					</div>
					<div v-if="overrideJavaInstall" class="flex gap-2">
						<ButtonStyled>
							<button @click="handleDetectJava">
								<SearchIcon />
								Detect
							</button>
						</ButtonStyled>
						<ButtonStyled>
							<button @click="handleBrowseJava">
								<FolderSearchIcon />
								Browse
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>
		</div>
		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaMemory) }}
		</h2>
		<Combobox
			id="memory-mode"
			v-model="memoryAllocationMode"
			name="Memory allocation mode"
			class="mb-2 max-w-60"
			:options="[
				{ value: '2', label: formatMessage(messages.memoryModeFollowGlobal) },
				{ value: '0', label: formatMessage(messages.memoryModeAuto) },
				{ value: '1', label: formatMessage(messages.memoryModeCustom) },
			]"
		/>
		<Slider
			v-if="isCustomMemory"
			id="max-memory"
			v-model="memory.maximum"
			:min="512"
			:max="maxMemory"
			:step="64"
			:snap-points="snapPoints"
			:snap-range="512"
			unit="MB"
		/>
		<div v-else class="rounded-lg bg-bg px-3 py-2 text-sm text-secondary">
			{{ formatMessage(messages.memoryModeHint) }}
		</div>
		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaArguments) }}
		</h2>
		<Checkbox
			v-model="overrideJavaArgs"
			:label="formatMessage(messages.customJavaArguments)"
			class="my-2"
		/>
		<StyledInput
			id="java-args"
			v-model="javaArgs"
			autocomplete="off"
			:disabled="!overrideJavaArgs"
			:placeholder="formatMessage(messages.enterJavaArguments)"
			wrapper-class="w-full"
		/>
		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast block">
			{{ formatMessage(messages.javaEnvironmentVariables) }}
		</h2>
		<Checkbox
			v-model="overrideEnvVars"
			:label="formatMessage(messages.customEnvironmentVariables)"
			class="mb-2"
		/>
		<StyledInput
			id="env-vars"
			v-model="envVars"
			autocomplete="off"
			:disabled="!overrideEnvVars"
			:placeholder="formatMessage(messages.enterEnvironmentVariables)"
			wrapper-class="w-full"
		/>
	</div>
</template>
