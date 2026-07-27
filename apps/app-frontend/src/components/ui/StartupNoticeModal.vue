<script setup lang="ts">
import { ModrinthIcon } from '@modrinth/assets'
import { ButtonStyled, Checkbox, NewModal } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { onMounted, ref } from 'vue'

const STORAGE_KEY_DISMISSED = 'modrinth-app-startup-notice-dismissed'

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const doNotShowAgain = ref(false)

onMounted(() => {
	const dismissed = localStorage.getItem(STORAGE_KEY_DISMISSED)
	if (dismissed !== 'true') {
		modal.value?.show()
	}
})

function dismiss() {
	if (doNotShowAgain.value) {
		localStorage.setItem(STORAGE_KEY_DISMISSED, 'true')
	}
	modal.value?.hide()
}

function openGitHub() {
	openUrl('https://github.com/luolita13')
}
</script>

<template>
	<NewModal
		ref="modal"
		:closable="false"
		:close-on-click-outside="false"
		:close-on-esc="false"
		max-width="520px"
		:hide-header="true"
	>
		<div class="flex flex-col items-center px-2 py-2 text-center">
			<!-- Icon -->
			<div
				class="mb-5 inline-flex items-center justify-center rounded-full p-3"
				style="background: radial-gradient(circle at center, var(--color-brand) 0%, var(--color-brand) 40%, transparent 70%);"
			>
				<ModrinthIcon class="size-9 text-white" />
			</div>

			<!-- Title -->
			<h2 class="mb-1.5 text-2xl font-bold text-contrast tracking-tight">
				Modrinth App
			</h2>
			<p class="m-0 mb-4 text-sm font-semibold tracking-wide text-brand uppercase">
				Community Custom Edition
			</p>

			<!-- Intro -->
			<p class="m-0 mb-6 max-w-md text-sm leading-relaxed text-secondary">
				A community-customized redistribution with additional features not
				included in the official Modrinth release.
			</p>

			<!-- Disclaimer -->
			<div
				class="mb-6 w-full rounded-xl border border-surface-5 bg-surface-1/50 px-5 py-4 text-left"
			>
				<ul class="m-0 space-y-2 pl-4 text-xs leading-relaxed text-tertiary list-disc">
					<li>
						Unofficial third-party build — not affiliated with or endorsed by
						Modrinth / Rinth, Inc.
					</li>
					<li>
						Original trademarks, assets, and code remain the property of
						their respective owners.
					</li>
					<li>
						Provided &ldquo;as is&rdquo; without warranty of any kind. Use
						at your own risk.
					</li>
				</ul>
			</div>

			<!-- Action -->
			<ButtonStyled type="brand">
				<button class="flex items-center gap-2 px-6" @click="dismiss">
					<svg
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M20 6 9 17l-5-5" />
					</svg>
					I understand and agree
				</button>
			</ButtonStyled>

			<!-- Don't show again -->
			<Checkbox
				v-model="doNotShowAgain"
				label="Don't show again"
				label-class="text-xs text-tertiary"
				class="mt-4"
			/>

			<!-- Credit -->
			<div class="mt-6 flex items-center gap-2 text-xs text-tertiary">
				<span>Customized by</span>
				<button
					class="text-secondary hover:text-brand hover:underline transition-colors bg-transparent border-0 p-0 cursor-pointer"
					@click="openGitHub"
				>
					github.com/luolita13
				</button>
			</div>
		</div>
	</NewModal>
</template>
