<script setup lang="ts">
import {
	injectModrinthClient,
	injectModrinthServerContext,
	ServersManageFilesPage,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { onMounted } from 'vue'

const client = injectModrinthClient()
const { serverId } = injectModrinthServerContext()
const queryClient = useQueryClient()

onMounted(async () => {
	try {
		await queryClient.ensureQueryData({
			queryKey: ['files', serverId, '/'],
			queryFn: () => client.kyros.files_v0.listDirectory('/', 1, 2000),
			staleTime: 30_000,
		})
	} catch {
		// Let mounted layouts' useQuery surface errors; do not fail route setup.
	}
})
</script>

<template>
	<ServersManageFilesPage />
</template>
