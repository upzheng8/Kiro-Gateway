fn main() {
    // Required for tauri-build 2.5.x - outputs the dev/release mode instruction
    // This prevents "missing cargo:dev instruction" error in CI environments
    println!("cargo:dev={}", cfg!(debug_assertions));
    tauri_build::build()
}
