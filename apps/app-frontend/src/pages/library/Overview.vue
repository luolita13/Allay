<script setup lang="ts">
import dayjs from 'dayjs'
import { PlusIcon, SearchIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import { NewInstanceImage } from '@/assets/icons'
import GridDisplay from '@/components/GridDisplay.vue'

const { formatMessage } = useVIntl()
const router = useRouter()

const messages = defineMessages({
	libraryTitle: { id: 'app.library.overview.title', defaultMessage: 'Your library' },
	librarySubtitle: {
		id: 'app.library.overview.subtitle',
		defaultMessage: 'Manage and launch your Minecraft instances.',
	},
	createNewInstance: { id: 'app.library.create-new-instance', defaultMessage: 'Create new instance' },
	searchPlaceholder: { id: 'app.library.search.placeholder', defaultMessage: 'Search instances...' },
	allInstances: { id: 'app.library.all-instances', defaultMessage: 'All instances' },
	jumpBackIn: { id: 'app.library.jump-back-in', defaultMessage: 'Jump back in' },
	noInstances: { id: 'app.library.no-instances', defaultMessage: 'No instances found' },
	noSearchResults: {
		id: 'app.library.no-search-results',
		defaultMessage: 'No instances match your search.',
	},
})

const props = defineProps<{
	instances: any[]
}>()

const offline = ref(!navigator.onLine)
window.addEventListener('offline', () => {
	offline.value = true
})
window.addEventListener('online', () => {
	offline.value = false
})

const searchQuery = ref('')

const filteredInstances = computed(() => {
	const q = searchQuery.value.trim().toLowerCase()
	if (!q) return props.instances
	return props.instances.filter((instance: any) =>
		instance.name?.toLowerCase().includes(q),
	)
})

const recentInstances = computed(() => {
	return props.instances
		.filter((x: any) => x.last_played)
		.sort((a: any, b: any) => dayjs(b.last_played).diff(dayjs(a.last_played)))
		.slice(0, 4)
})
</script>

<template>
	<div class="library-layout">
		<!-- Hero -->
		<section class="hero">
			<div class="hero-text">
				<h1 class="m-0 text-3xl font-extrabold text-contrast">
					{{ formatMessage(messages.libraryTitle) }}
				</h1>
				<p class="m-0 mt-1 text-base text-secondary">
					{{ formatMessage(messages.librarySubtitle) }}
				</p>
			</div>
			<ButtonStyled color="brand">
				<button :disabled="offline" @click="router.push('/create')">
					<PlusIcon />
					{{ formatMessage(messages.createNewInstance) }}
				</button>
			</ButtonStyled>
		</section>

		<!-- Search -->
		<div class="search-bar">
			<SearchIcon class="search-icon" />
			<input
				v-model="searchQuery"
				type="text"
				class="search-input"
				:placeholder="formatMessage(messages.searchPlaceholder)"
			/>
		</div>

		<!-- Recent -->
		<section v-if="recentInstances.length > 0 && !searchQuery" class="recent-section">
			<h2 class="section-title">{{ formatMessage(messages.jumpBackIn) }}</h2>
			<GridDisplay :instances="recentInstances" />
		</section>

		<!-- All instances -->
		<section class="all-section">
			<h2 class="section-title">{{ formatMessage(messages.allInstances) }}</h2>
			<GridDisplay
				v-if="filteredInstances.length > 0"
				:instances="filteredInstances"
			/>
			<div v-else-if="props.instances.length > 0" class="empty-state">
				<h3 class="m-0">{{ formatMessage(messages.noSearchResults) }}</h3>
			</div>
			<div v-else class="no-instance">
				<div class="icon">
					<NewInstanceImage />
				</div>
				<h3>{{ formatMessage(messages.noInstances) }}</h3>
				<ButtonStyled color="brand">
					<button :disabled="offline" @click="router.push('/create')">
						<PlusIcon />
						{{ formatMessage(messages.createNewInstance) }}
					</button>
				</ButtonStyled>
			</div>
		</section>
	</div>
</template>

<style lang="scss" scoped>
.library-layout {
	display: flex;
	flex-direction: column;
	gap: var(--gap-xl);
	padding: var(--gap-md) 0;
}

.hero {
	display: flex;
	justify-content: space-between;
	align-items: flex-end;
	gap: var(--gap-lg);
	flex-wrap: wrap;
}

.search-bar {
	display: flex;
	align-items: center;
	gap: var(--gap-sm);
	padding: var(--gap-sm) var(--gap-md);
	background: var(--color-raised-bg);
	border-radius: var(--radius-lg);
	border: 1px solid var(--color-button-bg);
}

.search-icon {
	width: 1.25rem;
	height: 1.25rem;
	color: var(--color-text);
	flex-shrink: 0;
}

.search-input {
	flex: 1;
	background: transparent;
	border: none;
	outline: none;
	color: var(--color-contrast);
	font-size: 1rem;

	&::placeholder {
		color: var(--color-text);
	}
}

.section-title {
	margin: 0 0 var(--gap-md);
	font-size: 1.125rem;
	font-weight: 700;
	color: var(--color-contrast);
}

.empty-state {
	display: flex;
	align-items: center;
	justify-content: center;
	padding: var(--gap-3xl) var(--gap-md);
	color: var(--color-text);
}

.no-instance {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	padding: var(--gap-3xl) var(--gap-md);
	gap: var(--gap-md);

	p,
	h3 {
		margin: 0;
	}

	.icon {
		svg {
			width: 10rem;
			height: 10rem;
		}
	}
}
</style>
