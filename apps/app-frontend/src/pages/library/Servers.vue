<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, ref, watchEffect } from 'vue'

import GridDisplay from '@/components/GridDisplay.vue'
import { get_project_v3_many } from '@/helpers/cache.js'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	noServers: { id: 'app.library.no-servers', defaultMessage: 'No server instances found' },
})

const props = defineProps({
	instances: {
		type: Array,
		required: true,
	},
})

const serverProjectIds = ref(new Set())

const linkedInstances = computed(() => props.instances.filter((i) => i.link))

watchEffect(async () => {
	const projectIds = [
		...new Set(linkedInstances.value.map((i) => i.link?.project_id).filter(Boolean)),
	]
	if (projectIds.length === 0) {
		serverProjectIds.value = new Set()
		return
	}

	try {
		const projects = await get_project_v3_many(projectIds, 'must_revalidate')
		serverProjectIds.value = new Set(
			projects.filter((p) => p?.minecraft_server != null).map((p) => p.id),
		)
	} catch {
		serverProjectIds.value = new Set()
	}
})

const filteredInstances = computed(() =>
	linkedInstances.value.filter((i) => serverProjectIds.value.has(i.link?.project_id)),
)
</script>
<template>
	<GridDisplay
		v-if="filteredInstances && filteredInstances.length > 0"
		label="Instances"
		:instances="filteredInstances"
	/>
	<div v-else class="flex flex-1 items-center justify-center text-secondary min-h-[12rem]">
		{{ formatMessage(messages.noServers) }}
	</div>
</template>
