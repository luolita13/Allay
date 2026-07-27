<template>
	<Modal ref="detectJavaModal" :header="formatMessage(messages.header)">
		<div class="flex flex-col gap-4">
			<Table :columns="javaInstallColumns" :data="chosenInstallOptions" row-key="path">
				<template #cell-version="{ value }">
					<span class="font-semibold text-primary">{{ value }}</span>
				</template>
				<template #cell-path="{ value }">
					<span v-tooltip="value" class="block truncate font-mono text-xs">{{ value }}</span>
				</template>
				<template #cell-actions="{ row }">
					<div class="flex items-center justify-end">
						<ButtonStyled v-if="currentSelected.path === row.path">
							<button class="!shadow-none" disabled><CheckIcon /> {{ formatMessage(messages.selected) }}</button>
						</ButtonStyled>
						<ButtonStyled v-else>
							<button class="!shadow-none" @click="setJavaInstall(row)"><PlusIcon /> {{ formatMessage(messages.select) }}</button>
						</ButtonStyled>
					</div>
				</template>
				<template #empty-state>
					<div class="p-4 text-secondary">{{ formatMessage(messages.notFound) }}</div>
				</template>
			</Table>
			<div class="flex justify-end">
				<ButtonStyled type="outlined">
					<button
						class="!shadow-none !border-surface-4 !border"
						@click="$refs.detectJavaModal.hide()"
					>
						<XIcon />
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Modal>
</template>
<script setup>
import { CheckIcon, PlusIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	NewModal as Modal,
	Table,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import { trackEvent } from '@/helpers/analytics'
import { find_filtered_jres } from '@/helpers/jre.js'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'app.java-detection.header', defaultMessage: 'Select java version' },
	selected: { id: 'app.java-detection.selected', defaultMessage: 'Selected' },
	select: { id: 'app.java-detection.select', defaultMessage: 'Select' },
	notFound: { id: 'app.java-detection.not-found', defaultMessage: 'No java installations found!' },
	cancel: { id: 'app.java-detection.cancel', defaultMessage: 'Cancel' },
	version: { id: 'app.java-detection.column.version', defaultMessage: 'Version' },
	path: { id: 'app.java-detection.column.path', defaultMessage: 'Path' },
	actions: { id: 'app.java-detection.column.actions', defaultMessage: 'Actions' },
})

const chosenInstallOptions = ref([])
const detectJavaModal = ref(null)
const currentSelected = ref({})
const javaInstallColumns = [
	{ key: 'version', label: formatMessage(messages.version), width: '9rem' },
	{ key: 'path', label: formatMessage(messages.path) },
	{ key: 'actions', label: formatMessage(messages.actions), align: 'right', width: '10rem' },
]

defineExpose({
	show: async (version, currentSelectedJava) => {
		chosenInstallOptions.value = await find_filtered_jres(version).catch(handleError)

		currentSelected.value = currentSelectedJava
		if (!currentSelected.value) {
			currentSelected.value = { path: '', version: '' }
		}

		detectJavaModal.value.show()
	},
})

const emit = defineEmits(['submit'])

function setJavaInstall(javaInstall) {
	emit('submit', javaInstall)
	detectJavaModal.value.hide()
	trackEvent('JavaAutoDetect', {
		path: javaInstall.path,
		version: javaInstall.version,
	})
}
</script>
