//! Java version constraint system - maps Minecraft game versions to
//! required/suggested Java runtime versions.
//!
//! Ported and adapted from HMCL's `GameJavaVersion` and
//! `JavaVersionConstraint`. The rules encode which Java major version a given
//! Minecraft version needs (mandatory) or works best with (suggested).
//!
//! Key design:
//! - `get_minimum_java_version(game_version)` returns the minimum Java major
//!   version required by a vanilla Minecraft version.
//! - `JavaVersionConstraint` encodes both mandatory and suggested rules for
//!   modded scenarios (Forge, LaunchWrapper, etc.).
//! - `find_suitable_java()` picks the best available Java runtime for a given
//!   game version from a list of detected JREs.

use crate::state::JavaVersion;
use serde::{Deserialize, Serialize};

/// A (game version, minimum Java major) pair, ported from HMCL's GameJavaVersion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameJavaVersion {
    /// Component name used by Mojang's version manifest (e.g. "java-runtime-gamma").
    pub component: &'static str,
    /// Major version number (e.g. 8, 17, 21).
    pub major_version: u32,
}

/// Pre-defined Java version constants matching Mojang's manifest components.
pub const JAVA_25: GameJavaVersion = GameJavaVersion {
    component: "java-runtime-epsilon",
    major_version: 25,
};
pub const JAVA_21: GameJavaVersion = GameJavaVersion {
    component: "java-runtime-delta",
    major_version: 21,
};
pub const JAVA_17: GameJavaVersion = GameJavaVersion {
    component: "java-runtime-beta",
    major_version: 17,
};
pub const JAVA_16: GameJavaVersion = GameJavaVersion {
    component: "java-runtime-alpha",
    major_version: 16,
};
pub const JAVA_8: GameJavaVersion = GameJavaVersion {
    component: "jre-legacy",
    major_version: 8,
};

/// Returns the minimum Java major version required by vanilla Minecraft for
/// the given game version string (e.g. "1.20.5", "1.17", "1.7.10").
///
/// Rules (ported from HMCL `GameJavaVersion.getMinimumJavaVersion`):
/// - >= 26.1      → Java 25
/// - >= 1.20.5    → Java 21
/// - >= 1.18      → Java 17
/// - >= 1.17      → Java 16
/// - >= 1.13      → Java 8
/// - < 1.13       → None (any Java 6+ will do)
pub fn get_minimum_java_version(game_version: &str) -> Option<GameJavaVersion> {
    let v = normalize_game_version(game_version);
    if cmp_game_version(&v, "26.1") >= 0 {
        Some(JAVA_25)
    } else if cmp_game_version(&v, "1.20.5") >= 0 {
        Some(JAVA_21)
    } else if cmp_game_version(&v, "1.18") >= 0 {
        Some(JAVA_17)
    } else if cmp_game_version(&v, "1.17") >= 0 {
        Some(JAVA_16)
    } else if cmp_game_version(&v, "1.13") >= 0 {
        Some(JAVA_8)
    } else {
        None
    }
}

/// Constraint level for a Java version rule.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintLevel {
    /// The game will not start without a matching Java version.
    Mandatory,
    /// The game may start but could crash; this version is recommended.
    Suggested,
}

/// A single constraint rule linking a Minecraft version range to a Java
/// version range. Simplified from HMCL's enum-based approach to a data
/// structure for easier runtime construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionConstraint {
    /// Human-readable rule id (e.g. "vanilla", "modded_java_17").
    pub id: &'static str,
    pub level: ConstraintLevel,
    /// Minimum Minecraft version (inclusive). Empty = no lower bound.
    pub game_min: &'static str,
    /// Maximum Minecraft version (inclusive). Empty = no upper bound.
    pub game_max: &'static str,
    /// Minimum Java major version (inclusive).
    pub java_min: u32,
    /// Maximum Java major version (inclusive). 0 = no upper bound.
    pub java_max: u32,
    /// Optional loader filter; if set, the rule only applies when the
    /// instance uses this loader.
    pub loader: Option<&'static str>,
    /// Human-readable description of the rule.
    pub description: &'static str,
}

/// The built-in constraint table. Covers vanilla + Forge modded scenarios.
///
/// These are the most commonly encountered rules. More specialized rules
/// (ModLauncher-8, LaunchWrapper, Cleanroom) are included as suggested
/// constraints for informational purposes.
pub static CONSTRAINTS: &[JavaVersionConstraint] = &[
    // --- Vanilla mandatory rules (from game version → minimum Java) ---
    JavaVersionConstraint {
        id: "vanilla_java_8",
        level: ConstraintLevel::Mandatory,
        game_min: "1.13",
        game_max: "1.16.999",
        java_min: 8,
        java_max: 0,
        loader: None,
        description: "Minecraft 1.13–1.16 requires at least Java 8.",
    },
    JavaVersionConstraint {
        id: "vanilla_java_16",
        level: ConstraintLevel::Mandatory,
        game_min: "1.17",
        game_max: "1.17.999",
        java_min: 16,
        java_max: 0,
        loader: None,
        description: "Minecraft 1.17 requires at least Java 16.",
    },
    JavaVersionConstraint {
        id: "vanilla_java_17",
        level: ConstraintLevel::Mandatory,
        game_min: "1.18",
        game_max: "1.20.4",
        java_min: 17,
        java_max: 0,
        loader: None,
        description: "Minecraft 1.18–1.20.4 requires at least Java 17.",
    },
    JavaVersionConstraint {
        id: "vanilla_java_21",
        level: ConstraintLevel::Mandatory,
        game_min: "1.20.5",
        game_max: "",
        java_min: 21,
        java_max: 0,
        loader: None,
        description: "Minecraft 1.20.5+ requires at least Java 21.",
    },
    // --- Forge suggested rules (modded scenarios) ---
    JavaVersionConstraint {
        id: "modded_java_8_forge",
        level: ConstraintLevel::Suggested,
        game_min: "1.7.10",
        game_max: "1.16.999",
        java_min: 8,
        java_max: 8,
        loader: Some("forge"),
        description: "Forge for Minecraft 1.7.10–1.16 works best with Java 8.",
    },
    JavaVersionConstraint {
        id: "modded_java_16_forge",
        level: ConstraintLevel::Suggested,
        game_min: "1.17",
        game_max: "1.17.999",
        java_min: 16,
        java_max: 16,
        loader: Some("forge"),
        description: "Forge for Minecraft 1.17 works best with Java 16.",
    },
    JavaVersionConstraint {
        id: "modded_java_17_forge",
        level: ConstraintLevel::Suggested,
        game_min: "1.18",
        game_max: "1.20.4",
        java_min: 17,
        java_max: 17,
        loader: Some("forge"),
        description: "Forge for Minecraft 1.18–1.20.4 works best with Java 17.",
    },
    JavaVersionConstraint {
        id: "modded_java_21_forge",
        level: ConstraintLevel::Suggested,
        game_min: "1.20.5",
        game_max: "",
        java_min: 21,
        java_max: 21,
        loader: Some("forge"),
        description: "Forge for Minecraft 1.20.5+ works best with Java 21.",
    },
    // --- Legacy LaunchWrapper (MC <= 1.12 with Forge) ---
    JavaVersionConstraint {
        id: "launch_wrapper_java_8",
        level: ConstraintLevel::Suggested,
        game_min: "",
        game_max: "1.12.999",
        java_min: 8,
        java_max: 8,
        loader: Some("forge"),
        description: "Minecraft <= 1.12 with LaunchWrapper requires Java 8.",
    },
];

/// Result of checking a Java version against the constraint table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintCheckResult {
    /// Whether this Java version satisfies all mandatory constraints.
    pub satisfies_mandatory: bool,
    /// Whether this Java version satisfies all suggested constraints.
    pub satisfies_suggested: bool,
    /// The minimum Java major version required by the game version (if any).
    pub minimum_java_major: Option<u32>,
    /// List of violated mandatory rule ids (empty if all satisfied).
    pub violated_mandatory: Vec<String>,
    /// List of violated suggested rule ids (informational).
    pub violated_suggested: Vec<String>,
    /// Human-readable summary of the check.
    pub summary: String,
}

/// Check whether a given Java runtime is suitable for a Minecraft version.
///
/// `loader` is an optional loader filter (e.g. "forge", "fabric", "quilt").
/// If `None`, loader-specific rules are skipped.
pub fn check_java_for_version(
    game_version: &str,
    java: &JavaVersion,
    loader: Option<&str>,
) -> ConstraintCheckResult {
    let min_java = get_minimum_java_version(game_version).map(|g| g.major_version);
    let v = normalize_game_version(game_version);

    let mut violated_mandatory = Vec::new();
    let mut violated_suggested = Vec::new();
    let java_major = java.parsed_version;

    for rule in CONSTRAINTS {
        // Skip loader-specific rules that don't match
        if let Some(rule_loader) = rule.loader {
            if let Some(loader) = loader {
                if rule_loader != loader {
                    continue;
                }
            } else {
                continue;
            }
        }

        // Check game version range
        if !rule.game_min.is_empty() && cmp_game_version(&v, rule.game_min) < 0 {
            continue;
        }
        if !rule.game_max.is_empty() && cmp_game_version(&v, rule.game_max) > 0 {
            continue;
        }

        // Check Java version range
        let too_low = java_major < rule.java_min;
        let too_high = rule.java_max > 0 && java_major > rule.java_max;

        if too_low || too_high {
            match rule.level {
                ConstraintLevel::Mandatory => {
                    violated_mandatory.push(rule.id.to_string());
                }
                ConstraintLevel::Suggested => {
                    violated_suggested.push(rule.id.to_string());
                }
            }
        }
    }

    // Also check the vanilla minimum (the authoritative mandatory floor)
    if let Some(min) = min_java {
        if java_major < min {
            if !violated_mandatory
                .iter()
                .any(|r| r.starts_with("vanilla_"))
            {
                violated_mandatory.push(format!("vanilla_minimum_java_{}", min));
            }
        }
    }

    let satisfies_mandatory = violated_mandatory.is_empty();
    let satisfies_suggested = violated_suggested.is_empty();

    let summary = build_summary(
        satisfies_mandatory,
        satisfies_suggested,
        min_java,
        java_major,
        &violated_mandatory,
        &violated_suggested,
    );

    ConstraintCheckResult {
        satisfies_mandatory,
        satisfies_suggested,
        minimum_java_major: min_java,
        violated_mandatory,
        violated_suggested,
        summary,
    }
}

/// Find the best Java runtime for a given game version from a list of
/// detected JREs. Prefers the smallest major version that satisfies all
/// mandatory constraints, then suggested constraints.
///
/// Returns `None` if no suitable Java is found.
pub fn find_suitable_java<'a>(
    game_version: &str,
    available: &'a [JavaVersion],
    loader: Option<&str>,
) -> Option<&'a JavaVersion> {
    let min_java = get_minimum_java_version(game_version).map(|g| g.major_version);

    // Filter: must satisfy mandatory constraints
    let mut candidates: Vec<&JavaVersion> = available
        .iter()
        .filter(|jv| {
            check_java_for_version(game_version, jv, loader).satisfies_mandatory
        })
        .collect();

    if candidates.is_empty() {
        // No Java satisfies mandatory - return the closest match if any
        if let Some(min) = min_java {
            // Return the lowest version Java >= min, if any
            return available
                .iter()
                .filter(|jv| jv.parsed_version >= min)
                .min_by_key(|jv| jv.parsed_version);
        }
        return None;
    }

    // Prefer the smallest major version that also satisfies suggested constraints
    let suggested_ok: Vec<&JavaVersion> = candidates
        .iter()
        .copied()
        .filter(|jv| {
            check_java_for_version(game_version, jv, loader).satisfies_suggested
        })
        .collect();

    if !suggested_ok.is_empty() {
        candidates = suggested_ok;
    }

    // Pick the smallest major version (closest to the minimum requirement)
    candidates.into_iter().min_by_key(|jv| jv.parsed_version)
}

/// Get the recommended Java major version for a Minecraft game version.
/// This is the suggested version, not just the minimum.
pub fn get_recommended_java_major(
    game_version: &str,
    loader: Option<&str>,
) -> Option<u32> {
    let v = normalize_game_version(game_version);
    let min = get_minimum_java_version(game_version).map(|g| g.major_version)?;

    // Check if any suggested rule narrows the range further
    for rule in CONSTRAINTS {
        if rule.level != ConstraintLevel::Suggested {
            continue;
        }
        if let Some(rule_loader) = rule.loader {
            if Some(rule_loader) != loader {
                continue;
            }
        }
        if !rule.game_min.is_empty() && cmp_game_version(&v, rule.game_min) < 0 {
            continue;
        }
        if !rule.game_max.is_empty() && cmp_game_version(&v, rule.game_max) > 0 {
            continue;
        }
        if rule.java_max > 0 && rule.java_max >= min {
            return Some(rule.java_max);
        }
    }

    Some(min)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Normalize a game version string for comparison.
/// Strips leading "v", "snapshot"/"pre"/"rc" suffixes, etc.
fn normalize_game_version(version: &str) -> String {
    let v = version.trim().trim_start_matches('v');

    // Strip everything after a non-numeric suffix (e.g. "1.20.4-pre1" → "1.20.4")
    let mut result = String::new();
    let mut last_dot = true; // allow leading digits
    for (i, c) in v.chars().enumerate() {
        if c.is_ascii_digit() {
            result.push(c);
            last_dot = false;
        } else if c == '.' && !last_dot {
            result.push(c);
            last_dot = true;
        } else {
            // First non-digit, non-dot character ends the numeric portion
            let _ = i;
            break;
        }
    }

    // Trim trailing dots
    while result.ends_with('.') {
        result.pop();
    }

    result
}

/// Compare two game version strings (e.g. "1.20.5" vs "1.18").
/// Returns negative if `a < b`, 0 if equal, positive if `a > b`.
fn cmp_game_version(a: &str, b: &str) -> i32 {
    let a_parts: Vec<u32> = normalize_game_version(a)
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let b_parts: Vec<u32> = normalize_game_version(b)
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    let len = a_parts.len().max(b_parts.len());
    for i in 0..len {
        let av = a_parts.get(i).copied().unwrap_or(0);
        let bv = b_parts.get(i).copied().unwrap_or(0);
        if av != bv {
            return av as i32 - bv as i32;
        }
    }
    0
}

fn build_summary(
    satisfies_mandatory: bool,
    satisfies_suggested: bool,
    min_java: Option<u32>,
    java_major: u32,
    violated_mandatory: &[String],
    violated_suggested: &[String],
) -> String {
    if satisfies_mandatory && satisfies_suggested {
        if let Some(min) = min_java {
            return format!(
                "Java {java_major} is compatible (minimum required: Java {min})."
            );
        }
        return format!("Java {java_major} is compatible.");
    }

    if !satisfies_mandatory {
        let min_str = min_java
            .map(|m| format!("Java {m}"))
            .unwrap_or_else(|| "a suitable version".to_string());
        let rules = violated_mandatory.join(", ");
        return format!(
            "Java {java_major} does not meet the mandatory requirement ({min_str}). Violated rules: {rules}."
        );
    }

    // Mandatory OK but suggested not met
    let rules = violated_suggested.join(", ");
    format!(
        "Java {java_major} meets the minimum requirement but does not match the suggested version. Violated suggestions: {rules}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_java_version() {
        assert_eq!(
            get_minimum_java_version("1.20.5"),
            Some(JAVA_21)
        );
        assert_eq!(
            get_minimum_java_version("1.18.2"),
            Some(JAVA_17)
        );
        assert_eq!(
            get_minimum_java_version("1.17.1"),
            Some(JAVA_16)
        );
        assert_eq!(
            get_minimum_java_version("1.14.4"),
            Some(JAVA_8)
        );
        assert_eq!(get_minimum_java_version("1.7.10"), None);
    }

    #[test]
    fn test_version_comparison() {
        assert!(cmp_game_version("1.20.5", "1.18") > 0);
        assert!(cmp_game_version("1.16.5", "1.17") < 0);
        assert!(cmp_game_version("1.20.4", "1.20.4") == 0);
        assert!(cmp_game_version("1.20.5-pre1", "1.20.4") > 0);
    }

    #[test]
    fn test_find_suitable_java() {
        let javas = vec![
            JavaVersion {
                parsed_version: 8,
                path: "/java/8/bin/java".into(),
                version: "1.8.0_362".into(),
                architecture: "x86_64".into(),
            },
            JavaVersion {
                parsed_version: 17,
                path: "/java/17/bin/java".into(),
                version: "17.0.9".into(),
                architecture: "x86_64".into(),
            },
            JavaVersion {
                parsed_version: 21,
                path: "/java/21/bin/java".into(),
                version: "21.0.1".into(),
                architecture: "x86_64".into(),
            },
        ];

        // MC 1.20.5 requires Java 21
        let found = find_suitable_java("1.20.5", &javas, None);
        assert_eq!(found.map(|j| j.parsed_version), Some(21));

        // MC 1.18 requires Java 17
        let found = find_suitable_java("1.18.2", &javas, None);
        assert_eq!(found.map(|j| j.parsed_version), Some(17));

        // MC 1.14 requires Java 8
        let found = find_suitable_java("1.14.4", &javas, None);
        assert_eq!(found.map(|j| j.parsed_version), Some(8));
    }
}
