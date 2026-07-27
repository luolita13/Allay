<script setup lang="ts">
import { defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { optInAnalytics, optOutAnalytics } from '@/helpers/analytics'
import { get, set } from '@/helpers/settings.ts'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	personalizedAds: {
		id: 'app.privacy-settings.personalized-ads',
		defaultMessage: 'Personalized Ads',
	},
	personalizedAdsDesc: {
		id: 'app.privacy-settings.personalized-ads.desc',
		defaultMessage:
			'Modrinth\'s ad provider, Aditude, shows ads based on your preferences. When disabled, ads will no longer be based on your interests.',
	},
	telemetry: {
		id: 'app.privacy-settings.telemetry',
		defaultMessage: 'Telemetry',
	},
	telemetryDesc: {
		id: 'app.privacy-settings.telemetry.desc',
		defaultMessage:
			'Modrinth collects anonymous usage analytics and data to improve the user experience and provide personalized services. When disabled, this data will no longer be collected.',
	},
	discordRpc: {
		id: 'app.privacy-settings.discord-rpc',
		defaultMessage: 'Discord RPC',
	},
	discordRpcDesc: {
		id: 'app.privacy-settings.discord-rpc.desc',
		defaultMessage:
			'Manages Discord Rich Presence integration. When disabled, "Modrinth" will no longer appear as a game or app on your Discord profile.',
	},
	discordRpcNote: {
		id: 'app.privacy-settings.discord-rpc.note',
		defaultMessage:
			'Note: This setting does not prevent instance-specific Discord Rich Presence integrations (such as those added by mods). (Requires app restart)',
	},
	disabledInBuild: {
		id: 'app.privacy-settings.disabled-in-build',
		defaultMessage: 'Disabled in this build',
	},
})

const settings = ref(await get())

// Permanently disable privacy features for customized build
settings.value.personalized_ads = false
settings.value.telemetry = false
settings.value.discord_rpc = false
// Save the disabled state immediately
await set(settings.value)

watch(
	settings,
	async () => {
		if (settings.value.telemetry) {
			optInAnalytics()
		} else {
			optOutAnalytics()
		}

		await set(settings.value)
	},
	{ deep: true },
)
</script>

<template>
	<div class="flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.personalizedAds) }}</h2>
			<p class="m-0 mt-1 text-sm">
				{{ formatMessage(messages.personalizedAdsDesc) }}
			</p>
			<span class="text-xs text-secondary italic">{{ formatMessage(messages.disabledInBuild) }}</span>
		</div>
		<Toggle id="personalized-ads" v-model="settings.personalized_ads" :disabled="true" />
	</div>

	<div class="mt-4 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.telemetry) }}</h2>
			<p class="m-0 mt-1 text-sm">
				{{ formatMessage(messages.telemetryDesc) }}
			</p>
			<span class="text-xs text-secondary italic">{{ formatMessage(messages.disabledInBuild) }}</span>
		</div>
		<Toggle id="opt-out-analytics" v-model="settings.telemetry" :disabled="true" />
	</div>

	<div class="mt-4 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.discordRpc) }}</h2>
			<p class="m-0 mt-1 text-sm">
				{{ formatMessage(messages.discordRpcDesc) }}
			</p>
			<p class="m-0 mt-2 text-sm">
				{{ formatMessage(messages.discordRpcNote) }}
			</p>
			<span class="text-xs text-secondary italic">{{ formatMessage(messages.disabledInBuild) }}</span>
		</div>
		<Toggle id="disable-discord-rpc" v-model="settings.discord_rpc" :disabled="true" />
	</div>
</template>
