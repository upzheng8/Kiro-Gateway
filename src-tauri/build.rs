fn main() {
    // Workaround for tauri-build 2.5.3 compatibility issue
    // See: https://github.com/tauri-apps/tauri/issues/10591
    // tauri-build expects DEP_TAURI_DEV env var but tauri doesn't always provide it
    if std::env::var("DEP_TAURI_DEV").is_err() {
        println!("cargo::metadata=DEP_TAURI_DEV={}", cfg!(debug_assertions));
        unsafe {
            std::env::set_var("DEP_TAURI_DEV", if cfg!(debug_assertions) { "true" } else { "false" });
        }
    }

    tauri_build::build();
}
