<script setup lang="ts">
import { ref } from 'vue'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { getCurrentWindow } from '@tauri-apps/api/window'
import fullGpl3License from '../../LICENSE'

const { formatMessage } = useVIntl()

const props = defineProps<{
	requireAccept?: boolean
}>()

const emit = defineEmits<{
	close: []
	accepted: []
}>()

const activeTab = ref<'tos' | 'privacy' | 'license'>('tos')

const messages = defineMessages({
	title: {
		id: 'app.license-modal.title',
		defaultMessage: 'Agreement & License',
	},
	agree: {
		id: 'app.license-modal.agree',
		defaultMessage: 'Agree & Continue',
	},
	decline: {
		id: 'app.license-modal.decline',
		defaultMessage: 'Decline',
	},
	tabTos: {
		id: 'app.license-modal.tab-tos',
		defaultMessage: 'Terms of Use',
	},
	tabPrivacy: {
		id: 'app.license-modal.tab-privacy',
		defaultMessage: 'Privacy Policy',
	},
	tabLicense: {
		id: 'app.license-modal.tab-license',
		defaultMessage: 'Open Source',
	},
	tosContent: {
		id: 'app.license-modal.tos-content',
		defaultMessage: '',
	},
	privacyContent: {
		id: 'app.license-modal.privacy-content',
		defaultMessage: '',
	},
	notice: {
		id: 'app.license-modal.notice',
		defaultMessage: '',
	},
	packageLicenses: {
		id: 'app.license-modal.package-licenses',
		defaultMessage: '',
	},
})

const licenseText = [
	formatMessage(messages.notice),
	'',
	formatMessage(messages.packageLicenses),
	'',
	'─────────────────────────────────────────────',
	'',
	fullGpl3License,
].join('\n')

const tabs = [
	{ key: 'tos' as const, label: formatMessage(messages.tabTos) },
	{ key: 'privacy' as const, label: formatMessage(messages.tabPrivacy) },
	{ key: 'license' as const, label: formatMessage(messages.tabLicense) },
]

function onAccept() {
	localStorage.setItem('allay-license-accepted', 'true')
	emit('accepted')
	if (!props.requireAccept) {
		emit('close')
	}
}

async function onDecline() {
	if (props.requireAccept) {
		await getCurrentWindow().close()
	} else {
		emit('close')
	}
}
</script>

<template>
	<Transition name="modal">
		<div class="license-overlay" @click.self="!requireAccept && onDecline()">
			<div class="license-modal">
				<h2 class="license-title">{{ formatMessage(messages.title) }}</h2>

				<div class="tab-bar">
					<button
						v-for="tab in tabs"
						:key="tab.key"
						class="tab-btn"
						:class="{ active: activeTab === tab.key }"
						@click="activeTab = tab.key"
					>
						{{ tab.label }}
					</button>
				</div>

				<div class="license-content">
					<pre v-if="activeTab === 'tos'" class="license-text">{{ formatMessage(messages.tosContent) }}</pre>
					<pre v-else-if="activeTab === 'privacy'" class="license-text">{{ formatMessage(messages.privacyContent) }}</pre>
					<pre v-else class="license-text">{{ licenseText }}</pre>
				</div>

				<div class="license-actions">
					<ButtonStyled color="brand" @click="onAccept">
						<button>{{ formatMessage(messages.agree) }}</button>
					</ButtonStyled>
					<ButtonStyled v-if="requireAccept" @click="onDecline">
						<button>{{ formatMessage(messages.decline) }}</button>
					</ButtonStyled>
				</div>
			</div>
		</div>
	</Transition>
</template>

<style lang="scss" scoped>
.license-overlay {
	position: fixed;
	inset: 0;
	z-index: 300;
	display: flex;
	align-items: center;
	justify-content: center;
	background: rgba(0, 0, 0, 0.7);
	backdrop-filter: blur(8px);
}

.license-modal {
	background: var(--color-raised-bg);
	border-radius: var(--radius-xl);
	padding: 2rem;
	max-width: 700px;
	width: 92vw;
	max-height: 88vh;
	display: flex;
	flex-direction: column;
	box-shadow: 0 25px 50px rgba(0, 0, 0, 0.5);
	border: 1px solid var(--color-divider, rgba(255, 255, 255, 0.1));
}

.license-title {
	margin: 0 0 1rem 0;
	font-size: 1.5rem;
	font-weight: 700;
	color: var(--color-contrast);
}

.tab-bar {
	display: flex;
	gap: 0.25rem;
	margin-bottom: 1rem;
	border-bottom: 1px solid var(--color-divider-dark, rgba(255, 255, 255, 0.08));
}

.tab-btn {
	padding: 0.5rem 1rem;
	border: none;
	border-bottom: 2px solid transparent;
	background: none;
	color: var(--color-text, #a1a7b3);
	font-size: 0.9rem;
	font-weight: 500;
	cursor: pointer;
	transition: color 0.15s, border-color 0.15s;

	&:hover {
		color: var(--color-contrast);
	}

	&.active {
		color: var(--color-brand, #4a9eff);
		border-bottom-color: var(--color-brand, #4a9eff);
	}
}

.license-content {
	flex: 1;
	overflow-y: auto;
	margin-bottom: 1.5rem;
}

.license-text {
	margin: 0;
	white-space: pre-wrap;
	font-family: 'SF Mono', 'Fira Code', 'Fira Mono', 'Roboto Mono', monospace;
	font-size: 0.85rem;
	line-height: 1.7;
	color: var(--color-text, #a1a7b3);
}

.license-actions {
	display: flex;
	gap: 0.75rem;
	justify-content: flex-end;
}

/* --- Transitions --- */
.modal-enter-active {
	transition: opacity 0.2s ease;
}
.modal-enter-active .license-modal {
	transition: transform 0.25s cubic-bezier(0.51, 1.08, 0.35, 1.15), opacity 0.2s ease;
}
.modal-leave-active {
	transition: opacity 0.15s ease;
}
.modal-leave-active .license-modal {
	transition: transform 0.15s ease, opacity 0.15s ease;
}
.modal-enter-from,
.modal-leave-to {
	opacity: 0;
}
.modal-enter-from .license-modal {
	transform: scale(0.9) translateY(1rem);
	opacity: 0;
}
.modal-leave-to .license-modal {
	transform: scale(0.95);
	opacity: 0;
}
</style>
