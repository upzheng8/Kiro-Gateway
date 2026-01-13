use std::env;

fn main() {
    // Workaround for tauri-build 2.5.3 compatibility issue
    // tauri-build expects DEP_TAURI_DEV env var from tauri crate
    // Set it before calling tauri_build::build()
    if env::var("DEP_TAURI_DEV").is_err() {
        let is_release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
        env::set_var("DEP_TAURI_DEV", if is_release { "false" } else { "true" });
    }

    tauri_build::build();
}
