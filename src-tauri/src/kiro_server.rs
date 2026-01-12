use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::{
    admin, anthropic, 
    kiro::{self, provider::KiroProvider, token_manager::MultiTokenManager},
    model::config::Config,
    token,
    logs::LOG_COLLECTOR,
};
use kiro::model::credentials::CredentialsConfig;
use tokio::sync::watch;
use tower_http::cors::{CorsLayer, Any};

/// 尝试绑定端口，如果被占用则自动递增
async fn try_bind_port(host: &str, port: u16, max_attempts: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    for offset in 0..max_attempts {
        let try_port = port + offset;
        let addr = format!("{}:{}", host, try_port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                if offset > 0 {
                    tracing::warn!("端口 {} 被占用，改用端口 {}", port, try_port);
                }
                return Ok((listener, try_port));
            }
            Err(e) => {
                if offset == max_attempts - 1 {
                    return Err(anyhow::anyhow!("无法绑定端口 {}-{}: {}", port, port + max_attempts - 1, e));
                }
            }
        }
    }
    Err(anyhow::anyhow!("无法绑定端口"))
}

/// 共享的 Admin 上下文，用于反代服务控制
#[derive(Clone)]
pub struct AdminContext {
    pub config: Arc<parking_lot::Mutex<Config>>,
    pub token_manager: Arc<MultiTokenManager>,
    pub api_key: String,
    pub credentials_path: String,
}

/// 反代服务控制器
pub struct ProxyServerController {
    shutdown_tx: Option<watch::Sender<bool>>,
    is_running: Arc<AtomicBool>,
}

impl ProxyServerController {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }
    
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
    
    /// 启动反代服务器
    pub async fn start(&mut self, ctx: &AdminContext) -> anyhow::Result<()> {
        if self.is_running() {
            return Ok(());
        }
        
        let (tx, rx) = watch::channel(false);
        self.shutdown_tx = Some(tx);
        self.is_running.store(true, Ordering::SeqCst);
        
        let config = ctx.config.lock().clone();
        let token_manager = ctx.token_manager.clone();
        let api_key = ctx.api_key.clone();
        let is_running = self.is_running.clone();
        
        // 在新任务中运行反代服务器
        tokio::spawn(async move {
            let result = run_proxy_only_server(
                config,
                token_manager,
                api_key,
                rx,
            ).await;
            
            if let Err(e) = result {
                tracing::error!("[反代服务] 运行错误: {}", e);
            }
            
            is_running.store(false, Ordering::SeqCst);
            tracing::info!("[反代服务] 已停止");
        });
        
        // 等待一小段时间让服务器启动
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        Ok(())
    }
    
    /// 停止反代服务器
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        self.is_running.store(false, Ordering::SeqCst);
    }
}

/// 独立的反代服务器（只包含 Anthropic API 端点）
async fn run_proxy_only_server(
    config: Config,
    token_manager: Arc<MultiTokenManager>,
    api_key: String,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    // 同步活跃分组到 token_manager
    token_manager.set_active_group(config.active_group_id.clone());
    
    // 创建 KiroProvider
    let kiro_provider = KiroProvider::with_proxy(token_manager.clone(), None);
    
    // 创建共享的代理启用标志（始终启用，因为停止是通过 shutdown 信号）
    let proxy_enabled = Arc::new(AtomicBool::new(true));
    
    // 构建 Anthropic API 路由
    let first_credentials = token_manager.credentials();
    let anthropic_app = anthropic::create_router_with_provider_and_control(
        &api_key,
        Some(kiro_provider),
        first_credentials.profile_arn.clone(),
        proxy_enabled,
    );
    
    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // 健康检查
    async fn health_check() -> axum::Json<serde_json::Value> {
        axum::Json(serde_json::json!({
            "status": "ok",
            "service": "kiro-gateway-proxy"
        }))
    }
    
    let app = axum::Router::new()
        .route("/", axum::routing::get(health_check))
        .route("/health", axum::routing::get(health_check))
        .merge(anthropic_app)
        .layer(cors);
    
    let (listener, actual_port) = try_bind_port(&config.host, config.proxy_port, 10).await?;
    let group_info = match &config.active_group_id {
        Some(gid) => format!("分组: {}", gid),
        None => "分组: 全部".to_string(),
    };
    tracing::info!("[反代服务] 启动监听: {}:{} ({})", config.host, actual_port, group_info);
    LOG_COLLECTOR.add_log("INFO", &format!("🚀 反代服务已启动: {}:{} ({})", config.host, actual_port, group_info));
    
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
            tracing::info!("[反代服务] 收到停止信号");
            LOG_COLLECTOR.add_log("INFO", "🛑 反代服务已停止");
        })
        .await?;
    
    Ok(())
}

/// 核心启动逻辑（单端口模式，用于 CLI）
/// config_path: 配置文件路径
/// credentials_path: 凭证文件路径
/// shutdown_rx: 停机信号接收器
pub async fn run_server(
    config_path: String,
    credentials_path: String,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    // 加载配置（如果不存在则创建默认配置）
    let config = Config::load_or_create(&config_path).map_err(|e| {
        tracing::error!("加载配置失败: {}", e);
        anyhow::anyhow!("Load Config Error: {}", e)
    })?;

    // 加载凭证（如果不存在则创建空文件）
    let credentials_config = CredentialsConfig::load_or_create(&credentials_path).map_err(|e| {
        tracing::error!("加载凭证失败: {}", e);
        anyhow::anyhow!("Load Credentials Error: {}", e)
    })?;

    // 判断是否为多凭证格式
    let is_multiple_format = credentials_config.is_multiple();

    // 转换为按优先级排序的凭证列表
    let credentials_list = credentials_config.into_sorted_credentials();
    tracing::info!("已加载 {} 个凭证配置", credentials_list.len());

    // 获取 API Key
    let api_key = config.api_key.clone().unwrap_or_else(|| {
        tracing::error!("配置文件中未设置 apiKey");
        std::process::exit(1);
    });

    // 创建 MultiTokenManager 和 KiroProvider
    let token_manager = MultiTokenManager::new(
        config.clone(),
        credentials_list,
        None,
        Some(credentials_path.into()),
        is_multiple_format,
    )?;
    
    let token_manager = Arc::new(token_manager);
    let kiro_provider = KiroProvider::with_proxy(token_manager.clone(), None);

    // 初始化 count_tokens 配置（禁用外部 API）
    token::init_config(token::CountTokensConfig {
        api_url: None,
        api_key: None,
        auth_type: "x-api-key".to_string(),
        proxy: None,
    });

    // 创建共享的代理启用标志
    let proxy_enabled = Arc::new(AtomicBool::new(true));

    // 构建 Anthropic API 路由 (使用第一个凭证的 profile_arn 占位，实际由 Provider 动态处理)
    let first_credentials = token_manager.credentials();
    
    let anthropic_app = anthropic::create_router_with_provider_and_control(
        &api_key,
        Some(kiro_provider),
        first_credentials.profile_arn.clone(),
        proxy_enabled.clone(),
    );

    // 始终启用 Admin API，不再检查 admin_api_key
    let admin_service = admin::AdminService::new(token_manager.clone());
    let config_arc = Arc::new(parking_lot::Mutex::new(config.clone()));
    let mut admin_state = admin::AdminState::new("", admin_service, config_arc, token_manager.clone());
    // 共享代理启用标志
    admin_state.proxy_enabled = proxy_enabled.clone();
    // 设置代理控制器为运行状态
    admin_state.proxy_controller.set_running(true);
    
    let admin_app = admin::create_admin_router(admin_state);

    tracing::info!("Admin API 已启用");
    
    // 配置 CORS 允许跨域请求
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // 健康检查响应
    async fn health_check() -> axum::Json<serde_json::Value> {
        axum::Json(serde_json::json!({
            "status": "ok",
            "service": "kiro-gateway"
        }))
    }
    
    // 创建基础路由（健康检查和 Admin API）
    let base_routes = axum::Router::new()
        .route("/", axum::routing::get(health_check))
        .route("/health", axum::routing::get(health_check))
        .route("/ping", axum::routing::get(health_check))
        .nest("/api/admin", admin_app);
    
    // 合并所有路由
    let app = base_routes
        .merge(anthropic_app)
        .layer(cors);

    let (listener, actual_port) = try_bind_port(&config.host, config.port, 10).await?;
    tracing::info!("启动监听: {}:{}", config.host, actual_port);
    
    // 使用 with_graceful_shutdown 支持停止
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
            tracing::info!("收到停止信号，正在关闭服务...");
        })
        .await?;

    Ok(())
}

/// 双端口模式：Admin API（端口 8990）+ 反代服务（端口 8991）
/// 用于 GUI 模式下运行，支持反代服务独立启停
pub async fn run_dual_port_server(
    config_path: String,
    credentials_path: String,
) -> anyhow::Result<()> {
    // 加载配置
    let config = Config::load_or_create(&config_path).map_err(|e| {
        tracing::error!("加载配置失败: {}", e);
        anyhow::anyhow!("Load Config Error: {}", e)
    })?;

    // 加载凭证
    let credentials_config = CredentialsConfig::load_or_create(&credentials_path).map_err(|e| {
        tracing::error!("加载凭证失败: {}", e);
        anyhow::anyhow!("Load Credentials Error: {}", e)
    })?;

    let is_multiple_format = credentials_config.is_multiple();
    let credentials_list = credentials_config.into_sorted_credentials();
    tracing::info!("已加载 {} 个凭证配置", credentials_list.len());

    // 获取 API Key（反代需要）
    let api_key = config.api_key.clone().unwrap_or_else(|| {
        "sk-kiro-gateway-default".to_string()
    });

    // 创建 MultiTokenManager
    let token_manager = MultiTokenManager::new(
        config.clone(),
        credentials_list,
        None,
        Some(credentials_path.clone().into()),
        is_multiple_format,
    )?;
    
    let token_manager = Arc::new(token_manager);

    // 初始化 count_tokens 配置（禁用外部 API）
    token::init_config(token::CountTokensConfig {
        api_url: None,
        api_key: None,
        auth_type: "x-api-key".to_string(),
        proxy: None,
    });

    // 创建 Admin 上下文（用于反代服务控制）
    let config_arc = Arc::new(parking_lot::Mutex::new(config.clone()));
    let admin_ctx = AdminContext {
        config: config_arc.clone(),
        token_manager: token_manager.clone(),
        api_key: api_key.clone(),
        credentials_path,
    };

    // 创建反代服务控制器
    let mut proxy_controller = ProxyServerController::new();

    // 根据配置决定是否自动启动反代服务
    let proxy_auto_start = config.proxy_auto_start;
    if proxy_auto_start {
        if let Err(e) = proxy_controller.start(&admin_ctx).await {
            tracing::error!("自动启动反代服务失败: {}", e);
        }
    }

    // 启动模型锁定监控
    if let Some(ref locked_model) = config.locked_model {
        tracing::info!("从配置加载锁定模型: {}", locked_model);
        crate::model_lock::set_locked_model(Some(locked_model.clone()));
    }
    crate::model_lock::start_model_lock_watcher();

    // 创建 Admin 服务
    let admin_service = admin::AdminService::new(token_manager.clone());
    let mut admin_state = admin::AdminState::new("", admin_service, config_arc, token_manager.clone());
    
    // 设置代理运行状态
    admin_state.proxy_controller.set_running(proxy_auto_start && proxy_controller.is_running());
    admin_state.proxy_enabled = Arc::new(AtomicBool::new(proxy_auto_start && proxy_controller.is_running()));
    
    // 存储 Admin 上下文和反代控制器到 AdminState
    admin_state.admin_context = Some(Arc::new(admin_ctx));
    admin_state.proxy_server_controller = Some(Arc::new(tokio::sync::Mutex::new(proxy_controller)));
    
    let admin_app = admin::create_admin_router(admin_state);

    tracing::info!("[Admin API] 已启用（双端口模式）");
    
    // 启动后台自动刷新任务
    if config.auto_refresh_enabled {
        let interval_minutes = config.auto_refresh_interval_minutes.max(5); // 至少 5 分钟
        let token_manager_for_refresh = token_manager.clone();
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_secs(interval_minutes as u64 * 60);
            tracing::info!("[自动刷新] 已启动，间隔 {} 分钟", interval_minutes);
            LOG_COLLECTOR.add_log("INFO", &format!("🔄 自动刷新已启动，间隔 {} 分钟", interval_minutes));
            
            loop {
                tokio::time::sleep(interval).await;
                tracing::debug!("[自动刷新] 开始刷新所有凭证...");
                
                // 刷新所有凭证
                let result = token_manager_for_refresh.refresh_all_credentials().await;
                match result {
                    Ok(refreshed) => {
                        if refreshed > 0 {
                            tracing::info!("[自动刷新] 成功刷新 {} 个凭证", refreshed);
                            LOG_COLLECTOR.add_log("INFO", &format!("🔄 自动刷新完成：{} 个凭证已更新", refreshed));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[自动刷新] 刷新失败: {}", e);
                    }
                }
            }
        });
    }
    
    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // 健康检查
    async fn health_check() -> axum::Json<serde_json::Value> {
        axum::Json(serde_json::json!({
            "status": "ok",
            "service": "kiro-gateway-admin"
        }))
    }
    
    // Admin API 路由（不包含反代端点）
    let app = axum::Router::new()
        .route("/", axum::routing::get(health_check))
        .route("/health", axum::routing::get(health_check))
        .route("/ping", axum::routing::get(health_check))
        .nest("/api/admin", admin_app)
        .layer(cors);

    let (listener, actual_port) = try_bind_port(&config.host, config.port, 10).await?;
    tracing::info!("[Admin API] 启动监听: {}:{}", config.host, actual_port);
    tracing::info!("[反代服务] 配置端口: {}", config.proxy_port);
    
    axum::serve(listener, app).await?;

    Ok(())
}

/// 独立模式：Admin API + 可控的反代服务（单端口，旧版兼容）
/// 用于 GUI 模式下运行
pub async fn run_admin_server(
    config_path: String,
    credentials_path: String,
) -> anyhow::Result<()> {
    // 调用双端口模式
    run_dual_port_server(config_path, credentials_path).await
}
