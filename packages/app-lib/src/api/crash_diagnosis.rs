//! Crash diagnosis system - rule-based crash log analyzer
//!
//! Ported and adapted from HMCL and PCL-CE CrashReportAnalyzer. Rules are
//! encoded as Rust static data rather than YAML to avoid extra dependencies and
//! to keep pattern matching verifiable at compile time.
//!
//! Supports multi-source log collection: crash-reports, latest.log, debug.log,
//! hs_err_pid logs, and launcher-captured output. Includes stack keyword → mod
//! name mapping for precise mod identification.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::sync::LazyLock;
use std::time::SystemTime;

/// Severity of a diagnosed crash rule
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CrashSeverity {
    /// fatal - cannot run without fix (e.g. wrong Java version, missing libs)
    Fatal,
    /// warning - might still run but with major issues
    Warning,
    /// info - hints/tips only
    Info,
}

/// Whether a rule can be auto-fixed by the launcher
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AutoFixKind {
    /// No automatic fix available
    None,
    /// Suggest installing a specific Java major version (u32)
    InstallJava(u32),
    /// Suggest reinstalling the instance / loader
    ReinstallInstance,
    /// Suggest clearing mods folder
    ClearMods,
    /// Suggest updating graphics driver (opens browser)
    UpdateGraphicsDriver,
    /// Suggest bumping memory allocation
    IncreaseMemory,
    /// Open mods folder in file explorer
    OpenModsFolder,
    /// Open instance settings
    OpenInstanceSettings,
}

/// Log source type for multi-source diagnosis
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrashLogSource {
    /// Minecraft crash-report-NNN.txt
    CrashReport,
    /// logs/latest.log or debug.log
    MinecraftLog,
    /// hs_err_pidNNNN.log (JVM crash)
    HsErrLog,
    /// launcher_log.txt (captured stdout/stderr)
    LauncherOutput,
    /// User-pasted or imported log
    UserProvided,
}

/// A single crash diagnosis rule
pub struct CrashRule {
    /// Internal id, also used as i18n key suffix: `crash.rule.<id>.title/description/fix`
    pub id: &'static str,
    pub severity: CrashSeverity,
    /// Regex pattern applied to the crash log (case-insensitive, multiline)
    pub pattern: &'static str,
    pub auto_fix: AutoFixKind,
    /// Optional capture group transformer - produces a human-readable fragment
    pub transformer: Option<fn(&regex::Captures) -> String>,
}

/// Result of running the diagnosis against a crash log
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrashDiagnosisResult {
    /// Matched rule ids in order of appearance
    pub matched: Vec<CrashDiagnosisMatch>,
    /// Total bytes scanned
    pub scanned_bytes: usize,
    /// Whether a Minecraft crash report header was detected
    pub has_crash_report_header: bool,
    /// First 200 chars of the log for context
    pub excerpt: String,
    /// Timestamp the diagnosis was generated
    pub generated_at: u64,
    /// Log sources that contributed to this diagnosis
    pub sources: Vec<CrashLogSource>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrashDiagnosisMatch {
    pub rule_id: String,
    pub severity: CrashSeverity,
    pub title: String,
    pub description: String,
    pub fix: String,
    pub auto_fix: AutoFixKind,
    /// Optional fragment captured from the log (e.g. mod name, version)
    pub fragment: Option<String>,
    /// Optional mod file names identified from stack analysis
    pub mod_files: Option<Vec<String>>,
}

/// Default fallback used when no rule matches
pub const NO_MATCH_RULE_ID: &str = "unknown_crash";

// ---------------------------------------------------------------------------
// Capture transformers
// ---------------------------------------------------------------------------

fn extract_mod_name_capture(caps: &regex::Captures) -> String {
    caps.name("mod")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

fn extract_version_capture(caps: &regex::Captures) -> String {
    caps.name("version")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

fn extract_path_capture(caps: &regex::Captures) -> String {
    caps.name("path")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Static rule set - expanded from ~29 to ~45 rules covering PCL-CE patterns
// ---------------------------------------------------------------------------

// Known system/loader packages to filter out during stack analysis
static SYSTEM_PACKAGES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "java", "sun", "javax", "jdk", "oolloo", "org.lwjgl", "com.sun",
        "net.minecraftforge", "paulscode.sound", "com.mojang", "net.minecraft",
        "cpw.mods", "com.google", "org.apache", "org.spongepowered", "net.fabricmc",
        "com.mumfrey", "org.quiltmc", "com.electronwill.nightconfig", "it.unimi.dsi",
        "MojangTricksIntelDriversForPerformance_javaw",
    ]
    .iter()
    .copied()
    .collect()
});

// Common stack words that are not mod identifiers
static COMMON_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "com", "org", "net", "asm", "fml", "mod", "jar", "sun", "lib", "map", "gui",
        "dev", "nio", "api", "dsi", "top", "mcp", "core", "init", "mods", "main",
        "file", "game", "load", "read", "done", "util", "tile", "item", "base",
        "oshi", "impl", "data", "pool", "task", "forge", "setup", "block", "model",
        "mixin", "event", "unimi", "netty", "world", "lwjgl", "gitlab", "common",
        "server", "config", "mixins", "compat", "loader", "launch", "entity",
        "assist", "client", "plugin", "modapi", "mojang", "shader", "events",
        "github", "recipe", "render", "packet", "preinit", "preload", "machine",
        "reflect", "channel", "general", "handler", "content", "systems", "modules",
        "service", "fastutil", "optifine", "internal", "platform", "override",
        "fabricmc", "neoforge", "injection", "listeners", "scheduler", "minecraft",
        "universal", "multipart", "neoforged", "microsoft", "transformer",
        "transformers", "minecraftforge", "blockentity", "spongepowered",
        "electronwill",
    ]
    .iter()
    .copied()
    .collect()
});

pub static RULES: LazyLock<Vec<(CrashRule, Regex)>> = LazyLock::new(|| {
    let raw: Vec<CrashRule> = vec![
        // ---- Java version issues (5 rules) ----
        CrashRule {
            id: "too_old_java",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)has been compiled by a more recent version of the Java Runtime|UnsupportedClassVersionError|unsupported class file version|Unsupported major.minor version",
            auto_fix: AutoFixKind::InstallJava(17),
            transformer: None,
        },
        CrashRule {
            id: "need_jdk17",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.UnsupportedClassVersionError.*?version 61\.0|Java 17 is required|requires Java 17|Mod 需要 Java 11|class file version 55\.0",
            auto_fix: AutoFixKind::InstallJava(17),
            transformer: None,
        },
        CrashRule {
            id: "need_jdk21",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.UnsupportedClassVersionError.*?version 65\.0|Java 21 is required|requires Java 21",
            auto_fix: AutoFixKind::InstallJava(21),
            transformer: None,
        },
        CrashRule {
            id: "java_version_too_high",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)java\.lang\.IllegalArgumentException: (?P<version>\d+\.\d+\.\d+) is not a compatible Java version|because module java\.base does not export|java\.lang\.NoSuchFieldException: ucp|Unable to make protected final java\.lang\.Class java\.lang\.ClassLoader\.defineClass|java\.lang\.ClassNotFoundException: jdk\.nashorn",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_version_capture),
        },
        CrashRule {
            id: "jvm_32bit",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Could not reserve enough space|The JVM is running in 32-bit mode|wrong architecture|java\.lang\.UnsatisfiedLinkError.*(win32|x86)|Invalid maximum heap size",
            auto_fix: AutoFixKind::InstallJava(17),
            transformer: None,
        },
        // ---- Memory issues (2 rules) ----
        CrashRule {
            id: "out_of_memory",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.OutOfMemoryError|Failed to allocate|out of memory|Out of Memory Error|The system is out of physical RAM or swap space",
            auto_fix: AutoFixKind::IncreaseMemory,
            transformer: None,
        },
        CrashRule {
            id: "memory_exceeded",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)exceeds 32G|cannot allocate.*memory|GC overhead limit exceeded",
            auto_fix: AutoFixKind::IncreaseMemory,
            transformer: None,
        },
        // ---- Mod loader resolution (9 rules) ----
        CrashRule {
            id: "mod_resolution",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Mod resolution failed|errors were found!|(?P<mod>[\w-]+) requires (?P<version>[\w.-]+) which is missing|Missing or unsupported mandatory dependencies|Incompatible mods found!",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_mod_name_capture),
        },
        CrashRule {
            id: "duplicated_mod",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)DuplicateModsFoundException|Found a duplicate mod|Found duplicate mods|Duplicate mod|(?P<mod>[\w-]+) is already loaded|Duplicate entry|ModResolutionException: Duplicate",
            auto_fix: AutoFixKind::ClearMods,
            transformer: Some(extract_mod_name_capture),
        },
        CrashRule {
            id: "fabric_mod_conflict",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Fabric mod conflict|conflicts with|incompatible with (?P<mod>[\w-]+)",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_mod_name_capture),
        },
        CrashRule {
            id: "fabric_mod_missing",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Missing mod|requires (?P<mod>[\w-]+) which is missing|Could not find required mod",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_mod_name_capture),
        },
        CrashRule {
            id: "mod_name_invalid",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)Invalid mod name|Mod id .*? is invalid|(?P<mod>[\w-]+) has an invalid id|Invalid module name: '' is not a Java identifier",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_mod_name_capture),
        },
        CrashRule {
            id: "mixin_apply_failed",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Mixin prepare failed|Mixin apply failed|MixinApplyError|MixinTransformerError|mixin\.injection\.throwables|FAILED during|failed to apply mixin (?P<mod>[\w-]+)",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_mod_name_capture),
        },
        CrashRule {
            id: "forge_duplicate_mods",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Found duplicate mods|Forge found duplicate|Duplicate mods found",
            auto_fix: AutoFixKind::ClearMods,
            transformer: None,
        },
        CrashRule {
            id: "incomplete_forge_installation",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Forge installation is incomplete|Missing forge jar|net\.minecraftforge\.fml.*?Missing|Cannot find launch target fmlclient",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        // ---- Forge-specific (3 new rules) ----
        CrashRule {
            id: "forge_multiple_in_json",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Found multiple arguments for option fml\.forgeVersion",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        CrashRule {
            id: "forge_error_screen",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)An exception was thrown, the game will display an error screen and halt",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "forge_old_java_incompat",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.NoSuchMethodError: sun\.security\.util\.ManifestEntryVerifier",
            auto_fix: AutoFixKind::InstallJava(17),
            transformer: None,
        },
        // ---- Graphics / drivers / shaders (4 rules, expanded) ----
        CrashRule {
            id: "graphics_driver",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)GLFW error|OpenGL error|Failed to create display|Pixel format not accelerated|GLX.*?not supported|driver does not support OpenGL|The driver does not appear to support OpenGL|Couldn't set pixel format",
            auto_fix: AutoFixKind::UpdateGraphicsDriver,
            transformer: None,
        },
        CrashRule {
            id: "shaders_mod",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)Shaders? mod|shader.*?failed to (?:compile|link)|Could not load shader|1282: Invalid operation",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "shaders_mod_optifine_conflict",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)Shaders Mod detected.*?Please remove it",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "splashscreen",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)SplashProgress|SplashScreen|failed to initialize splash",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        // ---- ACCESS_VIOLATION by GPU vendor (3 new rules) ----
        CrashRule {
            id: "intel_gpu_access_violation",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)EXCEPTION_ACCESS_VIOLATION.*# C\s+\[ig",
            auto_fix: AutoFixKind::UpdateGraphicsDriver,
            transformer: None,
        },
        CrashRule {
            id: "amd_gpu_access_violation",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)EXCEPTION_ACCESS_VIOLATION.*# C\s+\[atio",
            auto_fix: AutoFixKind::UpdateGraphicsDriver,
            transformer: None,
        },
        CrashRule {
            id: "nvidia_gpu_access_violation",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)EXCEPTION_ACCESS_VIOLATION.*# C\s+\[nvoglv",
            auto_fix: AutoFixKind::UpdateGraphicsDriver,
            transformer: None,
        },
        // ---- Native libraries / class loading (3 rules) ----
        CrashRule {
            id: "unsatisfied_link_error",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.UnsatisfiedLinkError|no .*? in java\.library\.path|cannot load library|failed to load native",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        CrashRule {
            id: "no_class_def_found",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.NoClassDefFoundError|java\.lang\.ClassNotFoundException",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        CrashRule {
            id: "file_already_exists",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)java\.nio\.file\.FileAlreadyExistsException|File already exists at (?P<path>[^\n\r]+)",
            auto_fix: AutoFixKind::None,
            transformer: Some(extract_path_capture),
        },
        // ---- Specific mods (4 rules, expanded) ----
        CrashRule {
            id: "optifine_conflict",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)OptiFine.*?conflict|ShaderMod.*?conflict|jade.*?optifine|rtss.*?sodium",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "optifine_incompatible",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)OptiFine is incompatible|cannot run with OptiFine|TRANSFORMER/net\.optifine|OptiFine.*?与.*?Forge.*?不兼容|NoSuchMethodError.*optifine",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "content_verification_failed",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)signer information does not match signer information of other classes in the same package",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        CrashRule {
            id: "textures_too_large",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)Maybe try a lower resolution resourcepack\?|texture.*?too large|texture.*?resolution.*?exceed",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        // ---- JVM runtime issues (4 rules) ----
        CrashRule {
            id: "openj9_unsupported",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)OpenJ9|openj9.*?unsupported|Unsupported charset on OpenJ9|Open J9 is not supported|OpenJ9 is incompatible|\.J9VMInternals\.",
            auto_fix: AutoFixKind::InstallJava(17),
            transformer: None,
        },
        CrashRule {
            id: "javaagent_failed",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)processing of javaagent failed|-javaagent.*?failed",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "using_jdk_instead_of_jre",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)java\.lang\.ClassCastException: (?:java\.base/jdk|class jdk\.)",
            auto_fix: AutoFixKind::InstallJava(17),
            transformer: None,
        },
        CrashRule {
            id: "mixin_bootstrap_missing",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.ClassNotFoundException: org\.spongepowered\.asm\.launch\.MixinTweaker",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        // ---- Config / resource pack (3 rules) ----
        CrashRule {
            id: "config_error",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)Failed to load config|config.*?exception|MalformedJsonException.*?config|Failed loading config file",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "resourcepack_resolution",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)Failed to load resourcepack|resourcepack.*?resolution failed|invalid resource pack",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        // ---- Security / bootstrap (3 rules) ----
        CrashRule {
            id: "security_exception",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)java\.lang\.SecurityException|access denied|signer information does not match",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        CrashRule {
            id: "bootstrap_failed",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)Bootstrap failed|Failed to bootstrap|net\.minecraft\.server\.Bootstrap.*?failed",
            auto_fix: AutoFixKind::ReinstallInstance,
            transformer: None,
        },
        // ---- Specific game issues (4 new rules) ----
        CrashRule {
            id: "extracted_mod_jar",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)The directories below appear to be extracted jar files|Extracted mod jars found",
            auto_fix: AutoFixKind::OpenModsFolder,
            transformer: None,
        },
        CrashRule {
            id: "max_id_range_exceeded",
            severity: CrashSeverity::Fatal,
            pattern: r"(?mi)maximum id range exceeded",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "specific_block_crash",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)\tBlock location: World: ",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        CrashRule {
            id: "specific_entity_crash",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)\tEntity's Exact location: ",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        // ---- Night Config fix ----
        CrashRule {
            id: "night_config_fixes",
            severity: CrashSeverity::Warning,
            pattern: r"(?mi)NightConfigFixes|NightConfig.*?exception|com\.electronwill\.nightconfig.*?error|com\.electronwill\.nightconfig\.core\.io\.ParsingException",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
        // ---- Debug crash ----
        CrashRule {
            id: "debug_crash",
            severity: CrashSeverity::Info,
            pattern: r"(?mi)# This is a debug crash|Manually triggered crash|Manually triggered debug crash",
            auto_fix: AutoFixKind::None,
            transformer: None,
        },
    ];

    raw.into_iter()
        .filter_map(|rule| {
            let id = rule.id;
            let pattern = rule.pattern;
            Regex::new(pattern)
                .map(|re| (rule, re))
                .map_err(|e| {
                    tracing::error!(
                        "Invalid regex in crash rule {id}: {e} - {pattern}"
                    );
                    e
                })
                .ok()
        })
        .collect()
});

// ---------------------------------------------------------------------------
// Rule strings catalog
// ---------------------------------------------------------------------------

pub fn rule_strings(id: &str) -> (&'static str, &'static str, &'static str) {
    match id {
        "too_old_java" => (
            "Java is too old",
            "Your Java version is too old for this Minecraft version or installed mods. Most modern modpacks require at least Java 17, and the latest Minecraft versions require Java 21.",
            "Install a newer Java runtime (Java 17 for 1.17-1.20.4, Java 21 for 1.20.5+).",
        ),
        "need_jdk17" => (
            "Java 17 required",
            "The game or a mod was compiled for Java 17 (class file version 61) or requires Java 11+. The active runtime is too old.",
            "Install Java 17 (e.g. Eclipse Temurin 17) and select it for this instance in Instance Settings → Java.",
        ),
        "need_jdk21" => (
            "Java 21 required",
            "Minecraft 1.20.5 and newer require Java 21 (class file version 65). The active runtime is too old.",
            "Install Java 21 (e.g. Eclipse Temurin 21) and select it for this instance.",
        ),
        "java_version_too_high" => (
            "Java version is too high",
            "Minecraft rejected the active Java runtime because its version is too new or modules are not exported.",
            "Use a stable LTS release (Java 17 or Java 21) instead of cutting-edge builds (Java 22+).",
        ),
        "jvm_32bit" => (
            "32-bit JVM detected",
            "The game is running on a 32-bit Java runtime which cannot address enough memory and fails on modern modpacks.",
            "Install a 64-bit Java runtime and reselect it in Instance Settings → Java.",
        ),
        "out_of_memory" => (
            "Out of memory",
            "The JVM ran out of heap memory. Modern modpacks often need 4-8 GB. This could also indicate a system memory shortage.",
            "Increase the maximum memory allocation in Instance Settings → Java. 4 GB is the recommended floor for modpacks.",
        ),
        "memory_exceeded" => (
            "Memory allocation exceeded",
            "The configured memory is larger than what the OS could allocate, or GC could not keep up.",
            "Reduce memory to a sane value (under 12 GB) or free system RAM before launching.",
        ),
        "mod_resolution" => (
            "Mod dependency resolution failed",
            "A mod is missing a required dependency or the installed version does not satisfy another mod's requirements.",
            "Open the instance's Mod tab and review the dependencies listed. Install the missing mod(s) or downgrade the demanding mod.",
        ),
        "duplicated_mod" => (
            "Duplicate mod detected",
            "Two copies of the same mod (possibly different versions) are installed simultaneously.",
            "Open the instance's Mod tab and remove the duplicate. Check both `mods` and `mods/<version>` subfolders.",
        ),
        "fabric_mod_conflict" => (
            "Fabric mod conflict",
            "Two Fabric mods declare an incompatibility but are both loaded.",
            "Remove one of the conflicting mods, or check for a compat bridge mod such as PolyMc or compat layers.",
        ),
        "fabric_mod_missing" => (
            "Missing Fabric mod",
            "A required Fabric mod is not installed. The log lists which mod is missing.",
            "Install the missing mod listed in the diagnosis fragment, matching the loader and Minecraft version of the instance.",
        ),
        "mod_name_invalid" => (
            "Invalid mod identifier",
            "A mod's id or filename violates naming rules (uppercase letters, special characters, etc.).",
            "Rename the offending mod file to only use letters, numbers, hyphens (-), underscores (_), and periods (.).",
        ),
        "mixin_apply_failed" => (
            "Mixin application failed",
            "A Mixin could not be applied to its target class. Usually means a version mismatch between a mod and its target.",
            "Update all mixins-using mods to versions matching the active loader and Minecraft version.",
        ),
        "forge_duplicate_mods" => (
            "Forge found duplicate mods",
            "Forge detected duplicate mod entries in the load list.",
            "Remove duplicates from the `mods` folder and from any nested version-specific subfolders.",
        ),
        "incomplete_forge_installation" => (
            "Forge installation incomplete",
            "Forge libraries or the forge jar itself are missing. Usually caused by a corrupted download or interrupted install.",
            "Reinstall Forge via Instance Settings → Loader. This will re-download missing files.",
        ),
        "forge_multiple_in_json" => (
            "Multiple Forge in instance JSON",
            "The instance JSON contains conflicting Forge version entries, possibly from being modified by another launcher.",
            "Reinstall Forge cleanly rather than modifying the Forge version through other tools.",
        ),
        "forge_error_screen" => (
            "Forge error screen",
            "Forge encountered an error and displayed its error screen before halting. The error message itself is usually the key clue.",
            "Read the error message on the Forge error screen. It typically lists the specific mod or config that caused the problem.",
        ),
        "forge_old_java_incompat" => (
            "Old Forge + New Java incompatible",
            "An older Forge version is incompatible with the current Java runtime.",
            "Update Forge to 36.2.26 or newer, or downgrade Java to a version below 1.8.0.320.",
        ),
        "graphics_driver" => (
            "Graphics driver issue",
            "OpenGL/GLFW initialization failed. Usually means the GPU driver is outdated or the GPU is below minimum spec.",
            "Update your graphics driver to the latest version from the vendor (NVIDIA / AMD / Intel). On hybrid laptops, force the launcher to use the discrete GPU.",
        ),
        "shaders_mod" => (
            "Shader / OpenGL error",
            "A shader pack failed to compile or an OpenGL 1282 error occurred. Could be an incompatible shader, resource pack, or a driver issue.",
            "Try removing resource packs or shaders. Sodium + Iris users should ensure both are at matching versions.",
        ),
        "shaders_mod_optifine_conflict" => (
            "ShaderMod + OptiFine conflict",
            "ShaderMod and OptiFine are both installed, which is unnecessary — OptiFine has built-in shader support.",
            "Remove Shaders Mod. OptiFine already includes shader support.",
        ),
        "splashscreen" => (
            "Splash screen crash",
            "The vanilla splash screen resource failed to load. Usually caused by a missing asset or corrupted resource pack.",
            "Verify game assets via Instance Settings, or remove custom resource packs temporarily.",
        ),
        "intel_gpu_access_violation" => (
            "Intel GPU driver crash (ACCESS_VIOLATION)",
            "The Intel graphics driver caused a memory access violation. This is a known issue with certain Intel driver versions.",
            "Update your Intel graphics driver, or force Minecraft to use a discrete GPU instead of Intel integrated graphics.",
        ),
        "amd_gpu_access_violation" => (
            "AMD GPU driver crash (ACCESS_VIOLATION)",
            "The AMD graphics driver caused a memory access violation. This is a known issue with certain driver versions.",
            "Update your AMD graphics driver to the latest version or roll back to an OEM-provided version.",
        ),
        "nvidia_gpu_access_violation" => (
            "NVIDIA GPU driver crash (ACCESS_VIOLATION)",
            "The NVIDIA graphics driver caused a memory access violation.",
            "Update your NVIDIA graphics driver to the latest version or try rolling back to a stable release.",
        ),
        "unsatisfied_link_error" => (
            "Native library load failure",
            "A native library (LWJGL, GLFW, or mod-provided .dll/.so/.dylib) could not be loaded.",
            "Reinstall the instance so natives are re-extracted. On ARM64 devices, ensure you have a compatible Java runtime.",
        ),
        "no_class_def_found" => (
            "Class not found",
            "The JVM tried to load a class that is not on the classpath. Usually means a mod is corrupted or its dependency is missing.",
            "Reinstall the instance, or remove the mod listed in the exception. Check for missing library jars.",
        ),
        "file_already_exists" => (
            "File already exists",
            "A file operation was blocked because the target already exists. Often a leftover from a previous run.",
            "Manually delete the listed file, or use the Repair instance option to refresh assets.",
        ),
        "optifine_conflict" => (
            "OptiFine / shader conflict",
            "OptiFine (or a shader pack) conflicts with another installed mod such as Sodium, Jade, or RTSS overlay.",
            "Remove OptiFine or the conflicting mod. Consider switching to Sodium + Iris for better compatibility.",
        ),
        "optifine_incompatible" => (
            "OptiFine incompatible",
            "OptiFine explicitly refuses to run or crashes with the current loader / Minecraft version.",
            "Remove OptiFine, or switch to a Minecraft version that OptiFine supports.",
        ),
        "content_verification_failed" => (
            "File verification failed",
            "Jar signer information does not match, or file integrity check failed. Often caused by corrupted downloads.",
            "Reinstall the instance to refresh signed jars. Consider using a VPN during re-download if the issue persists.",
        ),
        "textures_too_large" => (
            "Texture too large for GPU",
            "The resource pack resolution exceeds what your GPU can handle.",
            "Remove high-resolution resource packs, or upgrade your graphics card.",
        ),
        "openj9_unsupported" => (
            "OpenJ9 not supported",
            "Minecraft and most mods require a HotSpot JVM. OpenJ9 (IBM's JVM) is not compatible.",
            "Install a HotSpot-based runtime such as Eclipse Temurin, Microsoft Build of OpenJDK, or Zulu.",
        ),
        "javaagent_failed" => (
            "JavaAgent failed",
            "A -javaagent argument failed to initialize. Often an outdated agent or one incompatible with the current Java.",
            "Remove the -javaagent argument from JVM flags, or update the agent jar.",
        ),
        "using_jdk_instead_of_jre" => (
            "Using JDK instead of JRE",
            "The game is using a JDK runtime. Some older mods are incompatible with JDK class structures.",
            "Switch to a JRE (not JDK) in Instance Settings → Java. Eclipse Temurin JRE is recommended.",
        ),
        "mixin_bootstrap_missing" => (
            "MixinBootstrap missing",
            "MixinBootstrap is required but not installed. This is a dependency for some mod mixins.",
            "Install MixinBootstrap. If it still crashes, try prefixing the filename with an exclamation mark (!).",
        ),
        "config_error" => (
            "Configuration error",
            "A configuration file failed to parse. Often caused by manual editing or a mod version mismatch.",
            "Delete the offending config file in the instance's `config` folder; the mod will recreate it with defaults.",
        ),
        "resourcepack_resolution" => (
            "Resource pack resolution failed",
            "A resource pack could not be loaded, often due to malformed metadata or wrong pack_format.",
            "Remove or update the offending resource pack from the instance's `resourcepacks` folder.",
        ),
        "security_exception" => (
            "Security exception",
            "A jar was signed with mismatched signer information, or the JVM sandbox blocked an operation.",
            "Reinstall the instance to refresh signed jars. Avoid mixing mods from untrusted sources.",
        ),
        "bootstrap_failed" => (
            "Bootstrap failure",
            "Minecraft's bootstrap registry failed to initialize. Usually means a critical registry entry is missing or corrupted.",
            "Reinstall the instance. If it persists, verify the Minecraft version jar is not corrupted.",
        ),
        "extracted_mod_jar" => (
            "Mod jar was extracted",
            "One or more mods were extracted instead of being kept as jar files. Extracted mod jars prevent the game from loading.",
            "Delete the extracted mod folders from the `mods` directory. Keep mods as jar files only.",
        ),
        "max_id_range_exceeded" => (
            "Mod ID limit exceeded",
            "Too many mods are installed, exceeding Minecraft's internal ID limit.",
            "Install JEID or similar ID-extension mod, or remove some large content mods.",
        ),
        "specific_block_crash" => (
            "Specific block caused crash",
            "A particular block in the world triggered the crash. This often happens with modded blocks.",
            "Create a new world to test if the issue persists. If not, the specific block/world needs to be removed or fixed.",
        ),
        "specific_entity_crash" => (
            "Specific entity caused crash",
            "A particular entity in the world triggered the crash. This often happens with modded entities.",
            "Create a new world to test. If the crash is entity-specific, you may need to remove the entity using external tools.",
        ),
        "night_config_fixes" => (
            "Night Config issue",
            "A mod using NightConfig (Forge config library) crashed while reading a config file.",
            "Delete the affected config file in the instance's `config` folder, or install a NightConfigFixes compat mod.",
        ),
        "debug_crash" => (
            "Manually triggered debug crash",
            "This crash was triggered manually using F3 + C. It is not a real crash.",
            "Ignore this crash. Avoid holding F3 + C if you do not intend to trigger a debug crash.",
        ),
        "unknown_crash" => (
            "Unknown crash",
            "No matching rule was found for this crash. Please review the raw crash report for details.",
            "Search the Modrinth help center (support.modrinth.com) or report this on GitHub with the crash log attached.",
        ),
        _ => (
            "Unknown crash",
            "No matching rule was found for this crash. Please review the raw crash report for details.",
            "Search the Modrinth help center (support.modrinth.com) or report this on GitHub with the crash log attached.",
        ),
    }
}

// ---------------------------------------------------------------------------
// Stack keyword analysis - extracts mod identifiers from stack traces
// ---------------------------------------------------------------------------

/// Extract potential mod identifiers from a stack trace by filtering out
/// Java/MC/Forge system packages and common words.
pub fn analyze_stack_keywords(error_text: &str) -> Vec<String> {
    let mut stack_results: Vec<String> = Vec::new();

    // Pattern 1: Standard Java package names in stack lines
    // e.g. "com.example.mod.SomeClass.method" → "com.example.mod"
    let re1 = Regex::new(r"(?m)(?:at |Caused by: )[\w.$]+?\.([a-zA-Z_]+\w*\.[a-zA-Z_]+\w*[\w.]*)\.[\w.$]+?\(")
        .unwrap();
    for cap in re1.captures_iter(error_text) {
        if let Some(pkg) = cap.get(1) {
            stack_results.push(pkg.as_str().to_string());
        }
    }

    // Pattern 2: Mixin stack with dollar signs
    // e.g. "xxx.xxx.xxxx$xxxx$xxx" → parts between $ become candidate keywords
    let re2 = Regex::new(r"(?m)at [^(]+?\.\w+\$\w+\$([\w$]+?)\$\w+\(").unwrap();
    for cap in re2.captures_iter(error_text) {
        if let Some(s) = cap.get(1) {
            for part in s.as_str().split('$') {
                if !part.is_empty() {
                    stack_results.push(part.to_string());
                }
            }
        }
    }

    stack_results.sort();
    stack_results.dedup();

    // Filter out system packages and common words
    let mut filtered: Vec<String> = Vec::new();
    for stack in &stack_results {
        let lower = stack.to_lowercase();
        let is_system = SYSTEM_PACKAGES.iter().any(|pkg| lower.starts_with(pkg));
        if is_system {
            continue;
        }

        // Also filter out names that are just common words
        let segments: Vec<&str> = stack.split('.').collect();
        let mut valid = false;
        for seg in &segments {
            if !COMMON_WORDS.contains(&seg.to_lowercase().as_str()) && seg.len() > 2 {
                valid = true;
                break;
            }
        }
        if valid {
            filtered.push(stack.clone());
        }
    }

    // If too many results, it's likely incorrect matching - discard
    if filtered.len() > 10 {
        tracing::info!(
            "[Crash] Stack analysis found {} keywords (>10), discarding as likely false positives",
            filtered.len()
        );
        return vec![];
    }

    tracing::info!(
        "[Crash] Stack analysis found {} potential mod keywords: {:?}",
        filtered.len(),
        filtered
    );
    filtered
}

/// Try to map stack keywords to actual mod jar filenames by cross-referencing
/// with the crash report's mod list.
pub fn map_keywords_to_mod_files(
    keywords: &[String],
    crash_report: &str,
    debug_log: Option<&str>,
) -> Option<Vec<String>> {
    if keywords.is_empty() {
        return None;
    }

    let mut matched_files: Vec<String> = Vec::new();

    // Strategy 1: Look for .jar files in the crash report after "System Details"
    if let Some(system_section) = crash_report.split("System Details").nth(1) {
        // Forge format: lines with .jar
        for line in system_section.lines() {
            if !line.contains(".jar") {
                continue;
            }
            let lower = line.to_lowercase().replace('_', "");
            for kw in keywords {
                if lower.contains(&kw.to_lowercase().replace('_', "")) {
                    // Try to extract the jar filename
                    if let Some(jar) = extract_jar_from_line(line) {
                        if !jar.contains("minecraft.jar")
                            && !jar.starts_with("forge-")
                            && !jar.starts_with("mixin-")
                        {
                            matched_files.push(jar);
                        }
                    }
                    break;
                }
            }
        }
    }

    // Strategy 2: Check crash report Fabric mod list format
    if matched_files.is_empty() && crash_report.contains("Fabric Mods") {
        if let Some(fabric_section) = crash_report.split("Fabric Mods").nth(1) {
            for line in fabric_section.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || !trimmed.starts_with('\t') {
                    continue;
                }
                let lower = trimmed.to_lowercase().replace('_', "");
                for kw in keywords {
                    if lower.contains(&kw.to_lowercase().replace('_', "")) {
                        // Fabric lines: "modid: ModName version"
                        if let Some(re) = Regex::new(r"(?<=: )[^\n]+(?= [^\n]+$)").ok() {
                            if let Some(cap) = re.captures(trimmed) {
                                matched_files.push(cap.get(0).unwrap().as_str().to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Strategy 3: Use debug.log for Forge mod discovery
    if let Some(debug) = debug_log {
        // "Found valid mod file Xxx.jar with {modid} mods"
        let re = Regex::new(r"(?m)Found valid mod file (.+?) with \{").unwrap();
        for cap in re.captures_iter(debug) {
            let file = cap.get(1).unwrap().as_str();
            let file_lower = file.to_lowercase().replace('_', "");
            for kw in keywords {
                if file_lower.contains(&kw.to_lowercase().replace('_', "")) {
                    matched_files.push(file.to_string());
                    break;
                }
            }
        }
    }

    if matched_files.is_empty() {
        return None;
    }

    matched_files.sort();
    matched_files.dedup();
    tracing::info!(
        "[Crash] Mapped {} keywords to {} mod files: {:?}",
        keywords.len(),
        matched_files.len(),
        matched_files
    );
    Some(matched_files)
}

fn extract_jar_from_line(line: &str) -> Option<String> {
    // Forge format: "| NAME | file.jar | ... |" or "(file.jar)"
    let patterns = [
        Regex::new(r"(?<=\| )[^\t|]+\.jar").ok(),
        Regex::new(r"\([^\t)]+\.jar\)").ok(),
        Regex::new(r"[^\s/\\]+\.jar").ok(),
    ];
    for pat in patterns.iter().flatten() {
        if let Some(cap) = pat.find(line) {
            let s = cap.as_str();
            let s = s.trim_start_matches('(').trim_end_matches(')');
            if s.ends_with(".jar") {
                return Some(s.to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Multi-source log collection
// ---------------------------------------------------------------------------

/// Collect logs from all available sources for an instance.
/// Returns prioritized list of (source, content) pairs.
pub async fn collect_all_logs(
    _instance_path: &str,
    logs_dir: &std::path::Path,
    crash_dir: &std::path::Path,
    max_age_secs: Option<u64>,
) -> Vec<(CrashLogSource, String)> {
    let mut results: Vec<(CrashLogSource, String)> = Vec::new();
    let now = SystemTime::now();

    // Helper to check if a file is fresh enough
    let is_fresh = |mtime: SystemTime| -> bool {
        if let Some(max) = max_age_secs {
            now.duration_since(mtime)
                .unwrap_or_default()
                .as_secs()
                <= max
        } else {
            true
        }
    };

    // 1) Crash reports (highest priority)
    if crash_dir.exists() {
        if let Ok(mut entries) = collect_crash_report_files(crash_dir) {
            entries.sort_by(|a, b| b.1.cmp(&a.1));
            for (path, mtime) in entries {
                if !is_fresh(mtime) {
                    continue;
                }
                if let Ok(content) = read_crash_report(&path).await {
                    tracing::info!(
                        "[Crash] Collected crash report: {}",
                        path.display()
                    );
                    results.push((CrashLogSource::CrashReport, content));
                }
                // Only take the most recent crash report
                break;
            }
        }
    }

    // 2) Minecraft logs (latest.log, debug.log)
    let log_files = ["latest.log", "debug.log"];
    for log_name in &log_files {
        let log_path = logs_dir.join(log_name);
        if log_path.exists() {
            if let Ok(meta) = std::fs::metadata(&log_path) {
                if let Ok(mtime) = meta.modified() {
                    if is_fresh(mtime) {
                        if let Ok(content) = tokio::fs::read_to_string(&log_path).await {
                            tracing::info!(
                                "[Crash] Collected Minecraft log: {}",
                                log_path.display()
                            );
                            results.push((CrashLogSource::MinecraftLog, content));
                        }
                    }
                }
            }
        }
    }

    // 3) hs_err_pid logs (JVM crash)
    if let Some(parent) = logs_dir.parent() {
        // hs_err_pid files are usually in the instance root or game dir
        let search_dirs = [parent, logs_dir];
        for dir in &search_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("hs_err_pid") && p.is_file() {
                            if let Ok(meta) = p.metadata() {
                                if let Ok(mtime) = meta.modified() {
                                    if is_fresh(mtime) {
                                        if let Ok(content) =
                                            tokio::fs::read_to_string(&p).await
                                        {
                                            tracing::info!(
                                                "[Crash] Collected hs_err log: {}",
                                                p.display()
                                            );
                                            results.push((
                                                CrashLogSource::HsErrLog,
                                                content,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 4) Launcher output (launcher_log.txt) - lowest priority fallback
    let launcher_log = logs_dir.join("launcher_log.txt");
    if launcher_log.exists() {
        if let Ok(meta) = std::fs::metadata(&launcher_log) {
            if let Ok(mtime) = meta.modified() {
                if is_fresh(mtime) {
                    if let Ok(content) = tokio::fs::read_to_string(&launcher_log).await {
                        tracing::info!(
                            "[Crash] Collected launcher output: {}",
                            launcher_log.display()
                        );
                        results.push((CrashLogSource::LauncherOutput, content));
                    }
                }
            }
        }
    }

    tracing::info!(
        "[Crash] Multi-source collection gathered {} log sources",
        results.len()
    );
    results
}

fn collect_crash_report_files(
    crash_dir: &std::path::Path,
) -> std::io::Result<Vec<(std::path::PathBuf, SystemTime)>> {
    let mut entries: Vec<(std::path::PathBuf, SystemTime)> = Vec::new();
    for entry in std::fs::read_dir(crash_dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("crash-") {
            continue;
        }
        if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
            entries.push((p, mtime));
        }
    }
    Ok(entries)
}

// ---------------------------------------------------------------------------
// Core analysis
// ---------------------------------------------------------------------------

/// Analyze a crash log (or any output buffer) and return matched rules in order.
///
/// Also performs stack keyword analysis for mod identification if applicable.
pub fn analyze(log: &str) -> CrashDiagnosisResult {
    let scanned_bytes = log.len();
    let has_crash_report_header = log.contains("---- Minecraft Crash Report ----")
        || log.contains("Minecraft Crash Report");

    let excerpt: String = log.chars().take(200).collect();

    let mut matched = Vec::new();
    let mut seen_ids: HashSet<&str> = HashSet::new();

    for (rule, regex) in RULES.iter() {
        if let Some(caps) = regex.captures(log) {
            if seen_ids.insert(rule.id) {
                let (title, description, fix) = rule_strings(rule.id);
                let fragment = rule.transformer.and_then(|t| {
                    let s = t(&caps);
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                });
                matched.push(CrashDiagnosisMatch {
                    rule_id: rule.id.to_string(),
                    severity: rule.severity,
                    title: title.to_string(),
                    description: description.to_string(),
                    fix: fix.to_string(),
                    auto_fix: rule.auto_fix,
                    fragment,
                    mod_files: None,
                });
            }
        }
    }

    // If nothing matched but we did detect a crash report header, surface an
    // "unknown" entry so the UI can still offer a "view raw report" affordance.
    if matched.is_empty() && has_crash_report_header {
        let (title, description, fix) = rule_strings(NO_MATCH_RULE_ID);
        matched.push(CrashDiagnosisMatch {
            rule_id: NO_MATCH_RULE_ID.to_string(),
            severity: CrashSeverity::Warning,
            title: title.to_string(),
            description: description.to_string(),
            fix: fix.to_string(),
            auto_fix: AutoFixKind::None,
            fragment: None,
            mod_files: None,
        });
    }

    CrashDiagnosisResult {
        matched,
        scanned_bytes,
        has_crash_report_header,
        excerpt,
        generated_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        sources: vec![],
    }
}

/// Fetch the latest crash report path for an instance.
pub async fn get_latest_crash_report_path(
    instance_id: &str,
    max_age_secs: Option<u64>,
) -> crate::Result<Option<std::path::PathBuf>> {
    let state = crate::State::get().await?;

    let instance_path: String = sqlx::query_scalar(
        "SELECT path FROM instances WHERE id = ? OR path = ? ORDER BY CASE WHEN id = ? THEN 0 ELSE 1 END LIMIT 1",
    )
    .bind(instance_id)
    .bind(instance_id)
    .bind(instance_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unknown instance id or path: {instance_id}"
        ))
        .as_error()
    })?;

    let crash_dir = state.directories.crash_reports_dir(&instance_path);
    if !crash_dir.exists() {
        return Ok(None);
    }

    let mut entries: Vec<(std::path::PathBuf, SystemTime)> = std::fs::read_dir(&crash_dir)
        .map_err(|e| crate::util::io::IOError::with_path(e, &crash_dir))?
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_file() {
                return None;
            }
            let name = p.file_name()?.to_string_lossy().to_string();
            if !name.starts_with("crash-") {
                return None;
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((p, mtime))
        })
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    if let Some((path, mtime)) = entries.into_iter().next() {
        if let Some(max) = max_age_secs {
            let elapsed = SystemTime::now()
                .duration_since(mtime)
                .unwrap_or_default();
            if elapsed.as_secs() > max {
                return Ok(None);
            }
        }
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

/// Read a crash report file (plain text or gzip-compressed) into a string.
pub async fn read_crash_report(path: &std::path::Path) -> crate::Result<String> {
    let bytes = tokio::fs::read(path).await.map_err(|e| {
        crate::util::io::IOError::with_path(e, path)
    })?;

    if path.extension().is_some_and(|e| e == "gz") {
        let mut decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
        let mut out = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut out).map_err(|e| {
            crate::util::io::IOError::with_path(e, path)
        })?;
        Ok(out)
    } else {
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// Top-level entry: analyze all available logs for the instance using
/// multi-source collection. Also performs stack keyword → mod name mapping.
pub async fn diagnose_latest_crash(
    instance_id: &str,
    max_age_secs: Option<u64>,
) -> crate::Result<CrashDiagnosisResult> {
    let state = crate::State::get().await?;

    let instance_path: String = sqlx::query_scalar(
        "SELECT path FROM instances WHERE id = ? OR path = ? ORDER BY CASE WHEN id = ? THEN 0 ELSE 1 END LIMIT 1",
    )
    .bind(instance_id)
    .bind(instance_id)
    .bind(instance_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unknown instance id or path: {instance_id}"
        ))
        .as_error()
    })?;

    let crash_dir = state.directories.crash_reports_dir(&instance_path);
    let logs_dir = state.directories.instance_logs_dir(&instance_path);

    // Multi-source collection
    let sources = collect_all_logs(
        &instance_path,
        &logs_dir,
        &crash_dir,
        max_age_secs,
    )
    .await;

    if sources.is_empty() {
        return Ok(CrashDiagnosisResult {
            matched: vec![],
            scanned_bytes: 0,
            has_crash_report_header: false,
            excerpt: String::new(),
            generated_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sources: vec![],
        });
    }

    let source_types: Vec<CrashLogSource> =
        sources.iter().map(|(s, _)| s.clone()).collect();

    // Merge all log content for analysis
    let mut combined = String::new();
    for (source, content) in &sources {
        if !combined.is_empty() {
            combined.push_str("\n---\n");
        }
        combined.push_str(&format!("[Source: {:?}]\n", source));
        combined.push_str(content);
    }

    // Run regex rules against the combined log
    let mut result = analyze(&combined);

    // Perform stack keyword analysis for mod identification
    let has_loader = combined.contains("orge")  // Forge
        || combined.contains("abric")           // Fabric
        || combined.contains("uilt")            // Quilt
        || combined.contains("iteloader");      // LiteLoader

    if has_loader {
        let keywords = analyze_stack_keywords(&combined);
        if !keywords.is_empty() {
            // Find the crash report content for mod file mapping
            let crash_content = sources
                .iter()
                .find(|(s, _)| matches!(s, CrashLogSource::CrashReport))
                .map(|(_, c)| c.as_str());

            let debug_content = sources
                .iter()
                .find(|(s, _)| matches!(s, CrashLogSource::MinecraftLog))
                .map(|(_, c)| c.as_str());

            let mod_files = map_keywords_to_mod_files(
                &keywords,
                crash_content.unwrap_or(&combined),
                debug_content,
            );

            if let Some(files) = mod_files {
                // Attach mod files to mod-related matches
                for m in &mut result.matched {
                    if m.rule_id.contains("mod")
                        || m.rule_id.contains("mixin")
                        || m.rule_id.contains("fabric")
                        || m.rule_id.contains("forge")
                        || m.rule_id == "duplicated_mod"
                        || m.rule_id == "mod_resolution"
                    {
                        m.mod_files = Some(files.clone());
                    }
                }
                // If no mod-related rule matched, create a stack analysis entry
                if result.matched.is_empty()
                    || !result.matched.iter().any(|m| {
                        m.rule_id.contains("mod")
                            || m.rule_id.contains("mixin")
                    })
                {
                    let (title, desc, fix) = if files.len() == 1 {
                        (
                            "Suspected mod: ".to_string() + &files[0],
                            format!(
                                "Stack analysis suggests the mod '{}' may have caused the crash. Try disabling it and see if the crash persists.",
                                files[0]
                            ),
                            "Disable the suspected mod and relaunch the game.".to_string(),
                        )
                    } else {
                        (
                            format!("Suspected mods: {}", files.join(", ")),
                            "Stack analysis suggests these mods may have caused the crash. Try disabling them one by one to identify the culprit.".to_string(),
                            "Disable the suspected mods one by one and relaunch the game.".to_string(),
                        )
                    };
                    result.matched.push(CrashDiagnosisMatch {
                        rule_id: "stack_suspected_mods".to_string(),
                        severity: CrashSeverity::Warning,
                        title,
                        description: desc,
                        fix,
                        auto_fix: AutoFixKind::OpenModsFolder,
                        fragment: Some(keywords.join(", ")),
                        mod_files: Some(files),
                    });
                } else if let Some(m) = result.matched.first_mut() {
                    // Attach keywords as fragment for richer display
                    if m.fragment.is_none() {
                        m.fragment = Some(keywords.join(", "));
                    }
                }
            }
        }
    }

    result.sources = source_types;
    Ok(result)
}

/// Analyze an arbitrary log string passed from the frontend (e.g. user pastes
/// a crash log into the diagnosis dialog manually).
pub fn diagnose_raw_log(log: &str) -> CrashDiagnosisResult {
    let mut result = analyze(log);
    result.sources = vec![CrashLogSource::UserProvided];
    result
}

/// Returns a list of all available rule ids (for diagnostic UI / debugging).
pub fn list_rule_ids() -> Vec<&'static str> {
    RULES.iter().map(|(r, _)| r.id).collect()
}

// ---------------------------------------------------------------------------
// Crash report export - bundle all relevant files into a ZIP
// ---------------------------------------------------------------------------

/// Export a crash report ZIP bundle containing:
/// - Crash report file(s)
/// - latest.log / launcher_log.txt
/// - Launcher session log
/// - Environment info summary
pub async fn export_crash_report(
    instance_id: &str,
) -> crate::Result<String> {
    let state = crate::State::get().await?;

    let instance_path: String = sqlx::query_scalar(
        "SELECT path FROM instances WHERE id = ? OR path = ? ORDER BY CASE WHEN id = ? THEN 0 ELSE 1 END LIMIT 1",
    )
    .bind(instance_id)
    .bind(instance_id)
    .bind(instance_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unknown instance id or path: {instance_id}"
        ))
        .as_error()
    })?;

    let crash_dir = state.directories.crash_reports_dir(&instance_path);
    let logs_dir = state.directories.instance_logs_dir(&instance_path);

    // Create temp directory for export
    let temp_dir = std::env::temp_dir().join(format!("mc_crash_export_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to create temp directory: {e}"
        ))
    })?;

    // Collect files to include
    let mut exported: Vec<(String, String)> = Vec::new();

    // 1) Most recent crash report
    if crash_dir.exists() {
        if let Ok(mut entries) = collect_crash_report_files(&crash_dir) {
            entries.sort_by(|a, b| b.1.cmp(&a.1));
            for (path, _) in entries.iter().take(3) {
                if let Ok(content) = tokio::fs::read_to_string(path).await {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        exported.push((format!("crash-reports/{}", name), content));
                    }
                }
            }
        }
    }

    // 2) latest.log
    let latest_log = logs_dir.join("latest.log");
    if latest_log.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&latest_log).await {
            exported.push(("latest.log".to_string(), content));
        }
    }

    // 3) launcher_log.txt
    let launcher_log = logs_dir.join("launcher_log.txt");
    if launcher_log.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&launcher_log).await {
            exported.push(("launcher_log.txt".to_string(), content));
        }
    }

    // 4) Launcher session log
    let settings_dir = &state.directories.settings_dir;
    if let Some(parent) = settings_dir.parent() {
        let launcher_logs = parent.join("launcher_logs");
        if launcher_logs.exists() {
            if let Ok(mut entries) = collect_latest_launcher_logs(&launcher_logs, 1) {
                for (_, content) in entries {
                    exported.push(("launcher_session.log".to_string(), content));
                }
            }
        }
    }

    // 5) Environment info
    let env_info = build_environment_info(&instance_path).await;
    exported.push(("environment_info.txt".to_string(), env_info));

    // Write all files to temp directory
    for (filename, content) in &exported {
        let file_path = temp_dir.join(filename);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&file_path, content).ok();
    }

    // Create ZIP
    let zip_path = temp_dir.with_extension("zip");
    let zip_file = std::fs::File::create(&zip_path).map_err(|e| {
        crate::ErrorKind::OtherError(format!("Failed to create zip file: {e}"))
    })?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (filename, content) in &exported {
        zip.start_file(filename, options).ok();
        zip.write_all(content.as_bytes()).ok();
    }

    zip.finish().map_err(|e| {
        crate::ErrorKind::OtherError(format!("Failed to finalize zip: {e}"))
    })?;

    let zip_path_str = zip_path.to_string_lossy().to_string();
    tracing::info!("[Crash] Exported crash report to: {}", zip_path_str);
    Ok(zip_path_str)
}

fn collect_latest_launcher_logs(
    dir: &std::path::Path,
    count: usize,
) -> std::io::Result<Vec<(String, String)>> {
    let mut entries: Vec<(std::path::PathBuf, SystemTime)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(ext) = p.extension() {
            if ext != "log" {
                continue;
            }
        }
        if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
            entries.push((p, mtime));
        }
    }
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    let mut results = Vec::new();
    for (path, _) in entries.into_iter().take(count) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                results.push((name.to_string(), content));
            }
        }
    }
    Ok(results)
}

async fn build_environment_info(instance_path: &str) -> String {
    let mut info = String::new();
    info.push_str(&format!("Instance path: {}\n", instance_path));
    info.push_str(&format!("OS: {} (64-bit: {})\n", std::env::consts::OS, cfg!(target_arch = "x86_64")));
    info.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    info.push_str(&format!(
        "Generated at: {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Try to get instance info from DB
    if let Ok(state) = crate::State::get().await {
        if let Ok(Some(java_ver)) = sqlx::query_scalar::<_, String>(
            "SELECT java_version FROM instances WHERE path = ? LIMIT 1",
        )
        .bind(instance_path)
        .fetch_optional(&state.pool)
        .await
        {
            info.push_str(&format!("Java version in settings: {}\n", java_ver));
        }
    }

    info
}

// ---------------------------------------------------------------------------
// Auto-fix implementations
// ---------------------------------------------------------------------------

/// Apply an auto-fix action for the given instance.
/// Returns a user-readable message describing what was done.
pub async fn apply_auto_fix(
    instance_id: &str,
    auto_fix: &AutoFixKind,
) -> crate::Result<String> {
    let state = crate::State::get().await?;

    let instance_path: String = sqlx::query_scalar(
        "SELECT path FROM instances WHERE id = ? OR path = ? ORDER BY CASE WHEN id = ? THEN 0 ELSE 1 END LIMIT 1",
    )
    .bind(instance_id)
    .bind(instance_id)
    .bind(instance_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unknown instance id or path: {instance_id}"
        ))
        .as_error()
    })?;

    match auto_fix {
        AutoFixKind::None => Ok("No automatic fix is available for this issue. Please follow the manual fix instructions above.".to_string()),

        AutoFixKind::InstallJava(version) => {
            let url = format!(
                "https://adoptium.net/download/?variant=openjdk{}&jvmVariant=hotspot",
                version
            );
            open::that(&url).map_err(|e| {
                crate::ErrorKind::OtherError(format!("Failed to open browser: {e}"))
            })?;
            Ok(format!(
                "Opened Adoptium download page for Java {} in your browser. Download and install it, then select it in Instance Settings → Java.",
                version
            ))
        }

        AutoFixKind::ReinstallInstance => {
            // Trigger instance reinstall by navigating to settings
            Ok("Please go to Instance Settings and use the 'Reinstall' option to repair this instance.".to_string())
        }

        AutoFixKind::ClearMods => {
            let mods_dir = std::path::Path::new(&instance_path).join("mods");
            if mods_dir.exists() {
                open::that(&mods_dir).map_err(|e| {
                    crate::ErrorKind::OtherError(format!("Failed to open mods folder: {e}"))
                })?;
                Ok("Opened the mods folder. Please remove duplicate or conflicting mods, then relaunch the game.".to_string())
            } else {
                Ok("Mods folder not found. Was the instance installed correctly?".to_string())
            }
        }

        AutoFixKind::UpdateGraphicsDriver => {
            // Open graphics driver download pages based on detected GPU
            open::that("https://www.nvidia.com/download/index.aspx").ok();
            open::that("https://www.amd.com/en/support").ok();
            open::that("https://www.intel.com/content/www/us/en/download-center/home.html").ok();
            Ok("Opened graphics driver download pages. Update your GPU driver, then restart the launcher and try again.".to_string())
        }

        AutoFixKind::IncreaseMemory => {
            Ok("Open Instance Settings → Java and increase the maximum memory allocation. For modpacks, 4-8 GB is recommended.".to_string())
        }

        AutoFixKind::OpenModsFolder => {
            let mods_dir = std::path::Path::new(&instance_path).join("mods");
            if mods_dir.exists() {
                open::that(&mods_dir).map_err(|e| {
                    crate::ErrorKind::OtherError(format!("Failed to open mods folder: {e}"))
                })?;
                Ok("Opened the mods folder.".to_string())
            } else {
                Ok("Mods folder not found.".to_string())
            }
        }

        AutoFixKind::OpenInstanceSettings => {
            Ok("Please navigate to Instance Settings to adjust the relevant options.".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Log cleanup - prevent infinite log growth
// ---------------------------------------------------------------------------

/// Maximum number of launcher session logs to keep
const MAX_LAUNCHER_SESSION_LOGS: usize = 16;

/// Maximum number of crash reports to keep per instance
const MAX_CRASH_REPORTS: usize = 20;

/// Run log cleanup for both launcher session logs and instance crash reports.
pub async fn cleanup_old_logs() {
    // Cleanup launcher session logs
    if let Ok(state) = crate::State::get().await {
        let settings_dir = &state.directories.settings_dir;
        if let Some(parent) = settings_dir.parent() {
            let launcher_logs = parent.join("launcher_logs");
            if launcher_logs.exists() {
                if let Err(e) = cleanup_session_logs(&launcher_logs) {
                    tracing::warn!("Failed to cleanup launcher session logs: {e}");
                }
            }
        }
    }

    // Cleanup crash reports for all instances
    if let Ok(state) = crate::State::get().await {
        if let Ok(instances) = sqlx::query_scalar::<_, String>("SELECT path FROM instances")
            .fetch_all(&state.pool)
            .await
        {
            for instance_path in instances {
                let crash_dir = state.directories.crash_reports_dir(&instance_path);
                if crash_dir.exists() {
                    if let Err(e) = cleanup_crash_reports(&crash_dir) {
                        tracing::warn!(
                            "Failed to cleanup crash reports for {}: {e}",
                            instance_path
                        );
                    }
                }
            }
        }
    }
}

fn cleanup_session_logs(dir: &std::path::Path) -> std::io::Result<()> {
    let mut entries: Vec<(std::path::PathBuf, SystemTime)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(ext) = p.extension() {
            if ext != "log" {
                continue;
            }
        }
        if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
            entries.push((p, mtime));
        }
    }

    if entries.len() <= MAX_LAUNCHER_SESSION_LOGS {
        return Ok(());
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));
    let to_remove: Vec<_> = entries
        .into_iter()
        .skip(MAX_LAUNCHER_SESSION_LOGS)
        .collect();

    let removed = to_remove.len();
    for (path, _) in to_remove {
        std::fs::remove_file(&path)?;
    }

    tracing::info!(
        "[Crash] Cleaned up {} old launcher session log(s), kept {}",
        removed,
        MAX_LAUNCHER_SESSION_LOGS
    );
    Ok(())
}

fn cleanup_crash_reports(dir: &std::path::Path) -> std::io::Result<()> {
    let mut entries: Vec<(std::path::PathBuf, SystemTime)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.starts_with("crash-") {
            continue;
        }
        if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
            entries.push((p, mtime));
        }
    }

    if entries.len() <= MAX_CRASH_REPORTS {
        return Ok(());
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));
    let to_remove: Vec<_> = entries.into_iter().skip(MAX_CRASH_REPORTS).collect();

    let removed = to_remove.len();
    for (path, _) in to_remove {
        std::fs::remove_file(&path)?;
    }

    tracing::info!(
        "[Crash] Cleaned up {} old crash report(s), kept {}",
        removed,
        MAX_CRASH_REPORTS
    );
    Ok(())
}