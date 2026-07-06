<script setup lang="ts">
import { CopyIcon, GlobeIcon, LinkIcon, UsersIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
	StyledInput,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import {
	create_lobby,
	get_lobby_status,
	join_lobby,
	leave_lobby,
	type LobbyStatus,
} from '@/helpers/link'
import { useBreadcrumbs } from '@/store/breadcrumbs'

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
	createLobbyTitle: {
		id: 'app.gamelink.create.title',
		defaultMessage: 'Create lobby',
	},
	createLobbyDescription: {
		id: 'app.gamelink.create.description',
		defaultMessage:
			'Open a Minecraft world to LAN, then create a lobby and share the code with your friends.',
	},
	joinLobbyTitle: {
		id: 'app.gamelink.join.title',
		defaultMessage: 'Join lobby',
	},
	joinLobbyDescription: {
		id: 'app.gamelink.join.description',
		defaultMessage: 'Paste a lobby code shared by the host and connect to their world.',
	},
	networkNameLabel: {
		id: 'app.gamelink.network-name.label',
		defaultMessage: 'Network name',
	},
	networkNamePlaceholder: {
		id: 'app.gamelink.network-name.placeholder',
		defaultMessage: 'My private network',
	},
	passwordLabel: {
		id: 'app.gamelink.password.label',
		defaultMessage: 'Password',
	},
	passwordPlaceholder: {
		id: 'app.gamelink.password.placeholder',
		defaultMessage: 'At least 6 characters',
	},
	mcPortLabel: {
		id: 'app.gamelink.mc-port.label',
		defaultMessage: 'Minecraft LAN port',
	},
	mcPortPlaceholder: {
		id: 'app.gamelink.mc-port.placeholder',
		defaultMessage: 'e.g. 61234',
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
	connectHint: {
		id: 'app.gamelink.connect-hint',
		defaultMessage: 'In Minecraft, use Direct Connect and enter {address}',
	},
	copiedNotification: {
		id: 'app.gamelink.copied.notification',
		defaultMessage: 'Lobby code copied to clipboard',
	},
	windowsOnlyWarning: {
		id: 'app.gamelink.windows-only',
		defaultMessage: 'LAN Multiplayer is currently only available on Windows.',
	},
})

breadcrumbs.setRootContext({ name: formatMessage(messages.title), link: '/gamelink' })

const isWindows = computed(() => typeof navigator !== 'undefined' && navigator.userAgent.includes('Windows'))

const status = ref<LobbyStatus | null>(null)
const loading = ref(false)
const pollInterval = ref<number | null>(null)

const networkName = ref('')
const password = ref('')
const mcPort = ref('')
const lobbyCodeInput = ref('')

async function refreshStatus() {
	try {
		status.value = await get_lobby_status()
	} catch (e) {
		console.error('Failed to get lobby status:', e)
	}
}

onMounted(() => {
	refreshStatus()
	pollInterval.value = window.setInterval(refreshStatus, 3000)
})

onUnmounted(() => {
	if (pollInterval.value) {
		clearInterval(pollInterval.value)
	}
})

async function handleCreate() {
	const port = parseInt(mcPort.value, 10)
	if (!networkName.value.trim() || !password.value.trim() || Number.isNaN(port)) {
		addNotification({
			title: formatMessage(messages.title),
			description: 'Please fill in all fields with valid values.',
			type: 'error',
		})
		return
	}
	loading.value = true
	try {
		await create_lobby(networkName.value.trim(), password.value.trim(), port)
		await refreshStatus()
	} catch (e) {
		handleError(e as Error)
	} finally {
		loading.value = false
	}
}

async function handleJoin() {
	const code = lobbyCodeInput.value.trim()
	if (!code) {
		addNotification({
			title: formatMessage(messages.title),
			description: 'Please enter a lobby code.',
			type: 'error',
		})
		return
	}
	loading.value = true
	try {
		await join_lobby(code)
		await refreshStatus()
	} catch (e) {
		handleError(e as Error)
	} finally {
		loading.value = false
	}
}

async function handleLeave() {
	loading.value = true
	try {
		await leave_lobby()
		await refreshStatus()
	} catch (e) {
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

const connectAddress = computed(() => {
	if (status.value?.state === 'host' && status.value.localPort) {
		return `localhost:${status.value.localPort}`
	}
	if (status.value?.state === 'client' && status.value.localPort) {
		return `localhost:${status.value.localPort}`
	}
	return ''
})
</script>

<template>
	<div class="page-container">
		<h1 class="text-2xl font-bold text-contrast mb-2">
			{{ formatMessage(messages.title) }}
		</h1>
		<p class="text-secondary mb-6">{{ formatMessage(messages.description) }}</p>

		<div v-if="!isWindows" class="p-4 rounded-xl bg-orange-bg text-orange-contrast mb-6">
			{{ formatMessage(messages.windowsOnlyWarning) }}
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
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

				<div v-if="status?.state === 'host'" class="space-y-4">
					<div>
						<label class="text-sm font-medium text-primary block mb-1">
							{{ formatMessage(messages.lobbyCodeLabel) }}
						</label>
						<div class="flex gap-2">
							<StyledInput
								v-model="status.lobbyCode"
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

					<div class="grid grid-cols-2 gap-4 text-sm">
						<div>
							<span class="text-secondary">{{ formatMessage(messages.virtualIpLabel) }}:</span>
							<span class="text-contrast ml-1 font-mono">{{ status.virtualIp ?? '-' }}</span>
						</div>
						<div>
							<span class="text-secondary">{{ formatMessage(messages.peersLabel) }}:</span>
							<span class="text-contrast ml-1">{{ status.peerCount }}</span>
						</div>
					</div>

					<ButtonStyled type="red">
						<button :disabled="loading" @click="handleLeave">
							<XIcon />
							{{ formatMessage(messages.leaveButton) }}
						</button>
					</ButtonStyled>
				</div>

				<form v-else class="space-y-4" @submit.prevent="handleCreate">
					<div>
						<label class="text-sm font-medium text-primary block mb-1">
							{{ formatMessage(messages.networkNameLabel) }}
						</label>
						<StyledInput
							v-model="networkName"
							:placeholder="formatMessage(messages.networkNamePlaceholder)"
							:disabled="loading || !isWindows"
						/>
					</div>
					<div>
						<label class="text-sm font-medium text-primary block mb-1">
							{{ formatMessage(messages.passwordLabel) }}
						</label>
						<StyledInput
							v-model="password"
							type="password"
							:placeholder="formatMessage(messages.passwordPlaceholder)"
							:disabled="loading || !isWindows"
						/>
					</div>
					<div>
						<label class="text-sm font-medium text-primary block mb-1">
							{{ formatMessage(messages.mcPortLabel) }}
						</label>
						<StyledInput
							v-model="mcPort"
							type="number"
							:placeholder="formatMessage(messages.mcPortPlaceholder)"
							:disabled="loading || !isWindows"
						/>
					</div>
					<ButtonStyled type="brand">
						<button type="submit" :disabled="loading || !isWindows">
							<LinkIcon />
							{{ formatMessage(messages.createButton) }}
						</button>
					</ButtonStyled>
				</form>
			</Card>

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

				<div v-if="status?.state === 'client'" class="space-y-4">
					<div class="grid grid-cols-2 gap-4 text-sm">
						<div>
							<span class="text-secondary">{{ formatMessage(messages.localPortLabel) }}:</span>
							<span class="text-contrast ml-1 font-mono">{{ status.localPort ?? '-' }}</span>
						</div>
						<div>
							<span class="text-secondary">{{ formatMessage(messages.peersLabel) }}:</span>
							<span class="text-contrast ml-1">{{ status.peerCount }}</span>
						</div>
					</div>

					<p v-if="connectAddress" class="text-sm text-secondary m-0">
						{{ formatMessage(messages.connectHint, { address: connectAddress }) }}
					</p>

					<ButtonStyled type="red">
						<button :disabled="loading" @click="handleLeave">
							<XIcon />
							{{ formatMessage(messages.leaveButton) }}
						</button>
					</ButtonStyled>
				</div>

				<form v-else class="space-y-4" @submit.prevent="handleJoin">
					<div>
						<label class="text-sm font-medium text-primary block mb-1">
							{{ formatMessage(messages.lobbyCodeLabel) }}
						</label>
						<StyledInput
							v-model="lobbyCodeInput"
							:placeholder="formatMessage(messages.lobbyCodePlaceholder)"
							:disabled="loading || !isWindows"
						/>
					</div>
					<ButtonStyled type="brand">
						<button type="submit" :disabled="loading || !isWindows">
							<LinkIcon />
							{{ formatMessage(messages.joinButton) }}
						</button>
					</ButtonStyled>
				</form>
			</Card>
		</div>

		<Card v-if="status?.state !== 'idle'" class="mt-4">
			<div class="flex items-center justify-between text-sm">
				<div class="flex items-center gap-2">
					<span
						class="size-2 rounded-full"
						:class="{
							'bg-green': status.state === 'host' || status.state === 'client',
							'bg-red': status.state === 'error',
						}"
					/>
					<span class="text-contrast font-medium">
						{{
							status.state === 'host'
								? formatMessage(messages.statusHost)
								: status.state === 'client'
									? formatMessage(messages.statusClient)
									: status.state === 'error'
										? formatMessage(messages.statusError)
										: formatMessage(messages.statusIdle)
						}}
					</span>
				</div>
				<span v-if="status.error" class="text-red">{{ status.error }}</span>
			</div>
		</Card>
	</div>
</template>
