use std::env;

fn main() {
    // Workaround for tauri-build 2.5.3 compatibility issue
    // See: https://github.com/tauri-apps/tauri/issues/10591
    //
    // Problem: tauri-build calls is_dev() which reads DEP_TAURI_DEV env var.
    // This var should be set by tauri crate's build script via `println!("cargo:dev=...")`.
    // However, Cargo compiles build-dependencies before dependencies,
    // so when tauri-build runs, tauri crate hasn't been compiled yet.
    //
    // Solution: Set DEP_TAURI_DEV before calling tauri_build::build()

    // Check if we're in release mode (custom-protocol feature is enabled in release builds)
    let is_release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let dev_value = if is_release { "false" } else { "true" };

    // Set the environment variable that tauri-build expects
    env::set_var("DEP_TAURI_DEV", dev_value);

    tauri_build::build();
}
