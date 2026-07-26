import {
	CoffeeIcon,
	GameIcon,
	GaugeIcon,
	GlobeIcon,
	HomeIcon,
	InfoIcon,
	LanguagesIcon,
	PaintbrushIcon,
	ShieldIcon,
	ToggleRightIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { commonMessages, defineMessage } from '@modrinth/ui'

import AppearanceSettings from '@/components/ui/settings/AppearanceSettings.vue'
import DefaultInstanceSettings from '@/components/ui/settings/DefaultInstanceSettings.vue'
import FeatureFlagSettings from '@/components/ui/settings/FeatureFlagSettings.vue'
import HomeSettings from '@/components/ui/settings/HomeSettings.vue'
import JavaSettings from '@/components/ui/settings/JavaSettings.vue'
import LanguageSettings from '@/components/ui/settings/LanguageSettings.vue'
import PrivacySettings from '@/components/ui/settings/PrivacySettings.vue'
import ResourceManagementSettings from '@/components/ui/settings/ResourceManagementSettings.vue'
import AdvancedInstanceSettings from '@/components/ui/settings/AdvancedInstanceSettings.vue'
import TranslationSettings from '@/components/ui/settings/TranslationSettings.vue'
import AboutSettings from '@/components/ui/settings/AboutSettings.vue'
import type { Component } from 'vue'

export interface SettingsTab {
	name: ReturnType<typeof defineMessage>
	icon: Component
	content: Component
	badge?: ReturnType<typeof defineMessage>
}

export const settingsTabs: SettingsTab[] = [
	{
		name: defineMessage({
			id: 'app.settings.tabs.appearance',
			defaultMessage: 'Appearance',
		}),
		icon: PaintbrushIcon,
		content: AppearanceSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.home',
			defaultMessage: 'Home page',
		}),
		icon: HomeIcon,
		content: HomeSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.language',
			defaultMessage: 'Language',
		}),
		icon: LanguagesIcon,
		content: LanguageSettings,
		badge: defineMessage({
			id: 'app.settings.language.beta',
			defaultMessage: 'Beta',
		}),
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.translation',
			defaultMessage: 'Translation',
		}),
		icon: GlobeIcon,
		content: TranslationSettings,
		badge: commonMessages.beta,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.privacy',
			defaultMessage: 'Privacy',
		}),
		icon: ShieldIcon,
		content: PrivacySettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.java-installations',
			defaultMessage: 'Java installations',
		}),
		icon: CoffeeIcon,
		content: JavaSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.default-instance-options',
			defaultMessage: 'Default instance options',
		}),
		icon: GameIcon,
		content: DefaultInstanceSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.advanced-instance-options',
			defaultMessage: 'Advanced instance options',
		}),
		icon: WrenchIcon,
		content: AdvancedInstanceSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.resource-management',
			defaultMessage: 'Resource management',
		}),
		icon: GaugeIcon,
		content: ResourceManagementSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.experimental-features',
			defaultMessage: '实验性功能',
		}),
		icon: ToggleRightIcon,
		content: FeatureFlagSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.about',
			defaultMessage: 'About',
		}),
		icon: InfoIcon,
		content: AboutSettings,
	},
]
