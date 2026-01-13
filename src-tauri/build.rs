fn main() {
    // Workaround for tauri-build 2.5.3 compatibility issue
    // See: https://github.com/tauri-apps/tauri/issues/10591
    // tauri-build expects DEP_TAURI_DEV env var from tauri crate's build script
    // but tauri 2.5.0 doesn't provide it, so we set it manually
    unsafe {
        std::env::set_var("DEP_TAURI_DEV", if cfg!(debug_assertions) { "true" } else { "false" });
    }

    tauri_build::build();
}
