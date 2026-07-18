//! Theseus settings file

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

// Types
/// Global Theseus settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
	pub max_concurrent_downloads: usize,
	pub max_concurrent_writes: usize,

	pub theme: Theme,
	pub locale: String,
	pub default_page: DefaultPage,
	pub collapsed_navigation: bool,
	pub hide_nametag_skins_page: bool,
	pub advanced_rendering: bool,
	pub native_decorations: bool,
	pub toggle_sidebar: bool,

	pub telemetry: bool,
	pub discord_rpc: bool,
	pub personalized_ads: bool,

	pub onboarded: bool,

	pub extra_launch_args: Vec<String>,
	pub custom_env_vars: Vec<(String, String)>,
	pub memory: MemorySettings,
	pub force_fullscreen: bool,
	pub game_resolution: WindowSize,
	pub launcher_visibility: LauncherVisibility,
	pub hooks: Hooks,

	pub custom_dir: Option<String>,
	pub prev_custom_dir: Option<String>,
	pub migrated: bool,

	pub developer_mode: bool,
	pub feature_flags: HashMap<FeatureFlag, bool>,

	pub skipped_update: Option<String>,
	pub pending_update_toast_for_version: Option<String>,
	pub auto_download_updates: Option<bool>,

	// Advanced launch options
	pub process_priority: ProcessPriority,
	pub renderer: Renderer,
	pub extra_game_args: Vec<String>,
	pub preferred_ip_stack: PreferredIpStack,
	pub custom_info: String,
	pub window_title: String,
	pub memory_allocation_mode: MemoryAllocationMode,
	pub set_gpu_preference: bool,
	pub use_java_exe: bool,
	pub pre_launch_wait: bool,
	pub disable_java_launch_wrapper: bool,
	pub disable_legacy_fix: bool,
	pub disable_lwjgl_unsafe_agent: bool,

	// Download mirror source settings
	pub game_file_source: crate::util::mirror::DownloadSource,
	pub community_source: crate::util::mirror::DownloadSource,


	pub version: usize,
}

/// Database row representation for sqlx::query_as
#[derive(sqlx::FromRow)]
struct SettingsRow {
	max_concurrent_writes: i32,
	max_concurrent_downloads: i32,
	theme: String,
	locale: String,
	default_page: String,
	collapsed_navigation: i32,
	hide_nametag_skins_page: i32,
	advanced_rendering: i32,
	native_decorations: i32,
	toggle_sidebar: i32,
	discord_rpc: i32,
	developer_mode: i32,
	telemetry: i32,
	personalized_ads: i32,
	onboarded: i32,
	extra_launch_args: Option<String>,
	custom_env_vars: Option<String>,
	mc_memory_max: i32,
	mc_force_fullscreen: i32,
	mc_game_resolution_x: i32,
	mc_game_resolution_y: i32,
	hook_pre_launch: Option<String>,
	hook_wrapper: Option<String>,
	hook_post_exit: Option<String>,
	custom_dir: Option<String>,
	prev_custom_dir: Option<String>,
	migrated: i32,
	feature_flags: Option<String>,
	skipped_update: Option<String>,
	pending_update_toast_for_version: Option<String>,
	auto_download_updates: Option<i32>,
	version: i32,
	launcher_visibility: i32,
	process_priority: i32,
	renderer: i32,
	extra_game_args: Option<String>,
	preferred_ip_stack: i32,
	custom_info: String,
	window_title: String,
	memory_allocation_mode: i32,
	set_gpu_preference: i32,
	use_java_exe: i32,
	pre_launch_wait: i32,
	disable_java_launch_wrapper: i32,
	disable_legacy_fix: i32,
	disable_lwjgl_unsafe_agent: i32,
	game_file_source: i32,
	community_source: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
	PagePath,
	ProjectBackground,
	WorldsTab,
	WorldsInHome,
	ServerRamAsBytesAlwaysOn,
	AlwaysShowAppControls,
	SkipUnknownPackWarning,
	PrideFundraiser,
	ServersInApp,
	ServerProjectQa,
	I18nDebug,
	ShowInstancePlayTime,
	SkipNonEssentialWarnings,
}

impl Settings {
	const CURRENT_VERSION: usize = 5;

	pub async fn get(
		exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
	) -> crate::Result<Self> {
	 let row = sqlx::query_as::<_, SettingsRow>(
	  "
	  SELECT
	   max_concurrent_writes, max_concurrent_downloads,
	   theme, locale, default_page, collapsed_navigation, hide_nametag_skins_page, advanced_rendering, native_decorations,
	   discord_rpc, developer_mode, telemetry, personalized_ads,
	   onboarded,
	   json(extra_launch_args) AS extra_launch_args, json(custom_env_vars) AS custom_env_vars,
	   mc_memory_max, mc_force_fullscreen, mc_game_resolution_x, mc_game_resolution_y,
	   hook_pre_launch, hook_wrapper, hook_post_exit,
	   custom_dir, prev_custom_dir, migrated, json(feature_flags) AS feature_flags, toggle_sidebar,
	   skipped_update, pending_update_toast_for_version, auto_download_updates,
	   version,
	   launcher_visibility, process_priority, renderer,
	   json(extra_game_args) AS extra_game_args,
	   preferred_ip_stack, custom_info, window_title, memory_allocation_mode,
	   set_gpu_preference, use_java_exe, pre_launch_wait,
	   disable_java_launch_wrapper, disable_legacy_fix, disable_lwjgl_unsafe_agent,
	   game_file_source, community_source
	  FROM settings
	  ",
	 )
	 .fetch_one(exec)
	 .await?;

	 let result = Self {
	  max_concurrent_downloads: row.max_concurrent_downloads as usize,
	  max_concurrent_writes: row.max_concurrent_writes as usize,
	  theme: Theme::from_string(&row.theme),
	  locale: row.locale,
	  default_page: DefaultPage::from_string(&row.default_page),
	  collapsed_navigation: row.collapsed_navigation != 0,
	  hide_nametag_skins_page: row.hide_nametag_skins_page != 0,
	  advanced_rendering: row.advanced_rendering != 0,
	  native_decorations: row.native_decorations != 0,
	  toggle_sidebar: row.toggle_sidebar != 0,
	  telemetry: row.telemetry != 0,
	  discord_rpc: row.discord_rpc != 0,
	  developer_mode: row.developer_mode != 0,
	  personalized_ads: row.personalized_ads != 0,
	  onboarded: row.onboarded != 0,
	  extra_launch_args: row
	   .extra_launch_args
	   .as_ref()
	   .and_then(|x| serde_json::from_str(x).ok())
	   .unwrap_or_default(),
	  custom_env_vars: row
	   .custom_env_vars
	   .as_ref()
	   .and_then(|x| serde_json::from_str(x).ok())
	   .unwrap_or_default(),
	  memory: MemorySettings {
	   maximum: row.mc_memory_max as u32,
	  },
	  force_fullscreen: row.mc_force_fullscreen != 0,
	  game_resolution: WindowSize(
	   row.mc_game_resolution_x as u16,
	   row.mc_game_resolution_y as u16,
	  ),
	  launcher_visibility: LauncherVisibility::from_i32(row.launcher_visibility),
	  hooks: Hooks {
	   pre_launch: row.hook_pre_launch,
	   wrapper: row.hook_wrapper,
	   post_exit: row.hook_post_exit,
	  },
	  custom_dir: row.custom_dir,
	  prev_custom_dir: row.prev_custom_dir,
	  migrated: row.migrated != 0,
	  feature_flags: row
	   .feature_flags
	   .as_ref()
	   .and_then(|x| serde_json::from_str(x).ok())
	   .unwrap_or_default(),
	  skipped_update: row.skipped_update,
	  pending_update_toast_for_version: row.pending_update_toast_for_version,
	  auto_download_updates: row.auto_download_updates.map(|x| x != 0),

	  process_priority: ProcessPriority::from_i32(row.process_priority),
	  renderer: Renderer::from_i32(row.renderer),
	  extra_game_args: row
	   .extra_game_args
	   .as_ref()
	   .and_then(|x| serde_json::from_str(x).ok())
	   .unwrap_or_default(),
	  preferred_ip_stack: PreferredIpStack::from_i32(row.preferred_ip_stack),
	  custom_info: row.custom_info,
	  window_title: row.window_title,
	  memory_allocation_mode: MemoryAllocationMode::from_i32(
	   row.memory_allocation_mode,
	  ),
	  set_gpu_preference: row.set_gpu_preference != 0,
	  use_java_exe: row.use_java_exe != 0,
	  pre_launch_wait: row.pre_launch_wait != 0,
	  disable_java_launch_wrapper: row.disable_java_launch_wrapper != 0,
	  disable_legacy_fix: row.disable_legacy_fix != 0,
	  disable_lwjgl_unsafe_agent: row.disable_lwjgl_unsafe_agent != 0,

	  game_file_source: crate::util::mirror::DownloadSource::from_i32(row.game_file_source),
	  community_source: crate::util::mirror::DownloadSource::from_i32(row.community_source),

	  version: row.version as usize,
	 };

	 // Sync global mirror cache
	 crate::util::mirror::update_mirror_settings(
	  row.game_file_source,
	  row.community_source,
	 );

	 Ok(result)
	}

	pub async fn update(
		&self,
		exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
	) -> crate::Result<()> {
		let extra_launch_args = serde_json::to_string(&self.extra_launch_args)?;
		let custom_env_vars = serde_json::to_string(&self.custom_env_vars)?;
		let extra_game_args = serde_json::to_string(&self.extra_game_args)?;
		let feature_flags = serde_json::to_string(&self.feature_flags)?;
		let theme = self.theme.as_str();
		let default_page = self.default_page.as_str();

		sqlx::query(
			"
			UPDATE settings
			SET
				max_concurrent_writes = $1,
				max_concurrent_downloads = $2,
				theme = $3,
				locale = $4,
				default_page = $5,
				collapsed_navigation = $6,
				advanced_rendering = $7,
				native_decorations = $8,
				discord_rpc = $9,
				developer_mode = $10,
				telemetry = $11,
				personalized_ads = $12,
				onboarded = $13,
				extra_launch_args = jsonb($14),
				custom_env_vars = jsonb($15),
				mc_memory_max = $16,
				mc_force_fullscreen = $17,
				mc_game_resolution_x = $18,
				mc_game_resolution_y = $19,
				hook_pre_launch = $20,
				hook_wrapper = $21,
				hook_post_exit = $22,
				custom_dir = $23,
				prev_custom_dir = $24,
				migrated = $25,
				toggle_sidebar = $26,
				feature_flags = jsonb($27),
				hide_nametag_skins_page = $28,
				skipped_update = $29,
				pending_update_toast_for_version = $30,
				auto_download_updates = $31,
				version = $32,
				launcher_visibility = $33,
				process_priority = $34,
				renderer = $35,
				extra_game_args = jsonb($36),
				preferred_ip_stack = $37,
				custom_info = $38,
				window_title = $39,
				memory_allocation_mode = $40,
				set_gpu_preference = $41,
				use_java_exe = $42,
				pre_launch_wait = $43,
				disable_java_launch_wrapper = $44,
			disable_legacy_fix = $45,
			disable_lwjgl_unsafe_agent = $46,
			game_file_source = $47,
			community_source = $48
		",
		)
		.bind(self.max_concurrent_writes as i32)
		.bind(self.max_concurrent_downloads as i32)
		.bind(theme)
		.bind(&self.locale)
		.bind(default_page)
		.bind(self.collapsed_navigation)
		.bind(self.advanced_rendering)
		.bind(self.native_decorations)
		.bind(self.discord_rpc)
		.bind(self.developer_mode)
		.bind(self.telemetry)
		.bind(self.personalized_ads)
		.bind(self.onboarded)
		.bind(&extra_launch_args)
		.bind(&custom_env_vars)
		.bind(self.memory.maximum as i32)
		.bind(self.force_fullscreen)
		.bind(self.game_resolution.0 as i32)
		.bind(self.game_resolution.1 as i32)
		.bind(&self.hooks.pre_launch)
		.bind(&self.hooks.wrapper)
		.bind(&self.hooks.post_exit)
		.bind(&self.custom_dir)
		.bind(&self.prev_custom_dir)
		.bind(self.migrated)
		.bind(self.toggle_sidebar)
		.bind(&feature_flags)
		.bind(self.hide_nametag_skins_page)
		.bind(&self.skipped_update)
		.bind(&self.pending_update_toast_for_version)
		.bind(self.auto_download_updates)
		.bind(self.version as i32)
		.bind(self.launcher_visibility.as_i32())
		.bind(self.process_priority.as_i32())
		.bind(self.renderer.as_i32())
		.bind(&extra_game_args)
		.bind(self.preferred_ip_stack.as_i32())
		.bind(&self.custom_info)
		.bind(&self.window_title)
		.bind(self.memory_allocation_mode.as_i32())
		.bind(self.set_gpu_preference)
		.bind(self.use_java_exe)
		.bind(self.pre_launch_wait)
		.bind(self.disable_java_launch_wrapper)
		.bind(self.disable_legacy_fix)
		.bind(self.disable_lwjgl_unsafe_agent)
		.bind(self.game_file_source.as_i32())
		.bind(self.community_source.as_i32())
		.execute(exec)
		.await?;

		// Sync global mirror cache
		crate::util::mirror::update_mirror_settings(
			self.game_file_source.as_i32(),
			self.community_source.as_i32(),
		);

		Ok(())
	}

	pub async fn migrate(exec: &Pool<Sqlite>) -> crate::Result<()> {
		let mut settings = Self::get(exec).await?;

		if settings.version < Settings::CURRENT_VERSION {
			tracing::info!(
				"Migrating settings version {} to {:?}",
				settings.version,
				Settings::CURRENT_VERSION
			);
		}
		while settings.version < Settings::CURRENT_VERSION {
			if let Err(err) = settings.perform_migration() {
				tracing::error!(
					"Failed to migrate settings from version {}: {}",
					settings.version,
					err
				);
				return Err(err);
			}
		}

		settings.update(exec).await?;

		Ok(())
	}

	pub fn perform_migration(&mut self) -> crate::Result<()> {
		match self.version {
			1 => {
				let quoter = shlex::Quoter::new().allow_nul(true);

				if let Some(pre_launch) = self.hooks.pre_launch.as_ref() {
					self.hooks.pre_launch =
						Some(quoter.join(pre_launch.split(' ')).unwrap())
				}

				if let Some(wrapper) = self.hooks.wrapper.as_ref() {
					self.hooks.wrapper =
						Some(quoter.quote(wrapper).unwrap().to_string())
				}

				if let Some(post_exit) = self.hooks.post_exit.as_ref() {
					self.hooks.post_exit =
						Some(quoter.join(post_exit.split(' ')).unwrap())
				}

				self.version = 2;
			}
			2 => {
				const LEGACY_DEFAULT_MEMORY_MB: u32 = 2048;
				if self.memory.maximum == LEGACY_DEFAULT_MEMORY_MB {
					self.memory.maximum =
						crate::api::jre::default_memory_max_mb();
				}

				self.version = 3;
			}
			3 => {
			// New launch option fields are added via SQL migration with defaults.
			// memory_allocation_mode defaults to Custom (1) to preserve existing behavior,
			// since the existing memory.maximum is already a user-set value.
			self.version = 4;
		}
		4 => {
		// Download mirror source fields are added via SQL migration with defaults.
		// game_file_source and community_source both default to 1 (Auto).
		self.version = 5;
	}
	version => {
				return Err(crate::ErrorKind::OtherError(format!(
					"Invalid settings version: {version}"
				))
				.into());
			}
		}

		Ok(())
	}
}

/// Theseus theme
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
	Dark,
	Light,
	Oled,
	System,
}

impl Theme {
	pub fn as_str(&self) -> &'static str {
		match self {
			Theme::Dark => "dark",
			Theme::Light => "light",
			Theme::Oled => "oled",
			Theme::System => "system",
		}
	}

	pub fn from_string(string: &str) -> Theme {
		match string {
			"dark" => Theme::Dark,
			"light" => Theme::Light,
			"oled" => Theme::Oled,
			"system" => Theme::System,
			_ => Theme::Dark,
		}
	}
}

/// Minecraft memory settings
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MemorySettings {
	pub maximum: u32,
}

/// Game window size
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct WindowSize(pub u16, pub u16);

/// Game initialization hooks
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde_with::serde_as]
pub struct Hooks {
	#[serde_as(as = "serde_with::NoneAsEmptyString")]
	pub pre_launch: Option<String>,
	#[serde_as(as = "serde_with::NoneAsEmptyString")]
	pub wrapper: Option<String>,
	#[serde_as(as = "serde_with::NoneAsEmptyString")]
	pub post_exit: Option<String>,
}

/// Opening window to start with
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DefaultPage {
	Home,
	Library,
}

impl DefaultPage {
	pub fn as_str(&self) -> &'static str {
		match self {
			DefaultPage::Home => "home",
			DefaultPage::Library => "library",
		}
	}

	pub fn from_string(string: &str) -> Self {
		match string {
			"home" => Self::Home,
			"library" => Self::Library,
			_ => Self::Home,
		}
	}
}

/// Launcher visibility behavior when game starts
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "i32", from = "i32")]
pub enum LauncherVisibility {
	ExitImmediately = 0,
	HideAndExit = 2,
	HideAndReopen = 3,
	MinimizeAndReopen = 4,
	DoNothing = 5,
}

impl LauncherVisibility {
	pub fn from_i32(val: i32) -> Self {
		match val {
			0 => Self::ExitImmediately,
			2 => Self::HideAndExit,
			3 => Self::HideAndReopen,
			4 => Self::MinimizeAndReopen,
			_ => Self::DoNothing,
		}
	}

	pub fn as_i32(&self) -> i32 {
		*self as i32
	}
}

impl From<i32> for LauncherVisibility {
	fn from(val: i32) -> Self {
		Self::from_i32(val)
	}
}

impl From<LauncherVisibility> for i32 {
	fn from(val: LauncherVisibility) -> i32 {
		val.as_i32()
	}
}

impl Default for LauncherVisibility {
	fn default() -> Self {
		Self::DoNothing
	}
}

/// Game process priority
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "i32", from = "i32")]
pub enum ProcessPriority {
	AboveNormal = 0,
	Normal = 1,
	BelowNormal = 2,
	High = 3,
	RealTime = 4,
}

impl ProcessPriority {
	pub fn from_i32(val: i32) -> Self {
		match val {
			0 => Self::AboveNormal,
			1 => Self::Normal,
			2 => Self::BelowNormal,
			3 => Self::High,
			4 => Self::RealTime,
			_ => Self::Normal,
		}
	}

	pub fn as_i32(&self) -> i32 {
		*self as i32
	}
}

impl From<i32> for ProcessPriority {
	fn from(val: i32) -> Self {
		Self::from_i32(val)
	}
}

impl From<ProcessPriority> for i32 {
	fn from(val: ProcessPriority) -> i32 {
		val.as_i32()
	}
}

impl Default for ProcessPriority {
	fn default() -> Self {
		Self::Normal
	}
}

/// Game renderer type
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "i32", from = "i32")]
pub enum Renderer {
	Default = 0,
	Llvmpipe = 1,
	D3d12 = 2,
	Zink = 3,
}

impl Renderer {
	pub fn from_i32(val: i32) -> Self {
		match val {
			0 => Self::Default,
			1 => Self::Llvmpipe,
			2 => Self::D3d12,
			3 => Self::Zink,
			_ => Self::Default,
		}
	}

	pub fn as_i32(&self) -> i32 {
		*self as i32
	}
}

impl From<i32> for Renderer {
	fn from(val: i32) -> Self {
		Self::from_i32(val)
	}
}

impl From<Renderer> for i32 {
	fn from(val: Renderer) -> i32 {
		val.as_i32()
	}
}

impl Default for Renderer {
	fn default() -> Self {
		Self::Default
	}
}

/// JVM preferred IP stack
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "i32", from = "i32")]
pub enum PreferredIpStack {
	PreferV4 = 0,
	Default = 1,
	PreferV6 = 2,
}

impl PreferredIpStack {
	pub fn from_i32(val: i32) -> Self {
		match val {
			0 => Self::PreferV4,
			1 => Self::Default,
			2 => Self::PreferV6,
			_ => Self::Default,
		}
	}

	pub fn as_i32(&self) -> i32 {
		*self as i32
	}
}

impl From<i32> for PreferredIpStack {
	fn from(val: i32) -> Self {
		Self::from_i32(val)
	}
}

impl From<PreferredIpStack> for i32 {
	fn from(val: PreferredIpStack) -> i32 {
		val.as_i32()
	}
}

impl Default for PreferredIpStack {
	fn default() -> Self {
		Self::Default
	}
}

/// Memory allocation mode
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "i32", from = "i32")]
pub enum MemoryAllocationMode {
	Auto = 0,
	Custom = 1,
	FollowGlobal = 2,
}

impl MemoryAllocationMode {
	pub fn from_i32(val: i32) -> Self {
		match val {
			0 => Self::Auto,
			1 => Self::Custom,
			2 => Self::FollowGlobal,
			_ => Self::Custom,
		}
	}

	pub fn as_i32(&self) -> i32 {
		*self as i32
	}
}

impl From<i32> for MemoryAllocationMode {
	fn from(val: i32) -> Self {
		Self::from_i32(val)
	}
}

impl From<MemoryAllocationMode> for i32 {
	fn from(val: MemoryAllocationMode) -> i32 {
		val.as_i32()
	}
}

impl Default for MemoryAllocationMode {
	fn default() -> Self {
		Self::Custom
	}
}
