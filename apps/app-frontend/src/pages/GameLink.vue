<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { get_default_user, users } from '@/helpers/auth'

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
  hostLobbyTitle: { id: 'app.game-link.host.title', defaultMessage: 'Host a lobby' },
  hostLobbySubtitle: {
    id: 'app.game-link.host.subtitle',
    defaultMessage: 'Open a world to LAN, then scan and create',
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
    defaultMessage: 'Enter a lobby code to connect',
  },
  lobbyCodeLabel: { id: 'app.game-link.lobby-code.label', defaultMessage: 'Lobby code' },
  joinLobbyButton: { id: 'app.game-link.join.button', defaultMessage: 'Join lobby' },
  joiningLobbyButton: { id: 'app.game-link.join.joining', defaultMessage: 'Joining...' },
  copyTitle: { id: 'app.game-link.copy', defaultMessage: 'Copy' },
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
  mcRunningLabel: { id: 'app.game-link.diagnostics.mc-running', defaultMessage: 'Minecraft running' },
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
  wayLocal: { id: 'app.game-link.way.local', defaultMessage: 'local' },
  wayP2p: { id: 'app.game-link.way.p2p', defaultMessage: 'p2p' },
  wayRelay: { id: 'app.game-link.way.relay', defaultMessage: 'relay' },
  wayUnknown: { id: 'app.game-link.way.unknown', defaultMessage: 'unknown' },
  yes: { id: 'app.game-link.yes', defaultMessage: 'Yes' },
  no: { id: 'app.game-link.no', defaultMessage: 'No' },
})

const username = ref('')
const lobbyCode = ref('')
const joinCodeInput = ref('')
const selectedWorld = ref<FoundWorld | null>(null)
const mcPort = ref(0)

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

// Collapsible section states (default collapsed, except players when connected)
const showPlayers = ref(false)
const showConnection = ref(false)
const showModCompat = ref(false)
const showNetworkDiagnostics = ref(false)
const showWorlds = ref(false)

const isBusy = ref(false)
const isScanning = ref(false)

// Event unlisteners
const unlisteners: UnlistenFn[] = []

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

const isConnected = computed(() => state.value === 'connected')
const hasUsername = computed(() => username.value.trim().length > 0)
const canCreate = computed(() => !isBusy.value && hasUsername.value && selectedWorld.value !== null && state.value === 'idle')
const canJoin = computed(() => !isBusy.value && hasUsername.value && joinCodeInput.value.trim().length > 0 && state.value === 'idle')
const canLeave = computed(() => !isBusy.value && state.value !== 'idle')

const stateLabel = computed(() => {
  switch (state.value) {
    case 'idle': return formatMessage(messages.stateIdle)
    case 'creating': return formatMessage(messages.stateCreating)
    case 'joining': return formatMessage(messages.stateJoining)
    case 'connected': return formatMessage(messages.stateConnected)
    case 'leaving': return formatMessage(messages.stateLeaving)
    case 'error': return formatMessage(messages.stateError)
    default: return state.value
  }
})

const stateColor = computed(() => {
  switch (state.value) {
    case 'connected': return 'var(--color-brand)'
    case 'creating':
    case 'joining': return 'var(--color-orange)'
    case 'error': return 'var(--color-red)'
    default: return 'var(--color-base)'
  }
})

const connectionQualityColor = computed(() => {
  switch (connectionInfo.value.quality) {
    case 'Good': return 'var(--color-brand)'
    case 'Fair': return 'var(--color-orange)'
    case 'Poor': return 'var(--color-red)'
    default: return 'var(--color-base)'
  }
})

function formatConnectionWay(way: ConnectionWay): string {
  switch (way) {
    case 'local': return formatMessage(messages.wayLocal)
    case 'p2p': return formatMessage(messages.wayP2p)
    case 'relay': return formatMessage(messages.wayRelay)
    default: return formatMessage(messages.wayUnknown)
  }
}

function formatConnectionQuality(quality: ConnectionQuality): string {
  switch (quality) {
    case 'Good': return formatMessage(messages.qualityGood)
    case 'Fair': return formatMessage(messages.qualityFair)
    case 'Poor': return formatMessage(messages.qualityPoor)
  }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

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
    showPlayers.value = true
  } catch (e: any) {
    errorMsg.value = String(e)
    state.value = 'error'
  } finally {
    isBusy.value = false
  }
}

async function joinLobby() {
  if (!canJoin.value) return
  isBusy.value = true
  errorMsg.value = ''
  try {
    await invoke<boolean>('plugin:link|link_join_lobby', {
      lobbyCode: joinCodeInput.value.trim(),
      username: username.value,
    })
    lobbyCode.value = joinCodeInput.value.trim()
    showPlayers.value = true
  } catch (e: any) {
    errorMsg.value = String(e)
    state.value = 'error'
  } finally {
    isBusy.value = false
  }
}

function selectWorld(world: FoundWorld) {
  selectedWorld.value = world
  mcPort.value = world.port
}

async function leaveLobby() {
  if (!canLeave.value) return
  isBusy.value = true
  errorMsg.value = ''
  try {
    await invoke('plugin:link|link_leave_lobby')
    resetState()
  } catch (e: any) {
    errorMsg.value = String(e)
  } finally {
    isBusy.value = false
  }
}

async function refreshPlayers() {
  try {
    players.value = await invoke<PlayerProfile[]>('plugin:link|link_get_players')
  } catch (e: any) {
    errorMsg.value = String(e)
  }
}

async function refreshConnectionInfo() {
  try {
    connectionInfo.value = await invoke<ConnectionInfo>('plugin:link|link_get_connection_info')
  } catch (e: any) {
    errorMsg.value = String(e)
  }
}

async function checkModCompat() {
  try {
    // Pass empty instance path — the backend handles host case by returning compatible
    modCompatResult.value = await invoke<ModCompatibilityResult>('plugin:link|link_check_mod_compat', {
      instancePath: '',
    })
  } catch (e: any) {
    errorMsg.value = String(e)
  }
}

async function discoverWorlds() {
  isScanning.value = true
  errorMsg.value = ''
  try {
    foundWorlds.value = await invoke<FoundWorld[]>('plugin:link|link_discover_worlds')
    // Auto-select the first discovered world if none selected.
    if (selectedWorld.value === null && foundWorlds.value.length > 0) {
      selectWorld(foundWorlds.value[0])
    }
  } catch (e: any) {
    errorMsg.value = String(e)
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
  showPlayers.value = false
  showConnection.value = false
  showModCompat.value = false
  showNetworkDiagnostics.value = false
  showWorlds.value = false
}

// ---------------------------------------------------------------------------
// Event listeners
// ---------------------------------------------------------------------------

async function setupListeners() {
  unlisteners.push(
    await listen<LobbyState>('link_state_changed', (event) => {
      state.value = event.payload
      // When entering connected state, sync isHost from backend.
      if (event.payload === 'connected') {
        invoke<boolean>('plugin:link|link_is_host').then((h) => {
          isHost.value = h
        }).catch(() => {})
        invoke<string | null>('plugin:link|link_get_lobby_code').then((code) => {
          if (code) lobbyCode.value = code
        }).catch(() => {})
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
      errorMsg.value = formatMessage(messages.hostShutdownMessage)
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

  // Load current default Minecraft username (same source as the AccountsCard).
  try {
    const defaultId = await get_default_user()
    const accountList = await users()
    const defaultAccount = accountList?.find((acc: any) => acc.profile?.id === defaultId)
    if (defaultAccount?.profile?.name) {
      username.value = defaultAccount.profile.name
    }
  } catch {
    // ignore
  }

  // Sync current state
  try {
    state.value = await invoke<LobbyState>('plugin:link|link_get_state')
    isHost.value = await invoke<boolean>('plugin:link|link_is_host')
    const code = await invoke<string | null>('plugin:link|link_get_lobby_code')
    if (code) lobbyCode.value = code
    if (state.value === 'connected') {
      showPlayers.value = true
      await refreshPlayers()
      await refreshConnectionInfo()
    }
  } catch {
    // link module may not be initialized yet
  }
})

onUnmounted(() => {
  unlisteners.forEach((fn) => fn())
})
</script>

<template>
  <div class="game-link-page">
    <!-- Hero Section -->
    <section class="hero-section">
      <div class="hero-header">
        <h1 class="hero-title">{{ formatMessage(messages.title) }}</h1>
        <div class="state-badge" :style="{ color: stateColor, borderColor: stateColor }">
          {{ stateLabel }}
        </div>
      </div>
      <p class="hero-description">
        {{ formatMessage(messages.description) }}
      </p>

      <!-- Error display -->
      <div v-if="errorMsg" class="error-banner">
        <span>{{ errorMsg }}</span>
        <button class="error-dismiss" @click="errorMsg = ''">x</button>
      </div>

      <!-- Idle: show create/join controls -->
      <div v-if="state === 'idle'" class="hero-controls">
        <div class="user-banner">
          <span class="user-banner-label">{{ formatMessage(messages.playingAs) }}</span>
          <span v-if="hasUsername" class="user-banner-name">{{ username }}</span>
          <span v-else class="user-banner-name user-banner-name--missing">{{ formatMessage(messages.noAccount) }}</span>
        </div>

        <div class="action-row">
          <!-- Create lobby -->
          <div class="action-card action-card--host">
            <div class="action-card-header">
              <span class="action-card-icon">H</span>
              <div>
                <div class="action-card-title">{{ formatMessage(messages.hostLobbyTitle) }}</div>
                <div class="action-card-subtitle">{{ formatMessage(messages.hostLobbySubtitle) }}</div>
              </div>
            </div>

            <button
              class="btn btn-ghost"
              :disabled="isScanning || isBusy"
              @click="discoverWorlds"
            >
              {{ isScanning ? formatMessage(messages.scanningButton) : formatMessage(messages.scanButton) }}
            </button>

            <div v-if="foundWorlds.length > 0" class="world-select">
              <div class="input-label">{{ formatMessage(messages.selectWorld) }}</div>
              <div
                v-for="world in foundWorlds"
                :key="world.port"
                class="world-option"
                :class="selectedWorld?.port === world.port ? 'world-option--selected' : ''"
                @click="selectWorld(world)"
              >
                <span class="world-option-name">{{ world.name }}</span>
                <span class="world-option-port">:{{ world.port }}</span>
              </div>
            </div>
            <p v-else-if="!isScanning" class="empty-hint">
              {{ formatMessage(messages.noWorldsHint) }}
            </p>

            <button class="btn btn-brand" :disabled="!canCreate" @click="createLobby">
              {{ isBusy ? formatMessage(messages.creatingLobbyButton) : formatMessage(messages.createLobbyButton) }}
            </button>
          </div>

          <!-- Join lobby -->
          <div class="action-card action-card--client">
            <div class="action-card-header">
              <span class="action-card-icon action-card-icon--client">C</span>
              <div>
                <div class="action-card-title">{{ formatMessage(messages.joinLobbyTitle) }}</div>
                <div class="action-card-subtitle">{{ formatMessage(messages.joinLobbySubtitle) }}</div>
              </div>
            </div>
            <div class="input-group">
              <label class="input-label">{{ formatMessage(messages.lobbyCodeLabel) }}</label>
              <input
                v-model="joinCodeInput"
                type="text"
                class="text-input"
                placeholder="U/XXXX-XXXX-XXXX-XXXX"
                :disabled="isBusy"
              />
            </div>
            <button class="btn btn-primary" :disabled="!canJoin" @click="joinLobby">
              {{ isBusy ? formatMessage(messages.joiningLobbyButton) : formatMessage(messages.joinLobbyButton) }}
            </button>
          </div>
        </div>
      </div>

      <!-- Connected: show lobby code and leave -->
      <div v-else-if="isConnected" class="hero-connected">
        <div class="lobby-code-display">
          <div class="lobby-code-label">{{ formatMessage(messages.lobbyCodeLabel) }}</div>
          <div class="lobby-code-value">
            <code>{{ lobbyCode }}</code>
            <button class="btn btn-icon" :title="formatMessage(messages.copyTitle)" @click="copyLobbyCode">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
            </button>
          </div>
        </div>
        <div class="role-badge" :class="isHost ? 'role-badge--host' : 'role-badge--client'">
          {{ isHost ? formatMessage(messages.roleHost) : formatMessage(messages.roleClient) }}
        </div>
        <button class="btn btn-danger" :disabled="!canLeave" @click="leaveLobby">
          {{ formatMessage(messages.leaveLobbyButton) }}
        </button>
      </div>

      <!-- Busy states -->
      <div v-else-if="state === 'creating' || state === 'joining' || state === 'leaving'" class="hero-busy">
        <div class="spinner" />
        <span>{{ stateLabel }}</span>
      </div>
    </section>

    <!-- Collapsible: Players -->
    <section v-if="isConnected" class="collapsible-section">
      <button class="section-header" @click="showPlayers = !showPlayers">
        <span class="section-title">{{ formatMessage(messages.playersTitle, { count: players.length }) }}</span>
        <span class="section-chevron" :class="{ 'section-chevron--open': showPlayers }">v</span>
      </button>
      <div v-if="showPlayers" class="section-body">
        <div class="player-list">
          <div
            v-for="player in players"
            :key="player.machine_id"
            class="player-card"
            :class="player.kind === 'host' ? 'player-card--host' : 'player-card--client'"
          >
            <div class="player-avatar">{{ player.name.charAt(0).toUpperCase() }}</div>
            <div class="player-info">
              <div class="player-name">{{ player.name }}</div>
              <div class="player-role">{{ player.kind === 'host' ? formatMessage(messages.roleHost) : formatMessage(messages.roleClient) }}</div>
            </div>
            <div v-if="player.latency_ms !== null" class="player-latency">
              {{ player.latency_ms }}ms
            </div>
          </div>
        </div>
        <button class="btn btn-ghost" @click="refreshPlayers">{{ formatMessage(messages.refreshButton) }}</button>
      </div>
    </section>

    <!-- Collapsible: Connection Info -->
    <section v-if="isConnected" class="collapsible-section">
      <button class="section-header" @click="showConnection = !showConnection">
        <span class="section-title">{{ formatMessage(messages.connectionInfoTitle) }}</span>
        <span class="section-chevron" :class="{ 'section-chevron--open': showConnection }">v</span>
      </button>
      <div v-if="showConnection" class="section-body">
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">{{ formatMessage(messages.latencyLabel) }}</span>
            <span class="info-value" :style="{ color: connectionQualityColor }">
              {{ connectionInfo.latency_ms }} ms
            </span>
          </div>
          <div class="info-item">
            <span class="info-label">{{ formatMessage(messages.qualityLabel) }}</span>
            <span class="info-value" :style="{ color: connectionQualityColor }">
              {{ formatConnectionQuality(connectionInfo.quality) }}
            </span>
          </div>
          <div class="info-item">
            <span class="info-label">{{ formatMessage(messages.connectionTypeLabel) }}</span>
            <span class="info-value">{{ formatConnectionWay(connectionInfo.way) }}</span>
          </div>
        </div>
        <button class="btn btn-ghost" @click="refreshConnectionInfo">{{ formatMessage(messages.refreshButton) }}</button>
      </div>
    </section>

    <!-- Collapsible: Mod compatibility -->
    <section v-if="isConnected && !isHost" class="collapsible-section">
      <button class="section-header" @click="showModCompat = !showModCompat">
        <span class="section-title">{{ formatMessage(messages.modCompatTitle) }}</span>
        <span class="section-chevron" :class="{ 'section-chevron--open': showModCompat }">v</span>
      </button>
      <div v-if="showModCompat" class="section-body">
        <button class="btn btn-primary" @click="checkModCompat">{{ formatMessage(messages.checkCompatButton) }}</button>
        <div v-if="modCompatResult" class="mod-compat-result">
          <div
            class="compat-status"
            :class="modCompatResult.is_compatible ? 'compat-status--ok' : 'compat-status--warn'"
          >
            {{ modCompatResult.is_compatible ? formatMessage(messages.modsCompatible) : formatMessage(messages.modMismatch) }}
          </div>
          <div v-if="modCompatResult.host_only.length > 0" class="compat-group">
            <div class="compat-group-title">{{ formatMessage(messages.missingModsTitle, { count: modCompatResult.host_only.length }) }}</div>
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
            <div class="compat-group-title">{{ formatMessage(messages.extraModsTitle, { count: modCompatResult.local_only.length }) }}</div>
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
            <div class="compat-group-title">{{ formatMessage(messages.versionMismatchesTitle, { count: modCompatResult.version_mismatch.length }) }}</div>
            <div
              v-for="(pair, idx) in modCompatResult.version_mismatch"
              :key="idx"
              class="compat-mod-item"
            >
              <span class="mod-name">{{ pair[0].name || pair[0].mod_id }}</span>
              <span class="mod-version mod-version--mismatch">{{ pair[0].version }} vs {{ pair[1].version }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Collapsible: World discovery -->
    <section v-if="isConnected" class="collapsible-section">
      <button class="section-header" @click="showWorlds = !showWorlds">
        <span class="section-title">{{ formatMessage(messages.localWorldsTitle) }}</span>
        <span class="section-chevron" :class="{ 'section-chevron--open': showWorlds }">v</span>
      </button>
      <div v-if="showWorlds" class="section-body">
        <button class="btn btn-ghost" @click="discoverWorlds">{{ formatMessage(messages.scanWorldsButton) }}</button>
        <div v-if="foundWorlds.length > 0" class="world-list">
          <div v-for="(world, idx) in foundWorlds" :key="idx" class="world-item">
            <span class="world-name">{{ world.name }}</span>
            <span class="world-port">:{{ world.port }}</span>
          </div>
        </div>
        <p v-else class="empty-hint">{{ formatMessage(messages.noWorldsConnectedHint) }}</p>
      </div>
    </section>

    <!-- Collapsible: Network diagnostics -->
    <section v-if="isConnected" class="collapsible-section">
      <button class="section-header" @click="showNetworkDiagnostics = !showNetworkDiagnostics">
        <span class="section-title">{{ formatMessage(messages.networkDiagnosticsTitle) }}</span>
        <span class="section-chevron" :class="{ 'section-chevron--open': showNetworkDiagnostics }">v</span>
      </button>
      <div v-if="showNetworkDiagnostics" class="section-body">
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">{{ formatMessage(messages.mcRunningLabel) }}</span>
            <span class="info-value">{{ mcRunning ? formatMessage(messages.yes) : formatMessage(messages.no) }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">{{ formatMessage(messages.mcPortLabel) }}</span>
            <span class="info-value">{{ mcPort }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">{{ formatMessage(messages.roleLabel) }}</span>
            <span class="info-value">{{ isHost ? formatMessage(messages.roleHost) : formatMessage(messages.roleClient) }}</span>
          </div>
        </div>
        <button class="btn btn-ghost" @click="checkMcRunning">{{ formatMessage(messages.checkMcProcessButton) }}</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.game-link-page {
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

/* Hero section */
.hero-section {
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-md);
  padding: 1.75rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.hero-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.hero-title {
  font-size: 1.75rem;
  font-weight: 700;
  margin: 0;
  color: var(--color-text);
}

.state-badge {
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0.25rem 0.75rem;
  border: 1px solid;
  border-radius: var(--radius-sm);
}

.hero-description {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: 0.95rem;
  line-height: 1.5;
}

/* Error banner */
.error-banner {
  background-color: var(--color-red-bg);
  border: 1px solid var(--color-red);
  border-radius: var(--radius-sm);
  padding: 0.75rem 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  color: var(--color-red);
  font-size: 0.9rem;
}

.error-dismiss {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 1rem;
  padding: 0;
  opacity: 0.7;
}

.error-dismiss:hover {
  opacity: 1;
}

/* Controls */
.hero-controls {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.user-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0;
  font-size: 0.95rem;
}

.user-banner-label {
  color: var(--color-text-secondary);
}

.user-banner-name {
  font-weight: 600;
  color: var(--color-text);
}

.user-banner-name--missing {
  color: var(--color-orange);
}

.world-select {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.world-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.world-option:hover {
  border-color: var(--color-brand);
}

.world-option--selected {
  border-color: var(--color-brand);
  background-color: var(--color-brand-bg);
}

.world-option-name {
  color: var(--color-text);
  font-weight: 500;
}

.world-option-port {
  color: var(--color-text-secondary);
  font-family: monospace;
  font-size: 0.9rem;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.input-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.text-input {
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-sm);
  padding: 0.5rem 0.75rem;
  font-size: 0.95rem;
  color: var(--color-text);
  outline: none;
  transition: border-color 0.15s;
}

.text-input:focus {
  border-color: var(--color-brand);
}

.text-input--small {
  max-width: 120px;
}

.action-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

@media (max-width: 600px) {
  .action-row {
    grid-template-columns: 1fr;
  }
}

.action-card {
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-md);
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.action-card--host {
  border-top: 3px solid var(--color-brand);
}

.action-card--client {
  border-top: 3px solid var(--color-blue);
}

.action-card-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.action-card-icon {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background-color: var(--color-brand);
  color: var(--color-button-text-active);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 1rem;
  flex-shrink: 0;
}

.action-card-icon--client {
  background-color: var(--color-blue);
}

.action-card-title {
  font-weight: 600;
  font-size: 0.95rem;
  color: var(--color-text);
}

.action-card-subtitle {
  font-size: 0.8rem;
  color: var(--color-text-secondary);
}

/* Buttons */
.btn {
  border: none;
  border-radius: var(--radius-sm);
  padding: 0.5rem 1rem;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s, opacity 0.15s;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-brand {
  background-color: var(--color-brand);
  color: var(--color-button-text-active);
}

.btn-brand:hover:not(:disabled) {
  background-color: var(--color-brand-highlight);
}

.btn-primary {
  background-color: var(--color-blue);
  color: var(--color-button-text-active);
}

.btn-primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn-danger {
  background-color: var(--color-red);
  color: var(--color-button-text-active);
}

.btn-danger:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn-ghost {
  background-color: transparent;
  color: var(--color-text);
  border: 1px solid var(--color-button-border);
}

.btn-ghost:hover:not(:disabled) {
  background-color: var(--color-button-bg-hover);
}

.btn-icon {
  background-color: transparent;
  color: var(--color-text-secondary);
  padding: 0.35rem;
  border: 1px solid var(--color-button-border);
}

.btn-icon:hover {
  color: var(--color-text);
  background-color: var(--color-button-bg-hover);
}

/* Connected hero */
.hero-connected {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.lobby-code-display {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.lobby-code-label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-text-secondary);
}

.lobby-code-value {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.lobby-code-value code {
  font-family: 'Fira Code', 'Cascadia Code', monospace;
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--color-brand);
  background-color: var(--color-bg);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-button-border);
}

.role-badge {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  padding: 0.25rem 0.6rem;
  border-radius: var(--radius-sm);
}

.role-badge--host {
  background-color: var(--color-brand);
  color: var(--color-button-text-active);
}

.role-badge--client {
  background-color: var(--color-blue);
  color: var(--color-button-text-active);
}

/* Busy state */
.hero-busy {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  color: var(--color-text-secondary);
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--color-button-border);
  border-top-color: var(--color-brand);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* Collapsible sections */
.collapsible-section {
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.section-header {
  width: 100%;
  background: none;
  border: none;
  padding: 0.85rem 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  color: var(--color-text);
  font-size: 0.95rem;
  font-weight: 600;
  text-align: left;
  transition: background-color 0.15s;
}

.section-header:hover {
  background-color: var(--color-button-bg-hover);
}

.section-chevron {
  transition: transform 0.2s;
  color: var(--color-text-secondary);
  font-size: 0.8rem;
}

.section-chevron--open {
  transform: rotate(180deg);
}

.section-body {
  padding: 0 1rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* Player list */
.player-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.player-card {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-sm);
}

.player-card--host {
  border-left: 3px solid var(--color-brand);
}

.player-card--client {
  border-left: 3px solid var(--color-blue);
}

.player-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background-color: var(--color-button-border);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 0.9rem;
  color: var(--color-text);
  flex-shrink: 0;
}

.player-info {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.player-name {
  font-weight: 600;
  font-size: 0.9rem;
  color: var(--color-text);
}

.player-role {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  text-transform: capitalize;
}

.player-latency {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-secondary);
  font-family: 'Fira Code', monospace;
}

/* Info grid */
.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 0.75rem;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.6rem 0.75rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-sm);
}

.info-label {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-text-secondary);
}

.info-value {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--color-text);
  text-transform: capitalize;
}

/* Mod compatibility */
.mod-compat-result {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.compat-status {
  font-weight: 600;
  font-size: 0.95rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-sm);
}

.compat-status--ok {
  background-color: var(--color-brand-bg);
  color: var(--color-brand);
}

.compat-status--warn {
  background-color: var(--color-orange-bg);
  color: var(--color-orange);
}

.compat-group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.compat-group-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.compat-mod-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.4rem 0.6rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-sm);
  font-size: 0.85rem;
}

.mod-name {
  color: var(--color-text);
}

.mod-version {
  color: var(--color-text-secondary);
  font-family: 'Fira Code', monospace;
  font-size: 0.8rem;
}

.mod-version--mismatch {
  color: var(--color-orange);
}

/* World list */
.world-list {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.world-item {
  display: flex;
  justify-content: space-between;
  padding: 0.4rem 0.6rem;
  background-color: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-radius: var(--radius-sm);
  font-size: 0.85rem;
}

.world-name {
  color: var(--color-text);
}

.world-port {
  color: var(--color-text-secondary);
  font-family: 'Fira Code', monospace;
}

.empty-hint {
  color: var(--color-text-secondary);
  font-size: 0.85rem;
  margin: 0;
}
</style>
