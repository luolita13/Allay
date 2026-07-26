<script setup lang="ts">
import { CheckIcon, ImageIcon, TrashIcon, UploadIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, Slider, ThemeSelector, Toggle, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, useTemplateRef, watch } from 'vue'

import { createObjectUrlFromPath } from '@/helpers/image-url'
import { get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import type { AccentColor, ColorTheme } from '@/store/theme.ts'
import { ACCENT_COLOR_OPTIONS } from '@/store/theme.ts'
// Temporarily disabled — theme pack feature is a work in progress
// import ThemePackManager from '@/components/ui/settings/ThemePackManager.vue'

const themeStore = useTheming()
const { formatMessage } = useVIntl()

const backgroundBlur = computed({
	get: () => themeStore.backgroundBlur,
	set: (value) => themeStore.setBackgroundBlur(value),
})

const customAccentInput = useTemplateRef<HTMLInputElement>('customAccentInput')

function openCustomAccentPicker() {
	customAccentInput.value?.click()
}

const backgroundOpacity = computed({
	get: () => themeStore.backgroundOpacity,
	set: (value) => themeStore.setBackgroundOpacity(value),
})

const backgroundPreviewUrl = ref<string | null>(null)

watch(
	() => themeStore.backgroundImagePath,
	async (path) => {
		// Do NOT revoke — blob URL is cached per path and shared across consumers.
		if (path) {
			backgroundPreviewUrl.value = await createObjectUrlFromPath(path)
		} else {
			backgroundPreviewUrl.value = null
		}
	},
	{ immediate: true },
)

async function selectBackgroundImage() {
	const selected = await open({
		multiple: false,
		filters: [
			{
				name: 'Images',
				extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif', 'avif'],
			},
		],
	})
	if (selected && typeof selected === 'string') {
		themeStore.setBackgroundImagePath(selected)
	}
}

function clearBackgroundImage() {
	themeStore.setBackgroundImagePath(null)
}

const messages = defineMessages({
	colorThemeTitle: {
		id: 'app.appearance-settings.color-theme.title',
		defaultMessage: 'Color theme',
	},
	colorThemeDescription: {
		id: 'app.appearance-settings.color-theme.description',
		defaultMessage: 'Select your preferred color theme for Modrinth App.',
	},
	accentColorTitle: {
		id: 'app.appearance-settings.accent-color.title',
		defaultMessage: 'Accent color',
	},
	accentColorDescription: {
		id: 'app.appearance-settings.accent-color.description',
		defaultMessage: 'Choose the color used for buttons, selections, and highlights.',
	},
	accentColorOrange: {
		id: 'app.appearance-settings.accent-color.orange',
		defaultMessage: 'Orange',
	},
	accentColorGreen: {
		id: 'app.appearance-settings.accent-color.green',
		defaultMessage: 'Green',
	},
	accentColorBlue: {
		id: 'app.appearance-settings.accent-color.blue',
		defaultMessage: 'Blue',
	},
	accentColorPurple: {
		id: 'app.appearance-settings.accent-color.purple',
		defaultMessage: 'Purple',
	},
	customBackgroundTitle: {
		id: 'app.appearance-settings.custom-background.title',
		defaultMessage: 'Launcher background',
	},
	customBackgroundDescription: {
		id: 'app.appearance-settings.custom-background.description',
		defaultMessage: 'Choose a custom image and fine-tune how it blends with the launcher interface.',
	},
	customBackgroundEmpty: {
		id: 'app.appearance-settings.custom-background.empty',
		defaultMessage: 'No custom background selected',
	},
	customBackgroundChoose: {
		id: 'app.appearance-settings.custom-background.choose',
		defaultMessage: 'Choose image',
	},
	customBackgroundReplace: {
		id: 'app.appearance-settings.custom-background.replace',
		defaultMessage: 'Replace image',
	},
	customBackgroundRemove: {
		id: 'app.appearance-settings.custom-background.remove',
		defaultMessage: 'Remove',
	},
	customBackgroundBlur: {
		id: 'app.appearance-settings.custom-background.blur',
		defaultMessage: 'Background blur',
	},
	customBackgroundBlurDescription: {
		id: 'app.appearance-settings.custom-background.blur-description',
		defaultMessage: 'Soften image details to keep launcher content easy to read.',
	},
	customBackgroundOpacity: {
		id: 'app.appearance-settings.custom-background.opacity',
		defaultMessage: 'Background visibility',
	},
	customBackgroundOpacityDescription: {
		id: 'app.appearance-settings.custom-background.opacity-description',
		defaultMessage: 'Control how strongly the image shows through the interface.',
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
	imageViewerTitle: {
		id: 'app.appearance-settings.image-viewer.title',
		defaultMessage: 'In-app image viewer',
	},
	imageViewerDescription: {
		id: 'app.appearance-settings.image-viewer.description',
		defaultMessage:
			'Open screenshots and images inside the launcher instead of the system default image viewer.',
	},
	settingsAsPageTitle: {
		id: 'app.appearance-settings.settings-as-page.title',
		defaultMessage: 'Settings as page',
	},
	settingsAsPageDescription: {
		id: 'app.appearance-settings.settings-as-page.description',
		defaultMessage:
			'Display the settings panel as a full page instead of a modal dialog.',
	},
	skinClickParticlesTitle: {
		id: 'app.appearance-settings.skin-click-particles.title',
		defaultMessage: 'Skin preview click particles',
	},
	skinClickParticlesDescription: {
		id: 'app.appearance-settings.skin-click-particles.description',
		defaultMessage:
			'Spawn pixel particles when clicking the 3D skin model on the Skins page.',
	},
	skinHeadTrackingTitle: {
		id: 'app.appearance-settings.skin-head-tracking.title',
		defaultMessage: 'Skin preview head tracking',
	},
	skinHeadTrackingDescription: {
		id: 'app.appearance-settings.skin-head-tracking.description',
		defaultMessage:
			'The skin model\'s head gently follows your mouse cursor as it moves over the preview.',
	},
	skinParticleBgTitle: {
		id: 'app.appearance-settings.skin-particle-bg.title',
		defaultMessage: 'Skin preview particle background',
	},
	skinParticleBgDescription: {
		id: 'app.appearance-settings.skin-particle-bg.description',
		defaultMessage:
			'Add floating ambient particles behind the 3D skin model for extra atmosphere.',
	},
	toggleSidebarTitle: {
		id: 'app.appearance-settings.toggle-sidebar.title',
		defaultMessage: 'Toggle sidebar',
	},
	toggleSidebarDescription: {
		id: 'app.appearance-settings.toggle-sidebar.description',
		defaultMessage: 'Enables the ability to toggle the sidebar.',
	},
})

const accentColorOptions: Array<{ value: AccentColor; color: string; label: string }> = [
	{ value: 'orange', color: 'var(--color-orange)', label: formatMessage(messages.accentColorOrange) },
	{ value: 'green', color: 'var(--color-green)', label: formatMessage(messages.accentColorGreen) },
	{ value: 'blue', color: 'var(--color-blue)', label: formatMessage(messages.accentColorBlue) },
	{ value: 'purple', color: 'var(--color-purple)', label: formatMessage(messages.accentColorPurple) },
]

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

	<div class="mt-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.accentColorTitle) }}
		</h2>
		<p class="m-0 mt-1">{{ formatMessage(messages.accentColorDescription) }}</p>

		<div
				class="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-5"
				role="radiogroup"
				:aria-label="formatMessage(messages.accentColorTitle)"
			>
			<button
				v-for="accentColor in accentColorOptions"
				:key="accentColor.value"
				type="button"
				role="radio"
				:aria-checked="themeStore.selectedAccentColor === accentColor.value"
				class="flex min-w-0 items-center gap-2 rounded-xl border border-solid px-3 py-2.5 font-semibold transition-all active:scale-[0.97]"
				:class="
					themeStore.selectedAccentColor === accentColor.value
						? 'border-brand bg-brand-highlight text-brand'
						: 'border-divider bg-button-bg text-secondary hover:border-surface-5 hover:text-contrast'
				"
				@click="themeStore.setAccentColor(accentColor.value)"
			>
				<span
					class="size-4 shrink-0 rounded-full ring-2 ring-white/20"
					:style="{ backgroundColor: accentColor.color }"
				/>
				<span class="truncate">{{ accentColor.label }}</span>
				<CheckIcon
					v-if="themeStore.selectedAccentColor === accentColor.value && !themeStore.customAccentColor"
					class="ml-auto size-4 shrink-0"
				/>
			</button>

			<!-- Custom color picker -->
				<button
					type="button"
					role="radio"
					:aria-checked="!!themeStore.customAccentColor"
					class="custom-accent-btn flex min-w-0 items-center gap-2 rounded-xl border border-solid px-3 py-2.5 font-semibold transition-all active:scale-[0.97]"
					:class="
						themeStore.customAccentColor
							? 'border-brand bg-brand-highlight text-brand'
							: 'border-divider bg-button-bg text-secondary hover:border-surface-5 hover:text-contrast'
					"
					@click="openCustomAccentPicker"
				>
					<span
						class="size-4 shrink-0 rounded-full ring-2 ring-white/20"
						:style="{
							backgroundColor: themeStore.customAccentColor || 'transparent',
							backgroundImage: themeStore.customAccentColor
								? 'none'
								: 'conic-gradient(red, yellow, lime, aqua, blue, magenta, red)',
						}"
					/>
					<span class="truncate">Custom</span>
					<CheckIcon
						v-if="themeStore.customAccentColor"
						class="ml-auto size-4 shrink-0"
					/>
					<input
						ref="customAccentInput"
						type="color"
						class="sr-only"
						:value="themeStore.customAccentColor || '#ff496e'"
						@input="(e: Event) => themeStore.setCustomAccentColor((e.target as HTMLInputElement).value)"
						@click.stop
					/>
				</button>
		</div>
	</div>

	<div class="mt-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.customBackgroundTitle) }}
		</h2>
		<p class="m-0 mt-1">{{ formatMessage(messages.customBackgroundDescription) }}</p>

		<div
			class="relative mt-3 h-44 overflow-hidden rounded-2xl border border-solid border-divider bg-bg"
		>
			<div
				v-if="backgroundPreviewUrl"
				class="absolute -inset-10 bg-cover bg-center"
				:style="{
					backgroundImage: `url(&quot;${backgroundPreviewUrl}&quot;)`,
					filter: `blur(${themeStore.backgroundBlur}px)`,
					opacity: themeStore.backgroundOpacity / 100,
				}"
			/>
			<div class="absolute inset-0 bg-bg/35" />
			<div class="relative flex h-full items-center justify-center">
				<div
					v-if="!backgroundPreviewUrl"
					class="flex flex-col items-center gap-2 text-secondary"
				>
					<ImageIcon class="size-8" />
					<span class="font-semibold">{{ formatMessage(messages.customBackgroundEmpty) }}</span>
				</div>
			</div>
		</div>

		<div class="mt-3 flex flex-wrap gap-2">
			<ButtonStyled>
				<button type="button" @click="selectBackgroundImage">
					<UploadIcon />
					{{
						formatMessage(
							backgroundPreviewUrl ? messages.customBackgroundReplace : messages.customBackgroundChoose,
						)
					}}
				</button>
			</ButtonStyled>
			<ButtonStyled v-if="backgroundPreviewUrl" color="red" type="outlined">
				<button type="button" @click="clearBackgroundImage">
					<TrashIcon />
					{{ formatMessage(messages.customBackgroundRemove) }}
				</button>
			</ButtonStyled>
		</div>

		<div v-if="backgroundPreviewUrl" class="mt-5 grid gap-5 lg:grid-cols-2">
			<div class="flex flex-col gap-2">
				<h3 class="m-0 font-semibold text-contrast">
					{{ formatMessage(messages.customBackgroundBlur) }}
				</h3>
				<Slider
					id="custom-background-blur"
					v-model="backgroundBlur"
					:min="0"
					:max="40"
					:step="1"
					unit="px"
				/>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.customBackgroundBlurDescription) }}
				</p>
			</div>
			<div class="flex flex-col gap-2">
				<h3 class="m-0 font-semibold text-contrast">
					{{ formatMessage(messages.customBackgroundOpacity) }}
				</h3>
				<Slider
					id="custom-background-opacity"
					v-model="backgroundOpacity"
					:min="10"
					:max="100"
					:step="5"
					unit="%"
				/>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.customBackgroundOpacityDescription) }}
				</p>
			</div>
		</div>
	</div>

		<!-- Theme pack manager: temporarily disabled (feature is a work in progress) -->
		<!-- <ThemePackManager /> -->

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

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.imageViewerTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.imageViewerDescription) }}</p>
		</div>
		<Toggle
			id="app-image-viewer"
			:model-value="themeStore.useAppImageViewer"
			@update:model-value="(e: boolean) => themeStore.setUseAppImageViewer(!!e)"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.settingsAsPageTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.settingsAsPageDescription) }}</p>
		</div>
		<Toggle
			id="settings-as-page"
			:model-value="themeStore.settingsAsPage"
			@update:model-value="(e: boolean) => themeStore.setSettingsAsPage(!!e)"
		/>
	</div>

	<div class="mt-8 mb-2 border-t border-surface-5 pt-6">
		<h2 class="m-0 text-sm font-bold uppercase tracking-wide text-tertiary">
			Skin Preview Effects
		</h2>
	</div>

	<div class="mt-4 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.skinClickParticlesTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.skinClickParticlesDescription) }}</p>
		</div>
		<Toggle
			id="skin-click-particles"
			:model-value="themeStore.skinClickParticles"
			@update:model-value="(e: boolean) => themeStore.setSkinClickParticles(!!e)"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.skinHeadTrackingTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.skinHeadTrackingDescription) }}</p>
		</div>
		<Toggle
			id="skin-head-tracking"
			:model-value="themeStore.skinHeadTracking"
			@update:model-value="(e: boolean) => themeStore.setSkinHeadTracking(!!e)"
		/>
	</div>

	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.skinParticleBgTitle) }}
			</h2>
			<p class="m-0 mt-1">{{ formatMessage(messages.skinParticleBgDescription) }}</p>
		</div>
		<Toggle
			id="skin-particle-bg"
			:model-value="themeStore.skinParticleBackground"
			@update:model-value="(e: boolean) => themeStore.setSkinParticleBackground(!!e)"
		/>
	</div>
</template>
