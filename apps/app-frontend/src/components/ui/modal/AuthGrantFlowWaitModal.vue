<script setup lang="ts">
import { LogInIcon, SpinnerIcon } from '@modrinth/assets'
import { NewModal as Modal } from '@modrinth/ui'
import { ref } from 'vue'

defineProps({
	onFlowCancel: {
		type: Function,
		default() {
			return async () => {}
		},
	},
})

const modal = ref()

function show() {
	modal.value.show()
}

function hide() {
	modal.value.hide()
}

defineExpose({ show, hide })
</script>
<template>
	<Modal ref="modal" @hide="onFlowCancel">
		<template #title>
			<span class="items-center gap-2 text-lg font-extrabold text-contrast">
				<LogInIcon /> Sign in
			</span>
		</template>

		<div class="flex justify-center gap-2">
			<SpinnerIcon class="w-12 h-12 animate-spin" />
		</div>
		<p class="text-sm text-secondary">
			Please sign in at the browser window that just opened to continue.
		</p>
	</Modal>
</template>
