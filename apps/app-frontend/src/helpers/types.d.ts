import type { ModrinthId } from '@modrinth/utils'

export type GameInstance = {
	id: string
	path: string
	install_stage: InstallStage
	launcher_feature_version: string

	name: string
	icon_path?: string

	game_version: string
	protocol_version?: number
	loader: InstanceLoader
	loader_version?: string

	groups: string[]

	link?: InstanceLink | null
	update_channel: ReleaseChannel

	created: Date
	modified: Date
	last_played?: Date

	submitted_time_played: number
	recent_time_played: number

	java_path?: string
	extra_launch_args?: string[]
	custom_env_vars?: [string, string][]

	memory?: MemorySettings
	memory_allocation_mode?: number
	force_fullscreen?: boolean
	game_resolution?: [number, number]
	hooks: Hooks
}

type InstallStage =
	| 'installed'
	| 'minecraft_installing'
	| 'pack_installed'
	| 'pack_installing'
	| 'not_installed'

type InstanceLinkIdentity = {
	project_id?: ModrinthId | null
	version_id?: ModrinthId | null
	server_project_id?: ModrinthId | null
	content_project_id?: ModrinthId | null
	content_version_id?: ModrinthId | null
}

export type InstanceLink = InstanceLinkIdentity &
	(
		| {
				type: 'modrinth_modpack'
				project_id: ModrinthId
				version_id: ModrinthId
		  }
		| {
				type: 'server_project'
				project_id: ModrinthId
		  }
		| {
				type: 'server_project_modpack'
				server_project_id: ModrinthId
				content_project_id?: ModrinthId | null
				content_version_id: ModrinthId
				project_id?: ModrinthId
				version_id?: ModrinthId
		  }
		| {
				type: 'imported_modpack'
				project_id?: ModrinthId | null
				version_id?: ModrinthId | null
				name?: string | null
				version_number?: string | null
				filename?: string | null
		  }
		| {
				type: 'modrinth_hosting'
				server_id: string
				instance_ids: string[]
				active_instance_id?: string | null
		  }
		| {
				type: 'shared_instance'
				shared_instance_id: string
		  }
	)

export type Instance = GameInstance

type ReleaseChannel = 'release' | 'beta' | 'alpha'

export type InstanceLoader = 'vanilla' | 'forge' | 'fabric' | 'quilt' | 'neoforge'

type ContentFile = {
	enabled: boolean
	metadata?: {
		project_id: string
		version_id: string
	}
}

type ContentFileProjectType = 'mod' | 'datapack' | 'resourcepack' | 'shaderpack'

type CacheBehaviour =
	// Serve expired data. If fetch fails / launcher is offline, errors are ignored
	| 'stale_while_revalidate_skip_offline'
	// Serve expired data, revalidate in background
	| 'stale_while_revalidate'
	// Must revalidate if data is expired
	| 'must_revalidate'
	// Ignore cache- always fetch updated data from origin
	| 'bypass'

type MemorySettings = {
	maximum: number
}

type WindowSize = {
	width: number
	height: number
}

type Hooks = {
	pre_launch?: string
	wrapper?: string
	post_exit?: string
}

type Manifest = {
	gameVersions: ManifestGameVersion[]
}

type ManifestGameVersion = {
	id: string
	stable: boolean
	loaders: ManifestLoaderVersion[]
}

type ManifestLoaderVersion = {
	id: string
	url: string
	stable: boolean
}

type AppSettings = {
	max_concurrent_downloads: number
	max_concurrent_writes: number
	max_chunks_per_file: number

	theme: 'dark' | 'light' | 'oled' | 'system'
	locale: string
	default_page: 'Home' | 'Library'
	collapsed_navigation: boolean
	hide_nametag_skins_page: boolean
	advanced_rendering: boolean
	native_decorations: boolean
	toggle_sidebar: boolean

	telemetry: boolean
	discord_rpc: boolean
	developer_mode: boolean
	personalized_ads: boolean

	onboarded: boolean

	extra_launch_args: string[]
	custom_env_vars: [string, string][]
	memory: MemorySettings
	force_fullscreen: boolean
	game_resolution: [number, number]
	launcher_visibility: number
	hooks: Hooks

	custom_dir?: string
	prev_custom_dir?: string
	migrated: boolean

	feature_flags: Record<string, boolean>

	skipped_update: string | null
	pending_update_toast_for_version: string | null
	auto_download_updates: boolean | null

	process_priority: number
	renderer: number
	extra_game_args: string[]
	preferred_ip_stack: number
	custom_info: string
	window_title: string
	memory_allocation_mode: number
	set_gpu_preference: boolean
	use_java_exe: boolean
	pre_launch_wait: boolean
	disable_java_launch_wrapper: boolean
	disable_legacy_fix: boolean
	disable_lwjgl_unsafe_agent: boolean

	game_file_source: number
	community_source: number

	version: number
}
