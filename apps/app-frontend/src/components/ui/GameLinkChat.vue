<script setup lang="ts">
import { MessageIcon, SendIcon } from '@modrinth/assets'
import { ButtonStyled, Card, defineMessages, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'

const { formatMessage } = useVIntl()

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ChatMessage {
  id: string
  sender_id: string
  sender_name: string
  content: string
  timestamp: number
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

defineProps<{
  machineId: string
}>()

// ---------------------------------------------------------------------------
// i18n
// ---------------------------------------------------------------------------

const messages = defineMessages({
  chatTitle: { id: 'app.game-link.chat.title', defaultMessage: 'Room Chat' },
  chatEmpty: {
    id: 'app.game-link.chat.empty',
    defaultMessage: 'No messages yet. Say something!',
  },
  chatPlaceholder: {
    id: 'app.game-link.chat.placeholder',
    defaultMessage: 'Type a message...',
  },
  chatSend: { id: 'app.game-link.chat.send', defaultMessage: 'Send' },
})

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

const chatMessages = ref<ChatMessage[]>([])
const inputText = ref('')
const isSending = ref(false)
const lastPollTs = ref(0)
const chatContainer = ref<HTMLElement | null>(null)
const pollIntervalId = ref<ReturnType<typeof setInterval> | null>(null)

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

const canSend = computed(() => inputText.value.trim().length > 0 && !isSending.value)

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

function formatTime(ts: number): string {
  const d = new Date(ts)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function scrollToBottom() {
  nextTick(() => {
    if (chatContainer.value) {
      chatContainer.value.scrollTop = chatContainer.value.scrollHeight
    }
  })
}

async function pollMessages() {
  try {
    const newMessages = await invoke<ChatMessage[]>('plugin:link|link_poll_chat_messages', {
      sinceTs: lastPollTs.value,
    })
    if (newMessages.length > 0) {
      // Deduplicate by ID
      const existingIds = new Set(chatMessages.value.map((m) => m.id))
      const unique = newMessages.filter((m) => !existingIds.has(m.id))
      if (unique.length > 0) {
        chatMessages.value.push(...unique)
        lastPollTs.value = unique[unique.length - 1].timestamp
        scrollToBottom()
      } else if (newMessages.length > 0) {
        lastPollTs.value = newMessages[newMessages.length - 1].timestamp
      }
    }
  } catch {
    // ignore — likely not connected
  }
}

async function sendMessage() {
  if (!canSend.value) return
  isSending.value = true
  try {
    await invoke('plugin:link|link_send_chat_message', {
      content: inputText.value.trim(),
    })
    inputText.value = ''
    // Immediately poll to get our own message back
    await pollMessages()
  } catch {
    // ignore
  } finally {
    isSending.value = false
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

onMounted(async () => {
  // Initial poll
  await pollMessages()
  scrollToBottom()

  // Poll every 2 seconds
  pollIntervalId.value = setInterval(pollMessages, 2000)
})

onUnmounted(() => {
  if (pollIntervalId.value) {
    clearInterval(pollIntervalId.value)
    pollIntervalId.value = null
  }
})
</script>

<template>
	<Card class="chat-card">
		<template #header>
			<div class="section-header-row">
				<MessageIcon class="section-icon" />
				<h3>{{ formatMessage(messages.chatTitle) }}</h3>
			</div>
		</template>

		<div ref="chatContainer" class="chat-messages">
			<div v-if="chatMessages.length === 0" class="empty-hint">
				{{ formatMessage(messages.chatEmpty) }}
			</div>
			<div
				v-for="msg in chatMessages"
				:key="msg.id"
				class="chat-message"
				:class="{ 'chat-message--self': msg.sender_id === machineId }"
			>
				<div class="chat-message-header">
					<span
						class="chat-sender"
						:class="msg.sender_id === machineId ? 'chat-sender--self' : 'chat-sender--other'"
					>
						{{ msg.sender_name }}
					</span>
					<span class="chat-time">{{ formatTime(msg.timestamp) }}</span>
				</div>
				<div class="chat-content">{{ msg.content }}</div>
			</div>
		</div>

		<div class="chat-input-row">
			<input
				v-model="inputText"
				type="text"
				class="text-input chat-input"
				:placeholder="formatMessage(messages.chatPlaceholder)"
				:disabled="isSending"
				maxlength="500"
				@keyup.enter="sendMessage"
			/>
			<ButtonStyled color="brand" :disabled="!canSend">
				<button @click="sendMessage">
					<SendIcon />
					{{ formatMessage(messages.chatSend) }}
				</button>
			</ButtonStyled>
		</div>
	</Card>
</template>

<style scoped>
.chat-card :deep(.card) {
  display: flex;
  flex-direction: column;
}

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

/* ── Chat Messages ─────────────────────────── */
.chat-messages {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  max-height: 20rem;
  overflow-y: auto;
  padding: 0.5rem 0;
}

.chat-message {
  padding: 0.35rem 0.65rem;
  border-radius: var(--radius-sm);
  background: var(--color-bg);
  border: 1px solid var(--color-button-border);
  border-left: 3px solid var(--color-blue);
}

.chat-message--self {
  border-left-color: var(--color-brand);
}

.chat-message-header {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  margin-bottom: 0.15rem;
}

.chat-sender {
  font-weight: 600;
  font-size: 0.8rem;
}

.chat-sender--self {
  color: var(--color-brand);
}

.chat-sender--other {
  color: var(--color-blue);
}

.chat-time {
  font-size: 0.65rem;
  color: var(--color-text-secondary);
}

.chat-content {
  font-size: 0.85rem;
  color: var(--color-text);
  word-break: break-word;
}

.empty-hint {
  color: var(--color-text-secondary);
  font-size: 0.8rem;
  margin: 0;
  text-align: center;
  padding: 1rem 0;
}

/* ── Chat Input ────────────────────────────── */
.chat-input-row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  padding-top: 0.5rem;
  border-top: 1px solid var(--color-button-border);
}

.chat-input {
  flex: 1;
}
</style>
