//! Admin API 路由配置

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use super::{
    handlers::{
        add_credential, delete_credential, get_all_credentials, get_credential_balance,
        reset_failure_count, set_credential_disabled, set_credential_priority, import_credentials,
        get_logs, clear_logs, get_config, update_config,
        // 新增 handlers
        get_machine_id, backup_machine_id, restore_machine_id, reset_machine_id,
        batch_delete_credentials, export_credentials,
        get_locked_model, set_locked_model,
        // 本地账号
        get_local_credential, import_local_credential, switch_to_credential,
        // 刷新凭证
        refresh_credential, refresh_all_credentials,
        // 分组管理
        get_groups, add_group, delete_group, rename_group, set_active_group, set_credential_group,
        // 代理服务控制
        get_proxy_status, set_proxy_enabled,
    },
    middleware::AdminState,
};

/// 创建 Admin API 路由
///
/// # 端点
/// - `GET /credentials` - 获取所有凭证状态
/// - `POST /credentials` - 添加新凭证
/// - `POST /credentials/import` - 批量导入凭证
/// - `GET /credentials/local` - 获取本地凭证信息
/// - `POST /credentials/import-local` - 导入本地凭证
/// - `DELETE /credentials/:id` - 删除凭证
/// - `DELETE /credentials/batch` - 批量删除凭证
/// - `POST /credentials/export` - 导出凭证
/// - `POST /credentials/:id/disabled` - 设置凭证禁用状态
/// - `POST /credentials/:id/priority` - 设置凭证优先级
/// - `POST /credentials/:id/reset` - 重置失败计数
/// - `POST /credentials/:id/switch` - 切换到该账号
/// - `GET /credentials/:id/balance` - 获取凭证余额
/// - `GET /logs` - 获取运行日志
/// - `POST /logs/clear` - 清空日志
/// - `GET /config` - 获取配置
/// - `POST /config` - 更新配置
/// - `GET /config/model` - 获取锁定模型
/// - `POST /config/model` - 设置锁定模型
/// - `GET /machine-id` - 获取机器码
/// - `POST /machine-id/backup` - 备份机器码
/// - `POST /machine-id/restore` - 恢复机器码
/// - `POST /machine-id/reset` - 重置机器码
///
/// # 认证
/// 需要 Admin API Key 认证，支持：
/// - `x-api-key` header
/// - `Authorization: Bearer <token>` header
pub fn create_admin_router(state: AdminState) -> Router {
    Router::new()
        .route(
            "/credentials",
            get(get_all_credentials).post(add_credential),
        )
        .route("/credentials/import", post(import_credentials))
        .route("/credentials/refresh-all", post(refresh_all_credentials))
        .route("/credentials/local", get(get_local_credential))
        .route("/credentials/import-local", post(import_local_credential))
        .route("/credentials/batch", delete(batch_delete_credentials))
        .route("/credentials/export", post(export_credentials))
        .route("/credentials/{id}", delete(delete_credential))
        .route("/credentials/{id}/disabled", post(set_credential_disabled))
        .route("/credentials/{id}/priority", post(set_credential_priority))
        .route("/credentials/{id}/reset", post(reset_failure_count))
        .route("/credentials/{id}/switch", post(switch_to_credential))
        .route("/credentials/{id}/balance", get(get_credential_balance))
        .route("/credentials/{id}/refresh", post(refresh_credential))
        .route("/logs", get(get_logs))
        .route("/logs/clear", post(clear_logs))
        .route("/config", get(get_config).post(update_config))
        .route("/config/model", get(get_locked_model).post(set_locked_model))
        .route("/machine-id", get(get_machine_id))
        .route("/machine-id/backup", post(backup_machine_id))
        .route("/machine-id/restore", post(restore_machine_id))
        .route("/machine-id/reset", post(reset_machine_id))
        // 分组管理
        .route("/groups", get(get_groups).post(add_group))
        .route("/groups/{id}", delete(delete_group).put(rename_group))
        .route("/groups/active", post(set_active_group))
        .route("/credentials/{id}/group", post(set_credential_group))
        // 代理服务控制
        .route("/proxy/status", get(get_proxy_status))
        .route("/proxy/enabled", post(set_proxy_enabled))
        // 移除 API Key 认证中间件
        .with_state(state)
}
