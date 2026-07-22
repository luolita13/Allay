<script setup lang="ts">
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import type { SearchResult } from '@modrinth/utils'
import dayjs from 'dayjs'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import RowDisplay from '@/components/RowDisplay.vue'
import HomeRandomMods from '@/components/ui/home/HomeRandomMods.vue'
import HomeRecentScreenshots from '@/components/ui/home/HomeRecentScreenshots.vue'
import HomeSystemStatus from '@/components/ui/home/HomeSystemStatus.vue'
import HomeUpdateReminders from '@/components/ui/home/HomeUpdateReminders.vue'
import RecentWorldsList from '@/components/ui/world/RecentWorldsList.vue'
import { get_search_results } from '@/helpers/cache.js'
import { instance_listener } from '@/helpers/events'
import { list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useTheming } from '@/store/state'

const { handleError } = injectNotificationManager()
const route = useRoute()
const breadcrumbs = useBreadcrumbs()
const themeStore = useTheming()

const { formatMessage } = useVIntl()

const messages = defineMessages({
  home: { id: 'app.home.title', defaultMessage: 'Home' },
  welcomeBack: { id: 'app.home.welcome-back', defaultMessage: 'Welcome back!' },
  welcomeTo: { id: 'app.home.welcome-to', defaultMessage: 'Welcome to Modrinth App!' },
  discoverModpack: { id: 'app.home.discover-modpack', defaultMessage: 'Discover a modpack' },
  discoverMods: { id: 'app.home.discover-mods', defaultMessage: 'Discover mods' },
})

breadcrumbs.setRootContext({ name: formatMessage(messages.home), link: route.path })

const instances = ref<GameInstance[]>([])

const featuredModpacks = ref<SearchResult[]>([])
const featuredMods = ref<SearchResult[]>([])
const installedModpacksFilter = ref('')

const recentInstances = computed(() =>
	instances.value
		.filter((x) => x.last_played)
		.slice()
		.sort((a, b) => dayjs(b.last_played).diff(dayjs(a.last_played))),
)

const showJumpBackIn = computed(() => themeStore.getFeatureFlag('home_show_jump_back_in'))
const showDiscoverModpacks = computed(() => themeStore.getFeatureFlag('home_show_discover_modpacks'))
const showDiscoverMods = computed(() => themeStore.getFeatureFlag('home_show_discover_mods'))
const showUpdateReminders = computed(() => themeStore.getFeatureFlag('home_show_update_reminders'))
const showSystemStatus = computed(() => themeStore.getFeatureFlag('home_show_system_status'))
const showRecentScreenshots = computed(() => themeStore.getFeatureFlag('home_show_recent_screenshots'))
const showRandomMods = computed(() => themeStore.getFeatureFlag('home_show_random_mods'))

const showFeaturedRow = computed(
	() => (showDiscoverModpacks.value && featuredModpacks.value.length > 0) ||
		(showDiscoverMods.value && featuredMods.value.length > 0),
)

const offline = ref<boolean>(!navigator.onLine)
window.addEventListener('offline', () => {
	offline.value = true
})
window.addEventListener('online', () => {
	offline.value = false
})

async function fetchInstances() {
	instances.value = await list().catch(handleError)

	const filters = []
	for (const instance of instances.value) {
		if (instance.link && instance.link.project_id) {
			filters.push(`NOT"project_id"="${instance.link.project_id}"`)
		}
	}
	installedModpacksFilter.value = filters.join(' AND ')
}

async function fetchFeaturedModpacks() {
	const response = await get_search_results(
		`?facets=[["project_type:modpack"]]&limit=10&index=follows&filters=${installedModpacksFilter.value}`,
	)

	if (response) {
		featuredModpacks.value = response.result.hits
	} else {
		featuredModpacks.value = []
	}
}

async function fetchFeaturedMods() {
	const response = await get_search_results('?facets=[["project_type:mod"]]&limit=10&index=follows')

	if (response) {
		featuredMods.value = response.result.hits
	} else {
		featuredModpacks.value = []
	}
}

async function refreshFeaturedProjects() {
	await Promise.all([fetchFeaturedModpacks(), fetchFeaturedMods()])
}

const featuredRows = computed(() => {
	const rows: Array<{
		label: string
		route: string
		instances: SearchResult[]
		downloaded: boolean
		show?: boolean
	}> = []
	if (showDiscoverModpacks.value && featuredModpacks.value.length > 0) {
		rows.push({
			label: formatMessage(messages.discoverModpack),
			route: '/browse/modpack',
			instances: featuredModpacks.value,
			downloaded: false,
		})
	}
	if (showDiscoverMods.value && featuredMods.value.length > 0) {
		rows.push({
			label: formatMessage(messages.discoverMods),
			route: '/browse/mod',
			instances: featuredMods.value,
			downloaded: false,
		})
	}
	return rows
})

let unlistenInstance: (() => void) | null = null

onMounted(async () => {
	await fetchInstances()
	await refreshFeaturedProjects()

	unlistenInstance = await instance_listener(
		async (e: { event: string; instance_id: string }) => {
			await fetchInstances()

			if (e.event === 'added' || e.event === 'created' || e.event === 'removed') {
				await refreshFeaturedProjects()
			}
		},
	)
})

onUnmounted(() => {
	if (unlistenInstance) {
		unlistenInstance()
	}
})
</script>

<template>
	<div class="p-6 flex flex-col gap-4">
		<h1 v-if="recentInstances?.length > 0" class="m-0 text-2xl font-extrabold">{{ formatMessage(messages.welcomeBack) }}</h1>
		<h1 v-else class="m-0 text-2xl font-extrabold">{{ formatMessage(messages.welcomeTo) }}</h1>

		<RecentWorldsList v-if="showJumpBackIn" :recent-instances="recentInstances" />

		<HomeUpdateReminders v-if="showUpdateReminders" />

		<HomeSystemStatus v-if="showSystemStatus" />

		<RowDisplay
			v-if="showFeaturedRow && featuredRows.length > 0"
			:instances="featuredRows"
			:can-paginate="true"
		/>

		<HomeRandomMods v-if="showRandomMods" />

		<HomeRecentScreenshots v-if="showRecentScreenshots" />
	</div>
</template>
