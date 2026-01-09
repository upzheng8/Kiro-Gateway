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
mod model;
pub mod token;
mod kiro_server;

use clap::Parser;
use std::thread;
use model::arg::Args;
use model::config::Config;
use kiro::model::credentials::KiroCredentials;
use tauri::Manager;

#[derive(Parser, Debug)]
struct MainArgs {
    #[command(flatten)]
    server_args: Args,
}

fn main() {
    // Parse args to get config paths
    let args = MainArgs::parse();
    
    // 获取可执行文件所在目录，配置文件应放在 EXE 同级目录
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    
    // 默认路径：优先使用 EXE 目录，开发时回退到相对路径
    let default_config = if exe_dir.join("config.json").exists() {
        exe_dir.join("config.json")
    } else if std::path::Path::new("../config.json").exists() {
        std::path::PathBuf::from("../config.json")
    } else {
        std::path::PathBuf::from("config.json")
    };
    
    let default_credentials = if exe_dir.join("credentials.json").exists() {
        exe_dir.join("credentials.json")
    } else if std::path::Path::new("../credentials.json").exists() {
        std::path::PathBuf::from("../credentials.json")
    } else {
        std::path::PathBuf::from("credentials.json")
    };
    
    let config_path = args.server_args.config
        .map(std::path::PathBuf::from)
        .unwrap_or(default_config)
        .to_string_lossy()
        .to_string();
    let credentials_path = args.server_args.credentials
        .map(std::path::PathBuf::from)
        .unwrap_or(default_credentials)
        .to_string_lossy()
        .to_string();
    
    println!("Config path: {}", config_path);
    println!("Credentials path: {}", credentials_path);

    // Spawn the Kiro Server in a separate thread
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
            
        rt.block_on(async {
            // Create a dummy shutdown channel for now, or link it to Tauri app exit event later
            let (_tx, rx) = tokio::sync::watch::channel(false);
            
            if let Err(e) = kiro_server::run_server(config_path, credentials_path, rx).await {
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

