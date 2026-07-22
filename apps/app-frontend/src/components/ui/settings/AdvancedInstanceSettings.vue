<script setup lang="ts">
import { ButtonStyled, Combobox, defineMessages, StyledInput, Toggle, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	javaArgumentsTitle: {
		id: 'app.advanced-instance.java-arguments.title',
		defaultMessage: 'Java arguments',
	},
	resetToDefault: {
		id: 'app.advanced-instance.reset-to-default',
		defaultMessage: 'Reset to default',
	},
	javaArgumentsPlaceholder: {
		id: 'app.advanced-instance.java-arguments.placeholder',
		defaultMessage: 'Enter java arguments...',
	},
	envVariablesTitle: {
		id: 'app.advanced-instance.env-variables.title',
		defaultMessage: 'Environmental variables',
	},
	envVariablesPlaceholder: {
		id: 'app.advanced-instance.env-variables.placeholder',
		defaultMessage: 'Enter environmental variables...',
	},
	hooksTitle: {
		id: 'app.advanced-instance.hooks.title',
		defaultMessage: 'Hooks',
	},
	preLaunchHookTitle: {
		id: 'app.advanced-instance.pre-launch-hook.title',
		defaultMessage: 'Pre-launch hook',
	},
	preLaunchHookPlaceholder: {
		id: 'app.advanced-instance.pre-launch-hook.placeholder',
		defaultMessage: 'Enter pre-launch command...',
	},
	preLaunchHookDescription: {
		id: 'app.advanced-instance.pre-launch-hook.description',
		defaultMessage: 'Ran before the instance is launched.',
	},
	preLaunchWait: {
		id: 'app.advanced-instance.pre-launch-wait',
		defaultMessage: 'Wait for command to finish before launching',
	},
	wrapperHookTitle: {
		id: 'app.advanced-instance.wrapper-hook.title',
		defaultMessage: 'Wrapper hook',
	},
	wrapperHookPlaceholder: {
		id: 'app.advanced-instance.wrapper-hook.placeholder',
		defaultMessage: 'Enter wrapper command...',
	},
	wrapperHookDescription: {
		id: 'app.advanced-instance.wrapper-hook.description',
		defaultMessage: 'Wrapper command for launching Minecraft.',
	},
	postExitHookTitle: {
		id: 'app.advanced-instance.post-exit-hook.title',
		defaultMessage: 'Post-exit hook',
	},
	postExitHookPlaceholder: {
		id: 'app.advanced-instance.post-exit-hook.placeholder',
		defaultMessage: 'Enter post-exit command...',
	},
	postExitHookDescription: {
		id: 'app.advanced-instance.post-exit-hook.description',
		defaultMessage: 'Ran after the game closes.',
	},
	advancedLaunchOptionsTitle: {
		id: 'app.advanced-instance.advanced-launch-options.title',
		defaultMessage: 'Advanced launch options',
	},
	rendererTitle: {
		id: 'app.advanced-instance.renderer.title',
		defaultMessage: 'Renderer',
	},
	rendererDescription: {
		id: 'app.advanced-instance.renderer.description',
		defaultMessage: 'Override the OpenGL renderer. May cause instability.',
	},
	rendererDefault: {
		id: 'app.advanced-instance.renderer.default',
		defaultMessage: 'Default',
	},
	rendererLlvmpipe: {
		id: 'app.advanced-instance.renderer.llvmpipe',
		defaultMessage: 'llvmpipe (software)',
	},
	rendererD3d12: {
		id: 'app.advanced-instance.renderer.d3d12',
		defaultMessage: 'DirectX 12',
	},
	rendererZink: {
		id: 'app.advanced-instance.renderer.zink',
		defaultMessage: 'Vulkan (Zink)',
	},
	extraGameArgsTitle: {
		id: 'app.advanced-instance.extra-game-args.title',
		defaultMessage: 'Extra game arguments',
	},
	extraGameArgsPlaceholder: {
		id: 'app.advanced-instance.extra-game-args.placeholder',
		defaultMessage: 'e.g. --demo',
	},
	extraGameArgsHint: {
		id: 'app.advanced-instance.extra-game-args.hint',
		defaultMessage: 'Appended to the end of the Minecraft launch arguments.',
	},
	highPerfGpuTitle: {
		id: 'app.advanced-instance.high-perf-gpu.title',
		defaultMessage: 'High-performance GPU',
	},
	highPerfGpuDescription: {
		id: 'app.advanced-instance.high-perf-gpu.description',
		defaultMessage: 'Request the high-performance GPU for the game process.',
	},
	useJavaExeTitle: {
		id: 'app.advanced-instance.use-java-exe.title',
		defaultMessage: 'Use java.exe',
	},
	useJavaExeDescription: {
		id: 'app.advanced-instance.use-java-exe.description',
		defaultMessage:
			'Use java.exe instead of javaw.exe. Provides a console window for debugging.',
	},
	compatibilityTitle: {
		id: 'app.advanced-instance.compatibility.title',
		defaultMessage: 'Compatibility',
	},
	disableJlwTitle: {
		id: 'app.advanced-instance.disable-jlw.title',
		defaultMessage: 'Disable Java Launch Wrapper',
	},
	disableJlwDescription: {
		id: 'app.advanced-instance.disable-jlw.description',
		defaultMessage: 'Disable the theseus.jar launch wrapper. May break some features.',
	},
	disableLfTitle: {
		id: 'app.advanced-instance.disable-lf.title',
		defaultMessage: 'Disable LegacyFix',
	},
	disableLfDescription: {
		id: 'app.advanced-instance.disable-lf.description',
		defaultMessage: 'Disable compatibility fixes for old Minecraft versions.',
	},
	disableLwjglTitle: {
		id: 'app.advanced-instance.disable-lwjgl.title',
		defaultMessage: 'Disable LWJGL Unsafe Agent',
	},
	disableLwjglDescription: {
		id: 'app.advanced-instance.disable-lwjgl.description',
		defaultMessage: 'Disable the LWJGL unsafe agent that patches FFM API performance issues.',
	},
})

const fetchSettings = await get()
fetchSettings.launchArgs = fetchSettings.extra_launch_args.join(' ')
fetchSettings.envVars = fetchSettings.custom_env_vars
	.map((x) => x.join('='))
	.join(' ')
fetchSettings.gameArgs = fetchSettings.extra_game_args.join(' ')

const settings = ref(fetchSettings)

const DEFAULT_JVM_ARGS =
	'-XX:+UseG1GC -XX:-UseAdaptiveSizePolicy -XX:-OmitStackTraceInFastThrow -Djdk.lang.Process.allowAmbiguousCommands=true -Dfml.ignoreInvalidMinecraftCertificates=True -Dfml.ignorePatchDiscrepancies=True -Dlog4j2.formatMsgNoLookups=true'

const showJvmReset = computed(() => settings.value.launchArgs.trim() !== DEFAULT_JVM_ARGS)

function resetJvmArgs() {
	settings.value.launchArgs = DEFAULT_JVM_ARGS
}

watch(
	settings,
	async () => {
		const setSettings = JSON.parse(JSON.stringify(settings.value))

		setSettings.extra_launch_args = setSettings.launchArgs
			.trim()
			.split(/\s+/)
			.filter(Boolean)
		setSettings.custom_env_vars = setSettings.envVars
			.trim()
			.split(/\s+/)
			.filter(Boolean)
			.map((x) => x.split('=').filter(Boolean))
		setSettings.extra_game_args = setSettings.gameArgs
			.trim()
			.split(/\s+/)
			.filter(Boolean)

		if (!setSettings.hooks.pre_launch) {
			setSettings.hooks.pre_launch = null
		}
		if (!setSettings.hooks.wrapper) {
			setSettings.hooks.wrapper = null
		}
		if (!setSettings.hooks.post_exit) {
			setSettings.hooks.post_exit = null
		}

		if (!setSettings.custom_dir) {
			setSettings.custom_dir = null
		}

		await set(setSettings)
	},
	{ deep: true },
)
</script>
<template>
	<div class="flex flex-col gap-6">
		<!-- JVM Arguments -->
		<div class="flex flex-col gap-4">
			<div class="flex items-center justify-between">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.javaArgumentsTitle) }}
				</h3>
				<ButtonStyled v-if="showJvmReset" type="transparent">
					<button class="text-sm" @click="resetJvmArgs">
						{{ formatMessage(messages.resetToDefault) }}
					</button>
				</ButtonStyled>
			</div>
			<StyledInput
				id="java-args"
				v-model="settings.launchArgs"
				autocomplete="off"
				type="text"
				:placeholder="formatMessage(messages.javaArgumentsPlaceholder)"
				wrapper-class="w-full"
			/>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.envVariablesTitle) }}
				</h4>
				<StyledInput
					id="env-vars"
					v-model="settings.envVars"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.envVariablesPlaceholder)"
					wrapper-class="w-full"
				/>
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<!-- Hooks -->
		<div class="flex flex-col gap-6">
			<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.hooksTitle) }}</h3>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.preLaunchHookTitle) }}
				</h4>
				<StyledInput
					id="pre-launch"
					v-model="settings.hooks.pre_launch"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.preLaunchHookPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.preLaunchHookDescription) }}</p>
				<div v-if="settings.hooks.pre_launch" class="flex items-center gap-2 mt-1">
					<Toggle id="pre-launch-wait" v-model="settings.pre_launch_wait" />
					<label for="pre-launch-wait" class="text-sm text-secondary">
						{{ formatMessage(messages.preLaunchWait) }}
					</label>
				</div>
			</div>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.wrapperHookTitle) }}
				</h4>
				<StyledInput
					id="wrapper"
					v-model="settings.hooks.wrapper"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.wrapperHookPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.wrapperHookDescription) }}</p>
			</div>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.postExitHookTitle) }}
				</h4>
				<StyledInput
					id="post-exit"
					v-model="settings.hooks.post_exit"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.postExitHookPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.postExitHookDescription) }}</p>
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<!-- Advanced Launch Options -->
		<div class="flex flex-col gap-6">
			<h3 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.advancedLaunchOptionsTitle) }}
			</h3>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">
						{{ formatMessage(messages.rendererTitle) }}
					</h4>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.rendererDescription) }}
					</p>
				</div>
				<Combobox
					id="renderer"
					:model-value="String(settings.renderer)"
					name="Renderer"
					class="max-w-40"
					:options="[
						{ value: '0', label: formatMessage(messages.rendererDefault) },
						{ value: '1', label: formatMessage(messages.rendererLlvmpipe) },
						{ value: '2', label: formatMessage(messages.rendererD3d12) },
						{ value: '3', label: formatMessage(messages.rendererZink) },
					]"
					@update:model-value="(v: string) => settings.renderer = Number(v)"
				/>
			</div>

			<div class="flex flex-col gap-2.5">
				<h4 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.extraGameArgsTitle) }}
				</h4>
				<StyledInput
					id="game-args"
					v-model="settings.gameArgs"
					autocomplete="off"
					type="text"
					:placeholder="formatMessage(messages.extraGameArgsPlaceholder)"
					wrapper-class="w-full"
				/>
				<p class="m-0 leading-tight">{{ formatMessage(messages.extraGameArgsHint) }}</p>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">
						{{ formatMessage(messages.highPerfGpuTitle) }}
					</h4>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.highPerfGpuDescription) }}
					</p>
				</div>
				<Toggle id="gpu-pref" v-model="settings.set_gpu_preference" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">
						{{ formatMessage(messages.useJavaExeTitle) }}
					</h4>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.useJavaExeDescription) }}
					</p>
				</div>
				<Toggle id="use-java-exe" v-model="settings.use_java_exe" />
			</div>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

		<!-- Compatibility Toggles -->
		<div class="flex flex-col gap-6">
			<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.compatibilityTitle) }}</h3>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">
						{{ formatMessage(messages.disableJlwTitle) }}
					</h4>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.disableJlwDescription) }}
					</p>
				</div>
				<Toggle id="disable-jlw" v-model="settings.disable_java_launch_wrapper" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">
						{{ formatMessage(messages.disableLfTitle) }}
					</h4>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.disableLfDescription) }}
					</p>
				</div>
				<Toggle id="disable-lf" v-model="settings.disable_legacy_fix" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div class="flex flex-col gap-1">
					<h4 class="m-0 text-sm font-semibold text-contrast">
						{{ formatMessage(messages.disableLwjglTitle) }}
					</h4>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.disableLwjglDescription) }}
					</p>
				</div>
				<Toggle id="disable-lwjgl" v-model="settings.disable_lwjgl_unsafe_agent" />
			</div>
		</div>
	</div>
</template>
