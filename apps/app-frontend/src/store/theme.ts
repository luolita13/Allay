import { defineStore } from 'pinia'

let systemThemeMq: MediaQueryList | null = null

const LS_KEY_BG_PATH = 'modrinth-app-background-image-path'
const LS_KEY_BG_BLUR = 'modrinth-app-background-blur'
const LS_KEY_BG_OPACITY = 'modrinth-app-background-opacity'
const LS_KEY_ACCENT_COLOR = 'modrinth-app-accent-color'

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
	// Experimental features
	game_link: false,
}

export const THEME_OPTIONS = ['dark', 'light', 'oled', 'system'] as const
export const ACCENT_COLOR_OPTIONS = ['pink', 'orange', 'green', 'blue', 'purple'] as const

export type AccentColor = (typeof ACCENT_COLOR_OPTIONS)[number]

export type FeatureFlag = keyof typeof DEFAULT_FEATURE_FLAGS
export type FeatureFlags = Record<FeatureFlag, boolean>
export type ColorTheme = (typeof THEME_OPTIONS)[number]

export type ThemeStore = {
	selectedTheme: ColorTheme
	selectedAccentColor: AccentColor
	advancedRendering: boolean
	hideNametagSkinsPage: boolean
	toggleSidebar: boolean

	devMode: boolean
	featureFlags: FeatureFlags

	backgroundImagePath: string | null
	backgroundBlur: number
	backgroundOpacity: number
}

export const DEFAULT_THEME_STORE: ThemeStore = {
	selectedTheme: 'dark',
	selectedAccentColor: 'pink',
	advancedRendering: true,
	hideNametagSkinsPage: false,
	toggleSidebar: false,

	devMode: false,
	featureFlags: DEFAULT_FEATURE_FLAGS,

	backgroundImagePath: null,
	backgroundBlur: 20,
	backgroundOpacity: 65,
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

			const html = document.documentElement
			for (const accentColor of ACCENT_COLOR_OPTIONS) {
				html.classList.remove(`accent-${accentColor}`)
			}
			html.classList.add(`accent-${this.selectedAccentColor}`)
			localStorage.setItem(LS_KEY_ACCENT_COLOR, this.selectedAccentColor)
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
	},
})
