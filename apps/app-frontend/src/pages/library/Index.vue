<script setup lang="ts">
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import { instance_listener } from '@/helpers/events.js'
import { list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { useBreadcrumbs } from '@/store/breadcrumbs.js'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	libraryTitle: { id: 'app.library.title', defaultMessage: 'Library' },
})

const route = useRoute()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: formatMessage(messages.libraryTitle), link: route.path })

const instances = ref<GameInstance[]>([])

async function fetchInstances() {
	instances.value = (await list().catch(handleError)) ?? []
}

let unlistenInstance: (() => void) | null = null

onMounted(async () => {
	await fetchInstances()
	unlistenInstance = await instance_listener(async () => {
		await fetchInstances()
	})
})

onUnmounted(() => {
	if (unlistenInstance) {
		unlistenInstance()
	}
})
</script>

<template>
	<div class="p-6 flex flex-col gap-3 h-full">
		<h1 class="m-0 text-2xl hidden">{{ formatMessage(messages.libraryTitle) }}</h1>
		<div class="flex-1 min-h-0">
			<RouterView v-slot="{ Component }">
				<component :is="Component" :key="route.path" :instances="instances ?? []" />
			</RouterView>
		</div>
	</div>
</template>

<style lang="scss" scoped>
</style>
