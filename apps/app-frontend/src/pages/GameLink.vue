<script setup lang="ts">
import {
	CopyIcon,
	GlobeIcon,
	LinkIcon,
	PlayIcon,
	UsersIcon,
	XIcon,
	SpinnerIcon,
	UserIcon,
	SearchIcon,
	RefreshCwIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	Card,
	StyledInput,
	defineMessages,
	injectNotificationManager,
	SkinPreviewRenderer,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import { get_default_user, users } from '@/helpers/auth'
import * as instanceApi from '@/helpers/instance'
import { instance_listener, process_listener } from '@/helpers/events'
import {
	check_easytier_ready,
	create_lobby,
	discover_local_worlds,
	get_lobby_status,
	get_players,
	join_lobby,
	leave_lobby,
	type LobbyStatus,
	type LocalWorld,
	type PlayerProfile,
} from '@/helpers/link'
import { get_available_skins, get_normalized_skin_texture } from '@/helpers/skins'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import type { GameInstance } from '@/helpers/types'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const breadcrumbs = useBreadcrumbs()

const messages = defineMessages({
	title: {
		id: 'app.gamelink.title',
		defaultMessage: 'LAN Multiplayer',
	},
	description: {
		id: 'app.gamelink.description',
		defaultMessage:
			'Play local Minecraft worlds with friends over the internet using EasyTier virtual LAN.',
	},
	yourSkinTitle: {
		id: 'app.gamelink.your-skin.title',
		defaultMessage: 'Your skin',
	},
	yourSkinDescription: {
		id: 'app.gamelink.your-skin.description',
		defaultMessage: 'This is what other players will see when you join a lobby.',
	},
	signInHint: {
		id: 'app.gamelink.sign-in-hint',
		defaultMessage: 'Sign in to display your skin',
	},
	createLobbyTitle: {
		id: 'app.gamelink.create.title',
		defaultMessage: 'Create lobby',
	},
	createLobbyDescription: {
		id: 'app.gamelink.create.description',
		defaultMessage:
			'Launch a Minecraft instance, open it to LAN in-game, then create a lobby and share the code with your friends.',
	},
	joinLobbyTitle: {
		id: 'app.gamelink.join.title',
		defaultMessage: 'Join lobby',
	},
	joinLobbyDescription: {
		id: 'app.gamelink.join.description',
		defaultMessage: 'Paste a lobby code shared by the host and connect to their world.',
	},
	instancesSection: {
		id: 'app.gamelink.instances.section',
		defaultMessage: '1. Pick a Minecraft instance',
	},
	instancesHint: {
		id: 'app.gamelink.instances.hint',
		defaultMessage: 'Choose the instance you want to share with friends.',
	},
	launchInstance: {
		id: 'app.gamelink.launch-instance',
		defaultMessage: 'Launch',
	},
	launchHint: {
		id: 'app.gamelink.launch-hint',
		defaultMessage:
			'After Minecraft opens, click "Open to LAN" in the in-game pause menu to expose the world.',
	},
	noInstances: {
		id: 'app.gamelink.no-instances',
		defaultMessage: 'No instances available. Create one first.',
	},
	selected: {
		id: 'app.gamelink.selected',
		defaultMessage: 'Selected',
	},
	lanDiscoverySection: {
		id: 'app.gamelink.lan-discovery.section',
		defaultMessage: '2. Open Minecraft world to LAN',
	},
	lanDiscoveryHint: {
		id: 'app.gamelink.lan-discovery.hint',
		defaultMessage:
			'Local Minecraft worlds broadcasting on LAN are auto-discovered. Pick one or enter the port manually.',
	},
	scanning: {
		id: 'app.gamelink.scanning',
		defaultMessage: 'Scanning...',
	},
	noLocalWorlds: {
		id: 'app.gamelink.no-local-worlds',
		defaultMessage:
			'No local worlds detected yet. Open "Open to LAN" in Minecraft, or use delayed mode below.',
	},
	mcPortLabel: {
		id: 'app.gamelink.mc-port.label',
		defaultMessage: 'Minecraft LAN port',
	},
	mcPortPlaceholder: {
		id: 'app.gamelink.mc-port.placeholder',
		defaultMessage: 'e.g. 54321',
	},
	delayedMode: {
		id: 'app.gamelink.delayed-mode',
		defaultMessage: 'Delayed mode (create lobby now, detect MC port later)',
	},
	delayedModeHint: {
		id: 'app.gamelink.delayed-mode-hint',
		defaultMessage:
			'Useful if you have not opened Minecraft to LAN yet. The lobby will watch for MC LAN broadcasts and forward automatically.',
	},
	createButton: {
		id: 'app.gamelink.create.button',
		defaultMessage: 'Create lobby',
	},
	lobbyCodeLabel: {
		id: 'app.gamelink.lobby-code.label',
		defaultMessage: 'Lobby code',
	},
	lobbyCodePlaceholder: {
		id: 'app.gamelink.lobby-code.placeholder',
		defaultMessage: 'Paste lobby code here',
	},
	joinButton: {
		id: 'app.gamelink.join.button',
		defaultMessage: 'Join lobby',
	},
	copyCodeButton: {
		id: 'app.gamelink.copy-code.button',
		defaultMessage: 'Copy code',
	},
	leaveButton: {
		id: 'app.gamelink.leave.button',
		defaultMessage: 'Leave lobby',
	},
	statusIdle: {
		id: 'app.gamelink.status.idle',
		defaultMessage: 'Not connected',
	},
	statusHost: {
		id: 'app.gamelink.status.host',
		defaultMessage: 'Hosting lobby',
	},
	statusClient: {
		id: 'app.gamelink.status.client',
		defaultMessage: 'Connected to lobby',
	},
	statusError: {
		id: 'app.gamelink.status.error',
		defaultMessage: 'Connection error',
	},
	peersLabel: {
		id: 'app.gamelink.peers.label',
		defaultMessage: 'Peers',
	},
	localPortLabel: {
		id: 'app.gamelink.local-port.label',
		defaultMessage: 'Local proxy port',
	},
	virtualIpLabel: {
		id: 'app.gamelink.virtual-ip.label',
		defaultMessage: 'Virtual IP',
	},
	mcPortStatusLabel: {
		id: 'app.gamelink.mc-port-status.label',
		defaultMessage: 'MC port',
	},
	mcPortWaiting: {
		id: 'app.gamelink.mc-port-waiting',
		defaultMessage: 'Waiting for MC LAN...',
	},
	connectHint: {
		id: 'app.gamelink.connect-hint',
		defaultMessage: 'In Minecraft, use Direct Connect and enter {address}',
	},
	copiedNotification: {
		id: 'app.gamelink.copied.notification',
		defaultMessage: 'Lobby code copied to clipboard',
	},
	instanceLaunchedHint: {
		id: 'app.gamelink.instance-launched-hint',
		defaultMessage: 'Instance launched. Remember to open it to LAN in-game.',
	},
	invalidPortError: {
		id: 'app.gamelink.invalid-port-error',
		defaultMessage: 'Please enter a valid Minecraft LAN port (1-65535).',
	},
	portRequiredError: {
		id: 'app.gamelink.port-required-error',
		defaultMessage: 'Please pick a local world, enter a port, or enable delayed mode.',
	},
	codeRequiredError: {
		id: 'app.gamelink.code-required-error',
		defaultMessage: 'Please enter a lobby code.',
	},
	windowsOnlyWarning: {
		id: 'app.gamelink.windows-only',
		defaultMessage: 'LAN Multiplayer is currently only available on Windows.',
	},
	transitionCreating: {
		id: 'app.gamelink.transition.creating',
		defaultMessage: 'Creating lobby...',
	},
	transitionJoining: {
		id: 'app.gamelink.transition.joining',
		defaultMessage: 'Joining lobby...',
	},
	transitionComplete: {
		id: 'app.gamelink.transition.complete',
		defaultMessage: 'Connected!',
	},
	playersInLobby: {
		id: 'app.gamelink.players-in-lobby',
		defaultMessage: 'Players in lobby',
	},
	youLabel: {
		id: 'app.gamelink.you-label',
		defaultMessage: 'You',
	},
	hostLabel: {
		id: 'app.gamelink.host-label',
		defaultMessage: 'Host',
	},
	guestLabel: {
		id: 'app.gamelink.guest-label',
		defaultMessage: 'Guest',
	},
	waitingForPeers: {
		id: 'app.gamelink.waiting-for-peers',
		defaultMessage: 'Waiting for other players to join...',
	},
	// 5-stage transition labels
	stagePreparing: {
		id: 'app.gamelink.stage.preparing',
		defaultMessage: 'Preparing EasyTier VPN...',
	},
	stageDownloading: {
		id: 'app.gamelink.stage.downloading',
		defaultMessage: 'Downloading EasyTier core...',
	},
	stageConnecting: {
		id: 'app.gamelink.stage.connecting',
		defaultMessage: 'Establishing virtual LAN network...',
	},
	stageReady: {
		id: 'app.gamelink.stage.ready',
		defaultMessage: 'Negotiating Scaffolding protocol...',
	},
	stageComplete: {
		id: 'app.gamelink.stage.complete',
		defaultMessage: 'All set! Welcome to the lobby.',
	},
})

breadcrumbs.setRootContext({ name: formatMessage(messages.title), link: '/gamelink' })

const isWindows = computed(
	() => typeof navigator !== 'undefined' && navigator.userAgent.includes('Windows'),
)

// ===== Lobby state =====
const status = ref<LobbyStatus | null>(null)
const players = ref<PlayerProfile[]>([])
const localWorlds = ref<LocalWorld[]>([])
const instances = ref<GameInstance[]>([])
const loading = ref(false)
const initializing = ref(true)
const scanningLocal = ref(false)

// ===== Form inputs =====
const selectedInstanceId = ref<string | null>(null)
const selectedLocalPort = ref<number | null>(null)
const manualPortInput = ref('')
const useDelayedMode = ref(false)
const lobbyCodeInput = ref('')

// ===== 5-stage transition animation =====
const showTransition = ref(false)
const transitionPhase = ref<'preparing' | 'downloading' | 'connecting' | 'ready' | 'complete'>(
	'preparing',
)
const transitionMode = ref<'create' | 'join'>('create')
const transitionProgress = ref(0)

// ===== Skin preview =====
const skinTexture = ref('')
const skinVariant = ref<'SLIM' | 'CLASSIC' | 'UNKNOWN'>('CLASSIC')
const username = ref('')
const skinLoaded = ref(false)

// ===== Polling handles =====
let statusPoll: number | null = null
let localWorldsPoll: number | null = null
let playersPoll: number | null = null
let progressTimer: number | null = null
let unlistenProcess: (() => void) | null = null
let unlistenInstance: (() => void) | null = null

async function loadUserSkin() {
	try {
		const defaultId = await get_default_user()
		if (defaultId) {
			const allAccounts = await users()
			const user = allAccounts.find((acc) => acc.profile.id === defaultId)
			if (user) {
				username.value = user.profile.name ?? ''
			}
		}

		const skins = (await get_available_skins()) ?? []
		const equipped = skins.find((s) => s.is_equipped) ?? skins[0]
		if (equipped) {
			skinTexture.value = await get_normalized_skin_texture(equipped)
			skinVariant.value = equipped.variant
			skinLoaded.value = true
		}
	} catch (e) {
		console.warn('[GameLink] Failed to load user skin:', e)
		skinLoaded.value = false
	}
}

async function loadInstances() {
	try {
		instances.value = await instanceApi.list()
		// default-select the most recently played instance
		if (!selectedInstanceId.value && instances.value.length > 0) {
			const sorted = [...instances.value].sort(
				(a, b) =>
					new Date(b.last_played ?? 0).getTime() - new Date(a.last_played ?? 0).getTime(),
			)
			selectedInstanceId.value = sorted[0].id
		}
	} catch (e) {
		console.warn('[GameLink] Failed to load instances:', e)
		instances.value = []
	}
}

async function refreshStatus() {
	try {
		status.value = await get_lobby_status()
		if (status.value?.state === 'host' || status.value?.state === 'client') {
			try {
				players.value = await get_players()
			} catch {
				players.value = []
			}
		} else {
			players.value = []
		}
	} catch (e) {
		console.error('[GameLink] Failed to get lobby status:', e)
	} finally {
		initializing.value = false
	}
}

async function refreshLocalWorlds() {
	if (status.value?.state === 'client') return
	scanningLocal.value = true
	try {
		localWorlds.value = await discover_local_worlds()
	} catch (e) {
		console.warn('[GameLink] Failed to discover local worlds:', e)
		localWorlds.value = []
	} finally {
		scanningLocal.value = false
	}
}

async function launchInstance() {
	if (!selectedInstanceId.value) return
	try {
		await instanceApi.run(selectedInstanceId.value)
		addNotification({
			title: formatMessage(messages.title),
			description: formatMessage(messages.instanceLaunchedHint),
			type: 'success',
		})
		// Give MC a moment to boot, then start scanning for LAN broadcasts
		setTimeout(() => {
			refreshLocalWorlds()
		}, 4000)
	} catch (e) {
		handleError(e as Error)
	}
}

async function handleCreate() {
	let port = 0
	if (!useDelayedMode.value) {
		if (selectedLocalPort.value && selectedLocalPort.value > 0) {
			port = selectedLocalPort.value
		} else if (manualPortInput.value) {
			port = parseInt(manualPortInput.value, 10)
			if (Number.isNaN(port) || port <= 0 || port > 65535) {
				addNotification({
					title: formatMessage(messages.title),
					description: formatMessage(messages.invalidPortError),
					type: 'error',
				})
				return
			}
		} else {
			addNotification({
				title: formatMessage(messages.title),
				description: formatMessage(messages.portRequiredError),
				type: 'error',
			})
			return
		}
	}

	loading.value = true
	transitionMode.value = 'create'
	transitionPhase.value = 'preparing'
	transitionProgress.value = 5
	showTransition.value = true
	startProgressTimer()

	try {
		// Stage 1: preparing
		await stageDelay(900)
		// Stage 2: downloading (if needed)
		transitionPhase.value = 'downloading'
		transitionProgress.value = 25
		await check_easytier_ready()
		// Stage 3: connecting - start EasyTier
		transitionPhase.value = 'connecting'
		transitionProgress.value = 50
		const code = await create_lobby(port, username.value || 'Player')
		console.log('[GameLink] Lobby created with code:', code)
		// Stage 4: ready - Scaffolding server up, waiting for players / MC LAN
		transitionPhase.value = 'ready'
		transitionProgress.value = 80
		await refreshStatus()
		await stageDelay(1200)
		// Stage 5: complete
		transitionPhase.value = 'complete'
		transitionProgress.value = 100
		stopProgressTimer()
		await stageDelay(900)
	} catch (e) {
		console.error('[GameLink] Failed to create lobby:', e)
		handleError(e as Error)
	} finally {
		showTransition.value = false
		loading.value = false
	}
}

async function handleJoin() {
	const code = lobbyCodeInput.value.trim()
	if (!code) {
		addNotification({
			title: formatMessage(messages.title),
			description: formatMessage(messages.codeRequiredError),
			type: 'error',
		})
		return
	}

	loading.value = true
	transitionMode.value = 'join'
	transitionPhase.value = 'preparing'
	transitionProgress.value = 5
	showTransition.value = true
	startProgressTimer()

	try {
		// Stage 1: preparing
		await stageDelay(700)
		// Stage 2: downloading (if needed)
		transitionPhase.value = 'downloading'
		transitionProgress.value = 25
		await check_easytier_ready()
		// Stage 3: connecting - start EasyTier + join network
		transitionPhase.value = 'connecting'
		transitionProgress.value = 50
		await join_lobby(code, username.value || 'Player')
		// Stage 4: ready - wait for Scaffolding handshake + port-forward
		transitionPhase.value = 'ready'
		transitionProgress.value = 80
		let waited = 0
		while (waited < 10000) {
			await refreshStatus()
			if (status.value?.scaffoldingReady && status.value.localPort) break
			await stageDelay(500)
			waited += 500
		}
		// Stage 5: complete
		transitionPhase.value = 'complete'
		transitionProgress.value = 100
		stopProgressTimer()
		await stageDelay(900)
	} catch (e) {
		console.error('[GameLink] Failed to join lobby:', e)
		handleError(e as Error)
	} finally {
		showTransition.value = false
		loading.value = false
	}
}

function startProgressTimer() {
	stopProgressTimer()
	progressTimer = window.setInterval(() => {
		// Smoothly drift toward a cap depending on the stage
		const caps: Record<typeof transitionPhase.value, number> = {
			preparing: 22,
			downloading: 48,
			connecting: 75,
			ready: 95,
			complete: 100,
		}
		const cap = caps[transitionPhase.value] ?? 95
		if (transitionProgress.value < cap) {
			transitionProgress.value = Math.min(cap, transitionProgress.value + 0.6)
		}
	}, 60)
}

function stopProgressTimer() {
	if (progressTimer) {
		clearInterval(progressTimer)
		progressTimer = null
	}
}

function stageDelay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

async function handleLeave() {
	loading.value = true
	console.log('[GameLink] Leaving lobby...')
	try {
		await leave_lobby()
		console.log('[GameLink] Left lobby')
		await refreshStatus()
		players.value = []
	} catch (e) {
		console.error('[GameLink] Failed to leave lobby:', e)
		handleError(e as Error)
	} finally {
		loading.value = false
	}
}

async function copyCode() {
	if (!status.value?.lobbyCode) return
	try {
		await navigator.clipboard.writeText(status.value.lobbyCode)
		addNotification({
			title: formatMessage(messages.title),
			description: formatMessage(messages.copiedNotification),
			type: 'success',
		})
	} catch (e) {
		handleError(e as Error)
	}
}

function selectLocalWorld(world: LocalWorld) {
	selectedLocalPort.value = world.port
	manualPortInput.value = String(world.port)
	useDelayedMode.value = false
}

function clearLocalWorldSelection() {
	// When the user manually edits the port input, deselect any picked local world.
	// Do NOT clear manualPortInput here — that would wipe the user's keystrokes.
	selectedLocalPort.value = null
}

function selectInstance(inst: GameInstance) {
	selectedInstanceId.value = inst.id
}

const selectedInstance = computed(() =>
	instances.value.find((i) => i.id === selectedInstanceId.value) ?? null,
)

onMounted(async () => {
	await loadUserSkin()
	await loadInstances()
	await refreshStatus()
	await refreshLocalWorlds()

	statusPoll = window.setInterval(refreshStatus, 3000)
	localWorldsPoll = window.setInterval(refreshLocalWorlds, 3000)
	playersPoll = window.setInterval(async () => {
		if (status.value?.state === 'host' || status.value?.state === 'client') {
			try {
				players.value = await get_players()
			} catch {
				/* ignore */
			}
		}
	}, 5000)

	unlistenProcess = await process_listener(async () => {
		// Process events could be used to detect MC launch / exit
	})
	unlistenInstance = await instance_listener(async () => {
		await loadInstances()
	})
})

onUnmounted(() => {
	if (statusPoll) clearInterval(statusPoll)
	if (localWorldsPoll) clearInterval(localWorldsPoll)
	if (playersPoll) clearInterval(playersPoll)
	if (progressTimer) clearInterval(progressTimer)
	unlistenProcess?.()
	unlistenInstance?.()
})

// ====== Computed ======

const isInLobby = computed(() =>
	status.value ? status.value.state === 'host' || status.value.state === 'client' : false,
)

const connectAddress = computed(() => {
	if (status.value?.localPort) return `localhost:${status.value.localPort}`
	return ''
})

const transitionStages = computed(() => [
	{ key: 'preparing', label: formatMessage(messages.stagePreparing) },
	{ key: 'downloading', label: formatMessage(messages.stageDownloading) },
	{ key: 'connecting', label: formatMessage(messages.stageConnecting) },
	{ key: 'ready', label: formatMessage(messages.stageReady) },
	{ key: 'complete', label: formatMessage(messages.stageComplete) },
] as const)

const transitionTitle = computed(() => {
	if (transitionPhase.value === 'complete') {
		return formatMessage(messages.transitionComplete)
	}
	return transitionMode.value === 'create'
		? formatMessage(messages.transitionCreating)
		: formatMessage(messages.transitionJoining)
})

const transitionSubtitle = computed(() => {
	const stage = transitionStages.value.find((s) => s.key === transitionPhase.value)
	return stage?.label ?? ''
})

const transitionStageIndex = computed(() =>
	transitionStages.value.findIndex((s) => s.key === transitionPhase.value),
)

const statusBadgeText = computed(() => {
	if (!status.value) return formatMessage(messages.statusIdle)
	switch (status.value.state) {
		case 'host':
			return formatMessage(messages.statusHost)
		case 'client':
			return formatMessage(messages.statusClient)
		case 'error':
			return formatMessage(messages.statusError)
		default:
			return formatMessage(messages.statusIdle)
	}
})

const statusBadgeColor = computed(() => {
	if (!status.value) return 'bg-surface-5'
	switch (status.value.state) {
		case 'host':
		case 'client':
			return 'bg-green'
		case 'error':
			return 'bg-red'
		default:
			return 'bg-surface-5'
	}
})

const sortedPlayers = computed(() => {
	// Host first, then guests
	return [...players.value].sort((a, b) => {
		if (a.kind === 'HOST' && b.kind !== 'HOST') return -1
		if (a.kind !== 'HOST' && b.kind === 'HOST') return 1
		return a.name.localeCompare(b.name)
	})
})
</script>

<template>
	<div class="flex flex-col h-full">
		<!-- Main content -->
		<div class="p-6 flex flex-col gap-4 flex-1 overflow-y-auto">
			<div class="flex items-center justify-between flex-wrap gap-3">
				<div>
					<h1 class="text-2xl font-bold text-contrast m-0">
						{{ formatMessage(messages.title) }}
					</h1>
					<p class="text-secondary m-0 mt-1">{{ formatMessage(messages.description) }}</p>
				</div>
				<div
					v-if="!initializing && status"
					class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-surface-3"
				>
					<span class="size-2 rounded-full" :class="statusBadgeColor" />
					<span class="text-sm font-medium text-contrast">{{ statusBadgeText }}</span>
				</div>
			</div>

			<div v-if="initializing" class="p-4 rounded-xl bg-surface-3 text-secondary">Loading...</div>

			<template v-else>
				<div v-if="!isWindows" class="p-4 rounded-xl bg-bg-orange text-orange">
					{{ formatMessage(messages.windowsOnlyWarning) }}
				</div>

				<div class="grid grid-cols-1 lg:grid-cols-[1fr_1.3fr] gap-4">
					<!-- ===== Left column: 3D skin preview (always visible) + player list when in lobby ===== -->
					<div class="flex flex-col gap-4">
						<Card class="flex flex-col">
							<div class="flex items-center gap-3 mb-3">
								<div class="p-2 rounded-xl bg-brand-highlight">
									<UserIcon class="size-5 text-brand" />
								</div>
								<div>
									<h2 class="text-lg font-bold text-contrast m-0">
										{{ formatMessage(messages.yourSkinTitle) }}
									</h2>
									<p class="text-sm text-secondary m-0">
										{{ formatMessage(messages.yourSkinDescription) }}
									</p>
								</div>
							</div>

							<div
								v-if="skinLoaded && skinTexture"
								class="relative flex-1 min-h-[360px] rounded-xl overflow-hidden bg-surface-2"
							>
								<SkinPreviewRenderer
									:texture-src="skinTexture"
									:variant="skinVariant"
									:nametag="username || undefined"
									:initial-rotation="Math.PI / 8"
								/>
							</div>
							<div
								v-else
								class="flex-1 min-h-[360px] rounded-xl bg-surface-2 flex items-center justify-center"
							>
								<div class="text-center">
									<UserIcon class="size-16 mx-auto text-secondary opacity-50 mb-3" />
									<p class="text-secondary m-0">
										{{ formatMessage(messages.signInHint) }}
									</p>
								</div>
							</div>
						</Card>

						<!-- Player list card (only visible when in lobby) -->
						<Card v-if="isInLobby">
							<div class="flex items-center gap-3 mb-3">
								<div class="p-2 rounded-xl bg-brand-highlight">
									<UsersIcon class="size-5 text-brand" />
								</div>
								<div class="flex-1">
									<h2 class="text-lg font-bold text-contrast m-0">
										{{ formatMessage(messages.playersInLobby) }}
									</h2>
									<p class="text-sm text-secondary m-0">
										{{ status?.playerCount ?? 0 }} player(s) connected
									</p>
								</div>
							</div>
							<div class="flex flex-col gap-2">
								<div
									v-for="p in sortedPlayers"
									:key="p.machine_id || p.name"
									class="flex items-center gap-3 p-3 rounded-lg bg-surface-2"
								>
									<div class="size-9 rounded-full bg-brand-highlight flex items-center justify-center shrink-0">
										<UserIcon class="size-5 text-brand" />
									</div>
									<div class="flex-1 min-w-0">
										<div class="text-sm font-medium text-contrast truncate">
											{{ p.name }}
											<span
												v-if="p.name === username"
												class="text-xs text-secondary ml-1"
											>
												({{ formatMessage(messages.youLabel) }})
											</span>
										</div>
										<div class="text-xs text-secondary truncate">
											{{ p.vendor || 'Unknown client' }}
										</div>
									</div>
									<span
										v-if="p.kind === 'HOST'"
										class="px-2 py-0.5 rounded-md text-xs font-semibold border border-solid border-brand bg-brand-highlight text-contrast"
									>
										{{ formatMessage(messages.hostLabel) }}
									</span>
									<span
										v-else
										class="px-2 py-0.5 rounded-md text-xs font-semibold bg-surface-3 text-secondary"
									>
										{{ formatMessage(messages.guestLabel) }}
									</span>
								</div>
								<div
									v-if="sortedPlayers.length === 0"
									class="p-4 rounded-lg bg-surface-2 text-center text-sm text-secondary"
								>
									{{ formatMessage(messages.waitingForPeers) }}
								</div>
							</div>
						</Card>
					</div>

					<!-- ===== Right column: lobby controls ===== -->
					<div class="flex flex-col gap-4">
						<!-- ===== HOST VIEW ===== -->
						<template v-if="status?.state === 'host'">
							<Card>
								<div class="flex items-center gap-3 mb-4">
									<div class="p-2 rounded-xl bg-brand-highlight">
										<GlobeIcon class="size-5 text-brand" />
									</div>
									<div>
										<h2 class="text-lg font-bold text-contrast m-0">
											{{ formatMessage(messages.createLobbyTitle) }}
										</h2>
										<p class="text-sm text-secondary m-0">
											{{ formatMessage(messages.createLobbyDescription) }}
										</p>
									</div>
								</div>

								<div class="space-y-4">
									<!-- Lobby code + copy -->
									<div>
										<label class="text-sm font-medium text-primary block mb-1.5">
											{{ formatMessage(messages.lobbyCodeLabel) }}
										</label>
										<div class="flex gap-2">
											<StyledInput
												:model-value="status.lobbyCode ?? ''"
												:disabled="true"
												class="flex-1"
											/>
											<ButtonStyled type="brand">
												<button :disabled="loading" @click="copyCode">
													<CopyIcon />
													{{ formatMessage(messages.copyCodeButton) }}
												</button>
											</ButtonStyled>
										</div>
									</div>

									<!-- Status info grid -->
									<div class="grid grid-cols-2 gap-3 text-sm">
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.virtualIpLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.virtualIp ?? '-' }}
											</div>
										</div>
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.peersLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.peerCount }}
											</div>
										</div>
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.mcPortStatusLabel) }}
											</div>
											<div class="text-contrast font-mono">
												<template v-if="status.mcPort && status.mcPort > 0">
													{{ status.mcPort }}
												</template>
												<template v-else>
													<span class="text-orange">
														{{ formatMessage(messages.mcPortWaiting) }}
													</span>
												</template>
											</div>
										</div>
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.localPortLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.localPort ?? status.scfPort ?? '-' }}
											</div>
										</div>
									</div>

									<div
										v-if="!status.mcPort || status.mcPort === 0"
										class="p-3 rounded-lg bg-bg-orange text-orange text-sm"
									>
										{{ formatMessage(messages.delayedModeHint) }}
									</div>

									<ButtonStyled type="red">
										<button :disabled="loading" class="w-full" @click="handleLeave">
											<XIcon />
											{{ formatMessage(messages.leaveButton) }}
										</button>
									</ButtonStyled>
								</div>
							</Card>
						</template>

						<!-- ===== CLIENT VIEW ===== -->
						<template v-else-if="status?.state === 'client'">
							<Card>
								<div class="flex items-center gap-3 mb-4">
									<div class="p-2 rounded-xl bg-brand-highlight">
										<UsersIcon class="size-5 text-brand" />
									</div>
									<div>
										<h2 class="text-lg font-bold text-contrast m-0">
											{{ formatMessage(messages.joinLobbyTitle) }}
										</h2>
										<p class="text-sm text-secondary m-0">
											{{ formatMessage(messages.joinLobbyDescription) }}
										</p>
									</div>
								</div>

								<div class="space-y-4">
									<div class="grid grid-cols-2 gap-3 text-sm">
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.localPortLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.localPort ?? '-' }}
											</div>
										</div>
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.peersLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.peerCount }}
											</div>
										</div>
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.virtualIpLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.virtualIp ?? '-' }}
											</div>
										</div>
										<div class="p-3 rounded-lg bg-surface-2">
											<div class="text-secondary text-xs mb-1">
												{{ formatMessage(messages.mcPortStatusLabel) }}
											</div>
											<div class="text-contrast font-mono">
												{{ status.mcPort ?? '-' }}
											</div>
										</div>
									</div>

									<div
										v-if="connectAddress"
										class="p-3 rounded-lg bg-bg-blue text-blue text-sm"
									>
										{{
											formatMessage(messages.connectHint, {
												address: connectAddress,
											})
										}}
									</div>

									<div
										v-if="!status.scaffoldingReady"
										class="p-3 rounded-lg bg-bg-orange text-orange text-sm flex items-center gap-2"
									>
										<SpinnerIcon class="size-4 animate-spin" />
										{{ formatMessage(messages.waitingForPeers) }}
									</div>

									<ButtonStyled type="red">
										<button :disabled="loading" class="w-full" @click="handleLeave">
											<XIcon />
											{{ formatMessage(messages.leaveButton) }}
										</button>
									</ButtonStyled>
								</div>
							</Card>
						</template>

						<!-- ===== IDLE VIEW: Create + Join panels ===== -->
						<template v-else>
							<!-- Create lobby: full host flow -->
							<Card>
								<div class="flex items-center gap-3 mb-4">
									<div class="p-2 rounded-xl bg-brand-highlight">
										<GlobeIcon class="size-5 text-brand" />
									</div>
									<div>
										<h2 class="text-lg font-bold text-contrast m-0">
											{{ formatMessage(messages.createLobbyTitle) }}
										</h2>
										<p class="text-sm text-secondary m-0">
											{{ formatMessage(messages.createLobbyDescription) }}
										</p>
									</div>
								</div>

								<!-- Step 1: instance selection -->
								<div class="space-y-3 mb-4">
									<div class="flex items-center justify-between">
										<label class="text-sm font-medium text-primary">
											{{ formatMessage(messages.instancesSection) }}
										</label>
									</div>
									<p class="text-xs text-secondary m-0">
										{{ formatMessage(messages.instancesHint) }}
									</p>

									<div
										v-if="instances.length === 0"
										class="p-4 rounded-lg bg-surface-2 text-center text-sm text-secondary"
									>
										{{ formatMessage(messages.noInstances) }}
									</div>
									<div v-else class="flex flex-col gap-1.5 max-h-56 overflow-y-auto pr-1">
										<button
											v-for="inst in instances"
											:key="inst.id"
											type="button"
											class="flex items-center gap-3 p-2.5 rounded-lg text-left transition-colors"
											:class="
												selectedInstanceId === inst.id
													? 'border border-solid border-brand bg-brand-highlight text-contrast'
													: 'bg-surface-2 hover:bg-surface-3'
											"
											:disabled="loading || !isWindows"
											@click="selectInstance(inst)"
										>
											<Avatar
													size="36px"
													:src="inst.icon_path ? convertFileSrc(inst.icon_path) : null"
													:tint-by="inst.id"
													:alt="inst.name"
													class="shrink-0"
												/>
											<div class="flex-1 min-w-0">
												<div class="text-sm font-medium truncate">{{ inst.name }}</div>
												<div class="text-xs opacity-70 truncate">
													{{ inst.game_version }} · {{ inst.loader }}
												</div>
											</div>
											<span
												v-if="selectedInstanceId === inst.id"
												class="text-xs font-semibold px-2 py-0.5 rounded-md bg-brand text-[rgba(0,0,0,0.9)]"
											>
												{{ formatMessage(messages.selected) }}
											</span>
										</button>
									</div>

									<ButtonStyled
										v-if="selectedInstance"
										type="brand"
										class="w-full"
									>
										<button
											type="button"
											:disabled="loading || !isWindows"
											class="w-full"
											@click="launchInstance"
										>
											<PlayIcon class="translate-x-[1px]" />
											{{ formatMessage(messages.launchInstance) }}
										</button>
									</ButtonStyled>
									<p class="text-xs text-secondary m-0">
										{{ formatMessage(messages.launchHint) }}
									</p>
								</div>

								<!-- Step 2: LAN port discovery -->
								<div class="space-y-3 mb-4">
									<div class="flex items-center justify-between">
										<label class="text-sm font-medium text-primary">
											{{ formatMessage(messages.lanDiscoverySection) }}
										</label>
										<ButtonStyled type="transparent" circular>
											<button
												type="button"
												:disabled="scanningLocal || loading"
												@click="refreshLocalWorlds"
											>
												<RefreshCwIcon :class="scanningLocal ? 'animate-spin' : ''" />
											</button>
										</ButtonStyled>
									</div>
									<p class="text-xs text-secondary m-0">
										{{ formatMessage(messages.lanDiscoveryHint) }}
									</p>

									<div
										v-if="scanningLocal && localWorlds.length === 0"
										class="p-3 rounded-lg bg-surface-2 text-center text-sm text-secondary flex items-center justify-center gap-2"
									>
										<SpinnerIcon class="size-4 animate-spin" />
										{{ formatMessage(messages.scanning) }}
									</div>
									<div
										v-else-if="localWorlds.length === 0"
										class="p-3 rounded-lg bg-surface-2 text-center text-sm text-secondary"
									>
										{{ formatMessage(messages.noLocalWorlds) }}
									</div>
									<div v-else class="flex flex-col gap-1.5">
										<button
											v-for="(world, idx) in localWorlds"
											:key="`${world.port}-${idx}`"
											type="button"
											class="flex items-center gap-3 p-2.5 rounded-lg text-left transition-colors"
											:class="
												selectedLocalPort === world.port
													? 'border border-solid border-brand bg-brand-highlight text-contrast'
													: 'bg-surface-2 hover:bg-surface-3'
											"
											:disabled="loading || !isWindows"
											@click="selectLocalWorld(world)"
										>
											<div
												class="size-9 rounded-md bg-surface-3 flex items-center justify-center shrink-0"
											>
												<SearchIcon class="size-5 text-brand" />
											</div>
											<div class="flex-1 min-w-0">
												<div class="text-sm font-medium truncate">{{ world.motd }}</div>
												<div class="text-xs opacity-70">Port {{ world.port }}</div>
											</div>
										</button>
									</div>

									<!-- Manual port input -->
									<div>
										<label class="text-xs text-secondary block mb-1">
											{{ formatMessage(messages.mcPortLabel) }}
										</label>
										<StyledInput
											v-model="manualPortInput"
											type="number"
											:placeholder="formatMessage(messages.mcPortPlaceholder)"
											:disabled="loading || !isWindows || useDelayedMode"
											@input="clearLocalWorldSelection"
										/>
									</div>

									<!-- Delayed mode toggle -->
									<label
										class="flex items-start gap-2 p-3 rounded-lg bg-surface-2 cursor-pointer hover:bg-surface-3 transition-colors"
									>
										<input
											v-model="useDelayedMode"
											type="checkbox"
											class="mt-0.5"
											:disabled="loading || !isWindows"
										/>
										<div class="flex-1">
											<div class="text-sm font-medium text-contrast">
												{{ formatMessage(messages.delayedMode) }}
											</div>
											<div class="text-xs text-secondary mt-0.5">
												{{ formatMessage(messages.delayedModeHint) }}
											</div>
										</div>
									</label>
								</div>

								<form @submit.prevent="handleCreate">
									<ButtonStyled type="brand">
										<button
											type="submit"
											:disabled="loading || !isWindows"
											class="w-full"
										>
											<LinkIcon />
											{{ formatMessage(messages.createButton) }}
										</button>
									</ButtonStyled>
								</form>
							</Card>

							<!-- Join lobby -->
							<Card>
								<div class="flex items-center gap-3 mb-4">
									<div class="p-2 rounded-xl bg-brand-highlight">
										<UsersIcon class="size-5 text-brand" />
									</div>
									<div>
										<h2 class="text-lg font-bold text-contrast m-0">
											{{ formatMessage(messages.joinLobbyTitle) }}
										</h2>
										<p class="text-sm text-secondary m-0">
											{{ formatMessage(messages.joinLobbyDescription) }}
										</p>
									</div>
								</div>

								<form class="space-y-3" @submit.prevent="handleJoin">
									<div>
										<label
											class="text-sm font-medium text-primary block mb-1.5"
										>
											{{ formatMessage(messages.lobbyCodeLabel) }}
										</label>
										<StyledInput
											v-model="lobbyCodeInput"
											:placeholder="formatMessage(messages.lobbyCodePlaceholder)"
											:disabled="loading || !isWindows"
										/>
									</div>
									<ButtonStyled type="brand">
										<button
											type="submit"
											:disabled="loading || !isWindows"
											class="w-full"
										>
											<LinkIcon />
											{{ formatMessage(messages.joinButton) }}
										</button>
									</ButtonStyled>
								</form>
							</Card>
						</template>

						<!-- Error display -->
						<Card v-if="status?.error" class="border border-red">
							<div class="flex items-start gap-2">
								<XIcon class="size-5 text-red shrink-0 mt-0.5" />
								<div>
									<div class="text-sm font-medium text-red">
										{{ formatMessage(messages.statusError) }}
									</div>
									<div class="text-sm text-secondary mt-1">
										{{ status.error }}
									</div>
								</div>
							</div>
						</Card>
					</div>
				</div>
			</template>
		</div>

		<!-- ===== 5-stage transition animation overlay ===== -->
		<Transition name="gamelink-fade">
			<div
				v-if="showTransition"
				class="fixed inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md"
			>
				<div class="flex flex-col items-center gap-6 max-w-2xl w-full px-6">
					<!-- Player skins row: local user center, others fade in -->
					<div class="flex items-end justify-center gap-4 sm:gap-8">
						<!-- Placeholder: guest 2 -->
						<div
							class="hidden sm:flex flex-col items-center gap-2 transition-all duration-700"
							:class="
								transitionStageIndex >= 3
									? 'opacity-80 translate-y-0'
									: 'opacity-20 translate-y-3'
							"
						>
							<div
								class="w-24 h-32 rounded-lg bg-surface-3 flex items-center justify-center overflow-hidden"
							>
								<UserIcon class="size-12 text-secondary" />
							</div>
							<span class="text-xs text-secondary">?</span>
						</div>

						<!-- Local user skin (center, always visible) -->
						<div class="flex flex-col items-center gap-2">
							<div
								v-if="skinLoaded && skinTexture"
								class="w-48 h-64 sm:w-56 sm:h-72 rounded-xl overflow-hidden bg-surface-2 relative transition-transform duration-500"
								:class="transitionPhase === 'complete' ? 'scale-105' : 'scale-100'"
							>
								<SkinPreviewRenderer
									:texture-src="skinTexture"
									:variant="skinVariant"
									:nametag="username || undefined"
									:initial-rotation="Math.PI / 8"
								/>
							</div>
							<div
								v-else
								class="w-48 h-64 sm:w-56 sm:h-72 rounded-xl bg-surface-2 flex items-center justify-center"
							>
								<UserIcon class="size-16 text-secondary opacity-50" />
							</div>
							<div
								class="px-3 py-1 rounded-md text-sm font-medium"
								:class="
									transitionPhase === 'complete'
										? 'bg-bg-green text-green'
										: 'border border-solid border-brand bg-brand-highlight text-contrast'
								"
							>
								{{ username || formatMessage(messages.youLabel) }}
							</div>
						</div>

						<!-- Placeholder: guest 1 -->
						<div
							class="hidden sm:flex flex-col items-center gap-2 transition-all duration-700"
							:class="
								transitionStageIndex >= 2
									? 'opacity-80 translate-y-0'
									: 'opacity-20 translate-y-3'
							"
						>
							<div
								class="w-24 h-32 rounded-lg bg-surface-3 flex items-center justify-center overflow-hidden"
							>
								<UserIcon class="size-12 text-secondary" />
							</div>
							<span class="text-xs text-secondary">?</span>
						</div>
					</div>

					<!-- Stage indicator: 5 dots + labels -->
					<div class="flex items-center justify-center gap-2 w-full max-w-md">
						<template v-for="(stage, idx) in transitionStages" :key="stage.key">
							<div class="flex flex-col items-center gap-1 flex-1">
								<div
									class="size-2.5 rounded-full transition-all duration-300"
									:class="
										idx <= transitionStageIndex
											? 'bg-brand scale-125 shadow-[0_0_8px_var(--color-brand)]'
											: 'bg-surface-3'
									"
								/>
								<span
									class="text-[10px] text-center transition-colors"
									:class="
										idx === transitionStageIndex
											? 'text-brand font-semibold'
											: idx < transitionStageIndex
												? 'text-secondary'
												: 'text-secondary opacity-50'
									"
								>
									{{ stage.label.split('...')[0] }}
								</span>
							</div>
							<div
								v-if="idx < transitionStages.length - 1"
								class="h-px flex-1 -mt-4"
								:class="idx < transitionStageIndex ? 'bg-brand' : 'bg-surface-3'"
							/>
						</template>
					</div>

					<!-- Progress bar -->
					<div class="w-full max-w-md">
						<div class="h-1.5 rounded-full bg-surface-3 overflow-hidden">
							<div
								class="h-full bg-brand transition-all duration-300 ease-out"
								:style="{ width: `${transitionProgress}%` }"
							/>
						</div>
						<div class="text-xs text-secondary text-right mt-1">
							{{ Math.round(transitionProgress) }}%
						</div>
					</div>

					<!-- Status text -->
					<div class="text-center">
						<h2 class="text-xl font-bold text-contrast m-0 mb-2">
							{{ transitionTitle }}
						</h2>
						<p class="text-secondary m-0">{{ transitionSubtitle }}</p>
						<SpinnerIcon
							v-if="transitionPhase !== 'complete'"
							class="size-5 animate-spin text-brand mx-auto mt-3"
						/>
					</div>
				</div>
			</div>
		</Transition>
	</div>
</template>

<style scoped>
.gamelink-fade-enter-active,
.gamelink-fade-leave-active {
	transition: opacity 0.3s ease;
}

.gamelink-fade-enter-from,
.gamelink-fade-leave-to {
	opacity: 0;
}
</style>
