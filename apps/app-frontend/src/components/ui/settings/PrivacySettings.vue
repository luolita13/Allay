<script setup lang="ts">
import { defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { optInAnalytics, optOutAnalytics } from '@/helpers/analytics'
import { get, set } from '@/helpers/settings.ts'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	telemetry: {
		id: 'app.privacy-settings.telemetry',
		defaultMessage: 'Telemetry',
	},
	telemetryDesc: {
		id: 'app.privacy-settings.telemetry.desc',
		defaultMessage:
			'Allay collects anonymous usage analytics and data to improve the user experience and provide personalized services. When disabled, this data will no longer be collected.',
	},
	disabledInBuild: {
		id: 'app.privacy-settings.disabled-in-build',
		defaultMessage: 'Disabled in this build',
	},
})

const settings = ref(await get())

// Permanently disable privacy features for customized build
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
</template>
