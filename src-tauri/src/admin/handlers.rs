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

/// POST /api/admin/credentials/:id/refresh
/// 刷新单个凭证（刷新 Token + 更新余额）
pub async fn refresh_credential(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.refresh_credential(id).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/refresh-all
/// 批量刷新凭证（支持指定 ID 列表）
pub async fn refresh_all_credentials(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::RefreshBatchRequest>,
) -> impl IntoResponse {
    match state.service.refresh_credentials(payload.ids.unwrap_or_default()).await {
        Ok(response) => Json(response).into_response(),
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

// ============ 机器码管理 API ============

/// GET /api/admin/machine-id
/// 获取当前机器码信息
pub async fn get_machine_id() -> impl IntoResponse {
    use crate::model::config::Config;
    
    let config_path = get_config_path();
    match Config::load(&config_path) {
        Ok(config) => Json(serde_json::json!({
            "machineId": config.machine_id,
            "machineIdBackup": config.machine_id_backup
        })).into_response(),
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// POST /api/admin/machine-id/backup
/// 备份当前机器码
pub async fn backup_machine_id() -> impl IntoResponse {
    use crate::model::config::Config;
    
    let config_path = get_config_path();
    let mut config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    };
    
    if let Some(machine_id) = &config.machine_id {
        config.machine_id_backup = Some(machine_id.clone());
        if let Err(e) = config.save(&config_path) {
            let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
        Json(SuccessResponse::new("机器码已备份")).into_response()
    } else {
        let error = super::types::AdminErrorResponse::invalid_request("当前没有设置机器码");
        (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response()
    }
}

/// POST /api/admin/machine-id/restore
/// 从备份恢复机器码
pub async fn restore_machine_id() -> impl IntoResponse {
    use crate::model::config::Config;
    
    let config_path = get_config_path();
    let mut config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    };
    
    if let Some(backup) = &config.machine_id_backup {
        config.machine_id = Some(backup.clone());
        if let Err(e) = config.save(&config_path) {
            let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
        Json(SuccessResponse::new("机器码已从备份恢复")).into_response()
    } else {
        let error = super::types::AdminErrorResponse::invalid_request("没有可用的机器码备份");
        (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response()
    }
}

/// POST /api/admin/machine-id/reset
/// 重置机器码（清空，下次请求时自动生成新的）
pub async fn reset_machine_id() -> impl IntoResponse {
    use crate::model::config::Config;
    
    let config_path = get_config_path();
    let mut config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    };
    
    config.machine_id = None;
    if let Err(e) = config.save(&config_path) {
        let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
    }
    Json(SuccessResponse::new("机器码已重置")).into_response()
}

// ============ 批量操作 API ============

/// DELETE /api/admin/credentials/batch
/// 批量删除凭证
pub async fn batch_delete_credentials(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::BatchDeleteRequest>,
) -> impl IntoResponse {
    let mut deleted = 0;
    let mut failed = 0;
    
    for id in payload.ids {
        match state.service.delete_credential(id) {
            Ok(_) => deleted += 1,
            Err(_) => failed += 1,
        }
    }
    
    Json(serde_json::json!({
        "success": true,
        "deleted": deleted,
        "failed": failed,
        "message": format!("成功删除 {} 个凭证，{} 个失败", deleted, failed)
    }))
}

/// POST /api/admin/credentials/export
/// 导出凭证（支持完整数据或仅 token）
pub async fn export_credentials(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::ExportCredentialsRequest>,
) -> impl IntoResponse {
    // 获取要导出的凭证 ID
    let ids: Vec<u64> = payload.ids;
    
    // 获取完整凭证数据
    let credentials = state.service.get_credentials_for_export(&ids);
    
    // 根据导出类型返回不同格式
    match payload.export_type.as_deref() {
        Some("tokens_only") => {
            // 仅导出 refreshToken
            let tokens: Vec<serde_json::Value> = credentials
                .iter()
                .filter_map(|c| {
                    c.refresh_token.as_ref().map(|token| {
                        serde_json::json!({
                            "refreshToken": token
                        })
                    })
                })
                .collect();
            
            Json(serde_json::json!({
                "success": true,
                "type": "tokens_only",
                "count": tokens.len(),
                "credentials": tokens
            })).into_response()
        }
        _ => {
            // 导出完整数据（格式与 z-kiro 一致）
            let export_data: Vec<serde_json::Value> = credentials
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "accessToken": c.access_token,
                        "refreshToken": c.refresh_token,
                        "profileArn": c.profile_arn,
                        "expiresAt": c.expires_at,
                        "authMethod": c.auth_method.as_deref().unwrap_or("social")
                    })
                })
                .collect();
            
            Json(serde_json::json!({
                "success": true,
                "type": "full",
                "count": export_data.len(),
                "credentials": export_data
            })).into_response()
        }
    }
}

// ============ 模型锁定 API ============

/// GET /api/admin/config/model
/// 获取当前锁定的模型
pub async fn get_locked_model() -> impl IntoResponse {
    use crate::model::config::Config;
    
    let config_path = get_config_path();
    match Config::load(&config_path) {
        Ok(config) => Json(serde_json::json!({
            "lockedModel": config.locked_model
        })).into_response(),
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// POST /api/admin/config/model
/// 设置或取消锁定模型
pub async fn set_locked_model(
    Json(payload): Json<super::types::SetLockedModelRequest>,
) -> impl IntoResponse {
    use crate::model::config::Config;
    
    let config_path = get_config_path();
    let mut config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("读取配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    };
    
    config.locked_model = payload.model;
    
    if let Err(e) = config.save(&config_path) {
        let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
    }
    
    let msg = if config.locked_model.is_some() {
        format!("模型已锁定为: {}", config.locked_model.as_ref().unwrap())
    } else {
        "模型锁定已取消".to_string()
    };
    
    Json(SuccessResponse::new(msg)).into_response()
}

// ============ 本地账号 API ============

/// GET /api/admin/credentials/local
/// 获取本地 Kiro 客户端凭证信息
pub async fn get_local_credential() -> impl IntoResponse {
    use super::local_account;
    
    match local_account::read_local_credential() {
        Ok(cred) => Json(serde_json::json!({
            "success": true,
            "hasCredential": true,
            "refreshToken": cred.refresh_token,
            "authMethod": cred.auth_method,
            "expiresAt": cred.expires_at
        })).into_response(),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "hasCredential": false,
            "error": e.to_string()
        })).into_response()
    }
}

/// POST /api/admin/credentials/import-local
/// 导入本地 Kiro 客户端凭证
pub async fn import_local_credential(
    State(state): State<AdminState>,
) -> impl IntoResponse {
    use super::local_account;
    use super::types::AddCredentialRequest;
    
    // 读取本地凭证
    let local_cred = match local_account::read_local_credential() {
        Ok(c) => c,
        Err(e) => {
            let error = super::types::AdminErrorResponse::invalid_request(format!("读取本地凭证失败: {}", e));
            return (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response();
        }
    };
    
    let refresh_token = match local_cred.refresh_token {
        Some(t) => t,
        None => {
            let error = super::types::AdminErrorResponse::invalid_request("本地凭证没有 refreshToken");
            return (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response();
        }
    };
    
    // 构建添加请求
    let req = AddCredentialRequest {
        refresh_token,
        auth_method: local_cred.auth_method.unwrap_or_else(|| "social".to_string()),
        client_id: None,
        client_secret: None,
        priority: 0,
    };
    
    // 调用现有的添加逻辑
    match state.service.add_credential(req).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/switch
/// 切换到指定账号（写入本地凭证文件）
pub async fn switch_to_credential(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    use super::local_account::{self, LocalKiroCredential};
    
    // 获取凭证的完整信息
    let snapshot = state.service.get_all_credentials();
    let cred = snapshot.credentials.iter().find(|c| c.id == id);
    
    if cred.is_none() {
        let error = super::types::AdminErrorResponse::not_found(format!("凭证 #{} 不存在", id));
        return (axum::http::StatusCode::NOT_FOUND, Json(error)).into_response();
    }
    
    let cred = cred.unwrap();
    
    // 构建本地凭证
    let local_cred = LocalKiroCredential {
        access_token: cred.access_token.clone(),
        refresh_token: cred.refresh_token.clone(),
        profile_arn: cred.profile_arn.clone(),
        expires_at: cred.expires_at.clone(),
        auth_method: cred.auth_method.clone(),
        provider: Some("Google".to_string()),
    };
    
    match local_account::write_local_credential(&local_cred) {
        Ok(_) => Json(SuccessResponse::new(format!("已切换到凭证 #{}", id))).into_response(),
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(format!("写入本地凭证失败: {}", e));
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

// ============ 分组管理 ============

/// GET /api/admin/groups
/// 获取所有分组
pub async fn get_groups(State(state): State<AdminState>) -> impl IntoResponse {
    use super::types::{GroupInfo, GroupsResponse};
    
    let config = state.config.lock();
    let credentials = state.service.get_all_credentials();
    
    // 统计每个分组的凭证数量
    let groups: Vec<GroupInfo> = config.groups.iter().map(|g| {
        let count = credentials.credentials.iter()
            .filter(|c| c.group_id == g.id)
            .count() as u32;
        GroupInfo {
            id: g.id.clone(),
            name: g.name.clone(),
            credential_count: count,
        }
    }).collect();
    
    Json(GroupsResponse {
        groups,
        active_group_id: config.active_group_id.clone(),
    })
}

/// POST /api/admin/groups
/// 添加分组
pub async fn add_group(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::AddGroupRequest>,
) -> impl IntoResponse {
    use crate::model::config::GroupConfig;
    
    // 生成唯一 ID
    let group_id = format!("group_{}", chrono::Utc::now().timestamp_millis());
    
    {
        let mut config = state.config.lock();
        config.groups.push(GroupConfig {
            id: group_id.clone(),
            name: payload.name.clone(),
        });
        
        // 保存配置
        if let Err(e) = config.save(get_config_path()) {
            let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    }
    
    Json(SuccessResponse::new(format!("分组 '{}' 创建成功", payload.name))).into_response()
}

/// DELETE /api/admin/groups/:id
/// 删除分组
pub async fn delete_group(
    State(state): State<AdminState>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 不能删除默认分组
    if group_id == "default" {
        let error = super::types::AdminErrorResponse::invalid_request("不能删除默认分组".to_string());
        return (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response();
    }
    
    // 检查是否有凭证在该分组下
    let credentials = state.service.get_all_credentials();
    let has_credentials = credentials.credentials.iter().any(|c| c.group_id == group_id);
    if has_credentials {
        let error = super::types::AdminErrorResponse::invalid_request("该分组下还有凭证，无法删除".to_string());
        return (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response();
    }
    
    {
        let mut config = state.config.lock();
        
        // 找到并删除分组
        if let Some(pos) = config.groups.iter().position(|g| g.id == group_id) {
            config.groups.remove(pos);
            
            // 如果删除的是当前活跃分组，重置为空（使用所有）
            if config.active_group_id.as_ref() == Some(&group_id) {
                config.active_group_id = None;
            }
            
            // 保存配置
            if let Err(e) = config.save(get_config_path()) {
                let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
            }
        } else {
            let error = super::types::AdminErrorResponse::not_found(format!("分组 '{}' 不存在", group_id));
            return (axum::http::StatusCode::NOT_FOUND, Json(error)).into_response();
        }
    }
    
    Json(SuccessResponse::new("分组已删除".to_string())).into_response()
}

/// PUT /api/admin/groups/:id
/// 重命名分组
pub async fn rename_group(
    State(state): State<AdminState>,
    Path(group_id): Path<String>,
    Json(payload): Json<super::types::RenameGroupRequest>,
) -> impl IntoResponse {
    // 不能重命名默认分组
    if group_id == "default" {
        let error = super::types::AdminErrorResponse::invalid_request("不能重命名默认分组".to_string());
        return (axum::http::StatusCode::BAD_REQUEST, Json(error)).into_response();
    }
    
    {
        let mut config = state.config.lock();
        
        // 找到并重命名分组
        if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
            group.name = payload.name.clone();
            
            // 保存配置
            if let Err(e) = config.save(get_config_path()) {
                let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
            }
        } else {
            let error = super::types::AdminErrorResponse::not_found(format!("分组 '{}' 不存在", group_id));
            return (axum::http::StatusCode::NOT_FOUND, Json(error)).into_response();
        }
    }
    
    Json(SuccessResponse::new(format!("分组已重命名为 '{}'", payload.name))).into_response()
}

/// POST /api/admin/groups/active
/// 设置活跃分组（反代使用的分组）
pub async fn set_active_group(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::SetActiveGroupRequest>,
) -> impl IntoResponse {
    {
        let mut config = state.config.lock();
        
        // 如果指定了分组，验证分组是否存在
        if let Some(ref gid) = payload.group_id {
            if !config.groups.iter().any(|g| &g.id == gid) {
                let error = super::types::AdminErrorResponse::not_found(format!("分组 '{}' 不存在", gid));
                return (axum::http::StatusCode::NOT_FOUND, Json(error)).into_response();
            }
        }
        
        config.active_group_id = payload.group_id.clone();
        
        // 保存配置
        if let Err(e) = config.save(get_config_path()) {
            let error = super::types::AdminErrorResponse::internal_error(format!("保存配置失败: {}", e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response();
        }
    }
    
    let msg = match payload.group_id {
        Some(gid) => format!("已切换到分组 '{}'", gid),
        None => "已切换到所有分组".to_string(),
    };
    Json(SuccessResponse::new(msg)).into_response()
}

/// POST /api/admin/credentials/:id/group
/// 设置凭证分组
pub async fn set_credential_group(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<super::types::SetCredentialGroupRequest>,
) -> impl IntoResponse {
    // 验证分组是否存在
    {
        let config = state.config.lock();
        if !config.groups.iter().any(|g| g.id == payload.group_id) {
            let error = super::types::AdminErrorResponse::not_found(format!("分组 '{}' 不存在", payload.group_id));
            return (axum::http::StatusCode::NOT_FOUND, Json(error)).into_response();
        }
    }
    
    // 更新凭证分组
    match state.token_manager.set_group(id, &payload.group_id) {
        Ok(_) => Json(SuccessResponse::new(format!("凭证 #{} 已移动到分组 '{}'", id, payload.group_id))).into_response(),
        Err(e) => {
            let error = super::types::AdminErrorResponse::internal_error(e.to_string());
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

// ============ 代理服务控制 API ============

/// GET /api/admin/proxy/status
/// 获取代理服务状态
pub async fn get_proxy_status(
    State(state): State<AdminState>,
) -> impl IntoResponse {
    let config = state.config.lock();
    let response = super::types::ProxyStatusResponse {
        running: state.is_proxy_running(),
        host: config.host.clone(),
        port: config.port,
        active_group_id: config.active_group_id.clone(),
    };
    Json(response)
}

/// POST /api/admin/proxy/enabled
/// 设置代理服务启用状态（启动或停止代理服务）
pub async fn set_proxy_enabled(
    State(state): State<AdminState>,
    Json(payload): Json<super::types::SetProxyEnabledRequest>,
) -> impl IntoResponse {
    let was_enabled = state.is_proxy_enabled();
    
    // 设置代理启用状态
    state.set_proxy_enabled(payload.enabled);
    // 同步更新控制器运行状态
    state.proxy_controller.set_running(payload.enabled);
    
    let msg = if payload.enabled && !was_enabled {
        "代理服务已启用"
    } else if !payload.enabled && was_enabled {
        "代理服务已停止"
    } else if payload.enabled {
        "代理服务已在运行中"
    } else {
        "代理服务已停止"
    };
    
    Json(SuccessResponse::new(msg.to_string()))
}
