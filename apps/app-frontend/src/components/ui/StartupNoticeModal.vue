<script setup lang="ts">
import { CheckIcon, ModrinthIcon } from '@modrinth/assets'
import { ButtonStyled, NewModal } from '@modrinth/ui'
import { onMounted, ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'

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

function openRepo() {
	openUrl('https://github.com/luolita13/code')
}
</script>

<template>
	<NewModal
		ref="modal"
		header="Welcome to this customized build of Modrinth App"
		:closable="false"
		:close-on-click-outside="false"
		:close-on-esc="false"
		max-width="560px"
	>
		<div class="flex flex-col gap-4">
			<div class="flex items-center gap-3">
				<div class="p-3 rounded-xl bg-brand-highlight">
					<ModrinthIcon class="size-8 text-brand" />
				</div>
				<div>
					<p class="m-0 font-bold text-contrast text-lg">Modrinth App — Custom Edition</p>
					<p class="m-0 text-sm text-secondary">Unofficial modified distribution</p>
				</div>
			</div>

			<div class="text-sm text-primary leading-relaxed space-y-2">
				<p class="m-0">
					This application is a community-customized redistribution of the Modrinth App.
					It includes additional features and modifications that are not part of the
					official Modrinth release.
				</p>
				<p class="m-0">By continuing to use this software, you acknowledge that:</p>
				<ul class="m-0 pl-5 list-disc space-y-1">
					<li>
						This is an unofficial third-party build and is not affiliated with or endorsed
						by Modrinth / Rinth, Inc.
					</li>
					<li>
						All original Modrinth trademarks, assets, and code remain the property of their
						respective owners.
					</li>
					<li>
						Modifications are provided "as is" without warranty of any kind. Use at your
						own risk.
					</li>
					<li>
						This build may connect to Modrinth and other third-party services under their
						own terms of service and privacy policies.
					</li>
				</ul>
				<p class="m-0">
					Customized by
					<button
						class="text-brand hover:underline bg-transparent border-0 p-0 cursor-pointer"
						@click="openGitHub"
					>
						github.com/luolita13
					</button>
					.
				</p>
				<p class="m-0">
					Source repository:
					<button
						class="text-brand hover:underline bg-transparent border-0 p-0 cursor-pointer"
						@click="openRepo"
					>
						github.com/luolita13/code
					</button>
				</p>
			</div>

			<div class="flex items-center justify-between pt-2">
				<button
					class="flex items-center gap-2 text-sm text-secondary hover:text-contrast transition-colors"
					@click="doNotShowAgain = !doNotShowAgain"
				>
					<span
						class="size-4 rounded border border-secondary flex items-center justify-center"
						:class="{ 'bg-brand border-brand': doNotShowAgain }"
					>
						<CheckIcon v-if="doNotShowAgain" class="size-3 text-white" />
					</span>
					Don't show again
				</button>

				<ButtonStyled type="brand">
					<button class="flex items-center gap-2" @click="dismiss">
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
			</div>
		</div>
	</NewModal>
</template>
