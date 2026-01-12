//! Admin API 中间件

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::Mutex;
use tokio::sync::watch;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};

use super::service::AdminService;
use super::types::AdminErrorResponse;
use crate::common::auth;
use crate::model::config::Config;
use crate::kiro::token_manager::MultiTokenManager;
use crate::kiro_server::{AdminContext, ProxyServerController};

/// 反代服务控制器
#[derive(Clone)]
pub struct ProxyController {
    /// 启停控制信号发送器
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    /// 服务运行状态
    running: Arc<AtomicBool>,
}

impl ProxyController {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// 设置关闭信号发送器
    pub fn set_shutdown_sender(&self, tx: watch::Sender<bool>) {
        *self.shutdown_tx.lock() = Some(tx);
        self.running.store(true, Ordering::SeqCst);
    }
    
    /// 清除关闭信号发送器
    pub fn clear_shutdown_sender(&self) {
        *self.shutdown_tx.lock() = None;
        self.running.store(false, Ordering::SeqCst);
    }
    
    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// 停止服务
    pub fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(true);
            self.running.store(false, Ordering::SeqCst);
        }
    }
    
    /// 设置运行状态
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }
}

/// Admin API 共享状态
#[derive(Clone)]
pub struct AdminState {
    /// Admin API 密钥
    pub admin_api_key: String,
    /// Admin 服务
    pub service: Arc<AdminService>,
    /// 配置（用于分组管理）
    pub config: Arc<Mutex<Config>>,
    /// Token 管理器
    pub token_manager: Arc<MultiTokenManager>,
    /// 代理服务是否启用（用户设置的期望状态）
    pub proxy_enabled: Arc<AtomicBool>,
    /// 代理服务控制器（旧版，单端口模式）
    pub proxy_controller: ProxyController,
    /// Admin 上下文（双端口模式）
    pub admin_context: Option<Arc<AdminContext>>,
    /// 反代服务器控制器（双端口模式）
    pub proxy_server_controller: Option<Arc<tokio::sync::Mutex<ProxyServerController>>>,
}

impl AdminState {
    pub fn new(
        admin_api_key: impl Into<String>, 
        service: AdminService,
        config: Arc<Mutex<Config>>,
        token_manager: Arc<MultiTokenManager>,
    ) -> Self {
        Self {
            admin_api_key: admin_api_key.into(),
            service: Arc::new(service),
            config,
            token_manager,
            proxy_enabled: Arc::new(AtomicBool::new(true)), // 默认启用
            proxy_controller: ProxyController::new(),
            admin_context: None,
            proxy_server_controller: None,
        }
    }
    
    /// 获取代理是否启用
    pub fn is_proxy_enabled(&self) -> bool {
        self.proxy_enabled.load(Ordering::SeqCst)
    }
    
    /// 设置代理启用状态
    pub fn set_proxy_enabled(&self, enabled: bool) {
        self.proxy_enabled.store(enabled, Ordering::SeqCst);
    }
    
    /// 获取代理是否正在运行
    pub fn is_proxy_running(&self) -> bool {
        self.proxy_controller.is_running()
    }
}

/// Admin API 认证中间件
pub async fn admin_auth_middleware(
    State(state): State<AdminState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let api_key = auth::extract_api_key(&request);

    match api_key {
        Some(key) if auth::constant_time_eq(&key, &state.admin_api_key) => next.run(request).await,
        _ => {
            let error = AdminErrorResponse::authentication_error();
            (StatusCode::UNAUTHORIZED, Json(error)).into_response()
        }
    }
}
