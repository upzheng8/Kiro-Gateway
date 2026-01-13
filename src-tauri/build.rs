fn main() {
    // Workaround for tauri-build 2.5.3 compatibility issue
    // See: https://github.com/tauri-apps/tauri/issues/10591
    // tauri-build expects DEP_TAURI_DEV env var from tauri crate's build script
    println!("cargo:dev={}", cfg!(debug_assertions));

    tauri_build::build();
}
