<template>
	<div>
		<Card>
			<Breadcrumbs
				:current-title="version.name"
				:link-stack="[
					{
						href: buildProjectHref(`/project/${route.params.id}/versions`),
						label: formatMessage(messages.versions),
					},
				]"
			/>
			<div class="version-title">
				<h2>{{ version.name }}</h2>
			</div>
			<div class="button-group">
				<ButtonStyled color="brand">
					<button
						:disabled="installing || (installed && installedVersion === version.id)"
						@click="() => install(version.id)"
					>
						<DownloadIcon v-if="!installed" />
						<SwapIcon v-else-if="installedVersion !== version.id" />
						<CheckIcon v-else />
						{{
							installing
								? formatMessage(messages.installing)
								: installed && installedVersion === version.id
									? formatMessage(messages.installed)
									: formatMessage(messages.install)
						}}
					</button>
				</ButtonStyled>
				<ButtonStyled>
					<button>
						<ReportIcon />
						{{ formatMessage(messages.report) }}
					</button>
				</ButtonStyled>
				<ButtonStyled>
					<a
						:href="`https://modrinth.com/mod/${route.params.id}/version/${route.params.version}`"
						rel="external"
					>
						{{ formatMessage(messages.modrinthWebsite) }}
						<ExternalIcon />
					</a>
				</ButtonStyled>
			</div>
		</Card>
		<div class="version-container">
			<div class="description-cards">
				<Card>
					<h3 class="card-title">{{ formatMessage(messages.changelog) }}</h3>
					<div class="markdown-body" v-html="renderString(version.changelog ?? '')" />
				</Card>
				<Card>
					<h3 class="card-title">{{ formatMessage(messages.files) }}</h3>
					<Card
						v-for="file in version.files"
						:key="file.id"
						:class="{ primary: file.primary }"
						class="file"
					>
						<span class="label">
							<FileIcon />
							<span>
								<span class="title">
									{{ file.filename }}
								</span>
								({{ formatBytes(file.size) }})
								<span v-if="file.primary" class="primary-label"> {{ formatMessage(messages.primary) }} </span>
							</span>
						</span>
						<ButtonStyled v-if="project.project_type !== 'modpack' || file.primary" color="brand">
							<button class="download" :disabled="installed" @click="() => install(version.id)">
								<DownloadIcon v-if="!installed" />
								<CheckIcon v-else />
								{{ installed ? formatMessage(messages.installed) : formatMessage(messages.install) }}
							</button>
						</ButtonStyled>
					</Card>
				</Card>
				<Card v-if="displayDependencies.length > 0">
					<h2>{{ formatMessage(messages.dependencies) }}</h2>
					<div v-for="dependency in displayDependencies" :key="dependency.title">
						<router-link v-if="dependency.link" class="btn dependency" :to="dependency.link">
							<Avatar size="sm" :src="dependency.icon" />
							<div>
								<span class="title"> {{ dependency.title }} </span> <br />
								<span> {{ dependency.subtitle }} </span>
							</div>
						</router-link>
						<div v-else class="dependency disabled" disabled="">
							<Avatar size="sm" :src="dependency.icon" />
							<div class="text">
								<div class="title">{{ dependency.title }}</div>
								<div>{{ dependency.subtitle }}</div>
							</div>
						</div>
					</div>
				</Card>
			</div>
			<Card class="metadata-card">
				<h3 class="card-title">{{ formatMessage(messages.metadata) }}</h3>
				<div class="metadata">
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.releaseChannel) }}</span>
						<span class="metadata-value"
							><Badge
								:color="releaseColor(version.version_type)"
								:type="
									version.version_type.charAt(0).toUpperCase() + version.version_type.slice(1)
								"
						/></span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.versionNumber) }}</span>
						<span class="metadata-value">{{ version.version_number }}</span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.loaders) }}</span>
						<span class="metadata-value">{{
							version.loaders
								.map((loader) => loader.charAt(0).toUpperCase() + loader.slice(1))
								.join(', ')
						}}</span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.gameVersions) }}</span>
						<span class="metadata-value"> {{ version.game_versions.join(', ') }} </span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.downloads) }}</span>
						<span class="metadata-value">{{ version.downloads }}</span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.publicationDate) }}</span>
						<span class="metadata-value">
							{{ formatDateTime(version.date_published) }}
						</span>
					</div>
					<div v-if="author" class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.author) }}</span>
						<a
							:href="`https://modrinth.com/user/${author.user.username}`"
							rel="external"
							class="metadata-value btn author"
						>
							<Avatar size="sm" :src="author.user.avatar_url" circle />
							<span>
								<strong>
									{{ author.user.username }}
								</strong>
								<br />
								{{ author.role }}
							</span>
						</a>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">{{ formatMessage(messages.versionId) }}</span>
						<span class="metadata-value"><CopyCode class="copycode" :text="version.id" /></span>
					</div>
				</div>
			</Card>
		</div>
	</div>
</template>

<script setup lang="ts">
import { CheckIcon, DownloadIcon, ExternalIcon, FileIcon, ReportIcon } from '@modrinth/assets'
import {
	Avatar,
	Badge,
	Breadcrumbs,
	ButtonStyled,
	Card,
	CopyCode,
	defineMessages,
	useFormatBytes,
	useFormatDateTime,
	useVIntl,
} from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import { SwapIcon } from '@/assets/icons'
import { get_project_many, get_version_many } from '@/helpers/cache.js'
import { releaseColor } from '@/helpers/utils'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	versions: { id: 'app.project.version.versions', defaultMessage: 'Versions' },
	installing: { id: 'app.project.version.installing', defaultMessage: 'Installing...' },
	installed: { id: 'app.project.version.installed', defaultMessage: 'Installed' },
	install: { id: 'app.project.version.install', defaultMessage: 'Install' },
	report: { id: 'app.project.version.report', defaultMessage: 'Report' },
	modrinthWebsite: { id: 'app.project.version.modrinth-website', defaultMessage: 'Modrinth website' },
	changelog: { id: 'app.project.version.changelog', defaultMessage: 'Changelog' },
	files: { id: 'app.project.version.files', defaultMessage: 'Files' },
	primary: { id: 'app.project.version.primary', defaultMessage: 'Primary' },
	dependencies: { id: 'app.project.version.dependencies', defaultMessage: 'Dependencies' },
	metadata: { id: 'app.project.version.metadata', defaultMessage: 'Metadata' },
	releaseChannel: { id: 'app.project.version.release-channel', defaultMessage: 'Release Channel' },
	versionNumber: { id: 'app.project.version.version-number', defaultMessage: 'Version Number' },
	loaders: { id: 'app.project.version.loaders', defaultMessage: 'Loaders' },
	gameVersions: { id: 'app.project.version.game-versions', defaultMessage: 'Game Versions' },
	downloads: { id: 'app.project.version.downloads', defaultMessage: 'Downloads' },
	publicationDate: { id: 'app.project.version.publication-date', defaultMessage: 'Publication Date' },
	author: { id: 'app.project.version.author', defaultMessage: 'Author' },
	versionId: { id: 'app.project.version.version-id', defaultMessage: 'Version ID' },
	versionDependency: {
		id: 'app.project.version.dependency.subtitle',
		defaultMessage: 'Version {versionNumber} is {dependencyType}',
	},
	addedViaOverrides: {
		id: 'app.project.version.dependency.added-via-overrides',
		defaultMessage: 'Added via overrides',
	},
})

const formatDateTime = useFormatDateTime({
	timeStyle: 'short',
	dateStyle: 'long',
})
const formatBytes = useFormatBytes()

const breadcrumbs = useBreadcrumbs()

const route = useRoute()

const props = defineProps({
	project: {
		type: Object,
		required: true,
	},
	versions: {
		type: Array,
		required: true,
	},
	members: {
		type: Array,
		required: true,
	},
	install: {
		type: Function,
		required: true,
	},
	installed: {
		type: Boolean,
		required: true,
	},
	installing: {
		type: Boolean,
		required: true,
	},
	installedVersion: {
		type: String,
		required: true,
	},
})

const version = ref(props.versions.find((version) => version.id === route.params.version))
breadcrumbs.setName(formatMessage(messages.versions), version.value.name)

watch(
	() => props.versions,
	async () => {
		if (route.params.version) {
			version.value = props.versions.find((version) => version.id === route.params.version)
			await refreshDisplayDependencies()
			breadcrumbs.setName(formatMessage(messages.versions), version.value.name)
		}
	},
)

const author = computed(() =>
	props.members ? props.members.find((member) => member.user.id === version.value.author_id) : null,
)

const displayDependencies = ref({})

function buildProjectHref(path) {
	const params = new URLSearchParams()
	for (const [key, val] of Object.entries(route.query)) {
		if (Array.isArray(val)) {
			for (const v of val) params.append(key, v)
		} else if (val) {
			params.append(key, String(val))
		}
	}
	const qs = params.toString()
	return qs ? `${path}?${qs}` : path
}

async function refreshDisplayDependencies() {
	const projectIds = new Set()
	const versionIds = new Set()
	if (version.value.dependencies) {
		for (const dependency of version.value.dependencies) {
			if (dependency.project_id) {
				projectIds.add(dependency.project_id)
			}
			if (dependency.version_id) {
				versionIds.add(dependency.version_id)
			}
		}
	}
	const [projectDeps, versionDeps] = await Promise.all([
		get_project_many([...projectIds]),
		get_version_many([...versionIds]),
	])

	const dependencies = {
		projects: projectDeps,
		versions: versionDeps,
	}

	displayDependencies.value = version.value.dependencies.map((dependency) => {
		const version = dependencies.versions.find((obj) => obj.id === dependency.version_id)
		if (version) {
			const project = dependencies.projects.find(
				(obj) => obj.id === version.project_id || obj.id === dependency.project_id,
			)
			return {
				icon: project?.icon_url,
				title: project?.title || project?.name,
				subtitle: formatMessage(messages.versionDependency, { versionNumber: version.version_number, dependencyType: dependency.dependency_type }),
				link: buildProjectHref(`/project/${project.slug}/version/${version.id}`),
			}
		} else {
			const project = dependencies.projects.find((obj) => obj.id === dependency.project_id)

			if (project) {
				return {
					icon: project?.icon_url,
					title: project?.title || project?.name,
					subtitle: `${dependency.dependency_type}`,
					link: buildProjectHref(`/project/${project.slug}`),
				}
			} else {
				return {
					icon: null,
					title: dependency.file_name,
				subtitle: formatMessage(messages.addedViaOverrides),
					link: null,
				}
			}
		}
	})
}
onMounted(async () => {
	await refreshDisplayDependencies()
})
</script>

<style scoped lang="scss">
.version-container {
	display: flex;
	flex-direction: row;
	gap: 1rem;
}

.version-title {
	margin-bottom: 1rem;
	h2 {
		font-size: var(--font-size-2xl);
		font-weight: 700;
		color: var(--color-contrast);
		margin: 0;
	}
}

.dependency {
	display: flex;
	padding: 0.5rem 1rem 0.5rem 0.5rem;
	gap: 0.5rem;
	background: var(--color-raised-bg);
	color: var(--color-base);
	width: 100%;

	.title {
		font-weight: bolder;
	}

	:deep(svg) {
		margin-right: 0 !important;
	}
}

.file {
	display: flex;
	flex-direction: row;
	gap: 0.5rem;
	background: var(--color-button-bg);
	color: var(--color-base);
	padding: 0.5rem 1rem;

	.download {
		margin-left: auto;
		background-color: var(--color-raised-bg);
	}

	.label {
		display: flex;
		margin: auto 0 auto;
		gap: 0.5rem;

		.title {
			font-weight: bolder;
			word-break: break-all;
		}

		svg {
			min-width: 1.1rem;
			min-height: 1.1rem;
			width: 1.1rem;
			height: 1.1rem;
			margin: auto 0;
		}

		.primary-label {
			font-style: italic;
		}
	}
}

.primary {
	background: var(--color-brand-highlight);
	color: var(--color-contrast);
}

.button-group {
	display: flex;
	flex-wrap: wrap;
	flex-direction: row;
	gap: 0.5rem;
}

.card-title {
	font-size: var(--font-size-lg);
	color: var(--color-contrast);
	margin: 0 0 0.5rem;
}

.description-cards {
	width: 100%;
}

.metadata-card {
	width: 20rem;
	height: min-content;
}

.metadata {
	display: flex;
	flex-direction: column;
	flex-wrap: wrap;
	gap: 1rem;

	.metadata-item {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;

		.metadata-label {
			font-weight: bold;
		}
	}
}

.author {
	display: flex;
	flex-direction: row;
	gap: 0.5rem;
	align-items: center;
	text-decoration: none;
	color: var(--color-base);
	background: var(--color-raised-bg);
	padding: 0.5rem;
	width: 100%;
	box-shadow: none;
}

.markdown-body {
	:deep(hr),
	:deep(h1),
	:deep(h2),
	img {
		max-width: max(60rem, 90%) !important;
	}

	:deep(ul),
	:deep(ol) {
		margin-left: 2rem;
	}
}

.copycode {
	border: 0;
	color: var(--color-contrast);
}

.disabled {
	display: flex;
	flex-direction: row;
	vertical-align: center;
	align-items: center;
	cursor: not-allowed;
	border-radius: var(--radius-lg);

	.text {
		filter: brightness(0.5);
	}
}
</style>
