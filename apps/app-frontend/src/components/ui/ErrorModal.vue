<script setup>
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	HammerIcon,
	LogInIcon,
	UpdatedIcon,
	WrenchIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Collapsible,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { ChatIcon } from '@/assets/icons'
import MinecraftLoginModal from '@/components/ui/modal/MinecraftLoginModal.vue'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { trackEvent } from '@/helpers/analytics'
import { set_default_user } from '@/helpers/auth.js'
import { install_existing_instance } from '@/helpers/install'
import { cancel_directory_change } from '@/helpers/settings.ts'
import { handleSevereError } from '@/store/error.js'

const { handleError } = injectNotificationManager()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	titleError: { id: 'app.error-modal.title.error', defaultMessage: 'An error occurred' },
	titleMinecraftAuth: {
		id: 'app.error-modal.title.minecraft-auth',
		defaultMessage: 'Unable to sign in to Minecraft',
	},
	titleMinecraftSignIn: {
		id: 'app.error-modal.title.minecraft-sign-in',
		defaultMessage: 'Sign in to Minecraft',
	},
	titleDirectoryMove: {
		id: 'app.error-modal.title.directory-move',
		defaultMessage: 'Could not change app directory',
	},
	titleNoLoader: { id: 'app.error-modal.title.no-loader', defaultMessage: 'No loader selected' },
	titleStateInit: {
		id: 'app.error-modal.title.state-init',
		defaultMessage: 'Error initializing Modrinth App',
	},
	networkIssues: { id: 'app.error-modal.network-issues', defaultMessage: 'Network issues' },
	networkIssuesDesc: {
		id: 'app.error-modal.network-issues.description',
		defaultMessage:
			"It looks like there were issues with the Modrinth App connecting to Microsoft's servers. This is often the result of a poor connection, so we recommend trying again to see if it works. If issues continue to persist, follow the steps in",
	},
	networkIssuesDescSuffix: {
		id: 'app.error-modal.network-issues.description-suffix',
		defaultMessage: 'to troubleshoot.',
	},
	hostsFileDesc: {
		id: 'app.error-modal.hosts-file.description',
		defaultMessage:
			'The Modrinth App tried to connect to Microsoft / Xbox / Minecraft services, but the remote server rejected the connection. This may indicate that these services are blocked by the hosts file. Please visit',
	},
	hostsFileDescSuffix: {
		id: 'app.error-modal.hosts-file.description-suffix',
		defaultMessage: 'for steps on how to fix the issue.',
	},
	ourSupportArticle: {
		id: 'app.error-modal.our-support-article',
		defaultMessage: 'our support article',
	},
	tryAnotherAccount: {
		id: 'app.error-modal.try-another-account',
		defaultMessage: 'Try another Microsoft account',
	},
	tryAnotherAccountDesc: {
		id: 'app.error-modal.try-another-account.description',
		defaultMessage:
			"Double check you've signed in with the right account. You may own Minecraft on a different Microsoft account.",
	},
	tryAnotherAccountBtn: {
		id: 'app.error-modal.try-another-account.button',
		defaultMessage: 'Try another account',
	},
	gamePassTitle: {
		id: 'app.error-modal.game-pass.title',
		defaultMessage: 'Using PC Game Pass, coming from Bedrock, or just bought the game?',
	},
	gamePassDesc: {
		id: 'app.error-modal.game-pass.description',
		defaultMessage:
			"Try signing in with the official Minecraft Launcher first. Once you're done, come back here and sign in!",
	},
	trySigningInAgain: {
		id: 'app.error-modal.try-signing-in-again',
		defaultMessage: 'Try signing in again',
	},
	changeDirectoryPermissions: {
		id: 'app.error-modal.change-directory-permissions',
		defaultMessage: 'Change directory permissions',
	},
	changeDirectoryPermissionsDesc: {
		id: 'app.error-modal.change-directory-permissions.description',
		defaultMessage:
			'It looks like the Modrinth App is unable to write to the directory you selected. Please adjust the permissions of the directory and try again or cancel the directory change.',
	},
	notEnoughSpace: { id: 'app.error-modal.not-enough-space', defaultMessage: 'Not enough space' },
	notEnoughSpaceDesc: {
		id: 'app.error-modal.not-enough-space.description',
		defaultMessage:
			'It looks like there is not enough space on the disk containing the directory you selected. Please free up some space and try again or cancel the directory change.',
	},
	migrateFailedDesc: {
		id: 'app.error-modal.migrate-failed.description',
		defaultMessage:
			'The Modrinth App is unable to migrate to the new directory you selected. Please contact support for help or cancel the directory change.',
	},
	retryDirectoryChange: {
		id: 'app.error-modal.retry-directory-change',
		defaultMessage: 'Retry directory change',
	},
	cancelDirectoryChange: {
		id: 'app.error-modal.cancel-directory-change',
		defaultMessage: 'Cancel directory change',
	},
	signInToPlayDesc: {
		id: 'app.error-modal.sign-in-to-play.description',
		defaultMessage:
			"To play this instance, you must sign in to a Minecraft account below. If you don't have a Minecraft account, you can purchase the game on the Minecraft website.",
	},
	signInToMinecraft: {
		id: 'app.error-modal.sign-in-to-minecraft',
		defaultMessage: 'Sign in to Minecraft',
	},
	stateInitDesc: {
		id: 'app.error-modal.state-init.description',
		defaultMessage:
			'Modrinth App failed to load correctly. This may be because of a corrupted file, or because the app is missing crucial files.',
	},
	stateInitFix: {
		id: 'app.error-modal.state-init.fix',
		defaultMessage: 'You may be able to fix it through one of the following ways:',
	},
	stateInitFix1: {
		id: 'app.error-modal.state-init.fix-1',
		defaultMessage: 'Ensuring you are connected to the internet, then try restarting the app.',
	},
	stateInitFix2: { id: 'app.error-modal.state-init.fix-2', defaultMessage: 'Redownloading the app.' },
	noLoaderDesc: {
		id: 'app.error-modal.no-loader.description',
		defaultMessage: 'The Modrinth App failed to find the loader version for this instance.',
	},
	noLoaderFix: {
		id: 'app.error-modal.no-loader.fix',
		defaultMessage:
			'To resolve this, you need to repair the instance. Click the button below to do so.',
	},
	repairInstance: { id: 'app.error-modal.repair-instance', defaultMessage: 'Repair instance' },
	supportDesc: {
		id: 'app.error-modal.support.description',
		defaultMessage:
			'If nothing is working and you need help, visit our support page and start a chat using the widget in the bottom right and we will be more than happy to assist! Make sure to provide the following debug information to the agent:',
	},
	getSupport: { id: 'app.error-modal.get-support', defaultMessage: 'Get support' },
	close: { id: 'app.error-modal.close', defaultMessage: 'Close' },
	debugInfo: { id: 'app.error-modal.debug-info', defaultMessage: 'Debug information' },
	copyDebugInfo: { id: 'app.error-modal.copy-debug-info', defaultMessage: 'Copy debug info' },
	noErrorMessage: {
		id: 'app.error-modal.no-error-message',
		defaultMessage: 'No error message.',
	},
})

const errorModal = ref()
const loginModal = ref()
const error = ref()
const closable = ref(true)
const errorCollapsed = ref(false)

const title = ref(formatMessage(messages.titleError))
const errorType = ref('unknown')
const supportLink = ref('https://support.modrinth.com')
const metadata = ref({})

defineExpose({
	async show(errorVal, context, canClose = true, source = null) {
		console.log(errorVal, context, canClose, source)
		closable.value = canClose

		if (errorVal.message && errorVal.message.includes('Minecraft authentication error:')) {
			title.value = formatMessage(messages.titleMinecraftAuth)
			errorType.value = 'minecraft_auth'
			supportLink.value =
				'https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues'

			if (
				errorVal.message.includes('existing connection was forcibly closed') ||
				errorVal.message.includes('error sending request for url')
			) {
				metadata.value.network = true
			}
			if (errorVal.message.includes('because the target machine actively refused it')) {
				metadata.value.hostsFile = true
			}
		} else if (errorVal.message && errorVal.message.includes('User is not logged in')) {
			title.value = formatMessage(messages.titleMinecraftSignIn)
			errorType.value = 'minecraft_sign_in'
			supportLink.value = 'https://support.modrinth.com'
		} else if (errorVal.message && errorVal.message.includes('Move directory error:')) {
			title.value = formatMessage(messages.titleDirectoryMove)
			errorType.value = 'directory_move'
			supportLink.value = 'https://support.modrinth.com'

			if (errorVal.message.includes('directory is not writable')) {
				metadata.value.readOnly = true
			}

			if (errorVal.message.includes('Not enough space')) {
				metadata.value.notEnoughSpace = true
			}
		} else if (errorVal.message && errorVal.message.includes('No loader version selected for')) {
			title.value = formatMessage(messages.titleNoLoader)
			errorType.value = 'no_loader_version'
			supportLink.value = 'https://support.modrinth.com'
			metadata.value.instanceId = context.instanceId
		} else if (source === 'state_init') {
			title.value = formatMessage(messages.titleStateInit)
			errorType.value = 'state_init'
			supportLink.value = 'https://support.modrinth.com'
		} else {
			title.value = formatMessage(messages.titleError)
			errorType.value = 'unknown'
			supportLink.value = 'https://support.modrinth.com'
			metadata.value = {}
		}

		error.value = errorVal
		errorModal.value.show()
	},
})

const loadingMinecraft = ref(false)
function loginMinecraft() {
	loginModal.value?.show('microsoft')
	trackEvent('AccountLogIn', { source: 'ErrorModal' })
}

async function onLoginSuccess(credential) {
	await set_default_user(credential.profile.id).catch(handleError)
	errorModal.value.hide()
}

async function cancelDirectoryChange() {
	try {
		await cancel_directory_change()
		window.location.reload()
	} catch (err) {
		handleError(err)
	}
}

function retryDirectoryChange() {
	window.location.reload()
}

const loadingRepair = ref(false)
async function repairInstance() {
	loadingRepair.value = true
	try {
		await install_existing_instance(metadata.value.instanceId, false)
		errorModal.value.hide()
	} catch (err) {
		handleSevereError(err)
	}
	loadingRepair.value = false
}

const hasDebugInfo = computed(
	() =>
		errorType.value === 'directory_move' ||
		errorType.value === 'minecraft_auth' ||
		errorType.value === 'state_init' ||
		errorType.value === 'no_loader_version',
)

const debugInfo = computed(
	() => error.value.message ?? error.value ?? formatMessage(messages.noErrorMessage),
)

const copied = ref(false)

async function copyToClipboard(text) {
	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}
</script>

<template>
	<ModalWrapper ref="errorModal" :header="title" :closable="closable">
		<div class="modal-body max-w-[550px]">
			<div class="markdown-body">
				<template v-if="errorType === 'minecraft_auth'">
					<template v-if="metadata.network">
					<h3>{{ formatMessage(messages.networkIssues) }}</h3>
					<p>
						{{ formatMessage(messages.networkIssuesDesc) }}
						<a
							href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_e71a5f805f"
						>
							{{ formatMessage(messages.ourSupportArticle) }}
						</a>
						{{ formatMessage(messages.networkIssuesDescSuffix) }}
					</p>
				</template>
					<template v-else-if="metadata.hostsFile">
					<h3>{{ formatMessage(messages.networkIssues) }}</h3>
					<p>
						{{ formatMessage(messages.hostsFileDesc) }}
						<a
							href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_d694a29256"
						>
							{{ formatMessage(messages.ourSupportArticle) }}
						</a>
						{{ formatMessage(messages.hostsFileDescSuffix) }}
					</p>
				</template>
					<template v-else>
					<h3>{{ formatMessage(messages.tryAnotherAccount) }}</h3>
					<p>{{ formatMessage(messages.tryAnotherAccountDesc) }}</p>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
							<LogInIcon /> {{ formatMessage(messages.tryAnotherAccountBtn) }}
						</button>
					</div>
					<h3>{{ formatMessage(messages.gamePassTitle) }}</h3>
					<p>{{ formatMessage(messages.gamePassDesc) }}</p>
				</template>
					<div class="cta-button">
					<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
						<LogInIcon /> {{ formatMessage(messages.trySigningInAgain) }}
					</button>
				</div>
				</template>
				<template v-if="errorType === 'directory_move'">
					<template v-if="metadata.readOnly">
					<h3>{{ formatMessage(messages.changeDirectoryPermissions) }}</h3>
					<p>{{ formatMessage(messages.changeDirectoryPermissionsDesc) }}</p>
				</template>
					<template v-else-if="metadata.notEnoughSpace">
					<h3>{{ formatMessage(messages.notEnoughSpace) }}</h3>
					<p>{{ formatMessage(messages.notEnoughSpaceDesc) }}</p>
				</template>
					<template v-else>
					<p>{{ formatMessage(messages.migrateFailedDesc) }}</p>
				</template>

					<div class="cta-button">
					<button class="btn" @click="retryDirectoryChange">
						<UpdatedIcon /> {{ formatMessage(messages.retryDirectoryChange) }}
					</button>
					<button class="btn btn-danger" @click="cancelDirectoryChange">
						<XIcon /> {{ formatMessage(messages.cancelDirectoryChange) }}
					</button>
				</div>
				</template>
				<div v-else-if="errorType === 'minecraft_sign_in'">
				<p>{{ formatMessage(messages.signInToPlayDesc) }}</p>
				<div class="cta-button">
					<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
						<LogInIcon /> {{ formatMessage(messages.signInToMinecraft) }}
					</button>
				</div>
			</div>
				<template v-else-if="errorType === 'state_init'">
				<p>{{ formatMessage(messages.stateInitDesc) }}</p>
				<p>{{ formatMessage(messages.stateInitFix) }}</p>
				<ul>
					<li>{{ formatMessage(messages.stateInitFix1) }}</li>
					<li>{{ formatMessage(messages.stateInitFix2) }}</li>
				</ul>
			</template>
				<template v-else-if="errorType === 'no_loader_version'">
				<p>{{ formatMessage(messages.noLoaderDesc) }}</p>
				<p>{{ formatMessage(messages.noLoaderFix) }}</p>
				<div class="cta-button">
					<button class="btn btn-primary" :disabled="loadingRepair" @click="repairInstance">
						<HammerIcon /> {{ formatMessage(messages.repairInstance) }}
					</button>
				</div>
			</template>
				<template v-else>
					{{ debugInfo }}
				</template>
				<template v-if="hasDebugInfo">
				<div class="w-full h-[1px] bg-surface-5 mb-3"></div>
				<p>{{ formatMessage(messages.supportDesc) }}</p>
			</template>
			</div>
			<div class="flex items-center gap-2">
			<ButtonStyled>
				<a :href="supportLink" @click="errorModal.hide()"
					><ChatIcon /> {{ formatMessage(messages.getSupport) }}</a
				>
			</ButtonStyled>
			<ButtonStyled v-if="closable">
				<button @click="errorModal.hide()">
					<XIcon /> {{ formatMessage(messages.close) }}
				</button>
			</ButtonStyled>
		</div>
			<template v-if="hasDebugInfo">
				<div class="flex flex-col gap-2">
					<div class="w-full h-[1px] bg-surface-5"></div>

					<div class="overflow-clip">
						<button
							class="flex items-center justify-between w-full bg-transparent border-0 py-4 cursor-pointer"
							@click="errorCollapsed = !errorCollapsed"
						>
							<span class="flex items-center gap-2 text-contrast font-extrabold m-0">
							<WrenchIcon class="h-4 w-4" />
							{{ formatMessage(messages.debugInfo) }}
						</span>
							<DropdownIcon
								class="h-5 w-5 text-secondary transition-transform"
								:class="{ 'rotate-180': !errorCollapsed }"
							/>
						</button>
						<Collapsible :collapsed="errorCollapsed">
							<div
								class="p-3 bg-surface-2 rounded-2xl text-xs grid grid-cols-[1fr_auto] max-w-full items-start"
							>
								<div
									class="m-0 p-0 rounded-none bg-transparent text-sm font-mono break-words overflow-auto"
								>
									{{ debugInfo }}
								</div>
								<ButtonStyled circular>
									<button
									v-tooltip="formatMessage(messages.copyDebugInfo)"
									:disabled="copied"
									@click="copyToClipboard(debugInfo)"
								>
										<template v-if="copied"> <CheckIcon class="text-green" /> </template>
										<template v-else> <CopyIcon /> </template>
									</button>
								</ButtonStyled>
							</div>
						</Collapsible>
					</div>
				</div>
			</template>
		</div>
	</ModalWrapper>

	<MinecraftLoginModal ref="loginModal" @success="onLoginSuccess" />
</template>

<style>
.light-mode {
	--color-orange-bg: rgba(255, 163, 71, 0.2);
}

.dark-mode,
.oled-mode {
	--color-orange-bg: rgba(224, 131, 37, 0.2);
}
</style>

<style scoped lang="scss">
.cta-button {
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0.5rem;
	gap: 0.5rem;
}

.warning-banner {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: var(--gap-lg);
	background-color: var(--color-orange-bg);
	border: 2px solid var(--color-orange);
	border-radius: var(--radius-md);
	margin-bottom: 1rem;
}

.warning-banner__title {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-weight: 700;

	svg {
		color: var(--color-orange);
		height: 1.5rem;
		width: 1.5rem;
	}
}

.modal-body {
	display: flex;
	flex-direction: column;
	gap: var(--gap-md);
}

.markdown-body {
	overflow: auto;
}
</style>
