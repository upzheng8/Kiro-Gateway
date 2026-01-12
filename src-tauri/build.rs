fn main() {
    // Workaround for tauri-build 2.5.3 panic
    // DEP_TAURI_DEV should be provided by tauri crate, but Cargo may not pass it 
    // to build script in certain cross-compilation scenarios.
    // For production builds (tauri build), it should be "false".
    if std::env::var("DEP_TAURI_DEV").is_err() {
        std::env::set_var("DEP_TAURI_DEV", "false");
    }
    tauri_build::build()
}
