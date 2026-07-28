<script setup lang="ts">
import { ArrowLeftIcon, DownloadIcon, HeartIcon, SearchIcon, SpinnerIcon } from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	defineMessages,
	injectModrinthClient,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

dayjs.extend(relativeTime)

const route = useRoute()
const router = useRouter()
const client = injectModrinthClient()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.collection.title',
		defaultMessage: 'Collection',
	},
	backToBrowse: {
		id: 'app.collection.back-to-browse',
		defaultMessage: 'Back to browse',
	},
	projectCount: {
		id: 'app.collection.project-count',
		defaultMessage: '{count, plural, one {# project} other {# projects}}',
	},
	createdLabel: {
		id: 'app.collection.created',
		defaultMessage: 'Created',
	},
	updatedLabel: {
		id: 'app.collection.updated',
		defaultMessage: 'Updated',
	},
	searchPlaceholder: {
		id: 'app.collection.search-placeholder',
		defaultMessage: 'Search projects...',
	},
	loadingProjects: {
		id: 'app.collection.loading-projects',
		defaultMessage: 'Loading projects...',
	},
	emptyCollection: {
		id: 'app.collection.empty',
		defaultMessage: 'This collection is empty.',
	},
	noSearchResults: {
		id: 'app.collection.no-results',
		defaultMessage: 'No projects match your search.',
	},
	notFound: {
		id: 'app.collection.not-found',
		defaultMessage: 'Collection not found.',
	},
	byAuthor: {
		id: 'app.collection.by-author',
		defaultMessage: 'By {author}',
	},
})

const collectionId = computed(() => route.params.id as string)
const searchQuery = ref('')

const {
	data: collection,
	isLoading: isLoadingCollection,
	isError: isCollectionError,
} = useQuery({
	queryKey: ['collection', collectionId],
	queryFn: () => client.labrinth.collections.get(collectionId.value),
	enabled: computed(() => !!collectionId.value),
})

const { data: creator, isLoading: isLoadingCreator } = useQuery({
	queryKey: ['collection-creator', () => collection.value?.user],
	queryFn: () => client.labrinth.users_v3.get(collection.value!.user),
	enabled: computed(() => !!collection.value?.user),
})

const projectIds = computed(() => collection.value?.projects ?? [])

const { data: projects, isLoading: isLoadingProjects } = useQuery({
	queryKey: ['collection-projects', projectIds],
	queryFn: () => client.labrinth.projects_v3.getMultiple(projectIds.value),
	enabled: computed(() => projectIds.value.length > 0),
})

const filteredProjects = computed(() => {
	const q = searchQuery.value.toLowerCase().trim()
	const list = projects.value?.slice().sort((a, b) => b.followers - a.followers) ?? []
	if (!q) return list
	return list.filter((p) => p.name.toLowerCase().includes(q))
})

const isFullyLoading = computed(
	() => isLoadingCollection.value || isLoadingCreator.value || isLoadingProjects.value,
)
</script>

<template>
	<div class="collection-page">
		<!-- Loading -->
		<div v-if="isFullyLoading" class="flex min-h-[50vh] items-center justify-center">
			<SpinnerIcon class="h-12 w-12 animate-spin text-brand" />
		</div>

		<!-- Not found -->
		<div
			v-else-if="isCollectionError || !collection"
			class="flex min-h-[50vh] flex-col items-center justify-center gap-4"
		>
			<p class="text-lg text-secondary">{{ formatMessage(messages.notFound) }}</p>
			<ButtonStyled type="outlined" @click="router.push('/browse/mod')">
				<button>
					<ArrowLeftIcon class="size-4" />
					{{ formatMessage(messages.backToBrowse) }}
				</button>
			</ButtonStyled>
		</div>

		<!-- Collection content -->
		<template v-else>
			<!-- Header -->
			<div class="collection-header">
				<ButtonStyled type="transparent" @click="router.back()">
					<button class="flex items-center gap-1">
						<ArrowLeftIcon class="size-4" />
						{{ formatMessage(messages.backToBrowse) }}
					</button>
				</ButtonStyled>

				<div class="collection-hero">
					<Avatar :src="collection.icon_url" size="64px" />
					<div class="flex flex-col gap-1">
						<h1 class="m-0 text-2xl font-bold text-contrast">{{ collection.name }}</h1>
						<div class="flex items-center gap-2 text-sm text-secondary">
							<span v-if="creator">
								{{ formatMessage(messages.byAuthor, { author: creator.username }) }}
							</span>
							<span>•</span>
							<span>
								{{
									formatMessage(messages.projectCount, {
										count: collection.projects.length,
									})
								}}
							</span>
						</div>
					</div>
				</div>

				<p v-if="collection.description" class="m-0 text-secondary">
					{{ collection.description }}
				</p>

				<div class="collection-meta">
					<span class="text-xs text-secondary">
						{{ formatMessage(messages.createdLabel) }}
						{{ dayjs(collection.created).fromNow() }}
					</span>
					<span class="text-xs text-secondary">
						{{ formatMessage(messages.updatedLabel) }}
						{{ dayjs(collection.updated).fromNow() }}
					</span>
				</div>
			</div>

			<!-- Search -->
			<StyledInput
				v-if="projectIds.length > 5"
				v-model="searchQuery"
				:icon="SearchIcon"
				type="text"
				autocomplete="off"
				:spellcheck="false"
				input-class="!h-10"
				wrapper-class="flex-1 min-w-0 max-w-md"
				clearable
				:placeholder="formatMessage(messages.searchPlaceholder)"
			/>

			<!-- Project list -->
			<div class="flex flex-col gap-2">
				<span v-if="isLoadingProjects" class="w-full py-12 text-center text-secondary">
					{{ formatMessage(messages.loadingProjects) }}
				</span>
				<template v-else>
					<div
						v-for="project in filteredProjects"
						:key="project.id"
						class="project-card"
						@click="router.push(`/project/${project.id}`)"
					>
						<Avatar :src="project.icon_url" size="48px" />
						<div class="flex flex-col gap-1 min-w-0">
							<span class="flex items-center gap-2">
								<span class="font-medium text-contrast truncate">
									{{ project.name }}
								</span>
								<span class="flex items-center gap-1 text-sm text-secondary">
									<DownloadIcon class="size-4" />
									{{ project.downloads }}
								</span>
								<span class="flex items-center gap-1 text-sm text-secondary">
									<HeartIcon class="size-4" />
									{{ project.followers }}
								</span>
							</span>
							<span class="text-sm text-secondary truncate">
								{{ project.summary }}
							</span>
						</div>
					</div>
					<span v-if="projects?.length === 0" class="w-full py-12 text-center text-secondary">
						{{ formatMessage(messages.emptyCollection) }}
					</span>
					<span
						v-else-if="filteredProjects.length === 0"
						class="w-full py-12 text-center text-secondary"
					>
						{{ formatMessage(messages.noSearchResults) }}
					</span>
				</template>
			</div>
		</template>
	</div>
</template>

<style scoped lang="scss">
.collection-page {
	display: flex;
	flex-direction: column;
	gap: var(--gap-lg);
	padding: var(--gap-md) 0;
}

.collection-header {
	display: flex;
	flex-direction: column;
	gap: var(--gap-md);
}

.collection-hero {
	display: flex;
	align-items: center;
	gap: var(--gap-md);
}

.collection-meta {
	display: flex;
	gap: var(--gap-lg);
}

.project-card {
	display: flex;
	align-items: center;
	gap: var(--gap-md);
	padding: 0.75rem 1rem;
	border-radius: 0.5rem;
	background: var(--color-surface-2);
	border: 1px solid var(--color-surface-5);
	cursor: pointer;
	transition: all 0.15s ease;

	&:hover {
		background: var(--color-surface-3);
		border-color: var(--color-brand);
	}
}
</style>
