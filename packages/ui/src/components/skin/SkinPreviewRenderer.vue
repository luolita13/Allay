<template>
	<!-- eslint-disable vue/no-undef-components -->
	<div
		ref="skinPreviewContainer"
		class="relative w-full h-full overflow-visible cursor-grab"
		@click="handleCanvasClick"
	>
		<div
			class="absolute left-0 right-0 z-10 flex items-center justify-center pointer-events-none"
			:style="previewControlsPositionStyle"
		>
			<span
				class="flex items-center justify-center gap-1.5 text-base font-medium leading-6 text-primary"
			>
				<UnfoldHorizontalIcon class="size-5 shrink-0" />
				Drag to rotate
			</span>
		</div>
		<div
			v-if="$slots.subtitle"
			class="absolute left-0 right-0 z-10 flex items-center justify-center pointer-events-none"
			:style="subtitlePositionStyle"
		>
			<div ref="subtitleElement" class="pointer-events-auto" @click="ignoreControlClick">
				<slot name="subtitle" />
			</div>
		</div>
		<div
			v-if="nametag || $slots['nametag-badge']"
			class="absolute left-1/2 pointer-events-none z-10"
			:style="nametagStyle"
		>
			<div
				v-if="$slots['nametag-badge']"
				class="absolute bottom-[calc(100%+1rem)] left-1/2 flex -translate-x-1/2 items-center justify-center"
			>
				<slot name="nametag-badge" />
			</div>
			<div v-if="nametag" class="px-3 py-1 rounded-md font-minecraft text-gray nametag-bg">
				{{ nametagText }}
			</div>
		</div>

		<TresCanvas
			alpha
			:antialias="true"
			:dpr="rendererDpr"
			:renderer-options="{
				outputColorSpace: THREE.SRGBColorSpace,
				toneMapping: THREE.NoToneMapping,
				toneMappingExposure: 10.0,
			}"
			class="transition-opacity duration-500"
			:class="{ 'opacity-0': !isPreviewVisible, 'opacity-100': isPreviewVisible }"
			@pointerdown="onPointerDown"
			@pointermove="onPointerMove"
			@pointerup="onPointerUp"
			@pointerleave="onPointerUp"
		>
			<Suspense>
				<Group
					:rotation="animatedModelGroupRotation"
					:position="animatedModelGroupPosition"
					:scale="animatedModelGroupScale"
				>
					<Group :position="modelOffset">
						<primitive v-if="scene" :object="scene" />
					</Group>
				</Group>
			</Suspense>

			<!-- Particle background -->
			<primitive v-if="particleBgPoints" :object="particleBgPoints" />

			<Suspense>
				<TresMesh
					:position="spotlightPosition"
					:rotation="[-Math.PI / 2, 0, 0]"
					:scale="spotlightScale"
				>
					<TresCircleGeometry :args="[1, 128]" />
					<TresShaderMaterial v-bind="radialSpotlightShader" />
				</TresMesh>
			</Suspense>

			<TresPerspectiveCamera
				:make-default.camel="true"
				:fov="cameraConfig.fov"
				:position="cameraConfig.position"
				:look-at="cameraConfig.target"
			/>

			<TresAmbientLight :intensity="2" />
			<TresDirectionalLight :position="[-3, 4, -2]" :intensity="1.2" />
		</TresCanvas>

		<!-- Click particles overlay -->
		<div class="pointer-events-none absolute inset-0 overflow-visible z-20">
			<span
				v-for="p in clickParticleList"
				:key="p.id"
				class="click-particle"
				:style="{
					left: `${p.x}px`,
					top: `${p.y}px`,
					transform: `translate(-50%, -50%) rotate(${p.rotate}deg) scale(${p.scale})`,
				}"
			>
				{{ p.emoji }}
			</span>
		</div>

		<div v-if="showLoading" class="absolute inset-0 flex items-center justify-center">
			<div class="text-primary">Loading...</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { ClassicPlayerModel, SlimPlayerModel, UnfoldHorizontalIcon } from '@modrinth/assets'
import { TresCanvas, useRenderLoop } from '@tresjs/core'
import * as THREE from 'three'
import {
	computed,
	nextTick,
	onMounted,
	onUnmounted,
	ref,
	toRef,
	useSlots,
	useTemplateRef,
	watch,
} from 'vue'

import type {
	SkinPreviewAnimationConfig,
	SkinPreviewFitPadding,
	SkinPreviewFraming,
	SkinPreviewTuple,
} from '#ui/composables/skin-rendering'
import {
	useSkinPreviewAnimation,
	useSkinPreviewControls,
	useSkinPreviewFit,
	useSkinPreviewLoading,
	useSkinPreviewScene,
} from '#ui/composables/skin-rendering'

import { useDynamicFontSize } from '../../composables'
import { createRadialSpotlightShader, syncDamageFlashShader } from './skin-preview-shader'

const props = withDefaults(
	defineProps<{
		textureSrc: string
		capeSrc?: string
		variant?: 'SLIM' | 'CLASSIC' | 'UNKNOWN'
		nametag?: string
		fit?: boolean
		lockFit?: boolean
		framing?: SkinPreviewFraming
		fitZoom?: number
		fitPadding?: Partial<SkinPreviewFitPadding>
		/** @deprecated Manual framing fallback. */
		scale?: number
		/** @deprecated Manual framing fallback, or auto-fit FOV override when fit=true. */
		fov?: number
		initialRotation?: number
		animationConfig?: SkinPreviewAnimationConfig
		// Visual effect toggles (driven by app theme store)
		clickParticles?: boolean
		headTracking?: boolean
		particleBackground?: boolean
	}>(),
	{
		variant: 'CLASSIC',
		capeSrc: undefined,
		initialRotation: 15.75,
		nametag: undefined,
		fit: undefined,
		lockFit: true,
		framing: 'page',
		fitZoom: 1,
		animationConfig: () => ({
			baseAnimation: 'idle',
			randomAnimations: ['idle_sub_1', 'idle_sub_2', 'idle_sub_3'],
			randomAnimationInterval: 8000,
			transitionDuration: 0.2,
		}),
		clickParticles: false,
		headTracking: false,
		particleBackground: false,
	},
)

const skinPreviewContainer = useTemplateRef<HTMLElement>('skinPreviewContainer')
const subtitleElement = useTemplateRef<HTMLElement>('subtitleElement')
const slots = useSlots()
const nametagText = computed(() => props.nametag)
const hasSubtitle = computed(() => Boolean(slots.subtitle))
const hasNametagBadge = computed(() => Boolean(slots['nametag-badge']))
const isSubtitleWrapped = ref(false)
const selectedModelSrc = computed(() =>
	props.variant === 'SLIM' ? SlimPlayerModel : ClassicPlayerModel,
)

let subtitleResizeObserver: ResizeObserver | undefined

function getSubtitleLayoutRoot(element: HTMLElement) {
	const elementChildren = Array.from(element.children).filter(
		(child): child is HTMLElement => child instanceof HTMLElement,
	)

	return elementChildren.length === 1 ? elementChildren[0] : element
}

function updateSubtitleWrapped() {
	const element = subtitleElement.value
	if (!element) {
		isSubtitleWrapped.value = false
		return
	}

	const layoutRoot = getSubtitleLayoutRoot(element)
	const children = Array.from(layoutRoot.children).filter(
		(child): child is HTMLElement => child instanceof HTMLElement,
	)

	if (children.length < 2) {
		isSubtitleWrapped.value = false
		return
	}

	const firstTop = children[0].getBoundingClientRect().top
	isSubtitleWrapped.value = children.some(
		(child) => Math.abs(child.getBoundingClientRect().top - firstTop) > 1,
	)
}

function observeSubtitleElement() {
	subtitleResizeObserver?.disconnect()

	const element = subtitleElement.value
	if (!element) {
		isSubtitleWrapped.value = false
		return
	}

	const layoutRoot = getSubtitleLayoutRoot(element)

	subtitleResizeObserver = new ResizeObserver(updateSubtitleWrapped)
	subtitleResizeObserver.observe(element)
	if (layoutRoot !== element) {
		subtitleResizeObserver.observe(layoutRoot)
	}

	void nextTick(updateSubtitleWrapped)
}

const {
	cleanupAnimationState,
	clickImpulseOffsetX,
	clickImpulseRotationZ,
	clickImpulseScaleX,
	clickImpulseScaleY,
	currentAnimation,
	damageFlashIntensity,
	getAvailableAnimations,
	initializeAnimations,
	playAnimation,
	playClickInteraction,
	stopAnimations,
} = useSkinPreviewAnimation(toRef(props, 'animationConfig'))

const {
	ignoreControlClick,
	modelRotation,
	onCanvasClick,
	onPointerDown,
	onPointerMove,
	onPointerUp,
} = useSkinPreviewControls({
	initialRotation: toRef(props, 'initialRotation'),
	onClickWithoutDrag: () => {
		playClickInteraction()
	},
})

const { isModelLoaded, isTextureLoaded, modelCenter, modelSize, scene } = useSkinPreviewScene({
	selectedModelSrc,
	textureSrc: toRef(props, 'textureSrc'),
	capeSrc: toRef(props, 'capeSrc'),
	initializeAnimations,
	cleanupAnimationState,
})

function syncDamageFlashShaderMaterials() {
	syncDamageFlashShader(scene.value, damageFlashIntensity.value)
}

const {
	cameraConfig,
	fitEnabled,
	hasResolvedFit,
	modelGroupPosition,
	modelGroupScale,
	modelOffset,
	nametagTop,
	previewControlsPositionStyle,
	spotlightPosition,
	spotlightScale,
	subtitlePositionStyle,
} = useSkinPreviewFit({
	containerElement: computed(() => skinPreviewContainer.value),
	fit: toRef(props, 'fit'),
	lockFit: toRef(props, 'lockFit'),
	framing: toRef(props, 'framing'),
	fitZoom: toRef(props, 'fitZoom'),
	fitPadding: toRef(props, 'fitPadding'),
	scale: toRef(props, 'scale'),
	fov: toRef(props, 'fov'),
	modelRotation,
	nametag: toRef(props, 'nametag'),
	hasSubtitle,
	hasNametagBadge,
	subtitleWrapped: isSubtitleWrapped,
	modelCenter,
	modelSize,
	isModelLoaded,
})

const rendererDpr: [number, number] = [1, 1.5]
const radialSpotlightShader = createRadialSpotlightShader()
const isReady = computed(() => isModelLoaded.value && isTextureLoaded.value && hasResolvedFit.value)
const { isPreviewVisible, showLoading } = useSkinPreviewLoading(isReady)

onMounted(observeSubtitleElement)
onMounted(() => {
	window.addEventListener('mousemove', onGlobalMouseMove)
})

watch(hasSubtitle, () => nextTick(observeSubtitleElement), { flush: 'post' })
watch(scene, syncDamageFlashShaderMaterials, { immediate: true })
watch(damageFlashIntensity, syncDamageFlashShaderMaterials)

onUnmounted(() => {
	subtitleResizeObserver?.disconnect()
	window.removeEventListener('mousemove', onGlobalMouseMove)
})

const { fontSize: nametagFontSize } = useDynamicFontSize({
	containerElement: skinPreviewContainer,
	text: nametagText,
	baseFontSize: 1.8,
	minFontSize: 1.25,
	maxFontSize: 2,
	padding: 24,
	fontFamily: 'inherit',
})

const nametagStyle = computed(() => ({
	fontSize: nametagFontSize.value,
	top: nametagTop.value,
	transform: fitEnabled.value ? 'translate(-50%, calc(-100% - 0.75rem))' : 'translateX(-50%)',
}))

const animatedModelGroupRotation = computed<SkinPreviewTuple>(() => [
	0,
	modelRotation.value,
	clickImpulseRotationZ.value,
])

const animatedModelGroupPosition = computed<SkinPreviewTuple>(() => {
	const [x, y, z] = modelGroupPosition.value
	return [x + clickImpulseOffsetX.value, y, z]
})

const animatedModelGroupScale = computed<SkinPreviewTuple>(() => {
	const [x, y, z] = modelGroupScale.value
	return [x * clickImpulseScaleX.value, y * clickImpulseScaleY.value, z]
})

// === Effect: Click particles (DOM overlay) ===
interface ClickParticle {
	id: number
	x: number
	y: number
	emoji: string
	rotate: number
	scale: number
}
const clickParticleList = ref<ClickParticle[]>([])
let particleIdCounter = 0
const PARTICLE_EMOJIS = ['⭐', '✨', '💫', '★', '✦', '◆', '▲']

function spawnClickParticle(x: number, y: number) {
	if (!props.clickParticles) return
	const id = particleIdCounter++
	clickParticleList.value.push({
		id,
		x,
		y,
		emoji: PARTICLE_EMOJIS[Math.floor(Math.random() * PARTICLE_EMOJIS.length)],
		rotate: (Math.random() - 0.5) * 90,
		scale: 0.8 + Math.random() * 0.6,
	})
	// Auto-remove after animation
	setTimeout(() => {
		clickParticleList.value = clickParticleList.value.filter((p) => p.id !== id)
	}, 900)
}

function handleCanvasClick(event: MouseEvent) {
	if (props.clickParticles && skinPreviewContainer.value) {
		const rect = skinPreviewContainer.value.getBoundingClientRect()
		spawnClickParticle(event.clientX - rect.left, event.clientY - rect.top)
	}
	onCanvasClick()
}

// === Effect: Head tracking ===
const headObject = ref<THREE.Object3D | null>(null)
const mouseTargetX = ref(0)
const mouseTargetY = ref(0)

watch(scene, (newScene) => {
	if (!newScene) {
		headObject.value = null
		return
	}
	headObject.value = newScene.getObjectByName('Head') ?? null
})

// Global mouse tracking — head follows cursor anywhere on screen
function onGlobalMouseMove(event: MouseEvent) {
	if (!props.headTracking || !skinPreviewContainer.value) return
	const rect = skinPreviewContainer.value.getBoundingClientRect()
	// Compute mouse position relative to the container center, normalized to -1..1
	// using the larger of width/height so diagonal movement feels natural.
	const centerX = rect.left + rect.width / 2
	const centerY = rect.top + rect.height / 2
	const span = Math.max(rect.width, rect.height, 200)
	mouseTargetX.value = Math.max(-1.5, Math.min(1.5, (event.clientX - centerX) / span))
	mouseTargetY.value = Math.max(-1.5, Math.min(1.5, -(event.clientY - centerY) / span))
}

const { onLoop: onHeadTrackLoop } = useRenderLoop()
onHeadTrackLoop(() => {
	if (!props.headTracking || !headObject.value) return
	// Lerp head rotation towards mouse target
	const targetY = mouseTargetX.value * 0.6 // yaw
	const targetX = mouseTargetY.value * 0.4 // pitch
	headObject.value.rotation.y += (targetY - headObject.value.rotation.y) * 0.1
	headObject.value.rotation.x += (targetX - headObject.value.rotation.x) * 0.1
})

// === Effect: Particle background (Three.js Points) ===
const particleBgPoints = ref<THREE.Points | null>(null)
const PARTICLE_BG_COUNT = 120

function createParticleBackground() {
	const count = PARTICLE_BG_COUNT
	const geometry = new THREE.BufferGeometry()
	const positions = new Float32Array(count * 3)
	const speeds = new Float32Array(count)
	const colors = new Float32Array(count * 3)
	// Accent-ish palette: white, cyan, magenta, gold — visible on any background
	const palette = [
		[1.0, 1.0, 1.0],
		[0.4, 0.9, 1.0],
		[1.0, 0.5, 0.9],
		[1.0, 0.85, 0.3],
	]
	for (let i = 0; i < count; i++) {
		positions[i * 3] = (Math.random() - 0.5) * 10
		positions[i * 3 + 1] = Math.random() * 10 - 2
		positions[i * 3 + 2] = (Math.random() - 0.5) * 5 - 2
		speeds[i] = 0.004 + Math.random() * 0.006
		const c = palette[Math.floor(Math.random() * palette.length)]
		colors[i * 3] = c[0]
		colors[i * 3 + 1] = c[1]
		colors[i * 3 + 2] = c[2]
	}
	geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3))
	geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3))

	// Use a circular sprite texture for softer particles
	const sprite = makeCircleSprite()
	const material = new THREE.PointsMaterial({
		size: 0.18,
		map: sprite,
		transparent: true,
		opacity: 0.85,
		sizeAttenuation: true,
		depthWrite: false,
		blending: THREE.AdditiveBlending,
		vertexColors: true,
	})

	const points = new THREE.Points(geometry, material)
	points.userData.speeds = speeds
	particleBgPoints.value = points
}

function makeCircleSprite(): THREE.Texture {
	const size = 64
	const canvas = document.createElement('canvas')
	canvas.width = size
	canvas.height = size
	const ctx = canvas.getContext('2d')!
	const grad = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2)
	grad.addColorStop(0, 'rgba(255,255,255,1)')
	grad.addColorStop(0.4, 'rgba(255,255,255,0.7)')
	grad.addColorStop(1, 'rgba(255,255,255,0)')
	ctx.fillStyle = grad
	ctx.fillRect(0, 0, size, size)
	const tex = new THREE.CanvasTexture(canvas)
	tex.needsUpdate = true
	return tex
}

function disposeParticleBackground() {
	if (!particleBgPoints.value) return
	particleBgPoints.value.geometry.dispose()
	;(particleBgPoints.value.material as THREE.Material).dispose()
	particleBgPoints.value = null
}

watch(
	() => props.particleBackground,
	(enabled) => {
		if (enabled) {
			createParticleBackground()
		} else {
			disposeParticleBackground()
		}
	},
)

const { onLoop: onParticleBgLoop } = useRenderLoop()
onParticleBgLoop(() => {
	if (!particleBgPoints.value) return
	const points = particleBgPoints.value
	const positions = points.geometry.attributes.position.array as Float32Array
	const speeds = points.userData.speeds as Float32Array
	for (let i = 0; i < positions.length / 3; i++) {
		positions[i * 3 + 1] += speeds[i]
		if (positions[i * 3 + 1] > 6) {
			positions[i * 3 + 1] = -2
			positions[i * 3] = (Math.random() - 0.5) * 8
		}
	}
	points.geometry.attributes.position.needsUpdate = true
})

onUnmounted(() => {
	disposeParticleBackground()
})

defineExpose({
	playAnimation,
	playClickInteraction,
	stopAnimations,
	getAvailableAnimations,
	getCurrentAnimation: () => currentAnimation.value,
})
</script>

<style scoped lang="scss">
.nametag-bg {
	background:
		linear-gradient(308.68deg, rgba(50, 50, 50, 0.2) -52.46%, rgba(100, 100, 100, 0.2) 94.75%),
		rgba(0, 0, 0, 0.2);
	box-shadow:
		inset -0.5px -0.5px 0px rgba(0, 0, 0, 0.25),
		inset 0.5px 0.5px 0px rgba(255, 255, 255, 0.05);
}

.click-particle {
	position: absolute;
	font-size: 1.25rem;
	font-weight: 700;
	color: var(--color-brand, #ff496e);
	text-shadow: 0 0 8px currentColor;
	pointer-events: none;
	user-select: none;
	animation: click-particle-rise 0.9s cubic-bezier(0.25, 0.46, 0.45, 0.94) forwards;
	will-change: transform, opacity;
}

@keyframes click-particle-rise {
	0% {
		opacity: 1;
	}
	100% {
		opacity: 0;
		transform: translate(-50%, calc(-50% - 60px)) rotate(var(--rotate, 45deg))
			scale(0.3);
	}
}
</style>
