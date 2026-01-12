fn main() {
    // Workaround for tauri-build requiring DEP_TAURI_DEV instruction
    // This instruction is normally provided by the tauri crate, but may not be available
    // during cross-compilation or when version mismatch occurs.
    // For production builds (tauri build), it should be "false".
    if std::env::var("DEP_TAURI_DEV").is_err() {
        println!("cargo:rustc-env=DEP_TAURI_DEV=false");
    }
    tauri_build::build();
}
