<script setup lang="ts">
import { ButtonStyled, defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { useTheming } from '@/store/state'
import { DEFAULT_FEATURE_FLAGS, type FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()
const { formatMessage } = useVIntl()

const settings = ref(await getSettings())

const messages = defineMessages({
	resetToDefault: {
		id: 'app.feature-flag-settings.reset-to-default',
		defaultMessage: 'Reset to Default',
	},
	experimentalBadge: {
		id: 'app.feature-flag-settings.experimental-badge',
		defaultMessage: 'Experimental',
	},
	routeTransitionsLabel: {
		id: 'app.feature-flag-settings.route-transitions-label',
		defaultMessage: 'Route Transitions',
	},
	routeTransitionsDescription: {
		id: 'app.feature-flag-settings.route-transitions-description',
		defaultMessage: 'Enable route transitions between pages.\nIF YOU ENCOUNTER PROBLEMS, DISABLE THIS FEATURE.',
	},
})

interface FeatureFlagDefinition {
	key: FeatureFlag
	label: keyof typeof messages
	description: keyof typeof messages
	experimental?: boolean
}

const FEATURE_FLAGS: FeatureFlagDefinition[] = [
	{
		key: 'route_transitions',
		label: 'routeTransitionsLabel',
		description: 'routeTransitionsDescription',
		experimental: true,
	},
]

function setFeatureFlag(key: FeatureFlag, value: boolean) {
	themeStore.featureFlags[key] = value
	settings.value.feature_flags[key] = value
}

function isDefault(key: FeatureFlag) {
	return themeStore.getFeatureFlag(key) === DEFAULT_FEATURE_FLAGS[key]
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
	<div class="flex flex-col gap-2.5 min-w-[600px]">
		<div
			v-for="option in FEATURE_FLAGS"
			:key="option.key"
			class="flex items-center justify-between"
		>
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast flex items-center gap-2">
					{{ formatMessage(messages[option.label]) }}
					<span
						v-if="option.experimental"
						class="text-xs px-2 py-0.5 rounded-full bg-brand-highlight text-brand font-medium"
					>
						{{ formatMessage(messages.experimentalBadge) }}
					</span>
				</h2>
				<p class="m-0 mt-1 text-sm text-secondary max-w-md">
					{{ formatMessage(messages[option.description]) }}
				</p>
			</div>
			<div class="flex items-center gap-2 shrink-0">
				<ButtonStyled type="transparent">
					<button
						:disabled="isDefault(option.key)"
						@click="setFeatureFlag(option.key, DEFAULT_FEATURE_FLAGS[option.key])"
					>
						{{ formatMessage(messages.resetToDefault) }}
					</button>
				</ButtonStyled>
				<Toggle
					:id="`feature-flag-${option.key}`"
					:model-value="themeStore.getFeatureFlag(option.key)"
					@update:model-value="() => setFeatureFlag(option.key, !themeStore.getFeatureFlag(option.key))"
				/>
			</div>
		</div>
	</div>
</template>
