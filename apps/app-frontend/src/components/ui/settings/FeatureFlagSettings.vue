<script setup lang="ts">
import { ButtonStyled, Toggle } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { useTheming } from '@/store/state'
import { DEFAULT_FEATURE_FLAGS, type FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()

const settings = ref(await getSettings())

interface FeatureFlagDefinition {
	key: FeatureFlag
	label: string
	description: string
	experimental?: boolean
}

const FEATURE_FLAGS: FeatureFlagDefinition[] = [
	{
		key: 'game_link',
		label: '联机功能（Game Link）',
		description:
			'在侧边栏显示联机入口，允许创建或加入局域网游戏。此功能处于实验阶段，可能不稳定。',
		experimental: true,
	},
	{
		key: 'worlds_tab',
		label: '世界标签页',
		description: '在侧边栏显示独立的世界（Worlds）入口。',
	},
	{
		key: 'worlds_in_home',
		label: '主页显示世界',
		description: '在主页“继续游玩”区域包含最近玩过的世界。',
	},
	{
		key: 'show_instance_play_time',
		label: '显示游戏时长',
		description: '在实例卡片上显示累计游玩时间。',
	},
	{
		key: 'skip_unknown_pack_warning',
		label: '跳过未知整合包警告',
		description: '安装非 Modrinth 托管的 .mrpack 文件时不再弹出风险提示。',
	},
	{
		key: 'skip_non_essential_warnings',
		label: '跳过非必要警告',
		description:
			'自动跳过低风险确认弹窗（如重复安装、普通删除、批量更新、取消关联整合包、修复提示等）。危险警告仍会显示。',
	},
]

function setFeatureFlag(key: FeatureFlag, value: boolean) {
	themeStore.featureFlags[key] = value
	settings.value.feature_flags[key] = value
}

function isDefault(key: FeatureFlag) {
	return themeStore.getFeatureFlag(key) === DEFAULT_FEATURE_FLAGS[key]
}

watch(
	settings,
	async () => {
		await setSettings(settings.value)
	},
	{ deep: true },
)
</script>
<template>
	<div class="flex flex-col gap-2.5 min-w-[600px]">
		<div
			v-for="option in FEATURE_FLAGS"
			:key="option.key"
			class="flex items-center justify-between"
		>
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast flex items-center gap-2">
					{{ option.label }}
					<span
						v-if="option.experimental"
						class="text-xs px-2 py-0.5 rounded-full bg-brand-highlight text-brand font-medium"
					>
						实验性
					</span>
				</h2>
				<p class="m-0 mt-1 text-sm text-secondary max-w-md">
					{{ option.description }}
				</p>
			</div>
			<div class="flex items-center gap-2 shrink-0">
				<ButtonStyled type="transparent">
					<button
						:disabled="isDefault(option.key)"
						@click="setFeatureFlag(option.key, DEFAULT_FEATURE_FLAGS[option.key])"
					>
						Reset to default
					</button>
				</ButtonStyled>
				<Toggle
					:id="`feature-flag-${option.key}`"
					:model-value="themeStore.getFeatureFlag(option.key)"
					@update:model-value="() => setFeatureFlag(option.key, !themeStore.getFeatureFlag(option.key))"
				/>
			</div>
		</div>
	</div>
</template>
