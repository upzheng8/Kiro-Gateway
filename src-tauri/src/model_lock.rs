//! 模型锁定监控器
//! 持续监控 Kiro 的 settings.json，当检测到模型被修改时自动恢复为锁定的模型

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::RwLock;
use tokio::time::{interval, Duration};

/// 获取 Kiro settings.json 文件路径
/// 优先查找 profiles 目录下的活跃配置文件
fn get_kiro_settings_path() -> Option<PathBuf> {
    let base_path = get_kiro_base_path()?;
    
    // 优先查找 profiles 目录下的配置文件
    let profiles_dir = base_path.join("User").join("profiles");
    if profiles_dir.exists() {
        // 遍历 profiles 目录，找到包含 settings.json 的子目录
        if let Ok(entries) = fs::read_dir(&profiles_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let profile_settings = entry.path().join("settings.json");
                if profile_settings.exists() {
                    tracing::debug!("使用 profile 配置: {:?}", profile_settings);
                    return Some(profile_settings);
                }
            }
        }
    }
    
    // 回退到默认的 User/settings.json
    let default_path = base_path.join("User").join("settings.json");
    if default_path.exists() {
        return Some(default_path);
    }
    
    // 如果都不存在，返回默认路径（用于创建）
    Some(default_path)
}

/// 获取 Kiro 基础目录
fn get_kiro_base_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|appdata| PathBuf::from(appdata).join("Kiro"))
    }
    
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|home| home.join("Library/Application Support/Kiro"))
    }
    
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .map(|home| home.join(".config/Kiro"))
    }
    
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// 设置 Kiro 的模型选项
fn set_kiro_model(model: &str) -> Result<(), String> {
    let settings_path = get_kiro_settings_path()
        .ok_or("无法获取 Kiro 配置路径")?;
    
    // 读取现有配置
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("读取配置失败: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("解析配置失败: {}", e))?
    } else {
        // 创建目录
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }
        serde_json::json!({})
    };
    
    // 更新模型设置
    settings["kiroAgent.modelSelection"] = serde_json::json!(model);
    
    // 写回配置
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    fs::write(&settings_path, content)
        .map_err(|e| format!("写入配置失败: {}", e))?;
    
    Ok(())
}

/// 获取当前 Kiro 模型设置
fn get_kiro_model() -> Option<String> {
    let settings_path = get_kiro_settings_path()?;
    
    if !settings_path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&settings_path).ok()?;
    let settings: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    settings.get("kiroAgent.modelSelection")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 模型锁定监控器状态
pub struct ModelLockWatcher {
    /// 锁定的模型名称
    locked_model: Arc<RwLock<Option<String>>>,
    /// 是否正在更新（防止循环触发）
    is_updating: Arc<AtomicBool>,
    /// 是否正在运行
    is_running: Arc<AtomicBool>,
}

impl ModelLockWatcher {
    pub fn new() -> Self {
        Self {
            locked_model: Arc::new(RwLock::new(None)),
            is_updating: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// 设置锁定的模型
    pub fn set_locked_model(&self, model: Option<String>) {
        tracing::info!("set_locked_model 被调用: {:?}", model);
        if let Some(ref m) = model {
            // 获取配置路径用于日志
            let path = get_kiro_settings_path();
            tracing::info!("Kiro 配置路径: {:?}", path);
            
            // 立即应用模型设置
            if let Err(e) = set_kiro_model(m) {
                tracing::error!("设置 Kiro 模型失败: {}", e);
            } else {
                tracing::info!("模型已锁定并应用到 Kiro: {}", m);
            }
        } else {
            tracing::info!("模型锁定已取消");
        }
        *self.locked_model.write() = model;
    }
    
    /// 获取锁定的模型
    pub fn get_locked_model(&self) -> Option<String> {
        self.locked_model.read().clone()
    }
    
    /// 启动监控（在单独的任务中运行）
    pub fn start(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }
        
        self.is_running.store(true, Ordering::SeqCst);
        
        let locked_model = Arc::clone(&self.locked_model);
        let is_updating = Arc::clone(&self.is_updating);
        let is_running = Arc::clone(&self.is_running);
        
        tokio::spawn(async move {
            tracing::info!("模型锁定监控任务已启动，轮询间隔: 2秒");
            let mut check_interval = interval(Duration::from_secs(2));
            
            while is_running.load(Ordering::SeqCst) {
                check_interval.tick().await;
                
                // 检查是否有锁定的模型
                let locked = locked_model.read().clone();
                if let Some(locked_model_name) = locked {
                    // 检查是否正在更新
                    if is_updating.load(Ordering::SeqCst) {
                        continue;
                    }
                    
                    // 读取当前模型
                    if let Some(current_model) = get_kiro_model() {
                        if current_model != locked_model_name {
                            tracing::info!("检测到模型被修改: {} -> 恢复为: {}", current_model, locked_model_name);
                            
                            // 设置标志防止循环
                            is_updating.store(true, Ordering::SeqCst);
                            
                            // 恢复锁定的模型
                            if let Err(e) = set_kiro_model(&locked_model_name) {
                                tracing::error!("恢复锁定模型失败: {}", e);
                            }
                            
                            // 延迟后清除标志
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            is_updating.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
            
            tracing::info!("模型锁定监控已停止");
        });
        
        tracing::info!("模型锁定监控已启动");
    }
    
    /// 停止监控
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}

// 全局单例
lazy_static::lazy_static! {
    pub static ref MODEL_LOCK_WATCHER: ModelLockWatcher = ModelLockWatcher::new();
}

/// 启动模型锁定监控
pub fn start_model_lock_watcher() {
    MODEL_LOCK_WATCHER.start();
}

/// 设置锁定的模型
pub fn set_locked_model(model: Option<String>) {
    MODEL_LOCK_WATCHER.set_locked_model(model);
}

/// 获取锁定的模型
pub fn get_locked_model() -> Option<String> {
    MODEL_LOCK_WATCHER.get_locked_model()
}
