<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'

import GridDisplay from '@/components/GridDisplay.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	noCustom: { id: 'app.library.no-custom', defaultMessage: 'No custom instances found' },
})

defineProps({
	instances: {
		type: Array,
		required: true,
	},
})
</script>
<template>
	<GridDisplay
		v-if="instances && instances.filter((i) => !i.link).length > 0"
		label="Instances"
		:instances="instances.filter((i) => !i.link)"
	/>
	<div v-else class="flex flex-1 items-center justify-center text-secondary min-h-[12rem]">
		{{ formatMessage(messages.noCustom) }}
	</div>
</template>
