# Tauri v2 跨平台构建问题修复记录

## 问题描述

**日期**: 2026-01-13
**影响版本**: Tauri v2.x
**现象**: GitHub Actions CI 构建时，Windows 平台成功，但 macOS 和 Linux 平台失败

### 错误信息

```
error: failed to run custom build command for `kiro-gateway v1.0.0`

Caused by:
  process didn't exit successfully: `.../build-script-build` (exit status: 101)
  --- stderr
  thread 'main' panicked at /home/runner/.cargo/registry/src/.../tauri-build-2.5.3/src/lib.rs:84:14:
  missing 'cargo:dev' instruction, please update tauri to latest: NotPresent
```

## 根本原因

1. **tauri-build 2.5.3 兼容性问题**: `tauri-build` 期望从 `tauri` crate 的 build script 获取 `DEP_TAURI_DEV` 环境变量
2. **跨平台依赖解析差异**: Cargo 在不同平台上解析依赖时，传递依赖的版本可能不同
3. **Tauri 内部版本冲突**: `tauri` 的传递依赖（如 `tauri-runtime`、`tauri-utils`）在不同平台上可能解析到不兼容的版本

## 解决方案

将大部分依赖移至平台特定的 `[target.'cfg(...)'.dependencies]` 配置段，避免跨平台编译时的依赖冲突。

### 配置结构

```toml
[dependencies]
# 仅保留所有平台通用的核心依赖
crossbeam-channel = "0.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
# ... 其他通用依赖

# Tauri 核心依赖
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-shell = "2"

[target.'cfg(windows)'.dependencies]
# Windows 平台特定依赖
winreg = "0.55"
axum = "0.8"
reqwest = { version = "0.12", features = ["stream", "json", "socks"] }
# ... 其他 Windows 依赖

[target.'cfg(target_os = "macos")'.dependencies]
# macOS 平台特定依赖
axum = "0.8"
reqwest = { version = "0.12", features = ["stream", "json", "socks"] }
# ... 其他 macOS 依赖

[target.'cfg(target_os = "linux")'.dependencies]
# Linux 平台特定依赖
axum = "0.8"
reqwest = { version = "0.12", features = ["stream", "json", "socks"] }
# ... 其他 Linux 依赖

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

## 当前版本配置

### Rust 依赖 (src-tauri/Cargo.toml)

| 依赖 | 版本 | 说明 |
|------|------|------|
| tauri | 2 | Tauri 核心框架 |
| tauri-build | 2 | 构建工具 |
| tauri-plugin-shell | 2 | Shell 插件 |
| tokio | 1.0 | 异步运行时 |
| axum | 0.8 | Web 框架 (平台特定) |
| reqwest | 0.12 | HTTP 客户端 (平台特定) |
| serde | 1.0 | 序列化框架 |
| serde_json | 1.0 | JSON 处理 |

### Node.js 依赖 (package.json)

| 依赖 | 版本 | 说明 |
|------|------|------|
| @tauri-apps/api | ^2 | Tauri 前端 API |
| @tauri-apps/cli | ^2 | Tauri CLI 工具 |
| react | ^18.3.1 | React 框架 |
| vite | ^4.5.0 | 构建工具 |

### 构建环境

| 平台 | Target | 输出格式 |
|------|--------|----------|
| Windows | x86_64-pc-windows-msvc | NSIS (.exe) |
| macOS (Intel) | x86_64-apple-darwin | DMG |
| macOS (Apple Silicon) | aarch64-apple-darwin | DMG |
| Linux | x86_64-unknown-linux-gnu | DEB |

## build.rs 兼容性处理

```rust
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
```

## 其他注意事项

1. **Cargo.lock**: 已添加到 `.gitignore`，避免跨平台 lock 文件冲突
2. **Rust Cache**: CI 中使用 `swatinem/rust-cache@v2`，按平台和 target 区分缓存
3. **版本策略**: 使用宽松版本约束（如 `"2"` 而非 `"=2.5.0"`），让 Cargo 自动解析兼容版本

## 相关提交

- `d7e3e31` - fix: 将依赖拆分为平台特定配置解决跨平台构建问题
- `fcc393b` - fix: 固定 tauri/tauri-build 版本为 2.5 解决 cargo:dev 问题

## 参考资料

- [Tauri v2 Documentation](https://v2.tauri.app/)
- [Cargo Target-specific Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
- [GitHub Actions: tauri-action](https://github.com/tauri-apps/tauri-action)
