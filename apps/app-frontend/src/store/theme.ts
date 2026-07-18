import { defineStore } from 'pinia'

let systemThemeMq: MediaQueryList | null = null

const LS_KEY_BG_PATH = 'modrinth-app-background-image-path'
const LS_KEY_BG_BLUR = 'modrinth-app-background-blur'

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

export type FeatureFlag = keyof typeof DEFAULT_FEATURE_FLAGS
export type FeatureFlags = Record<FeatureFlag, boolean>
export type ColorTheme = (typeof THEME_OPTIONS)[number]

export type ThemeStore = {
	selectedTheme: ColorTheme
	advancedRendering: boolean
	hideNametagSkinsPage: boolean
	toggleSidebar: boolean

	devMode: boolean
	featureFlags: FeatureFlags

	backgroundImagePath: string | null
	backgroundBlur: number
}

export const DEFAULT_THEME_STORE: ThemeStore = {
	selectedTheme: 'dark',
	advancedRendering: true,
	hideNametagSkinsPage: false,
	toggleSidebar: false,

	devMode: false,
	featureFlags: DEFAULT_FEATURE_FLAGS,

	backgroundImagePath: null,
	backgroundBlur: 20,
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
			stored.backgroundBlur = parseInt(savedBlur, 10) || 20
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
	},
})
