<script setup lang="ts">
import {
	CheckCircleIcon,
	CircleAlertIcon,
	CircleUserIcon,
	ClipboardCopyIcon,
	CrownIcon,
	GlobeIcon,
	LogOutIcon,
	MonitorIcon,
	RefreshCwIcon,
	ScanEyeIcon,
	ServerPlusIcon,
	ShieldCheckIcon,
	SpinnerIcon,
	UsersIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
	defineMessages,
	ErrorInformationCard,
	injectNotificationManager,
	NewModal as Modal,
	Tabs,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import GameLinkChat from '@/components/ui/GameLinkChat.vue'
import GameLinkAgreementModal from '@/components/ui/modal/GameLinkAgreementModal.vue'
import ReportModal from '@/components/ui/modal/ReportModal.vue'
import { get_default_user, users } from '@/helpers/auth'
import { get as getSettings, set as setSettings } from '@/helpers/settings'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useTheming } from '@/store/state'

const { formatMessage } = useVIntl()

// ---------------------------------------------------------------------------
// Types (mirror of Rust types in link/types.rs)
// ---------------------------------------------------------------------------

type LobbyState =
	| 'idle'
	| 'initializing'
	| 'initialized'
	| 'discovering'
	| 'creating'
	| 'joining'
	| 'connected'
	| 'leaving'
	| 'error'

type ConnectionWay = 'local' | 'p2p' | 'relay' | 'unknown'

type ConnectionQuality = 'Poor' | 'Fair' | 'Good'

interface ConnectionInfo {
	way: ConnectionWay
	quality: ConnectionQuality
	latency_ms: number
}

type PlayerKind = 'host' | 'client'

interface PlayerProfile {
	name: string
	machine_id: string
	vendor: string
	kind: PlayerKind | null
	latency_ms: number | null
}

interface HostModInfo {
	mod_id: string
	version: string
	name: string
}

interface ModCompatibilityResult {
	is_compatible: boolean
	local_only: HostModInfo[]
	host_only: HostModInfo[]
	version_mismatch: [HostModInfo, HostModInfo][]
}

interface FoundWorld {
	name: string
	port: number
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

const breadcrumbs = useBreadcrumbs()
const activeTab = ref<'host' | 'join'>('host')

const messages = defineMessages({
	breadcrumbName: { id: 'app.game-link.breadcrumb', defaultMessage: 'Game Link' },
	title: { id: 'app.game-link.title', defaultMessage: 'Game Link' },
	description: {
		id: 'app.game-link.description',
		defaultMessage:
			'Create or join a virtual network lobby to play Minecraft with friends over the internet, powered by EasyTier peer-to-peer networking.',
	},
	stateIdle: { id: 'app.game-link.state.idle', defaultMessage: 'Not connected' },
	stateCreating: { id: 'app.game-link.state.creating', defaultMessage: 'Creating lobby...' },
	stateJoining: { id: 'app.game-link.state.joining', defaultMessage: 'Joining lobby...' },
	stateConnected: { id: 'app.game-link.state.connected', defaultMessage: 'Connected' },
	stateLeaving: { id: 'app.game-link.state.leaving', defaultMessage: 'Leaving...' },
	stateError: { id: 'app.game-link.state.error', defaultMessage: 'Error' },
	playingAs: { id: 'app.game-link.playing-as', defaultMessage: 'Playing as' },
	noAccount: {
		id: 'app.game-link.no-account',
		defaultMessage: 'No Minecraft account logged in',
	},
	hostTab: { id: 'app.game-link.tab.host', defaultMessage: 'Host' },
	joinTab: { id: 'app.game-link.tab.join', defaultMessage: 'Join' },
	hostLobbyTitle: { id: 'app.game-link.host.title', defaultMessage: 'Host a lobby' },
	hostLobbySubtitle: {
		id: 'app.game-link.host.subtitle',
		defaultMessage:
			'Open a world to LAN in Minecraft, then scan and create a lobby for others to join.',
	},
	scanButton: { id: 'app.game-link.host.scan', defaultMessage: 'Scan for LAN worlds' },
	scanningButton: { id: 'app.game-link.host.scanning', defaultMessage: 'Scanning...' },
	selectWorld: { id: 'app.game-link.host.select-world', defaultMessage: 'Select world' },
	noWorldsHint: {
		id: 'app.game-link.host.no-worlds',
		defaultMessage: 'No LAN worlds found. Open Minecraft, load a world, and click "Open to LAN".',
	},
	createLobbyButton: { id: 'app.game-link.host.create', defaultMessage: 'Create lobby' },
	creatingLobbyButton: { id: 'app.game-link.host.creating', defaultMessage: 'Creating...' },
	joinLobbyTitle: { id: 'app.game-link.join.title', defaultMessage: 'Join a lobby' },
	joinLobbySubtitle: {
		id: 'app.game-link.join.subtitle',
		defaultMessage: 'Enter a lobby code shared by the host to connect.',
	},
	lobbyCodeLabel: { id: 'app.game-link.lobby-code.label', defaultMessage: 'Lobby code' },
	joinLobbyButton: { id: 'app.game-link.join.button', defaultMessage: 'Join lobby' },
	joiningLobbyButton: { id: 'app.game-link.join.joining', defaultMessage: 'Joining...' },
	copyTitle: { id: 'app.game-link.copy', defaultMessage: 'Copy' },
	copiedTitle: { id: 'app.game-link.copied', defaultMessage: 'Copied!' },
	roleHost: { id: 'app.game-link.role.host', defaultMessage: 'Host' },
	roleClient: { id: 'app.game-link.role.client', defaultMessage: 'Client' },
	leaveLobbyButton: { id: 'app.game-link.leave', defaultMessage: 'Leave lobby' },
	playersTitle: {
		id: 'app.game-link.players.title',
		defaultMessage: 'Players ({count})',
	},
	refreshButton: { id: 'app.game-link.refresh', defaultMessage: 'Refresh' },
	connectionInfoTitle: { id: 'app.game-link.connection.title', defaultMessage: 'Connection info' },
	latencyLabel: { id: 'app.game-link.connection.latency', defaultMessage: 'Latency' },
	qualityLabel: { id: 'app.game-link.connection.quality', defaultMessage: 'Quality' },
	connectionTypeLabel: {
		id: 'app.game-link.connection.type',
		defaultMessage: 'Connection type',
	},
	modCompatTitle: { id: 'app.game-link.mod-compat.title', defaultMessage: 'Mod compatibility' },
	checkCompatButton: {
		id: 'app.game-link.mod-compat.check',
		defaultMessage: 'Check compatibility',
	},
	modsCompatible: { id: 'app.game-link.mod-compat.ok', defaultMessage: 'Mods are compatible' },
	modMismatch: {
		id: 'app.game-link.mod-compat.mismatch',
		defaultMessage: 'Mod mismatch detected',
	},
	missingModsTitle: {
		id: 'app.game-link.mod-compat.missing',
		defaultMessage: 'Missing mods ({count})',
	},
	extraModsTitle: { id: 'app.game-link.mod-compat.extra', defaultMessage: 'Extra mods ({count})' },
	versionMismatchesTitle: {
		id: 'app.game-link.mod-compat.version-mismatch',
		defaultMessage: 'Version mismatches ({count})',
	},
	localWorldsTitle: { id: 'app.game-link.worlds.title', defaultMessage: 'Local worlds' },
	scanWorldsButton: { id: 'app.game-link.worlds.scan', defaultMessage: 'Scan' },
	noWorldsConnectedHint: {
		id: 'app.game-link.worlds.none',
		defaultMessage: 'No worlds found. Make sure Minecraft is running with LAN open.',
	},
	networkDiagnosticsTitle: {
		id: 'app.game-link.diagnostics.title',
		defaultMessage: 'Network diagnostics',
	},
	mcRunningLabel: {
		id: 'app.game-link.diagnostics.mc-running',
		defaultMessage: 'Minecraft running',
	},
	mcPortLabel: { id: 'app.game-link.diagnostics.mc-port', defaultMessage: 'MC port' },
	roleLabel: { id: 'app.game-link.diagnostics.role', defaultMessage: 'Role' },
	checkMcProcessButton: {
		id: 'app.game-link.diagnostics.check-mc',
		defaultMessage: 'Check MC process',
	},
	hostShutdownMessage: {
		id: 'app.game-link.host-shutdown',
		defaultMessage: 'Host has shut down the lobby.',
	},
	qualityGood: { id: 'app.game-link.quality.good', defaultMessage: 'Good' },
	qualityFair: { id: 'app.game-link.quality.fair', defaultMessage: 'Fair' },
	qualityPoor: { id: 'app.game-link.quality.poor', defaultMessage: 'Poor' },
	wayLocal: { id: 'app.game-link.way.local', defaultMessage: 'Local' },
	wayP2p: { id: 'app.game-link.way.p2p', defaultMessage: 'P2P' },
	wayRelay: { id: 'app.game-link.way.relay', defaultMessage: 'Relay' },
	wayUnknown: { id: 'app.game-link.way.unknown', defaultMessage: 'Unknown' },
	yes: { id: 'app.game-link.yes', defaultMessage: 'Yes' },
	no: { id: 'app.game-link.no', defaultMessage: 'No' },
	mcStatusRunning: { id: 'app.game-link.mc-status.running', defaultMessage: 'Minecraft running' },
	mcStatusStopped: {
		id: 'app.game-link.mc-status.stopped',
		defaultMessage: 'Minecraft not running',
	},
	portLabel: { id: 'app.game-link.port.label', defaultMessage: 'Port' },
	joinTimeoutError: {
		id: 'app.game-link.join.timeout',
		defaultMessage: 'Connection timed out. The lobby code may be invalid or the host is offline.',
	},
	cancelButton: { id: 'app.game-link.cancel', defaultMessage: 'Cancel' },
	retryButton: { id: 'app.game-link.retry', defaultMessage: 'Retry' },
	joiningHint: {
		id: 'app.game-link.join.hint',
		defaultMessage:
			'Joining can take up to 15 seconds while the peer-to-peer network is established.',
	},
	reportButton: { id: 'app.game-link.report.button', defaultMessage: 'Report' },
	errorDiscoverWorlds: {
		id: 'app.game-link.error.discover-worlds',
		defaultMessage: 'Failed to scan for LAN worlds',
	},
	errorRefreshPlayers: {
		id: 'app.game-link.error.refresh-players',
		defaultMessage: 'Failed to refresh players',
	},
	errorRefreshConnection: {
		id: 'app.game-link.error.refresh-connection',
		defaultMessage: 'Failed to refresh connection info',
	},
	errorCheckModCompat: {
		id: 'app.game-link.error.check-mod-compat',
		defaultMessage: 'Failed to check mod compatibility',
	},
	errorLeaveLobby: {
		id: 'app.game-link.error.leave-lobby',
		defaultMessage: 'Failed to leave lobby',
	},
	errorTitle: { id: 'app.game-link.error.title', defaultMessage: 'Connection error' },
	errorRetryAction: { id: 'app.game-link.error.retry', defaultMessage: 'Try again' },
})

const { addNotification } = injectNotificationManager()

const username = ref('')
const lobbyCode = ref('')
const joinCodeInput = ref('')
const selectedWorld = ref<FoundWorld | null>(null)
const mcPort = ref(0)
const justCopied = ref(false)

const state = ref<LobbyState>('idle')
const isHost = ref(false)
const players = ref<PlayerProfile[]>([])
const connectionInfo = ref<ConnectionInfo>({
	way: 'unknown',
	quality: 'Good',
	latency_ms: 0,
})
const hostMods = ref<HostModInfo[]>([])
const modCompatResult = ref<ModCompatibilityResult | null>(null)
const foundWorlds = ref<FoundWorld[]>([])
const mcRunning = ref(false)
const errorMsg = ref('')

const isBusy = ref(false)
const isScanning = ref(false)
const joinElapsed = ref(0)
const joinTimerId = ref<ReturnType<typeof setInterval> | null>(null)
const _joinAbortController = ref<AbortController | null>(null)
const reportModal = ref<InstanceType<typeof ReportModal> | null>(null)

// Agreement (EULA) state — must be accepted before any Game Link feature is usable.
// Persisted in localStorage so the choice survives restarts; revoking disables the feature entirely.
const LS_KEY_GAMELINK_AGREEMENT = 'modrinth-app-gamelink-agreement-accepted'
const themeStore = useTheming()
const router = useRouter()

const agreementModal = ref<InstanceType<typeof GameLinkAgreementModal> | null>(null)
const revokeConfirmModal = ref<InstanceType<typeof Modal> | null>(null)
const agreementAccepted = ref(false)

// Event unlisteners
const unlisteners: UnlistenFn[] = []

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

const isConnected = computed(() => state.value === 'connected')
const hasUsername = computed(() => username.value.trim().length > 0)
const canCreate = computed(
	() =>
		!isBusy.value && hasUsername.value && selectedWorld.value !== null && state.value === 'idle',
)
const canJoin = computed(
	() =>
		!isBusy.value &&
		hasUsername.value &&
		joinCodeInput.value.trim().length > 0 &&
		state.value === 'idle',
)
const canLeave = computed(() => !isBusy.value && state.value !== 'idle')

const myMachineId = computed(() => {
	const me = players.value.find((p) => p.name === username.value)
	return me?.machine_id ?? ''
})

const stateLabel = computed(() => {
	switch (state.value) {
		case 'idle':
			return formatMessage(messages.stateIdle)
		case 'creating':
			return formatMessage(messages.stateCreating)
		case 'joining':
			return formatMessage(messages.stateJoining)
		case 'connected':
			return formatMessage(messages.stateConnected)
		case 'leaving':
			return formatMessage(messages.stateLeaving)
		case 'error':
			return formatMessage(messages.stateError)
		default:
			return state.value
	}
})

const stateColor = computed(() => {
	switch (state.value) {
		case 'connected':
			return 'green'
		case 'creating':
		case 'joining':
			return 'orange'
		case 'error':
			return 'red'
		default:
			return 'standard'
	}
})

const connectionQualityColor = computed(() => {
	switch (connectionInfo.value.quality) {
		case 'Good':
			return 'green'
		case 'Fair':
			return 'orange'
		case 'Poor':
			return 'red'
		default:
			return 'standard'
	}
})

const tabItems = computed(() => [
	{ value: 'host', label: formatMessage(messages.hostTab), icon: CrownIcon },
	{ value: 'join', label: formatMessage(messages.joinTab), icon: CircleUserIcon },
])

function formatConnectionWay(way: ConnectionWay): string {
	switch (way) {
		case 'local':
			return formatMessage(messages.wayLocal)
		case 'p2p':
			return formatMessage(messages.wayP2p)
		case 'relay':
			return formatMessage(messages.wayRelay)
		default:
			return formatMessage(messages.wayUnknown)
	}
}

function formatConnectionQuality(quality: ConnectionQuality): string {
	switch (quality) {
		case 'Good':
			return formatMessage(messages.qualityGood)
		case 'Fair':
			return formatMessage(messages.qualityFair)
		case 'Poor':
			return formatMessage(messages.qualityPoor)
	}
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

function formatError(e: unknown): string {
	if (e instanceof Error) return e.message
	if (typeof e === 'string') return e
	if (e && typeof e === 'object' && 'message' in e)
		return String((e as Record<string, unknown>).message)
	return String(e)
}

async function createLobby() {
	if (!canCreate.value) return
	isBusy.value = true
	errorMsg.value = ''
	try {
		const port = selectedWorld.value?.port ?? mcPort.value
		const code = await invoke<string>('plugin:link|link_create_lobby', {
			mcPort: port,
			username: username.value,
		})
		lobbyCode.value = code
		mcPort.value = port
	} catch (e: unknown) {
		errorMsg.value = formatError(e)
		state.value = 'error'
	} finally {
		isBusy.value = false
	}
}

async function joinLobby() {
	if (!canJoin.value || isBusy.value) return
	isBusy.value = true
	errorMsg.value = ''
	joinElapsed.value = 0

	// Count elapsed seconds and show the user that something is happening.
	joinTimerId.value = setInterval(() => {
		joinElapsed.value += 1
	}, 1000)

	// Tauri does not support aborting an in-flight invoke. We race it against
	// an 18-second client-side timeout so the UI never hangs forever.
	const timeoutPromise = new Promise<boolean>((_, reject) => {
		setTimeout(() => {
			reject(new Error(formatMessage(messages.joinTimeoutError)))
		}, 18000)
	})

	try {
		await Promise.race([
			invoke<boolean>('plugin:link|link_join_lobby', {
				lobbyCode: joinCodeInput.value.trim(),
				username: username.value,
			}),
			timeoutPromise,
		])
		lobbyCode.value = joinCodeInput.value.trim()
	} catch (e: unknown) {
		errorMsg.value = formatError(e)
		state.value = 'error'
		joinCodeInput.value = ''
	} finally {
		if (joinTimerId.value) {
			clearInterval(joinTimerId.value)
			joinTimerId.value = null
		}
		joinElapsed.value = 0
		isBusy.value = false
	}
}

async function cancelJoin() {
	// Best-effort cleanup: call leave to stop the half-initialized network.
	try {
		await invoke('plugin:link|link_leave_lobby')
	} catch {
		// ignore
	}
	resetState()
}

function retryJoin() {
	errorMsg.value = ''
	state.value = 'idle'
	activeTab.value = 'join'
}

function selectWorld(world: FoundWorld) {
	selectedWorld.value = world
	mcPort.value = world.port
}

async function leaveLobby() {
	if (!canLeave.value) return
	isBusy.value = true
	try {
		await invoke('plugin:link|link_leave_lobby')
		resetState()
	} catch (e: unknown) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.errorLeaveLobby),
			text: formatError(e),
		})
	} finally {
		isBusy.value = false
	}
}

async function refreshPlayers() {
	try {
		players.value = await invoke<PlayerProfile[]>('plugin:link|link_get_players')
	} catch (e: unknown) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.errorRefreshPlayers),
			text: formatError(e),
		})
	}
}

async function refreshConnectionInfo() {
	try {
		connectionInfo.value = await invoke<ConnectionInfo>('plugin:link|link_get_connection_info')
	} catch (e: unknown) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.errorRefreshConnection),
			text: formatError(e),
		})
	}
}

async function checkModCompat() {
	try {
		modCompatResult.value = await invoke<ModCompatibilityResult>(
			'plugin:link|link_check_mod_compat',
			{
				instancePath: '',
			},
		)
	} catch (e: unknown) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.errorCheckModCompat),
			text: formatError(e),
		})
	}
}

async function discoverWorlds() {
	isScanning.value = true
	try {
		foundWorlds.value = await invoke<FoundWorld[]>('plugin:link|link_discover_worlds')
		if (selectedWorld.value === null && foundWorlds.value.length > 0) {
			selectWorld(foundWorlds.value[0])
		}
	} catch (e: unknown) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.errorDiscoverWorlds),
			text: formatError(e),
		})
	} finally {
		isScanning.value = false
	}
}

async function checkMcRunning() {
	try {
		mcRunning.value = await invoke<boolean>('plugin:link|link_is_minecraft_running')
	} catch {
		mcRunning.value = false
	}
}

async function copyLobbyCode() {
	if (!lobbyCode.value) return
	try {
		await navigator.clipboard.writeText(lobbyCode.value)
		justCopied.value = true
		setTimeout(() => {
			justCopied.value = false
		}, 2000)
	} catch {
		// ignore
	}
}

function resetState() {
	lobbyCode.value = ''
	joinCodeInput.value = ''
	state.value = 'idle'
	isHost.value = false
	players.value = []
	connectionInfo.value = { way: 'unknown', quality: 'Good', latency_ms: 0 }
	hostMods.value = []
	modCompatResult.value = null
	foundWorlds.value = []
}

// ---------------------------------------------------------------------------
// Agreement (EULA) actions
// ---------------------------------------------------------------------------

function checkAgreementStatus() {
	agreementAccepted.value = localStorage.getItem(LS_KEY_GAMELINK_AGREEMENT) === 'true'
	if (!agreementAccepted.value) {
		// Defer to next tick so the modal ref is mounted before we call show().
		requestAnimationFrame(() => {
			agreementModal.value?.show()
		})
	}
}

async function initLinkBackend() {
	try {
		state.value = await invoke<LobbyState>('plugin:link|link_get_state')
		isHost.value = await invoke<boolean>('plugin:link|link_is_host')
		const code = await invoke<string | null>('plugin:link|link_get_lobby_code')
		if (code) lobbyCode.value = code
		if (state.value === 'connected') {
			await refreshPlayers()
			await refreshConnectionInfo()
			await checkMcRunning()
		}
	} catch {
		// link module may not be initialized yet
	}
}

function acceptAgreement() {
	localStorage.setItem(LS_KEY_GAMELINK_AGREEMENT, 'true')
	agreementAccepted.value = true
	// Now that the user has accepted, initialize the backend link module.
	initLinkBackend()
}

async function declineAgreement() {
	// User declined the agreement — disable the feature flag and leave the page.
	await disableGameLinkFeatureFlag()
	router.replace('/')
}

function requestRevokeAgreement() {
	revokeConfirmModal.value?.show()
}

async function confirmRevokeAgreement() {
	revokeConfirmModal.value?.hide()
	// Best-effort: leave any active lobby before disabling the feature.
	try {
		await invoke('plugin:link|link_leave_lobby')
	} catch {
		// ignore — may not be in a lobby
	}
	resetState()
	localStorage.removeItem(LS_KEY_GAMELINK_AGREEMENT)
	agreementAccepted.value = false
	await disableGameLinkFeatureFlag()
	router.replace('/')
}

async function disableGameLinkFeatureFlag() {
	themeStore.featureFlags.game_link = false
	try {
		const settings = await getSettings()
		if (settings.feature_flags) {
			settings.feature_flags.game_link = false
			await setSettings(settings)
		}
	} catch (e) {
		console.error('Failed to persist game_link feature flag disable:', e)
	}
}

// ---------------------------------------------------------------------------
// Event listeners
// ---------------------------------------------------------------------------

async function setupListeners() {
	unlisteners.push(
		await listen<LobbyState>('link_state_changed', async (event) => {
			state.value = event.payload
			if (event.payload === 'connected') {
				invoke<boolean>('plugin:link|link_is_host')
					.then((h) => {
						isHost.value = h
					})
					.catch(() => {})
				invoke<string | null>('plugin:link|link_get_lobby_code')
					.then((code) => {
						if (code) lobbyCode.value = code
					})
					.catch(() => {})
				// Ensure players list is populated immediately so that
				// myMachineId resolves correctly for the chat component.
				await refreshPlayers()
			}
		}),
	)
	unlisteners.push(
		await listen<PlayerProfile[]>('link_players_changed', (event) => {
			players.value = event.payload
		}),
	)
	unlisteners.push(
		await listen<number>('link_heartbeat', (event) => {
			connectionInfo.value.latency_ms = event.payload
			if (event.payload < 100) {
				connectionInfo.value.quality = 'Good'
			} else if (event.payload < 200) {
				connectionInfo.value.quality = 'Fair'
			} else {
				connectionInfo.value.quality = 'Poor'
			}
		}),
	)
	unlisteners.push(
		await listen('link_server_shutdown', () => {
			addNotification({
				type: 'warning',
				title: formatMessage(messages.hostShutdownMessage),
			})
			resetState()
		}),
	)
	unlisteners.push(
		await listen<HostModInfo[]>('link_mod_compat_result', (event) => {
			hostMods.value = event.payload
		}),
	)
	unlisteners.push(
		await listen<number>('link_server_port', (event) => {
			mcPort.value = event.payload
		}),
	)
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

onMounted(async () => {
	breadcrumbs.setName(formatMessage(messages.breadcrumbName))
	await setupListeners()

	// Gate the entire page behind the agreement. If the user hasn't accepted yet,
	// show the agreement modal and skip the rest of the initialization until they do.
	checkAgreementStatus()

	try {
		const defaultId = await get_default_user()
		const accountList = await users()
		const defaultAccount = accountList?.find((acc) => acc.profile?.id === defaultId)
		if (defaultAccount?.profile?.name) {
			username.value = defaultAccount.profile.name
		}
	} catch {
		// ignore
	}

	// Skip backend link initialization until the user accepts the agreement.
	if (!agreementAccepted.value) return

	await initLinkBackend()
})

onUnmounted(() => {
	unlisteners.forEach((fn) => fn())
})
</script>

<template>
	<div class="game-link-page">
		<!-- ── Page Header ────────────────────────────────────────── -->
		<div class="page-header">
			<div class="header-row">
				<h1 class="title">{{ formatMessage(messages.title) }}</h1>
				<ButtonStyled :color="stateColor" type="chip">
					<span class="status-chip">{{ stateLabel }}</span>
				</ButtonStyled>
				<div class="header-spacer" />
				<ButtonStyled v-if="agreementAccepted" type="transparent" :hover-filled="true">
					<button @click="reportModal?.show()">
						<CircleAlertIcon />
						{{ formatMessage(messages.reportButton) }}
					</button>
				</ButtonStyled>
			</div>
			<p class="description">{{ formatMessage(messages.description) }}</p>

			<!-- User identity -->
			<div v-if="agreementAccepted" class="user-identity">
				<span class="user-label">{{ formatMessage(messages.playingAs) }}</span>
				<span v-if="hasUsername" class="user-name">{{ username }}</span>
				<span v-else class="user-name user-name--missing">{{
					formatMessage(messages.noAccount)
				}}</span>
			</div>

			<!-- Agreement status row: shown once the user has accepted the EULA.
           Allows them to revoke authorization, which disables the feature entirely. -->
			<div v-if="agreementAccepted" class="agreement-status">
				<ShieldCheckIcon class="agreement-status-icon" />
				<span class="agreement-status-text">联机协议：已同意</span>
				<button class="agreement-revoke-btn" @click="requestRevokeAgreement">停用联机功能</button>
			</div>
		</div>

		<!-- ── Agreement gate: hide the feature body until the EULA is accepted ── -->
		<template v-if="agreementAccepted">
			<!-- ── IDLE: Tab-based Host / Join ─────────────────────────── -->
			<template v-if="state === 'idle'">
				<Tabs v-model:value="activeTab" :tabs="tabItems" />

				<!-- Host Tab -->
				<Card v-if="activeTab === 'host'" class="action-card">
					<template #header>
						<div class="card-header-row">
							<ServerPlusIcon class="card-header-icon host-icon" />
							<div>
								<h2 class="card-header-title">{{ formatMessage(messages.hostLobbyTitle) }}</h2>
								<p class="card-header-subtitle">{{ formatMessage(messages.hostLobbySubtitle) }}</p>
							</div>
						</div>
					</template>

					<div class="card-content">
						<div class="btn-row">
							<ButtonStyled color="brand" :disabled="isScanning || isBusy">
								<button @click="discoverWorlds">
									<ScanEyeIcon />
									{{
										isScanning
											? formatMessage(messages.scanningButton)
											: formatMessage(messages.scanButton)
									}}
								</button>
							</ButtonStyled>
						</div>

						<div v-if="foundWorlds.length > 0" class="world-list">
							<div class="input-label">{{ formatMessage(messages.selectWorld) }}</div>
							<div
								v-for="world in foundWorlds"
								:key="world.port"
								class="world-item"
								:class="{ 'world-item--selected': selectedWorld?.port === world.port }"
								@click="selectWorld(world)"
							>
								<div class="world-item-info">
									<GlobeIcon class="world-item-icon" />
									<span class="world-item-name">{{ world.name }}</span>
								</div>
								<span class="world-item-port">:{{ world.port }}</span>
							</div>
						</div>
						<p v-else-if="!isScanning" class="empty-hint">
							{{ formatMessage(messages.noWorldsHint) }}
						</p>

						<div class="btn-row btn-row--end">
							<ButtonStyled color="green" :disabled="!canCreate">
								<button @click="createLobby">
									<ServerPlusIcon />
									{{
										isBusy
											? formatMessage(messages.creatingLobbyButton)
											: formatMessage(messages.createLobbyButton)
									}}
								</button>
							</ButtonStyled>
						</div>
					</div>
				</Card>

				<!-- Join Tab -->
				<Card v-if="activeTab === 'join'" class="action-card">
					<template #header>
						<div class="card-header-row">
							<CircleUserIcon class="card-header-icon client-icon" />
							<div>
								<h2 class="card-header-title">{{ formatMessage(messages.joinLobbyTitle) }}</h2>
								<p class="card-header-subtitle">{{ formatMessage(messages.joinLobbySubtitle) }}</p>
							</div>
						</div>
					</template>

					<div class="card-content">
						<div class="input-group">
							<label class="input-label">{{ formatMessage(messages.lobbyCodeLabel) }}</label>
							<input
								v-model="joinCodeInput"
								type="text"
								class="text-input"
								placeholder="U/XXXX-XXXX-XXXX-XXXX"
								:disabled="isBusy"
								@keyup.enter="joinLobby"
							/>
						</div>
						<div class="btn-row btn-row--end">
							<ButtonStyled color="blue" :disabled="!canJoin">
								<button @click="joinLobby">
									<UsersIcon />
									{{
										isBusy
											? formatMessage(messages.joiningLobbyButton)
											: formatMessage(messages.joinLobbyButton)
									}}
								</button>
							</ButtonStyled>
						</div>
					</div>
				</Card>
			</template>

			<!-- ── CONNECTED: Lobby Dashboard ─────────────────────────── -->
			<template v-else-if="isConnected">
				<!-- Lobby code hero + live status chips -->
				<Card class="lobby-code-card">
					<div class="lobby-code-layout">
						<div class="lobby-code-main">
							<div class="lobby-code-label">{{ formatMessage(messages.lobbyCodeLabel) }}</div>
							<div class="lobby-code-value-row">
								<code class="lobby-code-text">{{ lobbyCode }}</code>
								<ButtonStyled color="brand" type="highlight" :hover-filled="true">
									<button @click="copyLobbyCode">
										<ClipboardCopyIcon />
										{{
											justCopied
												? formatMessage(messages.copiedTitle)
												: formatMessage(messages.copyTitle)
										}}
									</button>
								</ButtonStyled>
							</div>
							<!-- Live status bar: role + quality + latency + mc port -->
							<div class="status-bar">
								<ButtonStyled :color="isHost ? 'green' : 'blue'" type="chip">
									<span class="status-chip-inline">
										<CrownIcon v-if="isHost" class="status-chip-icon" />
										<CircleUserIcon v-else class="status-chip-icon" />
										{{
											isHost ? formatMessage(messages.roleHost) : formatMessage(messages.roleClient)
										}}
									</span>
								</ButtonStyled>
								<ButtonStyled :color="connectionQualityColor" type="chip">
									<span class="status-chip-inline">
										<GlobeIcon class="status-chip-icon" />
										{{ formatConnectionQuality(connectionInfo.quality) }} ·
										{{ connectionInfo.latency_ms }}ms
									</span>
								</ButtonStyled>
								<ButtonStyled :color="mcRunning ? 'green' : 'orange'" type="chip">
									<span class="status-chip-inline">
										<MonitorIcon v-if="mcRunning" class="status-chip-icon" />
										<CircleAlertIcon v-else class="status-chip-icon" />
										{{
											mcRunning
												? formatMessage(messages.mcStatusRunning)
												: formatMessage(messages.mcStatusStopped)
										}}
									</span>
								</ButtonStyled>
								<ButtonStyled type="chip">
									<span class="status-chip-inline">
										{{ formatMessage(messages.portLabel) }} {{ mcPort }}
									</span>
								</ButtonStyled>
							</div>
						</div>
					</div>
				</Card>

				<!-- Players list — always visible -->
				<Card>
					<template #header>
						<div class="section-header-row">
							<UsersIcon class="section-icon" />
							<h3>{{ formatMessage(messages.playersTitle, { count: players.length }) }}</h3>
						</div>
					</template>
					<div class="player-list">
						<div
							v-for="player in players"
							:key="player.machine_id"
							class="player-item"
							:class="player.kind === 'host' ? 'player-item--host' : 'player-item--client'"
						>
							<div
								class="player-avatar"
								:class="player.kind === 'host' ? 'player-avatar--host' : 'player-avatar--client'"
							>
								{{ player.name.charAt(0).toUpperCase() }}
							</div>
							<div class="player-info">
								<span class="player-name">{{ player.name }}</span>
								<span class="player-role">
									<CrownIcon v-if="player.kind === 'host'" class="role-icon-inline" />
									{{
										player.kind === 'host'
											? formatMessage(messages.roleHost)
											: formatMessage(messages.roleClient)
									}}
								</span>
							</div>
							<span
								v-if="player.latency_ms !== null"
								class="player-latency"
								:class="{
									'latency-good': player.latency_ms < 100,
									'latency-fair': player.latency_ms >= 100 && player.latency_ms < 200,
									'latency-poor': player.latency_ms >= 200,
								}"
							>
								{{ player.latency_ms }}ms
							</span>
						</div>
						<div v-if="players.length === 0" class="empty-hint">
							{{ formatMessage(messages.playersTitle, { count: 0 }) }}
						</div>
					</div>
					<div class="btn-row">
						<ButtonStyled type="transparent" :hover-filled="true">
							<button @click="refreshPlayers">
								<RefreshCwIcon />
								{{ formatMessage(messages.refreshButton) }}
							</button>
						</ButtonStyled>
					</div>
				</Card>

				<!-- Room Chat -->
				<GameLinkChat :machine-id="myMachineId" />

				<!-- Connection details — collapsed by default, only advanced info -->
				<Card collapsible default-collapsed>
					<template #header>
						<div class="section-header-row">
							<GlobeIcon class="section-icon" />
							<h3>{{ formatMessage(messages.connectionInfoTitle) }}</h3>
						</div>
					</template>
					<div class="info-grid">
						<div class="info-item">
							<span class="info-label">{{ formatMessage(messages.latencyLabel) }}</span>
							<span class="info-value" :class="'info-value--' + connectionQualityColor">
								{{ connectionInfo.latency_ms }} ms
							</span>
						</div>
						<div class="info-item">
							<span class="info-label">{{ formatMessage(messages.qualityLabel) }}</span>
							<span class="info-value" :class="'info-value--' + connectionQualityColor">
								{{ formatConnectionQuality(connectionInfo.quality) }}
							</span>
						</div>
						<div class="info-item">
							<span class="info-label">{{ formatMessage(messages.connectionTypeLabel) }}</span>
							<span class="info-value">{{ formatConnectionWay(connectionInfo.way) }}</span>
						</div>
					</div>
					<div class="btn-row">
						<ButtonStyled type="transparent" :hover-filled="true">
							<button @click="refreshConnectionInfo">
								<RefreshCwIcon />
								{{ formatMessage(messages.refreshButton) }}
							</button>
						</ButtonStyled>
					</div>
				</Card>

				<!-- Mod compatibility — collapsed by default, client only -->
				<Card v-if="!isHost" collapsible default-collapsed>
					<template #header>
						<div class="section-header-row">
							<ShieldCheckIcon class="section-icon" />
							<h3>{{ formatMessage(messages.modCompatTitle) }}</h3>
						</div>
					</template>
					<div class="btn-row">
						<ButtonStyled color="blue" :disabled="isBusy">
							<button @click="checkModCompat">
								<ShieldCheckIcon />
								{{ formatMessage(messages.checkCompatButton) }}
							</button>
						</ButtonStyled>
					</div>
					<div v-if="modCompatResult" class="mod-compat-result">
						<div
							class="compat-status"
							:class="modCompatResult.is_compatible ? 'compat-ok' : 'compat-warn'"
						>
							<CheckCircleIcon v-if="modCompatResult.is_compatible" />
							<CircleAlertIcon v-else />
							{{
								modCompatResult.is_compatible
									? formatMessage(messages.modsCompatible)
									: formatMessage(messages.modMismatch)
							}}
						</div>
						<div v-if="modCompatResult.host_only.length > 0" class="compat-group">
							<div class="compat-group-title">
								{{
									formatMessage(messages.missingModsTitle, {
										count: modCompatResult.host_only.length,
									})
								}}
							</div>
							<div
								v-for="mod in modCompatResult.host_only"
								:key="mod.mod_id"
								class="compat-mod-item"
							>
								<span class="mod-name">{{ mod.name || mod.mod_id }}</span>
								<span class="mod-version">{{ mod.version }}</span>
							</div>
						</div>
						<div v-if="modCompatResult.local_only.length > 0" class="compat-group">
							<div class="compat-group-title">
								{{
									formatMessage(messages.extraModsTitle, {
										count: modCompatResult.local_only.length,
									})
								}}
							</div>
							<div
								v-for="mod in modCompatResult.local_only"
								:key="mod.mod_id"
								class="compat-mod-item"
							>
								<span class="mod-name">{{ mod.name || mod.mod_id }}</span>
								<span class="mod-version">{{ mod.version }}</span>
							</div>
						</div>
						<div v-if="modCompatResult.version_mismatch.length > 0" class="compat-group">
							<div class="compat-group-title">
								{{
									formatMessage(messages.versionMismatchesTitle, {
										count: modCompatResult.version_mismatch.length,
									})
								}}
							</div>
							<div
								v-for="(pair, idx) in modCompatResult.version_mismatch"
								:key="idx"
								class="compat-mod-item"
							>
								<span class="mod-name">{{ pair[0].name || pair[0].mod_id }}</span>
								<span class="mod-version mod-version--mismatch"
									>{{ pair[0].version }} vs {{ pair[1].version }}</span
								>
							</div>
						</div>
					</div>
				</Card>

				<!-- Leave lobby -->
				<div class="leave-row">
					<ButtonStyled color="red" :disabled="!canLeave">
						<button @click="leaveLobby">
							<LogOutIcon />
							{{ formatMessage(messages.leaveLobbyButton) }}
						</button>
					</ButtonStyled>
				</div>
			</template>

			<!-- ── BUSY: Loading state ─────────────────────────────────── -->
			<Card
				v-else-if="state === 'creating' || state === 'joining' || state === 'leaving'"
				class="busy-card"
			>
				<div class="busy-content">
					<SpinnerIcon class="spinner-icon animate-spin" />
					<div class="busy-text-stack">
						<span class="busy-text">{{ stateLabel }}</span>
						<span v-if="state === 'joining'" class="busy-hint">
							{{ formatMessage(messages.joiningHint) }}
							<span v-if="joinElapsed > 0">({{ joinElapsed }}s)</span>
						</span>
					</div>
				</div>
				<div v-if="state === 'joining'" class="busy-actions">
					<ButtonStyled color="red" type="outline" @click="cancelJoin">
						<button>{{ formatMessage(messages.cancelButton) }}</button>
					</ButtonStyled>
				</div>
			</Card>

			<!-- ── ERROR state ─────────────────────────────────────────── -->
			<ErrorInformationCard
				v-else-if="state === 'error'"
				:title="formatMessage(messages.errorTitle)"
				:description="errorMsg || formatMessage(messages.stateError)"
				:icon="CircleAlertIcon"
				:action="{
					label: formatMessage(messages.errorRetryAction),
					onClick: retryJoin,
					color: 'brand',
					icon: RefreshCwIcon,
				}"
			/>

			<!-- Report Modal -->
			<ReportModal ref="reportModal" :players="players" :lobby-code="lobbyCode" />
		</template>
		<!-- ── End of agreement gate ──────────────────────────────── -->

		<!-- ── Agreement (EULA) Modal: shown when the user hasn't accepted yet ── -->
		<GameLinkAgreementModal
			ref="agreementModal"
			@accept="acceptAgreement"
			@decline="declineAgreement"
		/>

		<!-- ── Revoke confirmation modal ───────────────────────────── -->
		<Modal
			ref="revokeConfirmModal"
			:closable="true"
			:close-on-click-outside="true"
			:close-on-esc="true"
			header="撤销授权确认"
			max-width="440px"
		>
			<div class="revoke-confirm-body">
				<CircleAlertIcon class="revoke-confirm-icon" />
				<p class="revoke-confirm-text">你确定要撤销联机协议授权吗？</p>
				<p class="revoke-confirm-hint">
					撤销后，联机功能将被停用，侧边栏入口也会隐藏。如需再次使用，需要重新同意协议。
				</p>
				<div class="revoke-confirm-actions">
					<ButtonStyled type="transparent">
						<button @click="revokeConfirmModal?.hide()">取消</button>
					</ButtonStyled>
					<ButtonStyled color="red">
						<button @click="confirmRevokeAgreement">确认撤销</button>
					</ButtonStyled>
				</div>
			</div>
		</Modal>
	</div>
</template>

<style scoped>
.game-link-page {
	max-width: 48rem;
	margin: 0 auto;
	padding: 1.5rem 1rem 3rem;
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

/* ── Page Header ─────────────────────────────── */
.page-header {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	margin-bottom: 0.25rem;
}

.header-row {
	display: flex;
	align-items: center;
	gap: 0.75rem;
}

.header-spacer {
	flex: 1;
}

.title {
	font-size: 1.5rem;
	font-weight: 700;
	margin: 0;
	color: var(--color-text);
}

.status-chip {
	display: inline-flex;
	align-items: center;
	gap: 0.25rem;
	font-size: 0.8rem;
	font-weight: 600;
	text-transform: uppercase;
	letter-spacing: 0.04em;
}

.description {
	margin: 0;
	color: var(--color-text-secondary);
	font-size: 0.9rem;
	line-height: 1.5;
}

.user-identity {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	font-size: 0.85rem;
}

.user-label {
	color: var(--color-text-secondary);
}

.user-name {
	font-weight: 600;
	color: var(--color-text);
}

.user-name--missing {
	color: var(--color-orange);
	font-style: italic;
}

/* ── Agreement Status ────────────────────────── */
.agreement-status {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.45rem 0.75rem;
	background: var(--color-brand-bg);
	border: 1px solid var(--color-brand);
	border-radius: var(--radius-sm);
	font-size: 0.8rem;
}

.agreement-status-icon {
	width: 0.95rem;
	height: 0.95rem;
	color: var(--color-brand);
	flex-shrink: 0;
}

.agreement-status-text {
	color: var(--color-brand);
	font-weight: 600;
	flex: 1;
}

.agreement-revoke-btn {
	background: transparent;
	border: none;
	color: var(--color-text-secondary);
	font-size: 0.78rem;
	cursor: pointer;
	padding: 0.15rem 0.5rem;
	border-radius: var(--radius-sm);
	text-decoration: underline;
	transition:
		color 0.15s,
		background-color 0.15s;
}

.agreement-revoke-btn:hover {
	color: var(--color-red);
	background: var(--color-red-bg);
}

/* ── Revoke Confirm Modal ────────────────────── */
.revoke-confirm-body {
	display: flex;
	flex-direction: column;
	align-items: center;
	text-align: center;
	gap: 0.75rem;
	padding: 0.5rem 0.25rem;
}

.revoke-confirm-icon {
	width: 2rem;
	height: 2rem;
	color: var(--color-red);
}

.revoke-confirm-text {
	margin: 0;
	font-size: 1rem;
	font-weight: 600;
	color: var(--color-text);
}

.revoke-confirm-hint {
	margin: 0;
	font-size: 0.8rem;
	line-height: 1.5;
	color: var(--color-text-secondary);
	max-width: 24rem;
}

.revoke-confirm-actions {
	display: flex;
	gap: 0.75rem;
	margin-top: 0.5rem;
}

/* ── Card Headers ────────────────────────────── */
.card-header-row {
	display: flex;
	align-items: center;
	gap: 0.75rem;
}

.card-header-icon {
	width: 1.5rem;
	height: 1.5rem;
	flex-shrink: 0;
}

.host-icon {
	color: var(--color-green);
}

.client-icon {
	color: var(--color-blue);
}

.card-header-title {
	font-size: 1rem;
	font-weight: 700;
	margin: 0;
	color: var(--color-text);
}

.card-header-subtitle {
	margin: 0;
	font-size: 0.8rem;
	color: var(--color-text-secondary);
	line-height: 1.4;
}

/* ── Card Content ────────────────────────────── */
.card-content {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

.action-card :deep(.card) {
	/* ensure card body has room */
}

/* ── Buttons ─────────────────────────────────── */
.btn-row {
	display: flex;
	gap: 0.5rem;
	align-items: center;
}

.btn-row--end {
	justify-content: flex-end;
}

/* ── World List ──────────────────────────────── */
.world-list {
	display: flex;
	flex-direction: column;
	gap: 0.35rem;
}

.world-item {
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.5rem 0.75rem;
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	cursor: pointer;
	transition:
		border-color 0.15s,
		background-color 0.15s;
}

.world-item:hover {
	border-color: var(--color-brand);
}

.world-item--selected {
	border-color: var(--color-brand);
	background: var(--color-brand-bg);
}

.world-item-info {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.world-item-icon {
	width: 1rem;
	height: 1rem;
	color: var(--color-text-secondary);
}

.world-item-name {
	font-weight: 500;
	color: var(--color-text);
}

.world-item-port {
	font-family: 'Fira Code', 'Cascadia Code', monospace;
	font-size: 0.85rem;
	color: var(--color-text-secondary);
}

/* ── Input ───────────────────────────────────── */
.input-group {
	display: flex;
	flex-direction: column;
	gap: 0.35rem;
}

.input-label {
	font-size: 0.75rem;
	font-weight: 600;
	color: var(--color-text-secondary);
	text-transform: uppercase;
	letter-spacing: 0.04em;
}

.text-input {
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	padding: 0.5rem 0.75rem;
	font-size: 0.9rem;
	color: var(--color-text);
	outline: none;
	transition: border-color 0.15s;
	font-family: 'Fira Code', 'Cascadia Code', monospace;
}

.text-input:focus {
	border-color: var(--color-brand);
}

.text-input:disabled {
	opacity: 0.5;
}

/* ── Lobby Code Card (Connected) ─────────────── */
.lobby-code-card {
	background: linear-gradient(135deg, var(--color-brand-bg) 0%, var(--color-bg) 100%);
}

.lobby-code-layout {
	display: flex;
	align-items: flex-start;
	justify-content: space-between;
	gap: 1rem;
	flex-wrap: wrap;
}

.lobby-code-main {
	display: flex;
	flex-direction: column;
	gap: 0.6rem;
	flex: 1;
	min-width: min(100%, 18rem);
}

.lobby-code-label {
	font-size: 0.7rem;
	font-weight: 600;
	text-transform: uppercase;
	letter-spacing: 0.06em;
	color: var(--color-text-secondary);
}

.lobby-code-value-row {
	display: flex;
	align-items: center;
	gap: 0.75rem;
	flex-wrap: wrap;
}

.lobby-code-text {
	font-family: 'Fira Code', 'Cascadia Code', monospace;
	font-size: clamp(1.1rem, 4vw, 1.5rem);
	font-weight: 700;
	color: var(--color-brand);
	letter-spacing: 0.04em;
}

.lobby-code-side {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.status-bar {
	display: flex;
	flex-wrap: wrap;
	gap: 0.4rem;
	align-items: center;
}

.status-chip-inline {
	display: inline-flex;
	align-items: center;
	gap: 0.3rem;
	font-size: 0.75rem;
	font-weight: 600;
}

.status-chip-icon {
	width: 0.85rem;
	height: 0.85rem;
}

.role-chip {
	display: inline-flex;
	align-items: center;
	gap: 0.3rem;
}

.role-chip-icon {
	width: 0.9rem;
	height: 0.9rem;
}

/* ── Section Headers ─────────────────────────── */
.section-header-row {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.section-icon {
	width: 1.1rem;
	height: 1.1rem;
	color: var(--color-text-secondary);
}

/* ── Player List ─────────────────────────────── */
.player-list {
	display: flex;
	flex-direction: column;
	gap: 0.35rem;
}

.player-item {
	display: flex;
	align-items: center;
	gap: 0.65rem;
	padding: 0.5rem 0.65rem;
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	border-left: 3px solid transparent;
}

.player-item--host {
	border-left-color: var(--color-green);
}

.player-item--client {
	border-left-color: var(--color-blue);
}

.player-avatar {
	width: 28px;
	height: 28px;
	border-radius: 50%;
	display: flex;
	align-items: center;
	justify-content: center;
	font-weight: 700;
	font-size: 0.8rem;
	flex-shrink: 0;
}

.player-avatar--host {
	background: var(--color-green);
	color: var(--color-accent-contrast);
}

.player-avatar--client {
	background: var(--color-blue);
	color: var(--color-accent-contrast);
}

.player-info {
	flex: 1;
	display: flex;
	flex-direction: column;
	gap: 0.1rem;
}

.player-name {
	font-weight: 600;
	font-size: 0.85rem;
	color: var(--color-text);
}

.player-role {
	font-size: 0.7rem;
	color: var(--color-text-secondary);
	display: flex;
	align-items: center;
	gap: 0.2rem;
}

.role-icon-inline {
	width: 0.75rem;
	height: 0.75rem;
}

.player-latency {
	font-size: 0.75rem;
	font-weight: 600;
	font-family: 'Fira Code', 'Cascadia Code', monospace;
}

.latency-good {
	color: var(--color-green);
}

.latency-fair {
	color: var(--color-orange);
}

.latency-poor {
	color: var(--color-red);
}

/* ── Info Grid ───────────────────────────────── */
.info-grid {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
	gap: 0.5rem;
}

.info-item {
	display: flex;
	flex-direction: column;
	gap: 0.15rem;
	padding: 0.5rem 0.65rem;
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
}

.info-label {
	font-size: 0.65rem;
	font-weight: 600;
	text-transform: uppercase;
	letter-spacing: 0.05em;
	color: var(--color-text-secondary);
}

.info-value {
	font-size: 0.9rem;
	font-weight: 600;
	color: var(--color-text);
}

.info-value--green {
	color: var(--color-green);
}

.info-value--orange {
	color: var(--color-orange);
}

.info-value--red {
	color: var(--color-red);
}

/* ── Mod Compatibility ───────────────────────── */
.mod-compat-result {
	display: flex;
	flex-direction: column;
	gap: 0.65rem;
}

.compat-status {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	font-weight: 600;
	font-size: 0.9rem;
	padding: 0.5rem 0.65rem;
	border-radius: var(--radius-sm);
}

.compat-status svg {
	width: 1rem;
	height: 1rem;
}

.compat-ok {
	background: var(--color-brand-bg);
	color: var(--color-green);
}

.compat-warn {
	background: var(--color-orange-bg);
	color: var(--color-orange);
}

.compat-group {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
}

.compat-group-title {
	font-size: 0.75rem;
	font-weight: 600;
	color: var(--color-text-secondary);
}

.compat-mod-item {
	display: flex;
	justify-content: space-between;
	align-items: center;
	padding: 0.35rem 0.5rem;
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	font-size: 0.8rem;
}

.mod-name {
	color: var(--color-text);
}

.mod-version {
	color: var(--color-text-secondary);
	font-family: 'Fira Code', 'Cascadia Code', monospace;
	font-size: 0.75rem;
}

.mod-version--mismatch {
	color: var(--color-orange);
}

/* ── Leave Button ────────────────────────────── */
.leave-row {
	display: flex;
	justify-content: center;
	padding-top: 0.5rem;
}

/* ── Busy State ──────────────────────────────── */
.busy-card :deep(.card) {
	display: flex;
	flex-direction: column;
	align-items: center;
}

.busy-content {
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 0.75rem;
	padding: 2rem 2rem 0.75rem;
	color: var(--color-text-secondary);
}

.spinner-icon {
	width: 1.25rem;
	height: 1.25rem;
	color: var(--color-brand);
}

.busy-text-stack {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.25rem;
}

.busy-text {
	font-weight: 600;
	font-size: 0.95rem;
	color: var(--color-text);
}

.busy-hint {
	font-size: 0.75rem;
	text-align: center;
	max-width: 24rem;
	line-height: 1.4;
}

.busy-actions {
	padding: 0.75rem 0 1.5rem;
}

/* ── Misc ────────────────────────────────────── */
.empty-hint {
	color: var(--color-text-secondary);
	font-size: 0.8rem;
	margin: 0;
}
</style>
