<script setup lang="ts">
import {
	CpuIcon,
	DatabaseIcon,
	DownloadIcon,
	GameIcon,
	GlobeIcon,
	MemoryStickIcon,
	MonitorIcon,
	UploadIcon,
} from '@modrinth/assets'
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import { list } from '@/helpers/instance'
import { get_system_info, type SystemInfo } from '@/helpers/system'

const { handleError } = injectNotificationManager()

const { formatMessage } = useVIntl()

const messages = defineMessages({
  title: { id: 'app.home.system-status.title', defaultMessage: 'System status' },
  cpu: { id: 'app.home.system-status.cpu', defaultMessage: 'CPU' },
  memory: { id: 'app.home.system-status.memory', defaultMessage: 'Memory' },
  disk: { id: 'app.home.system-status.disk', defaultMessage: 'Disk' },
  gpu: { id: 'app.home.system-status.gpu', defaultMessage: 'GPU' },
  network: { id: 'app.home.system-status.network', defaultMessage: 'Network' },
  instances: { id: 'app.home.system-status.instances', defaultMessage: 'Instances' },
  available: { id: 'app.home.system-status.available', defaultMessage: 'available' },
})

const systemInfo = ref<SystemInfo | null>(null)
const instanceCount = ref(0)
const loading = ref(true)

function formatBytes(bytes: number, suffix = ''): string {
	if (bytes === 0) return `0 B${suffix}`
	const k = 1024
	const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
	const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1)
	const value = bytes / k ** i
	const decimals = i > 0 ? 1 : 0
	return `${value.toFixed(decimals)} ${sizes[i]}${suffix}`
}

function formatPercent(value: number): string {
	return `${Math.round(value)}%`
}

const memoryLabel = computed(() => {
	if (!systemInfo.value) return '—'
	return `${formatBytes(systemInfo.value.memory.usedBytes)} / ${formatBytes(systemInfo.value.memory.totalBytes)}`
})

const diskLabel = computed(() => {
	if (!systemInfo.value) return '—'
	return `${formatBytes(systemInfo.value.disk.availableBytes)} ${formatMessage(messages.available)}`
})

const gpuLabel = computed(() => {
	if (!systemInfo.value?.gpus?.length) return '—'
	const gpu = systemInfo.value.gpus[0]
	if (gpu.memoryTotalBytes) {
		return `${formatBytes(gpu.memoryTotalBytes)} VRAM`
	}
	return gpu.vendor
})

let refreshTimer: ReturnType<typeof setInterval> | null = null

async function fetchData() {
	try {
		const [info, instances] = await Promise.all([
			get_system_info().catch(handleError),
			list().catch(handleError),
		])
		systemInfo.value = info ?? null
		instanceCount.value = (instances ?? []).length
	} finally {
		loading.value = false
	}
}

onMounted(() => {
	fetchData()
	refreshTimer = setInterval(fetchData, 2000)
})

onUnmounted(() => {
	if (refreshTimer) clearInterval(refreshTimer)
})
</script>

<template>
	<div class="flex flex-col gap-2">
		<h3 class="m-0 text-base font-bold text-contrast flex items-center gap-2">
			<CpuIcon class="size-4" />
			{{ formatMessage(messages.title) }}
		</h3>
		<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
			<div class="stat-card p-3 rounded-xl bg-surface-2 flex flex-col gap-1">
				<div class="flex items-center gap-2 text-secondary">
					<CpuIcon class="size-4" />
					<span class="text-xs">{{ formatMessage(messages.cpu) }}</span>
				</div>
				<p class="m-0 text-2xl font-bold text-contrast">
					{{ loading ? '…' : formatPercent(systemInfo?.cpu.usagePercent ?? 0) }}
				</p>
				<p class="m-0 text-xs text-secondary truncate" :title="systemInfo?.cpu.brand">
					{{ loading ? '' : systemInfo?.cpu.brand }}
				</p>
			</div>
			<div class="stat-card p-3 rounded-xl bg-surface-2 flex flex-col gap-1">
				<div class="flex items-center gap-2 text-secondary">
					<MemoryStickIcon class="size-4" />
					<span class="text-xs">{{ formatMessage(messages.memory) }}</span>
				</div>
				<p class="m-0 text-2xl font-bold text-contrast">
					{{ loading ? '…' : formatPercent(systemInfo?.memory.usagePercent ?? 0) }}
				</p>
				<p class="m-0 text-xs text-secondary truncate">
					{{ loading ? '' : memoryLabel }}
				</p>
			</div>
			<div class="stat-card p-3 rounded-xl bg-surface-2 flex flex-col gap-1">
				<div class="flex items-center gap-2 text-secondary">
					<DatabaseIcon class="size-4" />
					<span class="text-xs">{{ formatMessage(messages.disk) }}</span>
				</div>
				<p class="m-0 text-2xl font-bold text-contrast">
					{{ loading ? '…' : formatPercent(systemInfo?.disk.usagePercent ?? 0) }}
				</p>
				<p class="m-0 text-xs text-secondary truncate" :title="systemInfo?.disk.mountPoint">
					{{ loading ? '' : diskLabel }} · {{ systemInfo?.disk.mountPoint }}
				</p>
			</div>
			<div class="stat-card p-3 rounded-xl bg-surface-2 flex flex-col gap-1">
				<div class="flex items-center gap-2 text-secondary">
					<MonitorIcon class="size-4" />
					<span class="text-xs">{{ formatMessage(messages.gpu) }}</span>
				</div>
				<p class="m-0 text-sm font-bold text-contrast truncate" :title="systemInfo?.gpus?.[0]?.name">
					{{ loading ? '…' : (systemInfo?.gpus?.[0]?.name ?? 'N/A') }}
				</p>
				<p class="m-0 text-xs text-secondary truncate">
					{{ loading ? '' : gpuLabel }}
				</p>
			</div>
		</div>
		<div class="stat-card p-3 rounded-xl bg-surface-2 flex flex-col gap-1">
			<div class="flex items-center gap-2 text-secondary">
				<GlobeIcon class="size-4" />
				<span class="text-xs">{{ formatMessage(messages.network) }}</span>
			</div>
			<div class="flex items-center gap-4">
				<p class="m-0 text-xl font-bold text-contrast flex items-center gap-1">
				<DownloadIcon class="size-4 text-secondary" />
				{{ loading ? '…' : formatBytes(systemInfo?.network.receivedBytesPerSecond ?? 0, '/s') }}
			</p>
			<p class="m-0 text-xl font-bold text-contrast flex items-center gap-1">
				<UploadIcon class="size-4 text-secondary" />
				{{ loading ? '' : formatBytes(systemInfo?.network.transmittedBytesPerSecond ?? 0, '/s') }}
			</p>
			</div>
		</div>
	</div>
</template>

<style scoped lang="scss">
.stat-card {
	border: 1px solid var(--color-bg);
}
</style>
