#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod admin;
mod anthropic;
mod common;
mod http_client;
mod kiro;
mod logs;
mod model;
pub mod token;
mod kiro_server;
mod model_lock;

use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use model::arg::Args;
use tauri::{Manager, WindowEvent};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tokio::sync::{Mutex, watch};

#[derive(Parser, Debug)]
struct MainArgs {
    #[command(flatten)]
    server_args: Args,
}

/// 服务器状态
#[derive(Clone)]
struct ServerState {
    config_path: String,
    credentials_path: String,
    /// 服务器停止信号发送端
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    /// 服务器运行状态
    is_running: Arc<Mutex<bool>>,
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

/// 确保凭证文件存在，不存在则创建空数组
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

// ============ Tauri Commands ============

/// 获取服务器状态
#[tauri::command]
async fn get_server_status(state: tauri::State<'_, ServerState>) -> Result<serde_json::Value, String> {
    let is_running = *state.is_running.lock().await;
    
    // 读取配置获取监听地址
    let config = match model::config::Config::load(&state.config_path) {
        Ok(c) => c,
        Err(e) => return Err(format!("读取配置失败: {}", e)),
    };
    
    Ok(serde_json::json!({
        "isRunning": is_running,
        "host": config.host,
        "port": config.port
    }))
}

/// 启动服务器
#[tauri::command]
async fn start_proxy_server(state: tauri::State<'_, ServerState>) -> Result<String, String> {
    let mut is_running = state.is_running.lock().await;
    
    if *is_running {
        return Err("服务器已在运行中".to_string());
    }
    
    let config_path = state.config_path.clone();
    let credentials_path = state.credentials_path.clone();
    let shutdown_tx = state.shutdown_tx.clone();
    let is_running_flag = state.is_running.clone();
    
    // 创建新的 shutdown channel
    let (tx, rx) = watch::channel(false);
    {
        let mut shutdown = shutdown_tx.lock().await;
        *shutdown = Some(tx);
    }
    
    // 标记为运行中
    *is_running = true;
    
    // 在新线程中启动服务器
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
            
        rt.block_on(async {
            if let Err(e) = kiro_server::run_server(config_path, credentials_path, rx).await {
                eprintln!("Server Error: {}", e);
            }
            
            // 服务器停止后更新状态
            let mut running = is_running_flag.lock().await;
            *running = false;
        });
    });
    
    Ok("服务器已启动".to_string())
}

/// 停止服务器
#[tauri::command]
async fn stop_proxy_server(state: tauri::State<'_, ServerState>) -> Result<String, String> {
    let mut is_running = state.is_running.lock().await;
    
    if !*is_running {
        return Err("服务器未运行".to_string());
    }
    
    // 发送停止信号
    let shutdown_tx = state.shutdown_tx.lock().await;
    if let Some(tx) = shutdown_tx.as_ref() {
        tx.send(true).map_err(|e| format!("发送停止信号失败: {}", e))?;
    }
    
    *is_running = false;
    
    Ok("服务器已停止".to_string())
}

/// 打开外部 URL
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("打开链接失败: {}", e))
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

    // 创建服务器状态（不自动启动）
    let server_state = ServerState {
        config_path: config_path_str,
        credentials_path: credentials_path_str,
        shutdown_tx: Arc::new(Mutex::new(None)),
        is_running: Arc::new(Mutex::new(false)),
    };

    // Run Tauri Application
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(server_state)
        .invoke_handler(tauri::generate_handler![
            get_server_status,
            start_proxy_server,
            stop_proxy_server,
            open_url,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Optional: Open DevTools in debug mode
            #[cfg(debug_assertions)]
            window.open_devtools();
            
            // 创建系统托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            
            // 创建系统托盘
            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Kiro Gateway")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键单击时显示窗口
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            
            // 保存托盘引用
            app.manage(tray);
            
            // 自动启动 Admin API 服务器（不包含反代）
            let server_state: tauri::State<ServerState> = app.state();
            let config_path = server_state.config_path.clone();
            let credentials_path = server_state.credentials_path.clone();
            
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                    
                rt.block_on(async {
                    if let Err(e) = kiro_server::run_admin_server(config_path, credentials_path).await {
                        eprintln!("Admin Server Error: {}", e);
                    }
                });
            });
            
            Ok(())
        })
        .on_window_event(|window, event| {
            // 拦截关闭事件，改为隐藏到托盘
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
