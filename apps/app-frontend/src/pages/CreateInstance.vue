<script setup lang="ts">
import {
	ArrowLeftIcon,
	BoxesIcon,
	BoxImportIcon,
	CheckIcon,
	ChevronDownIcon,
	ChevronRightIcon,
	FolderSearchIcon,
	ImageIcon,
	PackageIcon,
	SearchIcon,
	UploadIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	Card,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import anvilIcon from '@/assets/minecraft-icons/Anvil.png'
import cleanroomIcon from '@/assets/minecraft-icons/Cleanroom.png'
import cobblestoneIcon from '@/assets/minecraft-icons/CobbleStone.png'
import commandBlockIcon from '@/assets/minecraft-icons/CommandBlock.png'
import eggIcon from '@/assets/minecraft-icons/Egg.png'
import fabricIcon from '@/assets/minecraft-icons/Fabric.png'
import goldBlockIcon from '@/assets/minecraft-icons/GoldBlock.png'
import grassIcon from '@/assets/minecraft-icons/Grass.png'
import labymodIcon from '@/assets/minecraft-icons/LabyMod.png'
import neoforgeIcon from '@/assets/minecraft-icons/NeoForge.png'
import optifabricIcon from '@/assets/minecraft-icons/OptiFabric.png'
import quiltIcon from '@/assets/minecraft-icons/Quilt.png'
import redstoneLampIcon from '@/assets/minecraft-icons/RedstoneLampOn.png'
import {
	get_default_launcher_path,
	get_importable_instances,
	import_instance,
} from '@/helpers/import'
import { install_create_instance } from '@/helpers/install'
import { list } from '@/helpers/instance'
import { get_game_versions, get_loader_versions } from '@/helpers/metadata'
import type { InstanceLoader } from '@/helpers/types'

const versionIconMap: Record<string, string> = {
	latest: goldBlockIcon,
	release: grassIcon,
	snapshot: redstoneLampIcon,
	'old-beta': cobblestoneIcon,
	'old-alpha': cobblestoneIcon,
	'april-fools': goldBlockIcon,
}

const router = useRouter()
const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

// Messages
const messages = defineMessages({
	title: {
		id: 'app.create-instance.title',
		defaultMessage: 'Create new instance',
	},
	stepSetupType: {
		id: 'app.create-instance.step.setup-type',
		defaultMessage: 'Setup type',
	},
	setupTypeTitle: {
		id: 'app.create-instance.setup-type.title',
		defaultMessage: 'Choose instance type',
	},
	setupTypeDescription: {
		id: 'app.create-instance.setup-type.description',
		defaultMessage: 'An instance is a Minecraft setup with a specific loader, version, and mods.',
	},
	customSetupTitle: {
		id: 'app.create-instance.setup-type.custom.title',
		defaultMessage: 'Custom setup',
	},
	customSetupDescription: {
		id: 'app.create-instance.setup-type.custom.description',
		defaultMessage: 'Start from scratch by picking a loader and game version.',
	},
	modpackSetupTitle: {
		id: 'app.create-instance.setup-type.modpack.title',
		defaultMessage: 'Install modpack',
	},
	modpackSetupDescription: {
		id: 'app.create-instance.setup-type.modpack.description',
		defaultMessage: 'Browse modpacks on Modrinth or import one from a file.',
	},
	importSetupTitle: {
		id: 'app.create-instance.setup-type.import.title',
		defaultMessage: 'Import instance',
	},
	importSetupDescription: {
		id: 'app.create-instance.setup-type.import.description',
		defaultMessage: 'Import an instance from Prism, CurseForge, or similar.',
	},
	stepVersion: {
		id: 'app.create-instance.step.version',
		defaultMessage: 'Game version',
	},
	stepLoader: {
		id: 'app.create-instance.step.loader',
		defaultMessage: 'Loader',
	},
	stepConfig: {
		id: 'app.create-instance.step.config',
		defaultMessage: 'Final config',
	},
	searchPlaceholder: {
		id: 'app.create-instance.search-placeholder',
		defaultMessage: 'Search versions...',
	},
	showAllVersions: {
		id: 'app.create-instance.show-all',
		defaultMessage: 'Show snapshots and old versions',
	},
	latestVersions: {
		id: 'app.create-instance.group.latest',
		defaultMessage: 'Latest versions',
	},
	releases: {
		id: 'app.create-instance.group.releases',
		defaultMessage: 'Releases',
	},
	snapshots: {
		id: 'app.create-instance.group.snapshots',
		defaultMessage: 'Snapshots',
	},
	oldBeta: {
		id: 'app.create-instance.group.old-beta',
		defaultMessage: 'Old Beta',
	},
	oldAlpha: {
		id: 'app.create-instance.group.old-alpha',
		defaultMessage: 'Old Alpha',
	},
	aprilFools: {
		id: 'app.create-instance.group.april-fools',
		defaultMessage: 'April Fools',
	},
	noVersionsFound: {
		id: 'app.create-instance.no-versions',
		defaultMessage: 'No versions found',
	},
	selectVersionHint: {
		id: 'app.create-instance.select-version-hint',
		defaultMessage: 'Select a Minecraft version to continue',
	},
	selectedGameVersion: {
		id: 'app.create-instance.selected-game-version',
		defaultMessage: 'Selected Minecraft version',
	},
	changeVersion: {
		id: 'app.create-instance.change-version',
		defaultMessage: 'Change',
	},
	selectLoaderHint: {
		id: 'app.create-instance.select-loader-hint',
		defaultMessage: 'Expand a loader card to choose a version, or select Vanilla',
	},
	vanilla: {
		id: 'app.create-instance.loader.vanilla',
		defaultMessage: 'Vanilla',
	},
	vanillaDesc: {
		id: 'app.create-instance.loader.vanilla.desc',
		defaultMessage: 'No mod loader',
	},
	versionsAvailable: {
		id: 'app.create-instance.loader.versions-available',
		defaultMessage: '{count} versions available',
	},
	noVersions: {
		id: 'app.create-instance.loader.no-versions',
		defaultMessage: 'No compatible versions',
	},
	incompatible: {
		id: 'app.create-instance.loader.incompatible',
		defaultMessage: 'Incompatible with {version}',
	},
	notSupported: {
		id: 'app.create-instance.loader.not-supported',
		defaultMessage: 'Not supported yet',
	},
	selectedVersion: {
		id: 'app.create-instance.loader.selected',
		defaultMessage: 'Selected: {version}',
	},
	loaderVersionLabel: {
		id: 'app.create-instance.loader-version.label',
		defaultMessage: 'Loader version',
	},
	nameLabel: {
		id: 'app.create-instance.name.label',
		defaultMessage: 'Instance name',
	},
	namePlaceholder: {
		id: 'app.create-instance.name.placeholder',
		defaultMessage: 'Enter instance name',
	},
	imageFilter: {
		id: 'app.create-instance.image-filter',
		defaultMessage: 'Images',
	},
	selectIcon: {
		id: 'app.create-instance.icon.select',
		defaultMessage: 'Select icon',
	},
	removeIcon: {
		id: 'app.create-instance.icon.remove',
		defaultMessage: 'Remove icon',
	},
	createButton: {
		id: 'app.create-instance.create',
		defaultMessage: 'Create instance',
	},
	creatingButton: {
		id: 'app.create-instance.creating',
		defaultMessage: 'Creating...',
	},
	instanceCreated: {
		id: 'app.create-instance.instance-created',
		defaultMessage: 'Instance created',
	},
	versions: { id: 'app.create-instance.versions', defaultMessage: 'versions' },
	stable: { id: 'app.create-instance.stable', defaultMessage: 'Stable' },
	gameVersion: { id: 'app.create-instance.game-version', defaultMessage: 'Game version' },
	loader: { id: 'app.create-instance.loader', defaultMessage: 'Loader' },
	loaderVersion: { id: 'app.create-instance.loader-version', defaultMessage: 'Loader version' },
	// Import step messages
	stepImport: { id: 'app.create-instance.step.import', defaultMessage: 'Import instances' },
	detectingLaunchers: {
		id: 'app.create-instance.import.detecting',
		defaultMessage: 'Detecting launcher instances...',
	},
	noLaunchersFound: {
		id: 'app.create-instance.import.no-launchers',
		defaultMessage: 'No launchers detected. Add a custom path below.',
	},
	launcherInstancesTitle: {
		id: 'app.create-instance.import.launcher-instances',
		defaultMessage: 'Detected launchers',
	},
	searchInstances: {
		id: 'app.create-instance.import.search',
		defaultMessage: 'Search instances...',
	},
	addLauncherPath: {
		id: 'app.create-instance.import.add-path',
		defaultMessage: 'Add launcher path',
	},
	launcherPathPlaceholder: {
		id: 'app.create-instance.import.path-placeholder',
		defaultMessage: 'Path to launcher...',
	},
	addPath: { id: 'app.create-instance.import.add', defaultMessage: 'Add' },
	browsePath: { id: 'app.create-instance.import.browse', defaultMessage: 'Browse' },
	clearSelection: {
		id: 'app.create-instance.import.clear-selection',
		defaultMessage: 'Clear',
	},
	importButton: {
		id: 'app.create-instance.import.button',
		defaultMessage: 'Import {count} instance',
	},
	importButtonPlural: {
		id: 'app.create-instance.import.button.plural',
		defaultMessage: 'Import {count} instances',
	},
	importingButton: {
		id: 'app.create-instance.import.importing',
		defaultMessage: 'Importing...',
	},
	importSuccess: {
		id: 'app.create-instance.import.success',
		defaultMessage: 'Instance(s) imported successfully',
	},
	customLauncherName: {
		id: 'app.create-instance.import.custom-launcher',
		defaultMessage: 'Custom ({pathName})',
	},
	noInstancesAtPath: {
		id: 'app.create-instance.import.no-instances-at-path',
		defaultMessage: 'No importable instances found at the specified path.',
	},
	instancesAvailable: {
		id: 'app.create-instance.import.instances-available',
		defaultMessage: '{count} instances',
	},
})

// Steps
const STEP_SETUP_TYPE = 0
const STEP_VERSION = 1
const STEP_LOADER = 2
const STEP_CONFIG = 3
const STEP_IMPORT = 4
const step = ref(STEP_SETUP_TYPE)

type SetupType = 'custom' | 'modpack' | 'import'
const setupType = ref<SetupType | null>(null)

function selectSetupType(type: SetupType) {
	setupType.value = type
	if (type === 'modpack') {
		router.push('/browse/modpack')
		return
	}
	if (type === 'import') {
		step.value = STEP_IMPORT
		detectLaunchers()
		return
	}
	step.value = STEP_VERSION
}

function backToSetupType() {
	step.value = STEP_SETUP_TYPE
	setupType.value = null
}

// ---- Import Instance Logic ----
interface ImportableLauncher {
	name: string
	path: string
	instances: string[]
}

const LAUNCHER_NAMES = ['PrismLauncher', 'MultiMC', 'ATLauncher', 'Curseforge', 'GDLauncher']

const importLaunchers = ref<ImportableLauncher[]>([])
const importLoading = ref(false)
const importSearchQuery = ref('')
const importSelectedInstances = ref<Record<string, Set<string>>>({})
const importExpandedLaunchers = ref<Set<string>>(new Set())
const showAddLauncherPath = ref(false)
const newLauncherPath = ref('')
const importing = ref(false)

async function detectLaunchers() {
	if (importLaunchers.value.length > 0 && !importLoading.value) return
	importLoading.value = true
	const launchers: ImportableLauncher[] = []
	for (const name of LAUNCHER_NAMES) {
		try {
			const path = await get_default_launcher_path(name)
			if (!path) continue
			const instances = await get_importable_instances(name, path)
			if (instances?.length > 0) {
				launchers.push({ name, path, instances })
				importExpandedLaunchers.value.add(name)
			}
		} catch {
			// Skip launchers that fail detection
		}
	}
	importLaunchers.value = launchers
	importLoading.value = false
}

function filteredImportInstances(launcher: ImportableLauncher): string[] {
	const q = importSearchQuery.value.toLowerCase().trim()
	if (!q) return launcher.instances
	return launcher.instances.filter((name) => name.toLowerCase().includes(q))
}

const visibleImportLaunchers = computed(() => {
	const q = importSearchQuery.value.toLowerCase().trim()
	if (!q) return importLaunchers.value
	return importLaunchers.value.filter((l) => filteredImportInstances(l).length > 0)
})

function isImportInstanceSelected(launcherName: string, instance: string): boolean {
	return importSelectedInstances.value[launcherName]?.has(instance) ?? false
}

function toggleImportInstance(launcherName: string, instance: string, selected: boolean) {
	if (!importSelectedInstances.value[launcherName]) {
		importSelectedInstances.value[launcherName] = new Set()
	}
	if (selected) {
		importSelectedInstances.value[launcherName].add(instance)
	} else {
		importSelectedInstances.value[launcherName].delete(instance)
	}
	importSelectedInstances.value = { ...importSelectedInstances.value }
}

function toggleLauncherAll(launcher: ImportableLauncher, selected: boolean) {
	if (!importSelectedInstances.value[launcher.name]) {
		importSelectedInstances.value[launcher.name] = new Set()
	}
	for (const inst of filteredImportInstances(launcher)) {
		if (selected) {
			importSelectedInstances.value[launcher.name].add(inst)
		} else {
			importSelectedInstances.value[launcher.name].delete(inst)
		}
	}
	importSelectedInstances.value = { ...importSelectedInstances.value }
}

function getLauncherCheckState(launcher: ImportableLauncher): boolean {
	const set = importSelectedInstances.value[launcher.name]
	if (!set || set.size === 0) return false
	const visible = filteredImportInstances(launcher)
	return visible.length > 0 && visible.every((i) => set.has(i))
}

function getLauncherIndeterminate(launcher: ImportableLauncher): boolean {
	const set = importSelectedInstances.value[launcher.name]
	if (!set || set.size === 0) return false
	const visible = filteredImportInstances(launcher)
	const selectedVisible = visible.filter((i) => set.has(i))
	return selectedVisible.length > 0 && selectedVisible.length < visible.length
}

function toggleLauncherExpanded(name: string) {
	if (importExpandedLaunchers.value.has(name)) {
		importExpandedLaunchers.value.delete(name)
	} else {
		importExpandedLaunchers.value.add(name)
	}
	importExpandedLaunchers.value = new Set(importExpandedLaunchers.value)
}

const totalImportSelectedCount = computed(() => {
	let count = 0
	for (const set of Object.values(importSelectedInstances.value)) {
		count += set.size
	}
	return count
})

function clearImportSelection() {
	importSelectedInstances.value = {}
}

async function browseForLauncherPath() {
	try {
		const { open } = await import('@tauri-apps/plugin-dialog')
		const result = await open({ multiple: false, directory: true })
		if (result && typeof result === 'string') {
			newLauncherPath.value = result
		}
	} catch (err) {
		handleError(err as Error)
	}
}

async function addLauncherPathAction() {
	const path = newLauncherPath.value.trim()
	if (!path) return
	try {
		const instances = await get_importable_instances('Custom', path)
		if (!instances || instances.length === 0) {
			addNotification({ type: 'error', title: formatMessage(messages.noInstancesAtPath) })
			return
		}
		const launcher: ImportableLauncher = {
			name: formatMessage(messages.customLauncherName, {
				pathName: path.split(/[\\/]/).pop() || path,
			}),
			path,
			instances,
		}
		importLaunchers.value = [...importLaunchers.value, launcher]
		importExpandedLaunchers.value.add(launcher.name)
		importExpandedLaunchers.value = new Set(importExpandedLaunchers.value)
	} catch {
		addNotification({ type: 'error', title: formatMessage(messages.noInstancesAtPath) })
		return
	}
	newLauncherPath.value = ''
	showAddLauncherPath.value = false
}

async function doImport() {
	if (totalImportSelectedCount.value === 0) return
	importing.value = true
	let successCount = 0
	for (const [launcherName, instanceSet] of Object.entries(importSelectedInstances.value)) {
		const launcher = importLaunchers.value.find((l) => l.name === launcherName)
		if (!launcher) continue
		for (const instanceName of instanceSet) {
			try {
				await import_instance(
					launcherName.startsWith('Custom') ? 'Custom' : launcherName,
					launcher.path,
					instanceName,
				)
				successCount++
			} catch (err) {
				handleError(err as Error)
			}
		}
	}
	importing.value = false
	if (successCount > 0) {
		addNotification({ type: 'success', title: formatMessage(messages.importSuccess) })
		router.push('/library')
	}
}
// ---- End Import Logic ----

// Data
interface GameVersion {
	id: string
	type: 'release' | 'snapshot' | 'old_beta' | 'old_alpha' | 'april_fools'
	releaseTime: string
}

interface ApiGameVersion {
	id: string
	type: string
	releaseTime: string
}

interface GameVersionManifest {
	versions: ApiGameVersion[]
}

const manifest = ref<GameVersionManifest | null>(null)

async function loadManifest() {
	const result = await get_game_versions().catch((err) => {
		handleError(err)
		return null
	})
	manifest.value = result as GameVersionManifest | null
}

onMounted(() => {
	loadManifest()
})

const allVersions = computed<GameVersion[]>(() => {
	if (!manifest.value) return []
	return manifest.value.versions.map((v) => ({
		id: v.id,
		type: versionTypeFromApi(v.type),
		releaseTime: v.releaseTime,
	}))
})

const latestRelease = computed(() => manifest.value?.latest?.release ?? null)
const latestSnapshot = computed(() => manifest.value?.latest?.snapshot ?? null)

function versionTypeFromApi(type: string): GameVersion['type'] {
	switch (type) {
		case 'release':
			return 'release'
		case 'snapshot':
			return 'snapshot'
		case 'old_beta':
			return 'old_beta'
		case 'old_alpha':
			return 'old_alpha'
		case 'april_fools':
			return 'april_fools'
		default:
			return 'release'
	}
}

const showAllVersions = ref(false)
const searchQuery = ref('')

const filteredVersions = computed(() => {
	let list = allVersions.value
	if (!showAllVersions.value) {
		list = list.filter((v) => v.type === 'release')
	}
	const q = searchQuery.value.trim().toLowerCase()
	if (q) {
		list = list.filter((v) => v.id.toLowerCase().includes(q))
	}
	return list.sort((a, b) => Date.parse(b.releaseTime) - Date.parse(a.releaseTime))
})

const latestVersions = computed(() => {
	const result: GameVersion[] = []
	if (latestRelease.value) {
		const v = allVersions.value.find((x) => x.id === latestRelease.value)
		if (v) result.push(v)
	}
	if (latestSnapshot.value) {
		const v = allVersions.value.find((x) => x.id === latestSnapshot.value)
		if (v && !result.find((x) => x.id === v.id)) result.push(v)
	}
	return result
})

const releaseVersions = computed(() =>
	filteredVersions.value.filter(
		(v) => v.type === 'release' && !latestVersions.value.find((l) => l.id === v.id),
	),
)
const snapshotVersions = computed(() => filteredVersions.value.filter((v) => v.type === 'snapshot'))
const oldBetaVersions = computed(() => filteredVersions.value.filter((v) => v.type === 'old_beta'))
const oldAlphaVersions = computed(() =>
	filteredVersions.value.filter((v) => v.type === 'old_alpha'),
)
const aprilFoolsVersions = computed(() =>
	filteredVersions.value.filter((v) => v.type === 'april_fools'),
)

const versionGroups = computed(() => [
	{
		id: 'latest',
		label: formatMessage(messages.latestVersions),
		versions: latestVersions.value,
		expanded: true,
		pinned: true,
	},
	{ id: 'release', label: formatMessage(messages.releases), versions: releaseVersions.value },
	{ id: 'snapshot', label: formatMessage(messages.snapshots), versions: snapshotVersions.value },
	{ id: 'old-beta', label: formatMessage(messages.oldBeta), versions: oldBetaVersions.value },
	{ id: 'old-alpha', label: formatMessage(messages.oldAlpha), versions: oldAlphaVersions.value },
	{
		id: 'april-fools',
		label: formatMessage(messages.aprilFools),
		versions: aprilFoolsVersions.value,
	},
])

const expandedGroups = ref<Record<string, boolean>>({
	latest: true,
	release: false,
	snapshot: false,
	'old-beta': false,
	'old-alpha': false,
	'april-fools': false,
})

function toggleGroup(id: string) {
	expandedGroups.value[id] = !expandedGroups.value[id]
}

const selectedGameVersion = ref<string | null>(null)

const selectedGameVersionIcon = computed(() => {
	const version = allVersions.value.find((v) => v.id === selectedGameVersion.value)
	if (!version) return grassIcon
	return versionIconMap[version.type] ?? grassIcon
})

const selectedGameVersionClass = computed(() => {
	const version = allVersions.value.find((v) => v.id === selectedGameVersion.value)
	if (!version) return 'version-icon-release'
	return versionGroupClass(version.type)
})

function selectGameVersion(version: string) {
	selectedGameVersion.value = version
	step.value = STEP_LOADER
	// Reset loader selection when version changes
	selectedLoader.value = 'vanilla'
	selectedLoaderVersion.value = null
	loaderVersions.value = []
}

function goBackToVersions() {
	step.value = STEP_VERSION
}

// Loader selection
const selectedLoader = ref<string>('vanilla')
const selectedLoaderVersion = ref<string | null>(null)
const loaderVersions = ref<{ id: string; stable: boolean }[]>([])
const loaderVersionsLoading = ref(false)
const loaderVersionsCache = ref<
	Record<string, { gameVersions: { id: string; loaders: { id: string; stable: boolean }[] }[] }>
>({})
const expandedLoaderCards = ref<Record<string, boolean>>({})

const SUPPORTED_LOADERS = ['vanilla', 'forge', 'neoforge', 'fabric', 'quilt']

const loaderInfoMap: Record<string, { label: string; icon: string }> = {
	vanilla: { label: 'Vanilla', icon: commandBlockIcon },
	forge: { label: 'Forge', icon: anvilIcon },
	neoforge: { label: 'NeoForge', icon: neoforgeIcon },
	fabric: { label: 'Fabric', icon: fabricIcon },
	quilt: { label: 'Quilt', icon: quiltIcon },
	optifine: { label: 'OptiFine', icon: optifabricIcon },
	liteloader: { label: 'LiteLoader', icon: eggIcon },
	cleanroom: { label: 'Cleanroom', icon: cleanroomIcon },
	labymod: { label: 'LabyMod', icon: labymodIcon },
}

const loaders = computed(() => [
	{ id: 'vanilla', ...loaderInfoMap.vanilla },
	{ id: 'forge', ...loaderInfoMap.forge },
	{ id: 'neoforge', ...loaderInfoMap.neoforge },
	{ id: 'fabric', ...loaderInfoMap.fabric },
	{ id: 'quilt', ...loaderInfoMap.quilt },
	{ id: 'optifine', ...loaderInfoMap.optifine },
	{ id: 'liteloader', ...loaderInfoMap.liteloader },
	{ id: 'cleanroom', ...loaderInfoMap.cleanroom },
	{ id: 'labymod', ...loaderInfoMap.labymod },
])

function toApiLoaderName(loader: string): string {
	return loader === 'neoforge' ? 'neo' : loader
}

async function fetchLoaderManifest(loader: string) {
	const apiLoader = toApiLoaderName(loader)
	if (loaderVersionsCache.value[apiLoader]) return
	try {
		const data = await get_loader_versions(apiLoader)
		loaderVersionsCache.value[apiLoader] = data
	} catch {
		loaderVersionsCache.value[apiLoader] = { gameVersions: [] }
	}
}

function getLoaderVersionCount(loader: string): number {
	const gameVersion = selectedGameVersion.value
	if (!gameVersion) return 0
	const apiLoader = toApiLoaderName(loader)
	const manifest = loaderVersionsCache.value[apiLoader]
	if (!manifest) return 0
	const placeholder = manifest.gameVersions.find((x) => x.id === '${modrinth.gameVersion}')
	if (placeholder) {
		const hasVersion = manifest.gameVersions.some((x) => x.id === gameVersion)
		return hasVersion ? placeholder.loaders.length : 0
	}
	const entry = manifest.gameVersions.find((x) => x.id === gameVersion)
	return entry?.loaders?.length ?? 0
}

function isLoaderCompatible(loader: string): boolean {
	if (!SUPPORTED_LOADERS.includes(loader)) return false
	if (loader === 'vanilla') return true
	return getLoaderVersionCount(loader) > 0
}

function isLoaderSupported(loader: string): boolean {
	return SUPPORTED_LOADERS.includes(loader)
}

function getLoaderVersions(loader: string): { id: string; stable: boolean }[] {
	const gameVersion = selectedGameVersion.value
	if (!gameVersion || loader === 'vanilla') return []
	const apiLoader = toApiLoaderName(loader)
	const manifest = loaderVersionsCache.value[apiLoader]
	if (!manifest) return []
	const placeholder = manifest.gameVersions.find((x) => x.id === '${modrinth.gameVersion}')
	if (placeholder) {
		if (!manifest.gameVersions.some((x) => x.id === gameVersion)) return []
		return placeholder.loaders
	}
	const entry = manifest.gameVersions.find((x) => x.id === gameVersion)
	return entry?.loaders ?? []
}

async function expandLoaderCard(loader: string) {
	if (!isLoaderSupported(loader)) return
	expandedLoaderCards.value[loader] = !expandedLoaderCards.value[loader]
	if (loader !== 'vanilla' && expandedLoaderCards.value[loader]) {
		loaderVersionsLoading.value = true
		await fetchLoaderManifest(loader)
		loaderVersionsLoading.value = false
		if (selectedLoader.value === loader && loaderVersions.value.length === 0) {
			loaderVersions.value = getLoaderVersions(loader)
			if (!selectedLoaderVersion.value && loaderVersions.value.length > 0) {
				selectedLoaderVersion.value = loaderVersions.value[0].id
			}
		}
	}
}

function selectLoader(loader: string) {
	if (!isLoaderSupported(loader)) return
	selectedLoader.value = loader
	expandedLoaderCards.value = { [loader]: true }
	if (loader === 'vanilla') {
		selectedLoaderVersion.value = null
		loaderVersions.value = []
	} else {
		loaderVersions.value = getLoaderVersions(loader)
		if (loaderVersions.value.length > 0) {
			selectedLoaderVersion.value = loaderVersions.value[0].id
		} else {
			selectedLoaderVersion.value = null
		}
	}
}

function selectLoaderVersion(versionId: string) {
	selectedLoaderVersion.value = versionId
}

function selectLoaderAndVersion(loader: string, versionId: string) {
	selectLoader(loader)
	selectLoaderVersion(versionId)
}

watch(
	() => selectedGameVersion.value,
	async () => {
		loaderVersionsCache.value = {}
		loaderVersions.value = []
		selectedLoaderVersion.value = null
		expandedLoaderCards.value = {}
		selectLoader('vanilla')

		// Pre-fetch loader manifests in the background so compatibility/version
		// counts are shown immediately without requiring the user to expand each card.
		const loadersToFetch = SUPPORTED_LOADERS.filter((l) => l !== 'vanilla')
		await Promise.allSettled(loadersToFetch.map(fetchLoaderManifest))
	},
)

// Final config
const instanceName = ref('')
const instanceIconUrl = ref<string | null>(null)
const instanceIconPath = ref<string | null>(null)
const instanceIconFile = ref<File | null>(null)

const existingInstanceNames = ref<string[]>([])
list()
	.then((instances) => {
		existingInstanceNames.value = instances.map((i) => i.name)
	})
	.catch(handleError)

const autoInstanceName = computed(() => {
	const loader = selectedLoader.value
	const version = selectedGameVersion.value
	if (!version) return ''
	const loaderName = loader === 'vanilla' ? 'Vanilla' : (loaderInfoMap[loader]?.label ?? loader)
	const baseName = `${loaderName} ${version}`
	const names = new Set(existingInstanceNames.value)
	if (!names.has(baseName)) return baseName
	let counter = 1
	while (names.has(`${baseName} (${counter})`)) {
		counter++
	}
	return `${baseName} (${counter})`
})

const finalName = computed(() => instanceName.value.trim() || autoInstanceName.value)

async function selectIcon() {
	try {
		const { open } = await import('@tauri-apps/plugin-dialog')
		const result = await open({
			multiple: false,
			directory: false,
			filters: [
				{
					name: formatMessage(messages.imageFilter),
					extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'],
				},
			],
		})
		if (result && typeof result === 'string') {
			instanceIconPath.value = result
			instanceIconUrl.value = `file://${result}`
		}
	} catch (err) {
		handleError(err as Error)
	}
}

function removeIcon() {
	instanceIconUrl.value = null
	instanceIconPath.value = null
	instanceIconFile.value = null
}

const creating = ref(false)

async function createInstance() {
	if (!selectedGameVersion.value) return
	creating.value = true
	try {
		const loader = selectedLoader.value
		const loaderVersion = loader === 'vanilla' ? null : (selectedLoaderVersion.value ?? 'latest')
		await install_create_instance({
			name: finalName.value,
			gameVersion: selectedGameVersion.value,
			loader: loader as InstanceLoader,
			loaderVersion,
			iconPath: instanceIconPath.value,
		})
		addNotification({ type: 'success', title: formatMessage(messages.instanceCreated) })
		router.push('/library')
	} catch (err) {
		handleError(err as Error)
	} finally {
		creating.value = false
	}
}

function formatDate(dateStr: string): string {
	try {
		const d = new Date(dateStr)
		return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' })
	} catch {
		return ''
	}
}

function versionGroupClass(id: string): string {
	switch (id) {
		case 'latest':
			return 'version-icon-recent'
		case 'release':
			return 'version-icon-release'
		case 'snapshot':
			return 'version-icon-snapshot'
		case 'old-beta':
			return 'version-icon-old-beta'
		case 'old-alpha':
			return 'version-icon-old-alpha'
		case 'april-fools':
			return 'version-icon-april-fools'
		default:
			return 'version-icon-release'
	}
}
</script>

<template>
	<div class="flex flex-col gap-4 p-6 max-w-5xl mx-auto">
		<!-- Header -->
		<div class="flex items-center gap-4 mb-2">
			<ButtonStyled circular type="transparent" @click="router.push('/library')">
				<button>
					<ArrowLeftIcon class="size-5" />
				</button>
			</ButtonStyled>
			<h1 class="text-2xl font-bold text-contrast">{{ formatMessage(messages.title) }}</h1>
		</div>

		<!-- Stepper -->
		<div class="flex items-center gap-2 mb-4">
			<div
				class="step-item"
				:class="{
					'step-item-active': step >= STEP_SETUP_TYPE,
					'step-item-current': step === STEP_SETUP_TYPE,
				}"
			>
				<span class="step-number">1</span>
				<span>{{ formatMessage(messages.stepSetupType) }}</span>
			</div>
			<template v-if="setupType === 'import'">
				<div class="step-divider" :class="{ 'step-divider-active': step >= STEP_IMPORT }" />
				<div
					class="step-item"
					:class="{
						'step-item-active': step >= STEP_IMPORT,
						'step-item-current': step === STEP_IMPORT,
					}"
				>
					<span class="step-number">2</span>
					<span>{{ formatMessage(messages.stepImport) }}</span>
				</div>
			</template>
			<template v-else>
				<div class="step-divider" :class="{ 'step-divider-active': step >= STEP_VERSION }" />
				<div
					class="step-item"
					:class="{
						'step-item-active': step >= STEP_VERSION,
						'step-item-current': step === STEP_VERSION,
					}"
				>
					<span class="step-number">2</span>
					<span>{{ formatMessage(messages.stepVersion) }}</span>
				</div>
				<div class="step-divider" :class="{ 'step-divider-active': step >= STEP_LOADER }" />
				<div
					class="step-item"
					:class="{
						'step-item-active': step >= STEP_LOADER,
						'step-item-current': step === STEP_LOADER,
					}"
				>
					<span class="step-number">3</span>
					<span>{{ formatMessage(messages.stepLoader) }}</span>
				</div>
				<div class="step-divider" :class="{ 'step-divider-active': step >= STEP_CONFIG }" />
				<div
					class="step-item"
					:class="{
						'step-item-active': step >= STEP_CONFIG,
						'step-item-current': step === STEP_CONFIG,
					}"
				>
					<span class="step-number">4</span>
					<span>{{ formatMessage(messages.stepConfig) }}</span>
				</div>
			</template>
		</div>

		<!-- Step 0: Setup type selection -->
		<template v-if="step === STEP_SETUP_TYPE">
			<Card class="p-4">
				<div class="flex flex-col gap-4">
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.setupTypeTitle) }}
					</h2>
					<p class="m-0 text-secondary">
						{{ formatMessage(messages.setupTypeDescription) }}
					</p>
					<div class="setup-type-grid">
						<button class="setup-type-button" @click="selectSetupType('custom')">
							<BoxesIcon class="setup-type-icon" />
							<h3 class="setup-type-title">{{ formatMessage(messages.customSetupTitle) }}</h3>
							<p class="setup-type-desc">
								{{ formatMessage(messages.customSetupDescription) }}
							</p>
						</button>
						<button class="setup-type-button" @click="selectSetupType('modpack')">
							<PackageIcon class="setup-type-icon" />
							<h3 class="setup-type-title">{{ formatMessage(messages.modpackSetupTitle) }}</h3>
							<p class="setup-type-desc">
								{{ formatMessage(messages.modpackSetupDescription) }}
							</p>
						</button>
						<button class="setup-type-button" @click="selectSetupType('import')">
							<BoxImportIcon class="setup-type-icon" />
							<h3 class="setup-type-title">{{ formatMessage(messages.importSetupTitle) }}</h3>
							<p class="setup-type-desc">
								{{ formatMessage(messages.importSetupDescription) }}
							</p>
						</button>
					</div>
				</div>
			</Card>
		</template>

		<!-- Step 4: Import instances -->
		<template v-if="step === STEP_IMPORT">
			<!-- Loading state -->
			<Card v-if="importLoading" class="p-8 text-center">
				<div class="text-secondary">{{ formatMessage(messages.detectingLaunchers) }}</div>
			</Card>

			<template v-else>
				<!-- Search & clear -->
				<Card v-if="importLaunchers.length > 0" class="p-4">
					<div class="flex items-center gap-3">
						<StyledInput
							v-model="importSearchQuery"
							:placeholder="formatMessage(messages.searchInstances)"
							type="text"
							class="flex-1"
						>
							<template #icon>
								<SearchIcon />
							</template>
						</StyledInput>
						<ButtonStyled
							v-if="totalImportSelectedCount > 0"
							type="transparent"
							size="small"
							@click="clearImportSelection"
						>
							<button>{{ formatMessage(messages.clearSelection) }}</button>
						</ButtonStyled>
					</div>
				</Card>

				<!-- No launchers found -->
				<Card v-if="importLaunchers.length === 0" class="p-6 text-center">
					<p class="text-secondary m-0">{{ formatMessage(messages.noLaunchersFound) }}</p>
				</Card>

				<!-- Launcher sections -->
				<div v-if="visibleImportLaunchers.length > 0" class="flex flex-col gap-3">
					<Card
						v-for="launcher in visibleImportLaunchers"
						:key="launcher.name"
						class="import-launcher-card"
					>
						<!-- Launcher header -->
						<div class="import-launcher-header" @click="toggleLauncherExpanded(launcher.name)">
							<div class="flex items-center gap-3 flex-1 min-w-0">
								<ChevronRightIcon
									class="size-5 shrink-0 text-secondary transition-transform"
									:class="{ 'rotate-90': importExpandedLaunchers.has(launcher.name) }"
								/>
								<input
									type="checkbox"
									class="import-checkbox"
									:checked="getLauncherCheckState(launcher)"
									:indeterminate="getLauncherIndeterminate(launcher)"
									@click.stop
									@change="toggleLauncherAll(launcher, ($event.target as HTMLInputElement).checked)"
								/>
								<div class="flex flex-col min-w-0">
									<span class="font-semibold text-contrast truncate">{{ launcher.name }}</span>
									<span class="text-xs text-secondary">
										{{
											formatMessage(messages.instancesAvailable, {
												count: launcher.instances.length,
											})
										}}
									</span>
								</div>
							</div>
						</div>

						<!-- Instance list -->
						<div v-if="importExpandedLaunchers.has(launcher.name)" class="import-launcher-content">
							<div
								v-for="(inst, i) in filteredImportInstances(launcher)"
								:key="inst"
								class="import-instance-row"
								:class="{ 'import-instance-alt': i % 2 !== 0 }"
							>
								<input
									type="checkbox"
									class="import-checkbox"
									:checked="isImportInstanceSelected(launcher.name, inst)"
									@change="
										toggleImportInstance(
											launcher.name,
											inst,
											($event.target as HTMLInputElement).checked,
										)
									"
								/>
								<span class="text-sm text-contrast">{{ inst }}</span>
							</div>
						</div>
					</Card>
				</div>

				<!-- Add launcher path -->
				<Card class="p-4">
					<template v-if="!showAddLauncherPath">
						<ButtonStyled type="outlined" class="w-full">
							<button @click="showAddLauncherPath = true">
								<FolderSearchIcon class="size-4" />
								{{ formatMessage(messages.addLauncherPath) }}
							</button>
						</ButtonStyled>
					</template>
					<div v-else class="flex items-center gap-2">
						<ButtonStyled circular type="outlined">
							<button @click="browseForLauncherPath">
								<FolderSearchIcon class="size-4" />
							</button>
						</ButtonStyled>
						<StyledInput
							v-model="newLauncherPath"
							:placeholder="formatMessage(messages.launcherPathPlaceholder)"
							type="text"
							class="flex-1"
						/>
						<ButtonStyled color="brand">
							<button :disabled="!newLauncherPath.trim()" @click="addLauncherPathAction">
								{{ formatMessage(messages.addPath) }}
							</button>
						</ButtonStyled>
					</div>
				</Card>

				<!-- Import action bar -->
				<div class="flex justify-between mt-4">
					<ButtonStyled type="outlined" @click="backToSetupType">
						<button>{{ formatMessage(commonMessages.backButton) }}</button>
					</ButtonStyled>
					<ButtonStyled
						color="brand"
						size="large"
						:disabled="totalImportSelectedCount === 0 || importing"
						@click="doImport"
					>
						<button>
							{{
								importing
									? formatMessage(messages.importingButton)
									: totalImportSelectedCount === 1
										? formatMessage(messages.importButton, { count: 1 })
										: formatMessage(messages.importButtonPlural, {
												count: totalImportSelectedCount,
											})
							}}
						</button>
					</ButtonStyled>
				</div>
			</template>
		</template>

		<!-- Step 1: Game version selection -->
		<template v-if="step === STEP_VERSION">
			<Card class="p-4">
				<div class="flex flex-col gap-4">
					<StyledInput
						v-model="searchQuery"
						:placeholder="formatMessage(messages.searchPlaceholder)"
						type="text"
					>
						<template #icon>
							<SearchIcon />
						</template>
					</StyledInput>
					<div class="flex items-center justify-between">
						<span class="text-sm text-secondary">{{
							formatMessage(messages.showAllVersions)
						}}</span>
						<Toggle v-model="showAllVersions" />
					</div>
				</div>
			</Card>

			<div class="flex flex-col gap-3">
				<template v-for="group in versionGroups" :key="group.id">
					<Card v-if="group.versions.length > 0" class="version-group-card">
						<div
							class="version-group-header"
							:class="{ 'cursor-pointer': !group.pinned }"
							@click="!group.pinned && toggleGroup(group.id)"
						>
							<div class="flex items-center gap-3">
								<div class="version-icon" :class="versionGroupClass(group.id)">
									<img :src="versionIconMap[group.id] ?? grassIcon" class="mc-icon" />
								</div>
								<div class="flex flex-col">
									<span class="font-semibold text-contrast">{{ group.label }}</span>
									<span class="text-xs text-secondary"
										>{{ group.versions.length }} {{ formatMessage(messages.versions) }}</span
									>
								</div>
							</div>
							<ChevronDownIcon
								v-if="!group.pinned"
								class="size-5 text-secondary transition-transform"
								:class="{ 'rotate-180': expandedGroups[group.id] }"
							/>
						</div>
						<div v-if="expandedGroups[group.id] || group.pinned" class="version-group-content">
							<div
								v-for="v in group.versions"
								:key="v.id"
								class="version-row"
								:class="{ 'version-row-selected': selectedGameVersion === v.id }"
								@click="selectGameVersion(v.id)"
							>
								<div class="flex items-center gap-3 flex-1 min-w-0">
									<div class="version-icon small" :class="versionGroupClass(group.id)">
										<img :src="versionIconMap[group.id] ?? grassIcon" class="mc-icon" />
									</div>
									<div class="flex flex-col min-w-0">
										<span class="font-semibold text-contrast truncate">{{ v.id }}</span>
										<span v-if="v.releaseTime" class="text-xs text-secondary">{{
											formatDate(v.releaseTime)
										}}</span>
									</div>
								</div>
								<CheckIcon v-if="selectedGameVersion === v.id" class="size-5 text-brand" />
							</div>
						</div>
					</Card>
				</template>
				<div v-if="filteredVersions.length === 0" class="text-center text-secondary py-8">
					{{ formatMessage(messages.noVersionsFound) }}
				</div>
			</div>
		</template>

		<!-- Step 2: Loader selection -->
		<template v-if="step === STEP_LOADER">
			<Card class="p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<div class="version-icon" :class="selectedGameVersionClass">
							<img :src="selectedGameVersionIcon" class="mc-icon" />
						</div>
						<div class="flex flex-col">
							<span class="text-sm text-secondary">{{
								formatMessage(messages.selectedGameVersion)
							}}</span>
							<span class="font-semibold text-contrast text-lg">{{ selectedGameVersion }}</span>
						</div>
					</div>
					<ButtonStyled type="outlined" @click="goBackToVersions">
						<button>{{ formatMessage(messages.changeVersion) }}</button>
					</ButtonStyled>
				</div>
			</Card>

			<div class="text-sm text-secondary">{{ formatMessage(messages.selectLoaderHint) }}</div>

			<div class="flex flex-col gap-3">
				<Card
					v-for="opt in loaders"
					:key="opt.id"
					class="loader-group-card"
					:class="{ 'loader-group-card-selected': selectedLoader === opt.id }"
				>
					<div
						class="loader-group-header"
						:class="{
							'cursor-pointer': isLoaderSupported(opt.id) && opt.id !== 'vanilla',
							'opacity-50': !isLoaderCompatible(opt.id),
						}"
						@click="opt.id === 'vanilla' ? selectLoader('vanilla') : expandLoaderCard(opt.id)"
					>
						<div class="flex items-center gap-3 flex-1">
							<div class="loader-icon" :class="`loader-icon-${opt.id}`">
								<img :src="opt.icon" class="mc-icon" />
							</div>
							<div class="flex flex-col">
								<span class="font-semibold text-contrast">{{ opt.label }}</span>
								<span class="text-xs text-secondary">
									<span v-if="opt.id === 'vanilla'">{{ formatMessage(messages.vanillaDesc) }}</span>
									<span v-else-if="!isLoaderSupported(opt.id)">
										{{ formatMessage(messages.notSupported) }}
									</span>
									<span v-else-if="!isLoaderCompatible(opt.id)">
										{{ formatMessage(messages.incompatible, { version: selectedGameVersion }) }}
									</span>
									<span v-else-if="selectedLoader === opt.id && selectedLoaderVersion">
										{{
											formatMessage(messages.selectedVersion, { version: selectedLoaderVersion })
										}}
									</span>
									<span v-else>
										{{
											formatMessage(messages.versionsAvailable, {
												count: getLoaderVersionCount(opt.id),
											})
										}}
									</span>
								</span>
							</div>
						</div>
						<CheckIcon v-if="selectedLoader === opt.id" class="size-5 text-brand" />
						<ChevronDownIcon
							v-else-if="opt.id !== 'vanilla' && isLoaderSupported(opt.id)"
							class="size-5 text-secondary transition-transform"
							:class="{ 'rotate-180': expandedLoaderCards[opt.id] }"
						/>
					</div>

					<div
						v-if="opt.id !== 'vanilla' && expandedLoaderCards[opt.id]"
						class="loader-group-content"
					>
						<div v-if="loaderVersionsLoading" class="text-center text-secondary py-4">
							{{ formatMessage(commonMessages.loadingLabel) }}
						</div>
						<template v-else>
							<div class="flex items-center justify-between mb-2">
								<span class="font-semibold text-contrast text-sm">{{
									formatMessage(messages.loaderVersionLabel)
								}}</span>
							</div>
							<div class="flex flex-col gap-1 max-h-60 overflow-y-auto">
								<div
									v-for="v in getLoaderVersions(opt.id)"
									:key="v.id"
									class="loader-version-row"
									:class="{
										'loader-version-row-selected':
											selectedLoader === opt.id && selectedLoaderVersion === v.id,
									}"
									@click="selectLoaderAndVersion(opt.id, v.id)"
								>
									<div class="flex items-center gap-2 flex-1 min-w-0">
										<span class="font-semibold text-contrast truncate">{{ v.id }}</span>
										<span
											v-if="v.stable"
											class="text-xs px-1.5 py-0.5 rounded bg-green-highlight text-green"
											>{{ formatMessage(messages.stable) }}</span
										>
									</div>
									<CheckIcon
										v-if="selectedLoader === opt.id && selectedLoaderVersion === v.id"
										class="size-4 text-brand"
									/>
								</div>
							</div>
						</template>
					</div>
				</Card>
			</div>

			<div class="flex justify-end mt-4">
				<ButtonStyled color="brand" size="large" @click="step = STEP_CONFIG">
					<button>
						{{ formatMessage(commonMessages.nextButton) }}
						<ArrowLeftIcon class="size-4 rotate-180" />
					</button>
				</ButtonStyled>
			</div>
		</template>

		<!-- Step 3: Final config -->
		<template v-if="step === STEP_CONFIG">
			<Card class="p-4">
				<div class="flex flex-col gap-6">
					<div class="flex items-center gap-4">
						<Avatar :src="instanceIconUrl ?? undefined" size="5rem">
							<ImageIcon class="size-8 text-secondary" />
						</Avatar>
						<div class="flex flex-col gap-2">
							<ButtonStyled type="outlined" @click="selectIcon">
								<button>
									<UploadIcon class="size-4" />
									{{ formatMessage(messages.selectIcon) }}
								</button>
							</ButtonStyled>
							<ButtonStyled type="outlined" @click="removeIcon">
								<button :disabled="!instanceIconUrl">
									<XIcon class="size-4" />
									{{ formatMessage(messages.removeIcon) }}
								</button>
							</ButtonStyled>
						</div>
					</div>

					<div class="flex flex-col gap-2">
						<span class="font-semibold text-contrast">{{ formatMessage(messages.nameLabel) }}</span>
						<StyledInput v-model="instanceName" :placeholder="autoInstanceName" type="text" />
					</div>

					<div class="rounded-lg bg-surface-2 border border-surface-5 p-4 flex flex-col gap-2">
						<div class="flex justify-between">
							<span class="text-sm text-secondary">{{ formatMessage(messages.gameVersion) }}</span>
							<span class="font-semibold text-contrast">{{ selectedGameVersion }}</span>
						</div>
						<div class="flex justify-between">
							<span class="text-sm text-secondary">{{ formatMessage(messages.loader) }}</span>
							<span class="font-semibold text-contrast">{{
								loaderInfoMap[selectedLoader]?.label ?? selectedLoader
							}}</span>
						</div>
						<div v-if="selectedLoaderVersion" class="flex justify-between">
							<span class="text-sm text-secondary">{{
								formatMessage(messages.loaderVersion)
							}}</span>
							<span class="font-semibold text-contrast">{{ selectedLoaderVersion }}</span>
						</div>
					</div>

					<div class="flex justify-between">
						<ButtonStyled type="outlined" @click="step = STEP_LOADER">
							<button>{{ formatMessage(commonMessages.backButton) }}</button>
						</ButtonStyled>
						<ButtonStyled color="brand" size="large" @click="createInstance">
							<button :disabled="creating">
								{{
									creating
										? formatMessage(messages.creatingButton)
										: formatMessage(messages.createButton)
								}}
							</button>
						</ButtonStyled>
					</div>
				</div>
			</Card>
		</template>
	</div>
</template>

<style scoped lang="scss">
.step-item {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.5rem 0.75rem;
	border-radius: 0.5rem;
	font-size: 0.875rem;
	font-weight: 500;
	color: var(--color-secondary);
	background: var(--color-surface-2);
	border: 1px solid var(--color-surface-5);
}

.step-item-active {
	color: var(--color-contrast);
	border-color: var(--color-brand);
}

.step-item-current {
	background: var(--color-brand-highlight);
}

.step-number {
	display: flex;
	align-items: center;
	justify-content: center;
	width: 1.5rem;
	height: 1.5rem;
	border-radius: 9999px;
	background: var(--color-surface-5);
	font-size: 0.75rem;
	font-weight: 700;
}

.step-item-active .step-number {
	background: var(--color-brand);
	color: white;
}

.step-divider {
	flex: 1;
	height: 2px;
	background: var(--color-surface-5);
	border-radius: 1px;
}

.step-divider-active {
	background: var(--color-brand);
}

.version-group-card,
.loader-group-card {
	overflow: hidden;
}

.version-group-header,
.loader-group-header {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 1rem;
	gap: 1rem;
}

.version-group-content,
.loader-group-content {
	padding: 0 1rem 1rem;
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
}

.version-row,
.loader-version-row {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.5rem 0.75rem;
	border-radius: 0.5rem;
	border: 1px solid var(--color-surface-5);
	background: var(--color-surface-2);
	cursor: pointer;
	transition: all 0.15s ease;
}

.version-row:hover,
.loader-version-row:hover {
	background: var(--color-surface-3);
	border-color: var(--color-brand);
}

.version-row-selected,
.loader-version-row-selected {
	background: var(--color-brand-highlight);
	border-color: var(--color-brand);
}

.version-icon {
	width: 2.5rem;
	height: 2.5rem;
	border-radius: 0.5rem;
	display: flex;
	align-items: center;
	justify-content: center;
	font-weight: 700;
	color: white;
	flex-shrink: 0;
}

.version-icon.small {
	width: 2rem;
	height: 2rem;
	border-radius: 0.375rem;
	font-size: 0.875rem;
}

.loader-icon {
	width: 2.5rem;
	height: 2.5rem;
	border-radius: 0.5rem;
	display: flex;
	align-items: center;
	justify-content: center;
	font-weight: 700;
	color: white;
	flex-shrink: 0;
}

.version-icon-recent,
.loader-icon-vanilla {
	background: linear-gradient(135deg, #00b884, #008660);
}

.version-icon-release {
	background: linear-gradient(135deg, #1bd96a, #0f9c47);
}

.version-icon-snapshot {
	background: linear-gradient(135deg, #ff9a3c, #ff6b00);
}

.version-icon-old-beta {
	background: linear-gradient(135deg, #6b7280, #374151);
}

.version-icon-old-alpha {
	background: linear-gradient(135deg, #9ca3af, #4b5563);
}

.version-icon-april-fools {
	background: linear-gradient(135deg, #f59e0b, #d97706);
}

.loader-icon-forge {
	background: linear-gradient(135deg, #d97706, #92400e);
}

.loader-icon-neoforge {
	background: linear-gradient(135deg, #f97316, #c2410c);
}

.loader-icon-fabric {
	background: linear-gradient(135deg, #8b5cf6, #6d28d9);
}

.loader-icon-quilt {
	background: linear-gradient(135deg, #ec4899, #be185d);
}

.loader-icon-optifine {
	background: linear-gradient(135deg, #06b6d4, #0e7490);
}

.loader-icon-liteloader {
	background: linear-gradient(135deg, #6b7280, #374151);
}

.loader-icon-cleanroom {
	background: linear-gradient(135deg, #14b8a6, #0f766e);
}

.loader-icon-labymod {
	background: linear-gradient(135deg, #3b82f6, #1d4ed8);
}

.loader-group-card-selected {
	border-color: var(--color-brand);
	box-shadow: 0 0 0 1px var(--color-brand);
}

.mc-icon {
	width: 100%;
	height: 100%;
	object-fit: cover;
	image-rendering: pixelated;
}

.setup-type-grid {
	display: grid;
	grid-template-columns: repeat(3, minmax(0, 1fr));
	gap: 1rem;
}

.setup-type-button {
	display: flex;
	flex-direction: column;
	align-items: flex-start;
	justify-content: flex-start;
	text-align: left;
	gap: 0.5rem;
	padding: 1.5rem;
	background: var(--color-button-bg);
	border: 1px solid var(--color-surface-5);
	border-radius: 0.75rem;
	box-shadow: var(--shadow-button);
	color: var(--color-contrast);
	cursor: pointer;
	transition:
		scale 0.125s ease-in-out,
		background-color 0.25s ease-in-out,
		border-color 0.25s ease-in-out;
	height: 100%;
	min-height: 10rem;
}

.setup-type-button:hover {
	background: var(--color-surface-3);
	border-color: var(--color-brand);
}

.setup-type-button:active {
	scale: 0.98;
}

.setup-type-icon {
	width: 2.5rem;
	height: 2.5rem;
	color: var(--color-brand);
	margin-bottom: 0.5rem;
	flex-shrink: 0;
}

.setup-type-title {
	margin: 0;
	font-size: 1rem;
	font-weight: 700;
}

.setup-type-desc {
	margin: 0;
	font-size: 0.875rem;
	color: var(--color-secondary);
	line-height: 1.4;
}

// Import instance styles
.import-launcher-card {
	overflow: hidden;
}

.import-launcher-header {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.75rem 1rem;
	cursor: pointer;
	background: var(--color-surface-3);
	transition: background-color 0.15s ease;
}

.import-launcher-header:hover {
	background: var(--color-surface-4);
}

.import-launcher-content {
	display: flex;
	flex-direction: column;
}

.import-instance-row {
	display: flex;
	align-items: center;
	gap: 0.75rem;
	padding: 0.5rem 1rem 0.5rem 2.75rem;
	background: var(--color-surface-2);
	border-top: 1px solid var(--color-surface-5);
}

.import-instance-alt {
	background: var(--color-raised-bg);
}

.import-checkbox {
	width: 1rem;
	height: 1rem;
	accent-color: var(--color-brand);
	flex-shrink: 0;
	cursor: pointer;
}

@media (max-width: 768px) {
	.setup-type-grid {
		grid-template-columns: 1fr;
	}
}
</style>
