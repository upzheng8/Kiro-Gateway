use std::sync::Arc;
use crate::{
    admin, admin_ui, anthropic, 
    http_client::ProxyConfig, 
    kiro::{self, provider::KiroProvider, token_manager::MultiTokenManager},
    model::config::Config,
    token,
};
use kiro::model::credentials::CredentialsConfig;
use tokio::sync::watch;
use tower_http::cors::{CorsLayer, Any};

/// 核心启动逻辑
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

    // 判断是否为多凭据格式
    let is_multiple_format = credentials_config.is_multiple();

    // 转换为按优先级排序的凭据列表
    let credentials_list = credentials_config.into_sorted_credentials();
    tracing::info!("已加载 {} 个凭据配置", credentials_list.len());

    // 获取 API Key
    let api_key = config.api_key.clone().unwrap_or_else(|| {
        tracing::error!("配置文件中未设置 apiKey");
        std::process::exit(1);
    });

    // 构建代理配置
    let proxy_config = config.proxy_url.as_ref().map(|url| {
        let mut proxy = ProxyConfig::new(url);
        if let (Some(username), Some(password)) = (&config.proxy_username, &config.proxy_password) {
            proxy = proxy.with_auth(username, password);
        }
        proxy
    });

    if let Some(url) = &config.proxy_url {
        tracing::info!("已配置 HTTP 代理: {}", url);
    }

    // 创建 MultiTokenManager 和 KiroProvider
    let token_manager = MultiTokenManager::new(
        config.clone(),
        credentials_list,
        proxy_config.clone(),
        Some(credentials_path.into()),
        is_multiple_format,
    )?;
    
    let token_manager = Arc::new(token_manager);
    let kiro_provider = KiroProvider::with_proxy(token_manager.clone(), proxy_config.clone());

    // 初始化 count_tokens 配置
    token::init_config(token::CountTokensConfig {
        api_url: config.count_tokens_api_url.clone(),
        api_key: config.count_tokens_api_key.clone(),
        auth_type: config.count_tokens_auth_type.clone(),
        proxy: proxy_config,
    });

    // 构建 Anthropic API 路由 (使用第一个凭据的 profile_arn 占位，实际由 Provider 动态处理)
    // 注意：这里逻辑稍微简化，Router 依赖 provider，provider 内部处理轮询
    let first_credentials = token_manager.credentials(); // 获取当前活跃的凭据
    
    let anthropic_app = anthropic::create_router_with_provider(
        &api_key,
        Some(kiro_provider),
        first_credentials.profile_arn.clone(),
    );

    // 始终启用 Admin API，不再检查 admin_api_key
    let admin_service = admin::AdminService::new(token_manager.clone());
    let admin_state = admin::AdminState::new("", admin_service); // 空密钥，因为已移除认证
    let admin_app = admin::create_admin_router(admin_state);
    let admin_ui_app = admin_ui::create_admin_ui_router();

    tracing::info!("Admin API / UI 已启用");
    
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
    
    let app = anthropic_app
        .route("/", axum::routing::get(health_check))
        .route("/health", axum::routing::get(health_check))
        .route("/ping", axum::routing::get(health_check))
        .nest("/api/admin", admin_app)
        .nest("/admin", admin_ui_app)
        .layer(cors);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("启动监听: {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    // 使用 with_graceful_shutdown 支持停止
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
            tracing::info!("收到停止信号，正在关闭服务...");
        })
        .await?;

    Ok(())
}
