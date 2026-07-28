<script setup lang="ts">
import { SaveIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'

import type { GameInstance } from '@/helpers/types'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.instance.notes.title',
		defaultMessage: 'Notes',
	},
	placeholder: {
		id: 'app.instance.notes.placeholder',
		defaultMessage: 'Add notes for this instance...',
	},
	saveButton: {
		id: 'app.instance.notes.save',
		defaultMessage: 'Save',
	},
	saved: {
		id: 'app.instance.notes.saved',
		defaultMessage: 'Notes saved',
	},
})

const props = defineProps<{
	instance: GameInstance
}>()

const notes = ref('')
const savedNotes = ref('')
const loading = ref(true)
const saving = ref(false)

onMounted(async () => {
	try {
		const result = await invoke<string | null>('plugin:instance|instance_get_notes', {
			instanceId: props.instance.id,
		})
		notes.value = result ?? ''
		savedNotes.value = notes.value
	} catch (err) {
		handleError(err as Error)
	} finally {
		loading.value = false
	}
})

const hasChanges = () => notes.value !== savedNotes.value

async function saveNotes() {
	if (!hasChanges()) return
	saving.value = true
	try {
		const content = notes.value.trim() || null
		await invoke('plugin:instance|instance_set_notes', {
			instanceId: props.instance.id,
			notes: content,
		})
		savedNotes.value = notes.value
	} catch (err) {
		handleError(err as Error)
	} finally {
		saving.value = false
	}
}
</script>

<template>
	<div class="notes-page">
		<div v-if="loading" class="flex min-h-[30vh] items-center justify-center">
			<SpinnerIcon class="h-8 w-8 animate-spin text-brand" />
		</div>
		<div v-else class="flex flex-col gap-4">
			<StyledInput
				v-model="notes"
				multiline
				:placeholder="formatMessage(messages.placeholder)"
				wrapper-class="w-full min-h-[16rem]"
				input-class="!min-h-[16rem] !resize-y"
			/>
			<div class="flex justify-end">
				<ButtonStyled color="brand" :disabled="!hasChanges() || saving">
					<button @click="saveNotes">
						<SpinnerIcon v-if="saving" class="animate-spin" />
						<SaveIcon v-else />
						{{ formatMessage(messages.saveButton) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</div>
</template>

<style scoped lang="scss">
.notes-page {
	display: flex;
	flex-direction: column;
	gap: var(--gap-lg);
	padding: var(--gap-md) 0;
}
</style>
