<script setup lang="ts">
import {
	CircleAlertIcon,
	GlobeIcon,
	LogInIcon,
	SpinnerIcon,
	UserCogIcon,
	UserIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal as Modal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { trackEvent } from '@/helpers/analytics'
import {
	login as login_microsoft,
	login_offline,
	login_yggdrasil,
} from '@/helpers/auth'
import { handleSevereError } from '@/store/error.js'

type LoginMethod = 'microsoft' | 'offline' | 'yggdrasil'

type MinecraftCredential = {
	profile: {
		id: string
		name: string
	}
	login_type?: LoginMethod
	server_url?: string | null
}

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const emit = defineEmits<{
	success: [credential: MinecraftCredential]
}>()

const modal = ref()

const selectedMethod = ref<LoginMethod>('microsoft')
const username = ref('')
const serverUrl = ref('')
const password = ref('')
const submitting = ref(false)

// Offline username validation: 3-16 chars, alphanumeric + underscore
const offlineUsernameRegex = /^[A-Za-z0-9_]{3,16}$/

const offlineUsernameError = computed(() => {
	const v = username.value.trim()
	if (v.length === 0) return false
	return !offlineUsernameRegex.test(v)
})

const canSubmit = computed(() => {
	if (submitting.value) return false
	if (selectedMethod.value === 'microsoft') return true
	if (selectedMethod.value === 'offline')
		return offlineUsernameRegex.test(username.value.trim())
	if (selectedMethod.value === 'yggdrasil')
		return (
			serverUrl.value.trim().length > 0 &&
			username.value.trim().length > 0 &&
			password.value.length > 0
		)
	return false
})

function show(method: LoginMethod = 'microsoft') {
	selectedMethod.value = method
	username.value = ''
	serverUrl.value = ''
	password.value = ''
	submitting.value = false
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })

async function submit() {
	if (!canSubmit.value) return
	submitting.value = true
	try {
		let credential: MinecraftCredential | null = null
		if (selectedMethod.value === 'microsoft') {
			const result = await login_microsoft().catch(handleSevereError)
			if (result) credential = result as MinecraftCredential
		} else if (selectedMethod.value === 'offline') {
			credential = await login_offline(username.value.trim()).catch(
				handleError,
			)
		} else if (selectedMethod.value === 'yggdrasil') {
			credential = await login_yggdrasil(
				serverUrl.value.trim(),
				username.value.trim(),
				password.value,
			).catch(handleError)
		}

		if (credential) {
			trackEvent('AccountLogIn', { method: selectedMethod.value })
			emit('success', credential)
			hide()
		} else if (selectedMethod.value === 'microsoft') {
			// User closed the Microsoft sign-in window without completing the flow
			hide()
		}
	} finally {
		submitting.value = false
	}
}

const messages = defineMessages({
	title: {
		id: 'minecraft-login-modal.title',
		defaultMessage: 'Sign in to Minecraft',
	},
	description: {
		id: 'minecraft-login-modal.description',
		defaultMessage: 'Choose how you want to sign in.',
	},
	microsoftLabel: {
		id: 'minecraft-login-modal.microsoft-label',
		defaultMessage: 'Microsoft',
	},
	microsoftDescription: {
		id: 'minecraft-login-modal.microsoft-description',
		defaultMessage: 'Sign in with a Microsoft account (official).',
	},
	offlineLabel: {
		id: 'minecraft-login-modal.offline-label',
		defaultMessage: 'Offline',
	},
	offlineDescription: {
		id: 'minecraft-login-modal.offline-description',
		defaultMessage: 'Play without authentication. Skins and servers requiring authentication are unavailable.',
	},
	yggdrasilLabel: {
		id: 'minecraft-login-modal.yggdrasil-label',
		defaultMessage: 'Third-party (Yggdrasil)',
	},
	yggdrasilDescription: {
		id: 'minecraft-login-modal.yggdrasil-description',
		defaultMessage: 'Sign in to an authlib-injector compatible server (e.g. LittleSkin).',
	},
	usernameLabel: {
		id: 'minecraft-login-modal.username-label',
		defaultMessage: 'Username',
	},
	usernamePlaceholder: {
		id: 'minecraft-login-modal.username-placeholder',
		defaultMessage: 'Enter username',
	},
	usernameError: {
		id: 'minecraft-login-modal.username-error',
		defaultMessage: 'Username must be 3-16 characters: letters, numbers, or underscore.',
	},
	serverUrlLabel: {
		id: 'minecraft-login-modal.server-url-label',
		defaultMessage: 'Server URL',
	},
	serverUrlPlaceholder: {
		id: 'minecraft-login-modal.server-url-placeholder',
		defaultMessage: 'https://example.com/api/yggdrasil',
	},
	passwordLabel: {
		id: 'minecraft-login-modal.password-label',
		defaultMessage: 'Password',
	},
	passwordPlaceholder: {
		id: 'minecraft-login-modal.password-placeholder',
		defaultMessage: 'Enter password',
	},
	proceedLabel: {
		id: 'minecraft-login-modal.proceed-label',
		defaultMessage: 'Sign in',
	},
	microsoftHelp: {
		id: 'minecraft-login-modal.microsoft-help',
		defaultMessage: 'A browser window will open for you to complete sign-in.',
	},
	offlineHelp: {
		id: 'minecraft-login-modal.offline-help',
		defaultMessage: 'No network connection required. Useful for offline play or LAN servers in offline mode.',
	},
	yggdrasilHelp: {
		id: 'minecraft-login-modal.yggdrasil-help',
		defaultMessage: 'Supports any authlib-injector compatible server. The authlib-injector jar will be downloaded on first launch.',
	},
})
</script>

<template>
	<Modal ref="modal" :closable="!submitting">
		<template #title>
			<span class="flex items-center gap-2 text-lg font-extrabold text-contrast">
				<LogInIcon class="w-5 h-5" />
				{{ formatMessage(messages.title) }}
			</span>
		</template>

		<div class="flex flex-col gap-4 w-full min-w-[24rem] max-w-md">
			<p class="text-sm text-secondary m-0">
				{{ formatMessage(messages.description) }}
			</p>

			<!-- Method selector -->
			<div class="flex flex-col gap-2">
				<button
					type="button"
					class="flex items-start gap-3 p-3 rounded-xl border border-solid transition-colors text-left"
					:class="
						selectedMethod === 'microsoft'
							? 'border-brand bg-brand-highlight text-contrast'
							: 'border-surface-5 bg-surface-4 text-primary hover:bg-surface-3'
					"
					:disabled="submitting"
					@click="selectedMethod = 'microsoft'"
				>
					<UserCogIcon class="w-5 h-5 mt-0.5 shrink-0" />
					<div class="flex flex-col gap-0.5 min-w-0">
						<span class="font-semibold">{{ formatMessage(messages.microsoftLabel) }}</span>
						<span class="text-xs text-secondary">{{ formatMessage(messages.microsoftDescription) }}</span>
					</div>
				</button>

				<button
					type="button"
					class="flex items-start gap-3 p-3 rounded-xl border border-solid transition-colors text-left"
					:class="
						selectedMethod === 'offline'
							? 'border-brand bg-brand-highlight text-contrast'
							: 'border-surface-5 bg-surface-4 text-primary hover:bg-surface-3'
					"
					:disabled="submitting"
					@click="selectedMethod = 'offline'"
				>
					<UserIcon class="w-5 h-5 mt-0.5 shrink-0" />
					<div class="flex flex-col gap-0.5 min-w-0">
						<span class="font-semibold">{{ formatMessage(messages.offlineLabel) }}</span>
						<span class="text-xs text-secondary">{{ formatMessage(messages.offlineDescription) }}</span>
					</div>
				</button>

				<button
					type="button"
					class="flex items-start gap-3 p-3 rounded-xl border border-solid transition-colors text-left"
					:class="
						selectedMethod === 'yggdrasil'
							? 'border-brand bg-brand-highlight text-contrast'
							: 'border-surface-5 bg-surface-4 text-primary hover:bg-surface-3'
					"
					:disabled="submitting"
					@click="selectedMethod = 'yggdrasil'"
				>
					<GlobeIcon class="w-5 h-5 mt-0.5 shrink-0" />
					<div class="flex flex-col gap-0.5 min-w-0">
						<span class="font-semibold">{{ formatMessage(messages.yggdrasilLabel) }}</span>
						<span class="text-xs text-secondary">{{ formatMessage(messages.yggdrasilDescription) }}</span>
					</div>
				</button>
			</div>

			<!-- Method-specific inputs -->
			<div v-if="selectedMethod === 'microsoft'" class="flex items-start gap-2 text-xs text-secondary">
				<CircleAlertIcon class="w-4 h-4 mt-0.5 shrink-0" />
				<span>{{ formatMessage(messages.microsoftHelp) }}</span>
			</div>

			<div v-else-if="selectedMethod === 'offline'" class="flex flex-col gap-3">
				<div class="flex flex-col gap-1">
					<label class="text-sm font-medium text-primary" for="mc-login-offline-username">
						{{ formatMessage(messages.usernameLabel) }}
					</label>
					<StyledInput
						id="mc-login-offline-username"
						v-model="username"
						type="text"
						:placeholder="formatMessage(messages.usernamePlaceholder)"
						autocomplete="off"
						:disabled="submitting"
						:error="offlineUsernameError"
						@keydown.enter="submit"
					/>
					<p v-if="offlineUsernameError" class="text-xs text-red m-0">
						{{ formatMessage(messages.usernameError) }}
					</p>
				</div>
				<div class="flex items-start gap-2 text-xs text-secondary">
					<CircleAlertIcon class="w-4 h-4 mt-0.5 shrink-0" />
					<span>{{ formatMessage(messages.offlineHelp) }}</span>
				</div>
			</div>

			<div v-else-if="selectedMethod === 'yggdrasil'" class="flex flex-col gap-3">
				<div class="flex flex-col gap-1">
					<label class="text-sm font-medium text-primary" for="mc-login-yggdrasil-server">
						{{ formatMessage(messages.serverUrlLabel) }}
					</label>
					<StyledInput
						id="mc-login-yggdrasil-server"
						v-model="serverUrl"
						type="url"
						:placeholder="formatMessage(messages.serverUrlPlaceholder)"
						autocomplete="off"
						:disabled="submitting"
						@keydown.enter="submit"
					/>
				</div>
				<div class="flex flex-col gap-1">
					<label class="text-sm font-medium text-primary" for="mc-login-yggdrasil-username">
						{{ formatMessage(messages.usernameLabel) }}
					</label>
					<StyledInput
						id="mc-login-yggdrasil-username"
						v-model="username"
						type="text"
						:placeholder="formatMessage(messages.usernamePlaceholder)"
						autocomplete="off"
						:disabled="submitting"
						@keydown.enter="submit"
					/>
				</div>
				<div class="flex flex-col gap-1">
					<label class="text-sm font-medium text-primary" for="mc-login-yggdrasil-password">
						{{ formatMessage(messages.passwordLabel) }}
					</label>
					<StyledInput
						id="mc-login-yggdrasil-password"
						v-model="password"
						type="password"
						:placeholder="formatMessage(messages.passwordPlaceholder)"
						autocomplete="off"
						:disabled="submitting"
						@keydown.enter="submit"
					/>
				</div>
				<div class="flex items-start gap-2 text-xs text-secondary">
					<CircleAlertIcon class="w-4 h-4 mt-0.5 shrink-0" />
					<span>{{ formatMessage(messages.yggdrasilHelp) }}</span>
				</div>
			</div>

			<!-- Action buttons -->
			<div class="flex justify-end gap-2 pt-2">
				<ButtonStyled>
					<button :disabled="!canSubmit" @click="submit">
						<SpinnerIcon v-if="submitting" class="animate-spin" />
						<LogInIcon v-else />
						{{ formatMessage(messages.proceedLabel) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Modal>
</template>
