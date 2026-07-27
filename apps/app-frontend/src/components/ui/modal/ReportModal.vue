<script setup lang="ts">
import { CircleAlertIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal as Modal,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

const { formatMessage } = useVIntl()
const notificationManager = injectNotificationManager()

// ---------------------------------------------------------------------------
// Props & Emits
// ---------------------------------------------------------------------------

interface PlayerProfile {
	name: string
	machine_id: string
	vendor: string
	kind: 'host' | 'client' | null
	latency_ms: number | null
}

defineProps<{
	players: PlayerProfile[]
	lobbyCode: string
}>()

const emit = defineEmits<{
	(e: 'close'): void
}>()

// ---------------------------------------------------------------------------
// i18n
// ---------------------------------------------------------------------------

const messages = defineMessages({
	title: { id: 'app.game-link.report.title', defaultMessage: 'Report an Issue' },
	categoryLabel: { id: 'app.game-link.report.category', defaultMessage: 'Category' },
	descriptionLabel: { id: 'app.game-link.report.description', defaultMessage: 'Description' },
	descriptionHint: {
		id: 'app.game-link.report.description-hint',
		defaultMessage: 'Describe the issue you encountered',
	},
	playerLabel: {
		id: 'app.game-link.report.player',
		defaultMessage: 'Report player (optional)',
	},
	submitButton: { id: 'app.game-link.report.submit', defaultMessage: 'Submit Report' },
	underConstruction: {
		id: 'app.game-link.report.under-construction',
		defaultMessage: 'This feature is under construction and will be available in a future version.',
	},
	attachLogs: {
		id: 'app.game-link.report.attach-logs',
		defaultMessage: 'Auto-attach session logs',
	},
	sessionInfo: {
		id: 'app.game-link.report.session-info',
		defaultMessage: 'Session info',
	},
	lobbyCodeLabel: {
		id: 'app.game-link.report.lobby-code',
		defaultMessage: 'Lobby code',
	},
	playerCountLabel: {
		id: 'app.game-link.report.player-count',
		defaultMessage: 'Players in room',
	},
	categoryInappropriateContent: {
		id: 'app.game-link.report.category.inappropriate-content',
		defaultMessage: 'Inappropriate Content',
	},
	categoryMultiplayerDisruption: {
		id: 'app.game-link.report.category.multiplayer-disruption',
		defaultMessage: 'Multiplayer Disruption',
	},
	categoryCheatingFairness: {
		id: 'app.game-link.report.category.cheating-fairness',
		defaultMessage: 'Cheating & Fairness',
	},
	categoryFraudTrading: {
		id: 'app.game-link.report.category.fraud-trading',
		defaultMessage: 'Fraud & Trading',
	},
	categoryAccountSecurity: {
		id: 'app.game-link.report.category.account-security',
		defaultMessage: 'Account & Security',
	},
	categoryPlatformAbuse: {
		id: 'app.game-link.report.category.platform-abuse',
		defaultMessage: 'Platform Abuse',
	},
	categoryOther: {
		id: 'app.game-link.report.category.other',
		defaultMessage: 'Other',
	},
})

// ---------------------------------------------------------------------------
// Report Categories
// ---------------------------------------------------------------------------

const categories = computed(() => [
	{ id: 'inappropriate_content', label: formatMessage(messages.categoryInappropriateContent) },
	{ id: 'multiplayer_disruption', label: formatMessage(messages.categoryMultiplayerDisruption) },
	{ id: 'cheating_fairness', label: formatMessage(messages.categoryCheatingFairness) },
	{ id: 'fraud_trading', label: formatMessage(messages.categoryFraudTrading) },
	{ id: 'account_security', label: formatMessage(messages.categoryAccountSecurity) },
	{ id: 'platform_abuse', label: formatMessage(messages.categoryPlatformAbuse) },
	{ id: 'other', label: formatMessage(messages.categoryOther) },
])

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

const modal = ref<InstanceType<typeof Modal> | null>(null)
const selectedCategory = ref('')
const selectedPlayer = ref('')
const description = ref('')
const attachLogs = ref(true)
const isSubmitting = ref(false)

const canSubmit = computed(
	() => selectedCategory.value && description.value.trim().length > 0 && !isSubmitting.value,
)

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

function handleClose() {
	resetForm()
	emit('close')
}

function resetForm() {
	selectedCategory.value = ''
	selectedPlayer.value = ''
	description.value = ''
	attachLogs.value = true
	isSubmitting.value = false
}

async function submitReport() {
	if (!canSubmit.value) return
	isSubmitting.value = true

	// Simulate a brief delay for UX
	await new Promise((r) => setTimeout(r, 500))

	// Show "under construction" notification
	notificationManager.addNotification({
		title: formatMessage(messages.title),
		text: formatMessage(messages.underConstruction),
		type: 'info',
	})

	resetForm()
	modal.value?.hide()
	emit('close')
}
</script>

<template>
	<Modal ref="modal" :closable="true" :noblur="false" class="report-modal" @hide="handleClose">
		<template #title>
			<div class="title-row">
				<CircleAlertIcon class="title-icon" />
				<span>{{ formatMessage(messages.title) }}</span>
			</div>
		</template>

		<div class="report-body">
			<!-- Category selection -->
			<div class="form-group">
				<label class="form-label">{{ formatMessage(messages.categoryLabel) }}</label>
				<div class="category-grid">
					<button
						v-for="cat in categories"
						:key="cat.id"
						class="category-chip"
						:class="{ 'category-chip--selected': selectedCategory === cat.id }"
						@click="selectedCategory = cat.id"
					>
						{{ cat.label }}
					</button>
				</div>
			</div>

			<!-- Player selection (optional) -->
			<div v-if="players.length > 0" class="form-group">
				<label class="form-label">{{ formatMessage(messages.playerLabel) }}</label>
				<select v-model="selectedPlayer" class="form-select">
					<option value="">—</option>
					<option v-for="player in players" :key="player.machine_id" :value="player.machine_id">
						{{ player.name }} ({{ player.kind ?? '?' }})
					</option>
				</select>
			</div>

			<!-- Description -->
			<div class="form-group">
				<label class="form-label">{{ formatMessage(messages.descriptionLabel) }}</label>
				<textarea
					v-model="description"
					class="form-textarea"
					:placeholder="formatMessage(messages.descriptionHint)"
					maxlength="500"
					rows="3"
				/>
				<div class="char-count">{{ description.length }} / 500</div>
			</div>

			<!-- Auto-attach session info -->
			<div class="form-group">
				<label class="checkbox-row">
					<input v-model="attachLogs" type="checkbox" class="form-checkbox" />
					<span>{{ formatMessage(messages.attachLogs) }}</span>
				</label>
			</div>

			<!-- Session info preview (auto-traced) -->
			<div v-if="lobbyCode || players.length > 0" class="session-info">
				<div class="session-info-title">{{ formatMessage(messages.sessionInfo) }}</div>
				<div v-if="lobbyCode" class="session-info-item">
					<span class="session-info-label">{{ formatMessage(messages.lobbyCodeLabel) }}</span>
					<code class="session-info-value">{{ lobbyCode }}</code>
				</div>
				<div v-if="players.length > 0" class="session-info-item">
					<span class="session-info-label">{{ formatMessage(messages.playerCountLabel) }}</span>
					<span class="session-info-value">{{ players.length }}</span>
				</div>
			</div>

			<!-- Submit -->
			<div class="submit-row">
				<ButtonStyled color="brand" :disabled="!canSubmit">
					<button @click="submitReport">
						<CircleAlertIcon />
						{{ formatMessage(messages.submitButton) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Modal>
</template>

<style scoped>
.report-modal :deep(.modal-container) {
	max-width: 32rem;
}

.title-row {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.title-icon {
	width: 1.25rem;
	height: 1.25rem;
	color: var(--color-orange);
}

.report-body {
	display: flex;
	flex-direction: column;
	gap: 1rem;
}

/* ── Form ─────────────────────────────── */
.form-group {
	display: flex;
	flex-direction: column;
	gap: 0.35rem;
}

.form-label {
	font-size: 0.75rem;
	font-weight: 600;
	color: var(--color-text-secondary);
	text-transform: uppercase;
	letter-spacing: 0.04em;
}

.category-grid {
	display: flex;
	flex-wrap: wrap;
	gap: 0.4rem;
}

.category-chip {
	display: inline-flex;
	align-items: center;
	padding: 0.35rem 0.75rem;
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-lg);
	background: var(--color-bg);
	color: var(--color-text);
	font-size: 0.8rem;
	font-weight: 500;
	cursor: pointer;
	transition:
		border-color 0.15s,
		background-color 0.15s;
}

.category-chip:hover {
	border-color: var(--color-brand);
}

.category-chip--selected {
	border-color: var(--color-brand);
	background: var(--color-brand-bg);
	color: var(--color-brand);
}

.form-select {
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	padding: 0.5rem 0.75rem;
	font-size: 0.9rem;
	color: var(--color-text);
	outline: none;
	transition: border-color 0.15s;
}

.form-select:focus {
	border-color: var(--color-brand);
}

.form-textarea {
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
	padding: 0.5rem 0.75rem;
	font-size: 0.9rem;
	color: var(--color-text);
	outline: none;
	transition: border-color 0.15s;
	resize: vertical;
	min-height: 4rem;
	font-family: inherit;
}

.form-textarea:focus {
	border-color: var(--color-brand);
}

.char-count {
	font-size: 0.7rem;
	color: var(--color-text-secondary);
	text-align: right;
}

.checkbox-row {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-size: 0.85rem;
	color: var(--color-text);
	cursor: pointer;
}

.form-checkbox {
	accent-color: var(--color-brand);
}

/* ── Session Info ─────────────────────── */
.session-info {
	display: flex;
	flex-direction: column;
	gap: 0.35rem;
	padding: 0.5rem 0.75rem;
	background: var(--color-bg);
	border: 1px solid var(--color-button-border);
	border-radius: var(--radius-sm);
}

.session-info-title {
	font-size: 0.7rem;
	font-weight: 600;
	text-transform: uppercase;
	letter-spacing: 0.05em;
	color: var(--color-text-secondary);
	margin-bottom: 0.15rem;
}

.session-info-item {
	display: flex;
	align-items: center;
	justify-content: space-between;
	font-size: 0.8rem;
}

.session-info-label {
	color: var(--color-text-secondary);
}

.session-info-value {
	color: var(--color-text);
	font-weight: 500;
}

code.session-info-value {
	font-family: 'Fira Code', 'Cascadia Code', monospace;
	font-size: 0.8rem;
}

/* ── Submit ───────────────────────────── */
.submit-row {
	display: flex;
	justify-content: flex-end;
	padding-top: 0.5rem;
	border-top: 1px solid var(--color-button-border);
}
</style>
