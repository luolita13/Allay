<script setup lang="ts">
import { ShieldCheckIcon } from '@modrinth/assets'
import { ButtonStyled, NewModal } from '@modrinth/ui'
import { ref } from 'vue'

// ---------------------------------------------------------------------------
// Props & Emits
// ---------------------------------------------------------------------------

const emit = defineEmits<{
	(e: 'accept' | 'decline'): void
}>()

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

const modal = ref<InstanceType<typeof NewModal> | null>(null)

// ---------------------------------------------------------------------------
// Expose
// ---------------------------------------------------------------------------

defineExpose({
	show: () => {
		modal.value?.show()
	},
	hide: () => {
		modal.value?.hide()
	},
})

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

function acceptAgreement() {
	emit('accept')
	modal.value?.hide()
}

function declineAgreement() {
	emit('decline')
	modal.value?.hide()
}
</script>

<template>
	<NewModal
		ref="modal"
		:closable="false"
		:close-on-click-outside="false"
		:close-on-esc="false"
		max-width="560px"
		:hide-header="true"
	>
		<div class="agreement-modal">
				<!-- Header -->
				<div class="agreement-header">
					<div class="agreement-icon-wrapper">
						<ShieldCheckIcon class="agreement-icon" />
					</div>
					<h2 class="agreement-title">Game Link Terms of Use</h2>
					<p class="agreement-subtitle">
						Please read the following terms before using the Game Link feature
					</p>
				</div>

				<!-- Body -->
				<div class="agreement-body">
					<p class="agreement-intro">By using the Game Link feature, you agree to the following terms:</p>

					<ul class="agreement-list">
						<li>
							I agree to comply with all applicable laws and regulations and will not use
							this feature for any unlawful purposes.
						</li>
						<li>
							I assume all risks arising from the use of the Game Link feature and agree
							that the developers are not liable for any damages.
						</li>
						<li>
							I acknowledge that the launcher may collect processed device identifiers and
							other information, and may disclose such data to law enforcement when legally
							required.
						</li>
						<li>
							To protect the privacy of minors, I confirm that I am at least 14 years of
							age before using this feature.
						</li>
					</ul>
				</div>

				<!-- Actions -->
				<div class="agreement-actions">
					<ButtonStyled type="brand">
						<button class="accept-btn" @click="acceptAgreement">
							I have read and agree
						</button>
					</ButtonStyled>
					<ButtonStyled type="transparent">
						<button class="decline-btn" @click="declineAgreement">Decline</button>
					</ButtonStyled>
				</div>
			</div>
	</NewModal>
</template>

<style scoped>
.agreement-modal {
	display: flex;
	flex-direction: column;
	gap: 1.25rem;
	padding: 0.5rem 0.25rem;
	max-height: 80vh;
}

/* ── Header ─────────────────────────────────── */
.agreement-header {
	display: flex;
	flex-direction: column;
	align-items: center;
	text-align: center;
	gap: 0.5rem;
}

.agreement-icon-wrapper {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 3.5rem;
	height: 3.5rem;
	border-radius: 50%;
	background: radial-gradient(
		circle at center,
		var(--color-brand) 0%,
		var(--color-brand) 40%,
		transparent 70%
	);
	margin-bottom: 0.25rem;
}

.agreement-icon {
	width: 1.75rem;
	height: 1.75rem;
	color: white;
}

.agreement-title {
	margin: 0;
	font-size: 1.25rem;
	font-weight: 700;
	color: var(--color-text);
	letter-spacing: -0.01em;
}

.agreement-subtitle {
	margin: 0;
	font-size: 0.8rem;
	color: var(--color-text-secondary);
	max-width: 28rem;
}

/* ── Body ───────────────────────────────────── */
.agreement-body {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	padding: 1rem 1.25rem;
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-md);
	background: var(--color-bg);
	max-height: 50vh;
	overflow-y: auto;
}

.agreement-intro {
	margin: 0 0 0.25rem;
	font-size: 0.85rem;
	font-weight: 600;
	color: var(--color-text);
}

.agreement-list {
	margin: 0;
	padding-left: 1.25rem;
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	list-style: disc;
}

.agreement-list li {
	font-size: 0.82rem;
	line-height: 1.55;
	color: var(--color-text);
}

/* ── Actions ────────────────────────────────── */
.agreement-actions {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	align-items: stretch;
}

.accept-btn {
	width: 100%;
	padding: 0.65rem 1.5rem;
	font-weight: 600;
	font-size: 0.9rem;
}

.decline-btn {
	width: 100%;
	padding: 0.5rem 1.5rem;
	font-size: 0.85rem;
	color: var(--color-text-secondary);
}
</style>
