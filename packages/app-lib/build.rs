use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, exit};
use std::{env, fs};

fn main() {
    println!("cargo::rerun-if-changed=.env");
    println!("cargo::rerun-if-changed=java/gradle");
    println!("cargo::rerun-if-changed=java/src");
    println!("cargo::rerun-if-changed=java/build.gradle.kts");
    println!("cargo::rerun-if-changed=java/settings.gradle.kts");
    println!("cargo::rerun-if-changed=java/gradle.properties");

    setup_easytier_native();
    set_env();
    build_java_jars();
}

/// Add EasyTier's third_party native library paths to the linker search
/// and copy required DLLs to the output directory on Windows.
fn setup_easytier_native() {
    // EasyTier crate is at ../../../EasyTier/easytier relative to this crate
    // CARGO_MANIFEST_DIR = e:\mc\code\packages\app-lib
    // -> .. = packages -> .. = code -> .. = mc -> EasyTier/easytier
    let easytier_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("..")
        .join("..")
        .join("..")
        .join("EasyTier")
        .join("easytier");

    // Add third_party lib search paths for linking (Windows)
    #[cfg(target_os = "windows")]
    {
        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "i686"
        };

        let third_party = easytier_dir.join("third_party").join(arch);
        // Canonicalize to resolve any .. components and get absolute path
        let third_party = dunce::canonicalize(&third_party)
            .unwrap_or_else(|_| third_party.clone());
        println!("cargo::warning=EasyTier third_party path: {}", third_party.display());

        if third_party.exists() {
            println!(
                "cargo::rustc-link-search=native={}",
                third_party.display()
            );

            // Explicitly link Packet.lib (needed by pnet_datalink on Windows)
            let packet_lib = third_party.join("Packet.lib");
            if packet_lib.exists() {
                println!("cargo::rustc-link-search=native={}", third_party.display());
            }

            // Copy DLLs to output directory
            if let Ok(out_dir) = env::var("OUT_DIR") {
                let out_dir = PathBuf::from(out_dir);
                // Go up to the deps directory
                let deps_dir = out_dir
                    .ancestors()
                    .nth(3)
                    .map(|p| p.to_path_buf())
                    .unwrap_or(out_dir.clone());

                for entry in fs::read_dir(&third_party).into_iter().flatten() {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("dll") {
                            let dest = deps_dir.join(entry.file_name());
                            let _ = fs::copy(&path, &dest);
                        }
                    }
                }
            }
        } else {
            println!("cargo::warning=EasyTier third_party directory NOT FOUND: {}", third_party.display());
        }
    }
}

fn set_env() {
    for (var_name, var_value) in
        dotenvy::dotenv_iter().into_iter().flatten().flatten()
    {
        if var_name == "DATABASE_URL" {
            // The sqlx database URL is a build-time detail that should not be exposed to the crate
            continue;
        }

        println!("cargo::rustc-env={var_name}={var_value}");
    }
}

fn build_java_jars() {
    let out_dir =
        dunce::canonicalize(PathBuf::from(env::var_os("OUT_DIR").unwrap()))
            .unwrap();

    println!(
        "cargo::rustc-env=JAVA_JARS_DIR={}",
        out_dir.join("java/libs").display()
    );

    let gradle_path = fs::canonicalize(
        #[cfg(target_os = "windows")]
        "java\\gradlew.bat",
        #[cfg(not(target_os = "windows"))]
        "java/gradlew",
    )
    .unwrap();

    let mut build_dir_str = OsString::from("-Dorg.gradle.project.buildDir=");
    build_dir_str.push(out_dir.join("java"));
    let exit_status = Command::new(gradle_path)
        .arg(build_dir_str)
        .arg("build")
        .arg("--no-daemon")
        .arg("--console=rich")
        .current_dir(dunce::canonicalize("java").unwrap())
        .status()
        .expect("Failed to wait on Gradle build");

    if !exit_status.success() {
        println!("cargo::error=Gradle build failed with {exit_status}");
        exit(exit_status.code().unwrap_or(1));
    }
}
