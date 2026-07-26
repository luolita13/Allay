/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

/*

JavaVersion {
    path: Path
    version: String
}

*/

export async function get_java_versions() {
	return await invoke('plugin:jre|get_java_versions')
}

export async function set_java_version(javaVersion) {
	return await invoke('plugin:jre|set_java_version', { javaVersion })
}

// Finds all the installation of Java 7, if it exists
// Returns [JavaVersion]
export async function find_filtered_jres(version) {
	return await invoke('plugin:jre|jre_find_filtered_jres', { version })
}

// Gets java version from a specific path by trying to run 'java -version' on it.
// This also validates it, as it returns null if no valid java version is found at the path
export async function get_jre(path) {
	return await invoke('plugin:jre|jre_get_jre', { path })
}

// Tests JRE version by running 'java -version' on it.
// Returns true if the version is valid, and matches given (after extraction)
export async function test_jre(path, majorVersion) {
	return await invoke('plugin:jre|jre_test_jre', { path, majorVersion })
}

// Automatically installs specified java version
export async function auto_install_java(javaVersion) {
	return await invoke('plugin:jre|jre_auto_install_java', { javaVersion })
}

// Get max memory in KiB
export async function get_max_memory() {
	return await invoke('plugin:jre|jre_get_max_memory')
}

// --- Java version constraint system ---

/**
 * Returns the minimum Java major version required by a vanilla Minecraft
 * version, or null if no specific minimum applies (e.g. MC < 1.13).
 *
 * @param {string} gameVersion - e.g. "1.20.5", "1.17", "1.7.10"
 * @returns {Promise<{component: string, major_version: number} | null>}
 */
export async function get_minimum_java_version(gameVersion) {
	return await invoke('plugin:jre|jre_get_minimum_java_version', { gameVersion })
}

/**
 * Returns the recommended Java major version for a Minecraft version and
 * optional loader. Falls back to the minimum if no suggested rule applies.
 *
 * @param {string} gameVersion
 * @param {string|null} loader - e.g. "forge", "fabric", "quilt"
 * @returns {Promise<number|null>}
 */
export async function get_recommended_java_major(gameVersion, loader = null) {
	return await invoke('plugin:jre|jre_get_recommended_java_major', {
		gameVersion,
		loader,
	})
}

/**
 * Check whether a given Java runtime is suitable for a Minecraft version.
 *
 * @param {string} gameVersion
 * @param {string} javaPath - path to the java executable
 * @param {string|null} loader
 * @returns {Promise<{
 *   satisfies_mandatory: boolean,
 *   satisfies_suggested: boolean,
 *   minimum_java_major: number|null,
 *   violated_mandatory: string[],
 *   violated_suggested: string[],
 *   summary: string,
 * }>}
 */
export async function check_java_for_version(gameVersion, javaPath, loader = null) {
	return await invoke('plugin:jre|jre_check_java_for_version', {
		gameVersion,
		javaPath,
		loader,
	})
}

/**
 * Find the best Java runtime for a game version from all detected JREs.
 *
 * @param {string} gameVersion
 * @param {string|null} loader
 * @returns {Promise<{path: string, version: string, parsed_version: number, architecture: string} | null>}
 */
export async function find_suitable_java(gameVersion, loader = null) {
	return await invoke('plugin:jre|jre_find_suitable_java', {
		gameVersion,
		loader,
	})
}
