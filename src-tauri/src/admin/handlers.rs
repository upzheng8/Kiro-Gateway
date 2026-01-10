//! Admin API HTTP 处理器

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use super::{
    middleware::AdminState,
    types::{AddCredentialRequest, SetDisabledRequest, SetPriorityRequest, SuccessResponse},
};

/// GET /api/admin/credentials
/// 获取所有凭证状态
pub async fn get_all_credentials(State(state): State<AdminState>) -> impl IntoResponse {
    let response = state.service.get_all_credentials();
    Json(response)
}

/// POST /api/admin/credentials/:id/disabled
/// 设置凭证禁用状态
pub async fn set_credential_disabled(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<SetDisabledRequest>,
) -> impl IntoResponse {
    match state.service.set_disabled(id, payload.disabled) {
        Ok(_) => {
            let action = if payload.disabled { "禁用" } else { "启用" };
            Json(SuccessResponse::new(format!("凭证 #{} 已{}", id, action))).into_response()
        }
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/priority
/// 设置凭证优先级
pub async fn set_credential_priority(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<SetPriorityRequest>,
) -> impl IntoResponse {
    match state.service.set_priority(id, payload.priority) {
        Ok(_) => Json(SuccessResponse::new(format!(
            "凭证 #{} 优先级已设置为 {}",
            id, payload.priority
        )))
        .into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/reset
/// 重置失败计数并重新启用
pub async fn reset_failure_count(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.reset_and_enable(id) {
        Ok(_) => Json(SuccessResponse::new(format!(
            "凭证 #{} 已重置并启用",
            id
        )))
        .into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// GET /api/admin/credentials/:id/balance
/// 获取指定凭证的余额
pub async fn get_credential_balance(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.get_balance(id).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials
/// 添加新凭证
pub async fn add_credential(
    State(state): State<AdminState>,
    Json(payload): Json<AddCredentialRequest>,
) -> impl IntoResponse {
    match state.service.add_credential(payload).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// DELETE /api/admin/credentials/:id
/// 删除凭证
pub async fn delete_credential(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.delete_credential(id) {
        Ok(_) => Json(SuccessResponse::new(format!("凭证 #{} 已删除", id))).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/import
/// 批量导入凭证
pub async fn import_credentials(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::ImportCredentialsRequest>,
) -> impl IntoResponse {
    match state.service.import_credentials(payload.credentials).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// GET /api/admin/logs
/// 获取运行日志
pub async fn get_logs() -> impl IntoResponse {
    use crate::logs::LOG_COLLECTOR;
    let logs = LOG_COLLECTOR.get_logs();
    Json(serde_json::json!({
        "logs": logs,
        "total": logs.len()
    }))
}

/// POST /api/admin/logs/clear
/// 清空日志
pub async fn clear_logs() -> impl IntoResponse {
    use crate::logs::LOG_COLLECTOR;
    LOG_COLLECTOR.clear();
    Json(super::types::SuccessResponse::new("日志已清空"))
}

/// GET /api/admin/config
/// 获取当前配置
pub async fn get_config() -> impl IntoResponse {
    use crate::model::config::Config;
    use super::types::GetConfigResponse;
    
    // 获取配置文件路径
    let config_path = get_config_path();
    
    match Config::load(&config_path) {
        Ok(config) => {
            let response = GetConfigResponse {
                host: config.host,
                port: config.port,
                api_key: config.api_key,
                region: config.region,
            };
            Json(serde_json::json!(response)).into_response()
        }
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// POST /api/admin/config
/// 更新配置
pub async fn update_config(
    Json(payload): Json<super::types::UpdateConfigRequest>,
) -> impl IntoResponse {
    use crate::model::config::Config;
    use super::types::SuccessResponse;
    
    let config_path = get_config_path();
    
    // 先读取现有配置
    let mut config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    };
    
    // 更新字段
    if let Some(host) = payload.host {
        config.host = host;
    }
    if let Some(port) = payload.port {
        config.port = port;
    }
    if let Some(api_key) = payload.api_key {
        config.api_key = Some(api_key);
    }
    if let Some(region) = payload.region {
        config.region = region;
    }
    
    // 保存配置
    match config.save(&config_path) {
        Ok(_) => {
            tracing::info!("配置已更新并保存到: {:?}", config_path);
            Json(SuccessResponse::new("配置已保存（需要重启服务生效）")).into_response()
        }
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// 获取配置文件路径
fn get_config_path() -> std::path::PathBuf {
    if let Some(home_dir) = dirs::home_dir() {
        home_dir.join(".kiro-gateway").join("config.json")
    } else if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join("config.json");
        }
        std::path::PathBuf::from("config.json")
    } else {
        std::path::PathBuf::from("config.json")
    }
}
