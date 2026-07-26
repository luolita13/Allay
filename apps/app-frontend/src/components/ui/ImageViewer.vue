<template>
	<Teleport to="body">
		<div
			v-if="visible"
			class="image-viewer-overlay"
			@click.self="close"
			@keydown="onKeydown"
			tabindex="0"
			ref="overlayRef"
		>
			<div class="image-viewer-content">
				<img
					v-if="currentUrl"
					:src="currentUrl"
					:alt="currentName"
					class="image-viewer-img"
					:class="{ 'zoomed-in': zoomed }"
					@click.stop
				/>

				<div class="image-viewer-toolbar" @click.stop>
					<span
						v-if="currentName"
						class="image-viewer-name"
					>{{ currentName }}</span>

					<div class="image-viewer-actions">
						<ButtonStyled circular>
							<button
								@click="openExternally"
								:title="'Open in system viewer'"
							>
								<ExternalIcon aria-hidden="true" />
							</button>
						</ButtonStyled>
						<ButtonStyled circular>
							<button @click="zoomed = !zoomed">
								<ExpandIcon v-if="!zoomed" aria-hidden="true" />
								<ContractIcon v-else aria-hidden="true" />
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="images.length > 1"
							circular
						>
							<button @click="prevImage" :title="'Previous'">
								<LeftArrowIcon aria-hidden="true" />
							</button>
						</ButtonStyled>
						<ButtonStyled
							v-if="images.length > 1"
							circular
						>
							<button @click="nextImage" :title="'Next'">
								<RightArrowIcon aria-hidden="true" />
							</button>
						</ButtonStyled>
						<ButtonStyled circular>
							<button @click="close" :title="'Close'">
								<XIcon aria-hidden="true" />
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>
		</div>
	</Teleport>
</template>

<script setup lang="ts">
import {
	ContractIcon,
	ExpandIcon,
	ExternalIcon,
	LeftArrowIcon,
	RightArrowIcon,
	XIcon,
} from '@modrinth/assets'
import { ButtonStyled } from '@modrinth/ui'
import { readFile } from '@tauri-apps/plugin-fs'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { openPath } from '@/helpers/utils.js'

interface ImageItem {
	path: string
	name?: string
	url?: string // pre-generated blob URL (optional)
}

const props = defineProps<{
	images: ImageItem[]
	initialIndex?: number
}>()

const emit = defineEmits<{
	close: []
}>()

const visible = ref(false)
const activeIndex = ref(0)
const zoomed = ref(false)
const blobUrls = ref<Map<number, string>>(new Map())
const overlayRef = ref<HTMLElement | null>(null)

const currentName = computed(() => {
	const img = props.images[activeIndex.value]
	return img?.name ?? ''
})
const currentPath = computed(() => {
	const img = props.images[activeIndex.value]
	return img?.path ?? ''
})
const currentUrl = computed(() => {
	// Prefer pre-generated blob URL, then our generated one
	const img = props.images[activeIndex.value]
	if (!img) return null
	if (img.url) return img.url
	return blobUrls.value.get(activeIndex.value) ?? null
})

const MIME_TYPES: Record<string, string> = {
	png: 'image/png',
	jpg: 'image/jpeg',
	jpeg: 'image/jpeg',
	bmp: 'image/bmp',
	webp: 'image/webp',
	tiff: 'image/tiff',
	gif: 'image/gif',
	avif: 'image/avif',
}

function guessExt(path: string): string {
	return path.split('.').pop()?.toLowerCase() ?? 'png'
}

async function loadImage(index: number) {
	if (blobUrls.value.has(index)) return

	const img = props.images[index]
	if (!img || img.url) return

	try {
		const bytes = await readFile(img.path)
		const ext = guessExt(img.path)
		const blob = new Blob([bytes], { type: MIME_TYPES[ext] || 'image/png' })
		const url = URL.createObjectURL(blob)
		blobUrls.value.set(index, url)
		// Trigger reactivity
		blobUrls.value = new Map(blobUrls.value)
	} catch (e) {
		console.warn('[ImageViewer] failed to load:', img.path, e)
	}
}

function close() {
	visible.value = false
	// Small delay to let the fade-out animate before emitting
	setTimeout(() => emit('close'), 100)
}

function prevImage() {
	if (props.images.length <= 1) return
	activeIndex.value =
		activeIndex.value <= 0
			? props.images.length - 1
			: activeIndex.value - 1
	zoomed.value = false
}

function nextImage() {
	if (props.images.length <= 1) return
	activeIndex.value =
		activeIndex.value >= props.images.length - 1
			? 0
			: activeIndex.value + 1
	zoomed.value = false
}

function openExternally() {
	const path = currentPath.value
	if (path) openPath(path)
}

function onKeydown(e: KeyboardEvent) {
	switch (e.key) {
		case 'Escape':
			close()
			break
		case 'ArrowLeft':
			prevImage()
			break
		case 'ArrowRight':
			nextImage()
			break
	}
}

// Load current image when index changes
watch(activeIndex, (idx) => {
	void loadImage(idx)
})

function open(initialIndex: number) {
	activeIndex.value = initialIndex
	zoomed.value = false
	visible.value = true
	void loadImage(initialIndex)
	nextTick(() => {
		overlayRef.value?.focus()
	})
}

// Clean up blob URLs
onBeforeUnmount(() => {
	for (const url of blobUrls.value.values()) {
		URL.revokeObjectURL(url)
	}
})

defineExpose({ open })
</script>

<style scoped>
.image-viewer-overlay {
	position: fixed;
	z-index: 9999;
	inset: 0;
	background-color: rgba(0, 0, 0, 0.85);
	display: flex;
	justify-content: center;
	align-items: center;
	outline: none;
}

.image-viewer-content {
	position: relative;
	width: calc(100vw - 4rem);
	height: calc(100vh - 4rem);
	display: flex;
	justify-content: center;
	align-items: center;
}

.image-viewer-img {
	max-width: 100%;
	max-height: 100%;
	object-fit: contain;
	cursor: zoom-in;
	transition: transform 0.2s ease;
}

.image-viewer-img.zoomed-in {
	cursor: zoom-out;
	transform: scale(1.5);
}

.image-viewer-toolbar {
	position: absolute;
	bottom: 0;
	left: 0;
	right: 0;
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0.75rem 1rem;
	background: linear-gradient(to top, rgba(0, 0, 0, 0.6), transparent);
}

.image-viewer-name {
	color: #fff;
	font-size: 0.875rem;
	font-weight: 500;
	max-width: 50%;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.image-viewer-actions {
	display: flex;
	gap: 0.25rem;
}
</style>
