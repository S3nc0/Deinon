use std::env;

fn main() {
    let mut attributes = tauri_build::Attributes::new();

    // Tauri needs Common Controls v6 on Windows, which only a manifest provides. Its default only
    // reaches the app itself, so test programs fail to start. Embedding the manifest through the
    // linker covers both.
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && msvc {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = env::current_dir()
            .expect("the build script has a working folder")
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }

    tauri_build::try_build(attributes).expect("error while building the Tauri context");
}
