fn main() {
    // On Windows we replace Tauri's default application manifest so the process
    // is declared per-monitor DPI aware v2. See windows-app-manifest.xml for why
    // the default is not enough and why the Common-Controls block is repeated
    // there. On other platforms the manifest is meaningless, so we hand
    // tauri-build its plain defaults.
    #[cfg(windows)]
    let attributes = {
        println!("cargo:rerun-if-changed=windows-app-manifest.xml");
        tauri_build::Attributes::new().windows_attributes(
            tauri_build::WindowsAttributes::new()
                .app_manifest(include_str!("windows-app-manifest.xml")),
        )
    };

    #[cfg(not(windows))]
    let attributes = tauri_build::Attributes::new();

    // Panicking is the right failure mode in a build script: it is build time,
    // not a runtime path, and a swallowed error here would produce a binary
    // that is quietly not DPI aware.
    if let Err(error) = tauri_build::try_build(attributes) {
        panic!("tauri-build failed: {error}");
    }
}
