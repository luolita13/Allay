<script setup lang="ts">
import { ExternalIcon, ShieldCheckIcon } from '@modrinth/assets'
import { ButtonStyled, NewModal } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
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

function openPrivacyPolicy() {
	// 门户说明：联机大厅隐私政策外链占位。
	// 如未来需要替换为实际隐私政策 URL，仅需修改此处。
	openUrl('https://github.com/MCFAB/PCL-CE/blob/main/PRIVACY.md')
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
				<h2 class="agreement-title">联机大厅说明与条款</h2>
				<p class="agreement-subtitle">此处列出了联机大厅使用的相关服务文档及介绍</p>
			</div>

			<!-- Body -->
			<div class="agreement-body">
				<p class="agreement-intro">使用大厅功能即代表你同意下列条款：</p>

				<ul class="agreement-list">
					<li>我承诺严格遵守中国大陆相关法律法规，不会将大厅功能用于违法违规用途。</li>
					<li>我承诺使用大厅功能带来的一切风险自行承担。</li>
					<li>我已知晓并同意启动器收集经处理的本机识别码与其他信息并在必要时提供给执法部门。</li>
					<li>为保护未成年人个人信息，使用联机大厅前，我确认我已满十四周岁。</li>
				</ul>

				<p class="agreement-extra">另外，你还需要同意联机大厅相关隐私政策。</p>

				<!-- Privacy policy link -->
				<button class="privacy-link" type="button" @click="openPrivacyPolicy">
					<ExternalIcon class="privacy-link-icon" />
					<span>联机大厅相关隐私政策</span>
					<span class="privacy-link-info">了解启动器如何处理您的个人信息</span>
				</button>
			</div>

			<!-- Actions -->
			<div class="agreement-actions">
				<ButtonStyled type="brand">
					<button class="accept-btn" @click="acceptAgreement">我已阅读并同意</button>
				</ButtonStyled>
				<ButtonStyled type="transparent">
					<button class="decline-btn" @click="declineAgreement">不同意</button>
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

.agreement-extra {
	margin: 0.25rem 0 0;
	font-size: 0.82rem;
	line-height: 1.5;
	color: var(--color-text);
}

/* ── Privacy link ───────────────────────────── */
.privacy-link {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	width: 100%;
	padding: 0.65rem 0.75rem;
	background: var(--color-brand-bg);
	border: 1px solid var(--color-brand);
	border-radius: var(--radius-sm);
	cursor: pointer;
	text-align: left;
	transition:
		background-color 0.15s,
		border-color 0.15s;
	margin-top: 0.25rem;
}

.privacy-link:hover {
	background: color-mix(in srgb, var(--color-brand-bg) 70%, var(--color-brand) 10%);
}

.privacy-link-icon {
	width: 1rem;
	height: 1rem;
	color: var(--color-brand);
	flex-shrink: 0;
}

.privacy-link span {
	font-size: 0.85rem;
	font-weight: 600;
	color: var(--color-brand);
}

.privacy-link-info {
	margin-left: auto;
	font-size: 0.72rem;
	font-weight: 400;
	color: var(--color-text-secondary);
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
