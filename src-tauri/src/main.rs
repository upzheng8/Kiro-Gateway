#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod admin;
mod admin_ui;
mod anthropic;
mod common;
mod http_client;
mod kiro;
mod logs;
mod model;
pub mod token;
mod kiro_server;

use clap::Parser;
use std::thread;
use std::path::PathBuf;
use model::arg::Args;
use tauri::Manager;

#[derive(Parser, Debug)]
struct MainArgs {
    #[command(flatten)]
    server_args: Args,
}

/// 获取配置文件目录（使用用户目录下的 .kiro-gateway 文件夹）
fn get_config_dir() -> PathBuf {
    // 使用用户目录下的 .kiro-gateway 文件夹
    if let Some(home_dir) = dirs::home_dir() {
        let config_dir = home_dir.join(".kiro-gateway");
        // 确保目录存在
        if !config_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                eprintln!("Warning: Failed to create config directory: {}", e);
            }
        }
        return config_dir;
    }
    
    // 回退：尝试使用 EXE 所在目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.to_path_buf();
        }
    }
    
    // 最终回退到当前工作目录
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 确保配置文件存在，不存在则创建默认配置
fn ensure_config_file(path: &PathBuf) {
    if !path.exists() {
        let default_config = r#"{
  "host": "127.0.0.1",
  "port": 8990,
  "apiKey": "sk-kiro-gateway-change-me",
  "region": "us-east-1"
}"#;
        if let Err(e) = std::fs::write(path, default_config) {
            eprintln!("Warning: Failed to create default config.json: {}", e);
        } else {
            println!("Created default config.json at: {}", path.display());
        }
    }
}

/// 确保凭据文件存在，不存在则创建空数组
fn ensure_credentials_file(path: &PathBuf) {
    if !path.exists() {
        let default_credentials = "[]";
        if let Err(e) = std::fs::write(path, default_credentials) {
            eprintln!("Warning: Failed to create default credentials.json: {}", e);
        } else {
            println!("Created default credentials.json at: {}", path.display());
        }
    }
}

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse args to get config paths
    let args = MainArgs::parse();
    
    // 获取配置文件目录
    let config_dir = get_config_dir();
    
    // 确定配置文件路径
    let config_path = args.server_args.config
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir.join("config.json"));
    
    let credentials_path = args.server_args.credentials
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir.join("credentials.json"));
    
    // 确保配置文件存在
    ensure_config_file(&config_path);
    ensure_credentials_file(&credentials_path);
    
    println!("=== Kiro Gateway ===");
    println!("Config: {}", config_path.display());
    println!("Credentials: {}", credentials_path.display());

    let config_path_str = config_path.to_string_lossy().to_string();
    let credentials_path_str = credentials_path.to_string_lossy().to_string();

    // Spawn the Kiro Server in a separate thread
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
            
        rt.block_on(async {
            let (_tx, rx) = tokio::sync::watch::channel(false);
            
            if let Err(e) = kiro_server::run_server(config_path_str, credentials_path_str, rx).await {
                eprintln!("Server Error: {}", e);
            }
        });
    });

    // Run Tauri Application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Optional: Open DevTools in debug mode
            #[cfg(debug_assertions)]
            window.open_devtools();
            
            Ok(())
        })
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
