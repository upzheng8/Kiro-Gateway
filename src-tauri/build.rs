fn main() {
    // Workaround for tauri-build 2.5.3 compatibility issue
    // See: https://github.com/tauri-apps/tauri/issues/10591
    //
    // tauri-build expects DEP_TAURI_DEV env var from tauri crate's build script.
    // This env var is set by Cargo when a dependency outputs `cargo:KEY=VALUE`.
    // The tauri crate should output `cargo:dev=true/false` which becomes DEP_TAURI_DEV.
    //
    // However, due to version mismatch between tauri and tauri-build, this may not work.
    // We set it manually here as a fallback.
    if std::env::var("DEP_TAURI_DEV").is_err() {
        // SAFETY: This is safe in build scripts as they run single-threaded
        unsafe {
            std::env::set_var(
                "DEP_TAURI_DEV",
                if cfg!(debug_assertions) { "true" } else { "false" },
            );
        }
    }

    tauri_build::build();
}
