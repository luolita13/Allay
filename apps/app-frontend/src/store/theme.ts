import { defineStore } from 'pinia'

import type { InstalledThemePack } from '@/helpers/theme_pack'
import { installFromPath, listInstalled as getInstalledThemePacks, uninstall as uninstallThemePack } from '@/helpers/theme_pack'

let systemThemeMq: MediaQueryList | null = null

const LS_KEY_BG_PATH = 'allay-app-background-image-path'
const LS_KEY_BG_BLUR = 'allay-app-background-blur'
const LS_KEY_BG_OPACITY = 'allay-app-background-opacity'
const LS_KEY_ACCENT_COLOR = 'allay-app-accent-color'
const LS_KEY_CUSTOM_ACCENT_COLOR = 'allay-app-custom-accent-color'

let accentColorSaveTimer: ReturnType<typeof setTimeout> | null = null
const LS_KEY_ACTIVE_THEME_PACK = 'allay-app-active-theme-pack'
const LS_KEY_APP_IMAGE_VIEWER = 'allay-app-image-viewer'
const LS_KEY_SETTINGS_AS_PAGE = 'allay-app-settings-as-page'
const LS_KEY_SKIN_CLICK_PARTICLES = 'allay-app-skin-click-particles'
const LS_KEY_SKIN_HEAD_TRACKING = 'allay-app-skin-head-tracking'
const LS_KEY_SKIN_PARTICLE_BG = 'allay-app-skin-particle-bg'

export const DEFAULT_FEATURE_FLAGS = {
	project_background: false,
	page_path: false,
	worlds_tab: false,
	worlds_in_home: true,
	server_project_qa: false,
	show_version_environment_column: false,
	server_ram_as_bytes_always_on: false,
	always_show_app_controls: false,
	skip_non_essential_warnings: false,
	skip_unknown_pack_warning: false,
	pride_fundraiser: true,
	i18n_debug: false,
	show_instance_play_time: true,
	// Home page section visibility
	home_show_jump_back_in: true,
	home_show_discover_modpacks: true,
	home_show_discover_mods: true,
	home_show_update_reminders: true,
	home_show_system_status: true,
	home_show_recent_screenshots: true,
	home_show_random_mods: true,
}

export const THEME_OPTIONS = ['dark', 'light', 'oled', 'system'] as const
export const ACCENT_COLOR_OPTIONS = ['orange', 'green', 'blue', 'purple'] as const

export type AccentColor = (typeof ACCENT_COLOR_OPTIONS)[number]

export type FeatureFlag = keyof typeof DEFAULT_FEATURE_FLAGS
export type FeatureFlags = Record<FeatureFlag, boolean>
export type ColorTheme = (typeof THEME_OPTIONS)[number]

export type ThemeStore = {
	selectedTheme: ColorTheme
	selectedAccentColor: AccentColor
	customAccentColor: string | null
	advancedRendering: boolean
	hideNametagSkinsPage: boolean
	toggleSidebar: boolean

	devMode: boolean
	featureFlags: FeatureFlags

	backgroundImagePath: string | null
	backgroundBlur: number
	backgroundOpacity: number

	// Theme pack system
	installedThemePacks: InstalledThemePack[]
	activeThemePackId: string | null

	// Image viewer: use in-app lightbox instead of system default
	useAppImageViewer: boolean

	// Settings: render as full page instead of modal
	settingsAsPage: boolean

	// Skin preview visual effects
	skinClickParticles: boolean
	skinHeadTracking: boolean
	skinParticleBackground: boolean
}

export const DEFAULT_THEME_STORE: ThemeStore = {
	selectedTheme: 'dark',
	selectedAccentColor: 'blue',
	customAccentColor: null,
	advancedRendering: true,
	hideNametagSkinsPage: false,
	toggleSidebar: false,

	devMode: false,
	featureFlags: DEFAULT_FEATURE_FLAGS,

	backgroundImagePath: null,
	backgroundBlur: 20,
	backgroundOpacity: 65,

	installedThemePacks: [],
	activeThemePackId: null,

	useAppImageViewer: false,
	settingsAsPage: false,
	skinClickParticles: true,
	skinHeadTracking: true,
	skinParticleBackground: false,
}

export const useTheming = defineStore('themeStore', {
	state: () => {
		const stored = { ...DEFAULT_THEME_STORE }
		// Restore background image from localStorage
		const savedPath = localStorage.getItem(LS_KEY_BG_PATH)
		if (savedPath) {
			stored.backgroundImagePath = savedPath
		}
		const savedBlur = localStorage.getItem(LS_KEY_BG_BLUR)
		if (savedBlur) {
			const parsed = parseInt(savedBlur, 10)
			stored.backgroundBlur = Number.isNaN(parsed) ? 20 : parsed
		}
		const savedOpacity = localStorage.getItem(LS_KEY_BG_OPACITY)
		if (savedOpacity) {
			const parsed = parseInt(savedOpacity, 10)
			stored.backgroundOpacity = Number.isNaN(parsed) ? 65 : parsed
		}
		const savedAccentColor = localStorage.getItem(LS_KEY_ACCENT_COLOR) as AccentColor | null
		if (savedAccentColor && ACCENT_COLOR_OPTIONS.includes(savedAccentColor)) {
			stored.selectedAccentColor = savedAccentColor
		}
		const savedCustomAccent = localStorage.getItem(LS_KEY_CUSTOM_ACCENT_COLOR)
		if (savedCustomAccent) {
			stored.customAccentColor = savedCustomAccent
		}
		const savedActiveThemePack = localStorage.getItem(LS_KEY_ACTIVE_THEME_PACK)
		if (savedActiveThemePack) {
			stored.activeThemePackId = savedActiveThemePack
		}
		const savedImageViewer = localStorage.getItem(LS_KEY_APP_IMAGE_VIEWER)
		if (savedImageViewer !== null) {
			stored.useAppImageViewer = savedImageViewer === 'true'
		}
		const savedSettingsAsPage = localStorage.getItem(LS_KEY_SETTINGS_AS_PAGE)
		if (savedSettingsAsPage !== null) {
			stored.settingsAsPage = savedSettingsAsPage === 'true'
		}
		const savedSkinClickParticles = localStorage.getItem(LS_KEY_SKIN_CLICK_PARTICLES)
		if (savedSkinClickParticles !== null) {
			stored.skinClickParticles = savedSkinClickParticles === 'true'
		}
		const savedSkinHeadTracking = localStorage.getItem(LS_KEY_SKIN_HEAD_TRACKING)
		if (savedSkinHeadTracking !== null) {
			stored.skinHeadTracking = savedSkinHeadTracking === 'true'
		}
		const savedSkinParticleBg = localStorage.getItem(LS_KEY_SKIN_PARTICLE_BG)
		if (savedSkinParticleBg !== null) {
			stored.skinParticleBackground = savedSkinParticleBg === 'true'
		}
		return stored
	},
	actions: {
		setThemeState(newTheme: ColorTheme) {
			if (THEME_OPTIONS.includes(newTheme)) {
				this.selectedTheme = newTheme
			} else {
				console.warn('Selected theme is not present. Check themeOptions.')
			}

			this.setThemeClass()
		},
		setAccentColor(newAccentColor: AccentColor) {
			if (ACCENT_COLOR_OPTIONS.includes(newAccentColor)) {
				this.selectedAccentColor = newAccentColor
			} else {
				console.warn('Selected accent color is not available.')
			}

			// Clear custom color when selecting a preset
			this.customAccentColor = null
			localStorage.removeItem(LS_KEY_CUSTOM_ACCENT_COLOR)

			const html = document.documentElement
			// Clear any inline custom accent overrides
			html.style.removeProperty('--color-brand')
			html.style.removeProperty('--color-brand-highlight')
			html.style.removeProperty('--color-brand-shadow')

			for (const accentColor of ACCENT_COLOR_OPTIONS) {
				html.classList.remove(`accent-${accentColor}`)
			}
			html.classList.add(`accent-${this.selectedAccentColor}`)
			localStorage.setItem(LS_KEY_ACCENT_COLOR, this.selectedAccentColor)
		},
		setCustomAccentColor(hex: string) {
			this.customAccentColor = hex

			const html = document.documentElement
			// Remove preset accent classes
			for (const accentColor of ACCENT_COLOR_OPTIONS) {
				html.classList.remove(`accent-${accentColor}`)
			}

			// Apply custom color via inline CSS variables.
			// Setting CSS custom properties on <html> is fast and does
			// not block the main thread — the browser batches style
			// recalculation asynchronously. This gives smooth real-time
			// preview during color picker scrubbing.
			html.style.setProperty('--color-brand', hex)
			html.style.setProperty(
				'--color-brand-highlight',
				`color-mix(in srgb, ${hex} 70%, white)`,
			)
			html.style.setProperty(
				'--color-brand-shadow',
				`color-mix(in srgb, ${hex} 68%, transparent)`,
			)

			// Debounce only the synchronous localStorage write (the
			// only truly blocking operation) to avoid frame drops.
			if (accentColorSaveTimer !== null) {
				clearTimeout(accentColorSaveTimer)
			}
			accentColorSaveTimer = setTimeout(() => {
				localStorage.setItem(LS_KEY_CUSTOM_ACCENT_COLOR, hex)
			}, 300)
		},
		setThemeClass() {
			const html = document.getElementsByTagName('html')[0]
			for (const theme of THEME_OPTIONS) {
				html.classList.remove(`${theme}-mode`)
			}

			systemThemeMq?.removeEventListener('change', this.setThemeClass)
			systemThemeMq = null

			let theme = this.selectedTheme
			if (this.selectedTheme === 'system') {
				systemThemeMq = window.matchMedia('(prefers-color-scheme: dark)')
				systemThemeMq.addEventListener('change', this.setThemeClass)
				theme = systemThemeMq.matches ? 'dark' : 'light'
			}

			html.classList.add(`${theme}-mode`)
		},
		getFeatureFlag(key: FeatureFlag) {
			return this.featureFlags[key] ?? DEFAULT_FEATURE_FLAGS[key]
		},
		getThemeOptions() {
			return THEME_OPTIONS
		},
		setBackgroundImagePath(path: string | null) {
			this.backgroundImagePath = path
			if (path) {
				localStorage.setItem(LS_KEY_BG_PATH, path)
			} else {
				localStorage.removeItem(LS_KEY_BG_PATH)
			}
		},
		setBackgroundBlur(blur: number) {
			this.backgroundBlur = blur
			localStorage.setItem(LS_KEY_BG_BLUR, String(blur))
		},
		setBackgroundOpacity(opacity: number) {
			this.backgroundOpacity = opacity
			localStorage.setItem(LS_KEY_BG_OPACITY, String(opacity))
		},
		setUseAppImageViewer(value: boolean) {
			this.useAppImageViewer = value
			localStorage.setItem(LS_KEY_APP_IMAGE_VIEWER, String(value))
		},
		setSettingsAsPage(value: boolean) {
			this.settingsAsPage = value
			localStorage.setItem(LS_KEY_SETTINGS_AS_PAGE, String(value))
		},
		setSkinClickParticles(value: boolean) {
			this.skinClickParticles = value
			localStorage.setItem(LS_KEY_SKIN_CLICK_PARTICLES, String(value))
		},
		setSkinHeadTracking(value: boolean) {
			this.skinHeadTracking = value
			localStorage.setItem(LS_KEY_SKIN_HEAD_TRACKING, String(value))
		},
		setSkinParticleBackground(value: boolean) {
			this.skinParticleBackground = value
			localStorage.setItem(LS_KEY_SKIN_PARTICLE_BG, String(value))
		},

		// ---- Theme pack system ----
		// Refreshes the list of installed theme packs from the backend.
		async refreshInstalledThemePacks() {
			try {
				this.installedThemePacks = await getInstalledThemePacks()
				// If the active theme pack is no longer installed, fall back to no pack.
				if (
					this.activeThemePackId &&
					!this.installedThemePacks.some((p) => p.id === this.activeThemePackId)
				) {
					this.setActiveThemePack(null)
				}
			} catch (e) {
				console.error('Failed to load installed theme packs:', e)
			}
		},

		// Installs a theme pack from a zip file path and refreshes the list.
		async installThemePackFromPath(zipPath: string) {
			await installFromPath(zipPath)
			await this.refreshInstalledThemePacks()
		},

		// Uninstalls a theme pack by id. If the active pack is uninstalled,
		// falls back to the no-pack state.
		async uninstallThemePackById(themeId: string) {
			await uninstallThemePack(themeId)
			if (this.activeThemePackId === themeId) {
				this.setActiveThemePack(null)
			}
			await this.refreshInstalledThemePacks()
		},

		// Activates a theme pack by id (or null to clear). Applies its
		// background image, accent color, blur, opacity and CSS variables.
		async setActiveThemePack(themeId: string | null) {
			this.activeThemePackId = themeId
			if (themeId) {
				localStorage.setItem(LS_KEY_ACTIVE_THEME_PACK, themeId)
			} else {
				localStorage.removeItem(LS_KEY_ACTIVE_THEME_PACK)
			}
			await this.applyActiveThemePack()
		},

		// Re-applies the currently active theme pack's overrides. Called
		// during app init and after install/uninstall/switch.
		async applyActiveThemePack() {
			const html = document.documentElement
			// Clear any previously-applied theme pack CSS variables
			html.removeAttribute('data-theme-pack')
			const inlineStyle = document.getElementById('theme-pack-inline-style')
			if (inlineStyle) {
				inlineStyle.remove()
			}

			if (!this.activeThemePackId) {
				return
			}
			const pack = this.installedThemePacks.find(
				(p) => p.id === this.activeThemePackId,
			)
			if (!pack) {
				return
			}

			html.setAttribute('data-theme-pack', pack.id)

			// Background image (overrides user-set path while pack is active)
			if (pack.background_image_path) {
				this.backgroundImagePath = pack.background_image_path
			}
			if (pack.background_blur != null) {
				this.setBackgroundBlur(pack.background_blur)
			}
			if (pack.background_opacity != null) {
				this.setBackgroundOpacity(pack.background_opacity)
			}

			// Apply CSS variables via a <style> tag with elevated specificity.
			const cssVars = pack.css_variables ?? {}
			if (pack.accent_color) {
				cssVars['--color-brand'] = pack.accent_color
			}
			if (pack.secondary_color) {
				cssVars['--color-brand-highlight'] = pack.secondary_color
			}
			if (pack.font_family) {
				cssVars['--default-font'] = pack.font_family
			}
			const cssEntries = Object.entries(cssVars)
			if (cssEntries.length > 0) {
				const style = document.createElement('style')
				style.id = 'theme-pack-inline-style'
				style.textContent = `:root[data-theme-pack="${pack.id}"] {\n${cssEntries
					.map(([k, v]) => `  ${k}: ${v};`)
					.join('\n')}\n}`
				document.head.appendChild(style)
			}
		},
	},
})
