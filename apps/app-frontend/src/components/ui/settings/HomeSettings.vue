<script setup lang="ts">
import { Combobox, defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { useTheming } from '@/store/state'
import type { FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()
const { formatMessage } = useVIntl()

const settings = ref(await getSettings())

const messages = defineMessages({
	homePageCustomizationTitle: {
		id: 'app.home-settings.title',
		defaultMessage: 'Home Page Customization',
	},
	homePageCustomizationDescription: {
		id: 'app.home-settings.description',
		defaultMessage: 'Toggle the various sections displayed on the home page.',
	},
	jumpBackInTitle: {
		id: 'app.home-settings.jump-back-in.title',
		defaultMessage: 'Jump Back In',
	},
	jumpBackInDescription: {
		id: 'app.home-settings.jump-back-in.description',
		defaultMessage: 'Show recently played worlds and instances.',
	},
	discoverModpacksTitle: {
		id: 'app.home-settings.discover-modpacks.title',
		defaultMessage: 'Discover Modpacks',
	},
	discoverModpacksDescription: {
		id: 'app.home-settings.discover-modpacks.description',
		defaultMessage: 'Show trending modpacks from Modrinth.',
	},
	discoverModsTitle: {
		id: 'app.home-settings.discover-mods.title',
		defaultMessage: 'Discover Mods',
	},
	discoverModsDescription: {
		id: 'app.home-settings.discover-mods.description',
		defaultMessage: 'Show trending mods from Modrinth.',
	},
	updateRemindersTitle: {
		id: 'app.home-settings.update-reminders.title',
		defaultMessage: 'Game Update Reminders',
	},
	updateRemindersDescription: {
		id: 'app.home-settings.update-reminders.description',
		defaultMessage: 'Show instances that have available modpack or mod updates.',
	},
	systemStatusTitle: {
		id: 'app.home-settings.system-status.title',
		defaultMessage: 'System Status',
	},
	systemStatusDescription: {
		id: 'app.home-settings.system-status.description',
		defaultMessage: 'Show CPU, memory, disk, and network usage along with quick launch actions.',
	},
	recentScreenshotsTitle: {
		id: 'app.home-settings.recent-screenshots.title',
		defaultMessage: 'Recent Screenshots',
	},
	recentScreenshotsDescription: {
		id: 'app.home-settings.recent-screenshots.description',
		defaultMessage: 'Show recent screenshots from all instances.',
	},
	randomModsTitle: {
		id: 'app.home-settings.random-mods.title',
		defaultMessage: 'Random Mod Recommendations',
	},
	randomModsDescription: {
		id: 'app.home-settings.random-mods.description',
		defaultMessage: 'Display a random mod recommendation each time the home page opens.',
	},
	worldsTabTitle: {
		id: 'app.home-settings.worlds-tab.title',
		defaultMessage: 'Worlds Tab',
	},
	worldsTabDescription: {
		id: 'app.home-settings.worlds-tab.description',
		defaultMessage: 'Show a standalone Worlds entry in the sidebar.',
	},
	worldsInHomeTitle: {
		id: 'app.home-settings.worlds-in-home.title',
		defaultMessage: 'Show Worlds on Home',
	},
	worldsInHomeDescription: {
		id: 'app.home-settings.worlds-in-home.description',
		defaultMessage: 'Include recently played worlds in the Jump Back In section on the home page.',
	},
	showPlayTimeTitle: {
		id: 'app.home-settings.show-play-time.title',
		defaultMessage: 'Show Play Time',
	},
	showPlayTimeDescription: {
		id: 'app.home-settings.show-play-time.description',
		defaultMessage: 'Show cumulative play time on instance cards.',
	},
	defaultLandingPageTitle: {
		id: 'app.home-settings.default-landing-page.title',
		defaultMessage: 'Default Landing Page',
	},
	defaultLandingPageDescription: {
		id: 'app.home-settings.default-landing-page.description',
		defaultMessage: 'Change the default page shown when the launcher opens.',
	},
	defaultLandingPageHome: {
		id: 'app.home-settings.default-landing-page.home',
		defaultMessage: 'Home',
	},
	defaultLandingPageLibrary: {
		id: 'app.home-settings.default-landing-page.library',
		defaultMessage: 'Library',
	},
	selectOption: {
		id: 'app.home-settings.default-landing-page.select-option',
		defaultMessage: 'Select an option',
	},
})

const sections: { flag: FeatureFlag; title: string; description: string }[] = [
	{
		flag: 'home_show_jump_back_in',
		title: formatMessage(messages.jumpBackInTitle),
		description: formatMessage(messages.jumpBackInDescription),
	},
	{
		flag: 'home_show_discover_modpacks',
		title: formatMessage(messages.discoverModpacksTitle),
		description: formatMessage(messages.discoverModpacksDescription),
	},
	{
		flag: 'home_show_discover_mods',
		title: formatMessage(messages.discoverModsTitle),
		description: formatMessage(messages.discoverModsDescription),
	},
	{
		flag: 'home_show_update_reminders',
		title: formatMessage(messages.updateRemindersTitle),
		description: formatMessage(messages.updateRemindersDescription),
	},
	{
		flag: 'home_show_system_status',
		title: formatMessage(messages.systemStatusTitle),
		description: formatMessage(messages.systemStatusDescription),
	},
	{
		flag: 'home_show_recent_screenshots',
		title: formatMessage(messages.recentScreenshotsTitle),
		description: formatMessage(messages.recentScreenshotsDescription),
	},
	{
		flag: 'home_show_random_mods',
		title: formatMessage(messages.randomModsTitle),
		description: formatMessage(messages.randomModsDescription),
	},
]

function setFeatureFlag(key: FeatureFlag, value: boolean) {
	themeStore.featureFlags[key] = value
	settings.value.feature_flags[key] = value
}

watch(
	settings,
	async () => {
		await setSettings(settings.value)
	},
	{ deep: true },
)
</script>

<template>
	<div class="flex flex-col gap-2">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.homePageCustomizationTitle) }}
		</h2>
		<p class="m-0 mt-1 mb-2">{{ formatMessage(messages.homePageCustomizationDescription) }}</p>

		<div
			v-for="section in sections"
			:key="section.flag"
			class="mt-2 flex items-center justify-between gap-4"
		>
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ section.title }}
				</h2>
				<p class="m-0 mt-1">{{ section.description }}</p>
			</div>
			<Toggle
				:model-value="themeStore.getFeatureFlag(section.flag)"
				@update:model-value="() => setFeatureFlag(section.flag, !themeStore.getFeatureFlag(section.flag))"
			/>
		</div>

		<div class="mt-6 flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.worldsTabTitle) }}
				</h2>
				<p class="m-0 mt-1">{{ formatMessage(messages.worldsTabDescription) }}</p>
			</div>
			<Toggle
				:model-value="themeStore.getFeatureFlag('worlds_tab')"
				@update:model-value="() => setFeatureFlag('worlds_tab', !themeStore.getFeatureFlag('worlds_tab'))"
			/>
		</div>

		<div class="mt-6 flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.worldsInHomeTitle) }}
				</h2>
				<p class="m-0 mt-1">{{ formatMessage(messages.worldsInHomeDescription) }}</p>
			</div>
			<Toggle
				:model-value="themeStore.getFeatureFlag('worlds_in_home')"
				@update:model-value="() => setFeatureFlag('worlds_in_home', !themeStore.getFeatureFlag('worlds_in_home'))"
			/>
		</div>

		<div class="mt-6 flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.showPlayTimeTitle) }}
				</h2>
				<p class="m-0 mt-1">{{ formatMessage(messages.showPlayTimeDescription) }}</p>
			</div>
			<Toggle
				:model-value="themeStore.getFeatureFlag('show_instance_play_time')"
				@update:model-value="() => setFeatureFlag('show_instance_play_time', !themeStore.getFeatureFlag('show_instance_play_time'))"
			/>
		</div>

		<hr class="my-6 bg-button-border border-none h-[1px]" />

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
				:display-value="settings.default_page ?? formatMessage(messages.selectOption)"
			/>
		</div>
	</div>
</template>
