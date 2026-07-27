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
import { defineMessage } from '@modrinth/ui'
import type { Component } from 'vue'

import AboutSettings from '@/components/ui/settings/AboutSettings.vue'
import AdvancedInstanceSettings from '@/components/ui/settings/AdvancedInstanceSettings.vue'
import AppearanceSettings from '@/components/ui/settings/AppearanceSettings.vue'
import DefaultInstanceSettings from '@/components/ui/settings/DefaultInstanceSettings.vue'
import FeatureFlagSettings from '@/components/ui/settings/FeatureFlagSettings.vue'
import HomeSettings from '@/components/ui/settings/HomeSettings.vue'
import JavaSettings from '@/components/ui/settings/JavaSettings.vue'
import LanguageSettings from '@/components/ui/settings/LanguageSettings.vue'
import PrivacySettings from '@/components/ui/settings/PrivacySettings.vue'
import ResourceManagementSettings from '@/components/ui/settings/ResourceManagementSettings.vue'
import TranslationSettings from '@/components/ui/settings/TranslationSettings.vue'

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
			defaultMessage: 'Home',
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
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.translation',
			defaultMessage: 'Translation',
		}),
		icon: GlobeIcon,
		content: TranslationSettings,
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
			defaultMessage: 'Java Installations',
		}),
		icon: CoffeeIcon,
		content: JavaSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.default-instance-options',
			defaultMessage: 'Default Instance Options',
		}),
		icon: GameIcon,
		content: DefaultInstanceSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.advanced-instance-options',
			defaultMessage: 'Advanced Instance Options',
		}),
		icon: WrenchIcon,
		content: AdvancedInstanceSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.resource-management',
			defaultMessage: 'Resource Management',
		}),
		icon: GaugeIcon,
		content: ResourceManagementSettings,
	},
	{
		name: defineMessage({
			id: 'app.settings.tabs.experimental-features',
			defaultMessage: 'Experimental Features',
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
