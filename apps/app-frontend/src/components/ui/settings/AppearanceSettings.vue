<script setup lang="ts">
import { Combobox, defineMessages, Slider, ThemeSelector, Toggle, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import type { ColorTheme, FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()
const { formatMessage } = useVIntl()

async function selectBackgroundImage() {
	const selected = await open({
		multiple: false,
		filters: [
			{
				name: 'Images',
				extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif'],
			},
		],
	})
	if (selected) {
		themeStore.setBackgroundImagePath(selected)
	}
}

function clearBackgroundImage() {
	themeStore.setBackgroundImagePath(null)
}

const worldsInHomeFlag: FeatureFlag = 'worlds_in_home'
const skipNonEssentialWarningsFlag: FeatureFlag = 'skip_non_essential_warnings'
const skipUnknownPackWarningFlag: FeatureFlag = 'skip_unknown_pack_warning'
const showPlayTimeFlag: FeatureFlag = 'show_instance_play_time'

const messages = defineMessages({
	colorThemeTitle: {
		id: 'app.appearance-settings.color-theme.title',
		defaultMessage: 'Color theme',
	},
	colorThemeDescription: {
		id: 'app.appearance-settings.color-theme.description',
		defaultMessage: 'Select your preferred color theme for Modrinth App.',
	},
	advancedRenderingTitle: {
		id: 'app.appearance-settings.advanced-rendering.title',
		defaultMessage: 'Advanced rendering',
	},
	advancedRenderingDescription: {
		id: 'app.appearance-settings.advanced-rendering.description',
		defaultMessage:
			'Enables advanced rendering such as blur effects that may cause performance issues without hardware-accelerated rendering.',
	},
	hideNametagTitle: {
		id: 'app.appearance-settings.hide-nametag.title',
		defaultMessage: 'Hide nametag',
	},
	hideNametagDescription: {
		id: 'app.appearance-settings.hide-nametag.description',
		defaultMessage: 'Disables the nametag above your player on the skins page.',
	},
	nativeDecorationsTitle: {
		id: 'app.appearance-settings.native-decorations.title',
		defaultMessage: 'Native decorations',
	},
	nativeDecorationsDescription: {
		id: 'app.appearance-settings.native-decorations.description',
		defaultMessage: 'Use system window frame (app restart required).',
	},
	minimizeLauncherTitle: {
		id: 'app.appearance-settings.minimize-launcher.title',
		defaultMessage: 'Launcher visibility',
	},
	minimizeLauncherDescription: {
		id: 'app.appearance-settings.minimize-launcher.description',
		defaultMessage: 'Behavior of the launcher window when a Minecraft process starts.',
	},
	defaultLandingPageTitle: {
		id: 'app.appearance-settings.default-landing-page.title',
		defaultMessage: 'Default landing page',
	},
	defaultLandingPageDescription: {
		id: 'app.appearance-settings.default-landing-page.description',
		defaultMessage: 'Change the page to which the launcher opens on.',
	},
	defaultLandingPageHome: {
		id: 'app.appearance-settings.default-landing-page.home',
		defaultMessage: 'Home',
	},
	defaultLandingPageLibrary: {
		id: 'app.appearance-settings.default-landing-page.library',
		defaultMessage: 'Library',
	},
	selectOption: {
		id: 'app.appearance-settings.select-option',
		defaultMessage: 'Select an option',
	},
	jumpBackIntoWorldsTitle: {
		id: 'app.appearance-settings.jump-back-into-worlds.title',
		defaultMessage: 'Jump back into worlds',
	},
	jumpBackIntoWorldsDescription: {
		id: 'app.appearance-settings.jump-back-into-worlds.description',
		defaultMessage: 'Includes recent worlds in the "Jump back in" section on the Home page.',
	},
	toggleSidebarTitle: {
		id: 'app.appearance-settings.toggle-sidebar.title',
		defaultMessage: 'Toggle sidebar',
	},
	toggleSidebarDescription: {
		id: 'app.appearance-settings.toggle-sidebar.description',
		defaultMessage: 'Enables the ability to toggle the sidebar.',
	},
	unknownPackWarningTitle: {
		id: 'app.appearance-settings.unknown-pack-warning.title',
		defaultMessage: 'Warn me before installing unknown modpacks',
	},
	unknownPackWarningDescription: {
		id: 'app.appearance-settings.unknown-pack-warning.description',
		defaultMessage:
			"If you attempt to install a Modrinth Pack file (.mrpack) that isn't hosted on Modrinth, we'll make sure you understand the risks before installing it.",
	},
	skipNonEssentialWarningsTitle: {
		id: 'app.appearance-settings.skip-non-essential-warnings.title',
		defaultMessage: 'Skip non-essential warnings',
	},
	skipNonEssentialWarningsDescription: {
		id: 'app.appearance-settings.skip-non-essential-warnings.description',
		defaultMessage:
			'Automatically skips low-risk confirmations like duplicate modpack installs, normal content deletion, bulk updates, unlinking modpacks, and repair prompts. Dangerous warnings will still be shown.',
	},
	showPlayTimeTitle: {
		id: 'app.appearance-settings.show-play-time.title',
		defaultMessage: 'Show play time',
	},
	showPlayTimeDescription: {
		id: 'app.appearance-settings.show-play-time.description',
		defaultMessage: `Displays how much time you've spent playing an instance.`,
	},
	backgroundImageTitle: {
		id: 'app.appearance-settings.background-image.title',
		defaultMessage: 'Background image',
	},
	backgroundImageDescription: {
		id: 'app.appearance-settings.background-image.description',
		defaultMessage: 'Set a custom background image for the launcher.',
	},
	selectImageButton: {
		id: 'app.appearance-settings.background-image.select-button',
		defaultMessage: 'Select image',
	},
	clearImageButton: {
		id: 'app.appearance-settings.background-image.clear-button',
		defaultMessage: 'Clear',
	},
	backgroundBlurTitle: {
		id: 'app.appearance-settings.background-blur.title',
		defaultMessage: 'Background blur',
	},
	backgroundBlurDescription: {
		id: 'app.appearance-settings.background-blur.description',
		defaultMessage: 'Adjust the blur amount applied to the background image.',
	},
})

const os = ref(await getOS())
const settings = ref(await get())

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)
</script>
<template>
	<h2 class="m-0 text-lg font-semibold text-contrast">
		{{ formatMessage(messages.colorThemeTitle) }}
	</h2>
	<p class="m-0 mt-1">{{ formatMessage(messages.colorThemeDescription) }}</p>

	<ThemeSelector
		:update-color-theme="
			(theme: ColorTheme) => {
				themeStore.setThemeState(theme)
				settings.theme = theme
			}
		"
		:current-theme="settings.theme"
		:theme-options="themeStore.getThemeOptions()"
		system-theme-color="system"
	/>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.advancedRenderingTitle) }}
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.advancedRenderingDescription) }}
			</p>
		</div>

		<Toggle
			id="advanced-rendering"
			:model-value="themeStore.advancedRendering"
			@update:model-value="
				(e) => {
					themeStore.advancedRendering = !!e
					settings.advanced_rendering = themeStore.advancedRendering
				}
			"
		/>
	</div>

	<div v-if="os !== 'MacOS'" class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.nativeDecorationsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.nativeDecorationsDescription) }}</p>
		</div>
		<Toggle id="native-decorations" v-model="settings.native_decorations" />
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.minimizeLauncherTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.minimizeLauncherDescription) }}</p>
		</div>
		<Combobox
			id="launcher-visibility"
			:model-value="String(settings.launcher_visibility)"
			@update:model-value="(v: string) => settings.launcher_visibility = Number(v)"
			name="Launcher visibility dropdown"
			class="max-w-56"
			:options="[
				{ value: '5', label: 'Keep open' },
				{ value: '4', label: 'Minimize' },
				{ value: '3', label: 'Hide, reopen on exit' },
				{ value: '2', label: 'Hide, exit on game exit' },
				{ value: '0', label: 'Exit immediately' },
			]"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.showPlayTimeTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.showPlayTimeDescription) }}</p>
		</div>
		<Toggle
			:model-value="themeStore.getFeatureFlag(showPlayTimeFlag)"
			@update:model-value="
				() => {
					const newValue = !themeStore.getFeatureFlag(showPlayTimeFlag)
					themeStore.featureFlags[showPlayTimeFlag] = newValue
					settings.feature_flags[showPlayTimeFlag] = newValue
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.hideNametagTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.hideNametagDescription) }}</p>
		</div>
		<Toggle
			id="hide-nametag-skins-page"
			:model-value="themeStore.hideNametagSkinsPage"
			@update:model-value="
				(e) => {
					themeStore.hideNametagSkinsPage = !!e
					settings.hide_nametag_skins_page = themeStore.hideNametagSkinsPage
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.defaultLandingPageTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.defaultLandingPageDescription) }}</p>
		</div>
		<Combobox
			id="opening-page"
			v-model="settings.default_page"
			name="Opening page dropdown"
			class="max-w-40"
			:options="[
				{
					value: 'Home',
					label: formatMessage(messages.defaultLandingPageHome),
				},
				{
					value: 'Library',
					label: formatMessage(messages.defaultLandingPageLibrary),
				},
			]"
			:display-value="settings.default_page ?? 'Select an option'"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.jumpBackIntoWorldsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.jumpBackIntoWorldsDescription) }}</p>
		</div>
		<Toggle
			:model-value="themeStore.getFeatureFlag(worldsInHomeFlag)"
			@update:model-value="
				() => {
					const newValue = !themeStore.getFeatureFlag(worldsInHomeFlag)
					themeStore.featureFlags[worldsInHomeFlag] = newValue
					settings.feature_flags[worldsInHomeFlag] = newValue
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.unknownPackWarningTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.unknownPackWarningDescription) }}</p>
		</div>
		<Toggle
			:model-value="!themeStore.getFeatureFlag(skipUnknownPackWarningFlag)"
			@update:model-value="
				(e) => {
					const warnBeforeUnknownPackInstall = !!e
					const skipUnknownPackWarning = !warnBeforeUnknownPackInstall
					themeStore.featureFlags[skipUnknownPackWarningFlag] = skipUnknownPackWarning
					settings.feature_flags[skipUnknownPackWarningFlag] = skipUnknownPackWarning
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.skipNonEssentialWarningsTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.skipNonEssentialWarningsDescription) }}</p>
		</div>
		<Toggle
			:model-value="themeStore.getFeatureFlag(skipNonEssentialWarningsFlag)"
			@update:model-value="
				() => {
					const newValue = !themeStore.getFeatureFlag(skipNonEssentialWarningsFlag)
					themeStore.featureFlags[skipNonEssentialWarningsFlag] = newValue
					settings.feature_flags[skipNonEssentialWarningsFlag] = newValue
				}
			"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.toggleSidebarTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.toggleSidebarDescription) }}</p>
		</div>
		<Toggle
			id="toggle-sidebar"
			:model-value="settings.toggle_sidebar"
			@update:model-value="
				(e) => {
					settings.toggle_sidebar = !!e
					themeStore.toggleSidebar = settings.toggle_sidebar
				}
			"
		/>
	</div>

	<hr class="mt-6 mb-6 border-solid border-[var(--color-button-bg)]" />

	<h2 class="m-0 text-lg font-semibold text-contrast">
		{{ formatMessage(messages.backgroundImageTitle) }}
	</h2>
	<p class="m-0 mt-1">{{ formatMessage(messages.backgroundImageDescription) }}</p>

	<div class="mt-4 flex items-center gap-4">
		<button class="btn btn-highlight" @click="selectBackgroundImage">
			{{ formatMessage(messages.selectImageButton) }}
		</button>
		<button
			v-if="themeStore.backgroundImagePath"
			class="btn"
			@click="clearBackgroundImage"
		>
			{{ formatMessage(messages.clearImageButton) }}
		</button>
	</div>

	<div v-if="themeStore.backgroundImagePath" class="mt-4">
		<div
			class="relative w-full h-32 rounded-xl overflow-hidden border border-solid border-[var(--color-button-bg)]"
		>
			<img
				:src="convertFileSrc(themeStore.backgroundImagePath)"
				class="w-full h-full object-cover"
			/>
			<div
				class="absolute inset-0"
				:style="{
					backdropFilter: `blur(${themeStore.backgroundBlur}px)`,
					backgroundColor: 'rgba(0,0,0,0.4)',
				}"
			/>
		</div>
	</div>

	<div
		v-if="themeStore.backgroundImagePath"
		class="mt-4 flex flex-col gap-2"
	>
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.backgroundBlurTitle) }}
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.backgroundBlurDescription) }}
			</p>
		</div>
		<Slider
			v-model="themeStore.backgroundBlur"
			:min="0"
			:max="50"
			:step="1"
			unit="px"
		/>
	</div>
</template>
