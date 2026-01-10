//! Admin API 业务逻辑服务

use std::sync::Arc;

use crate::kiro::model::credentials::KiroCredentials;
use crate::kiro::token_manager::MultiTokenManager;

use super::error::AdminServiceError;
use super::types::{
    AddCredentialRequest, AddCredentialResponse, BalanceResponse, CredentialStatusItem,
    CredentialsStatusResponse,
};

/// Admin 服务
///
/// 封装所有 Admin API 的业务逻辑
pub struct AdminService {
    token_manager: Arc<MultiTokenManager>,
}

impl AdminService {
    pub fn new(token_manager: Arc<MultiTokenManager>) -> Self {
        Self { token_manager }
    }

    /// 获取所有凭证状态
    pub fn get_all_credentials(&self) -> CredentialsStatusResponse {
        let snapshot = self.token_manager.snapshot();

        let mut credentials: Vec<CredentialStatusItem> = snapshot
            .entries
            .into_iter()
            .map(|entry| CredentialStatusItem {
                id: entry.id,
                priority: entry.priority,
                disabled: entry.disabled,
                failure_count: entry.failure_count,
                is_current: entry.id == snapshot.current_id,
                expires_at: entry.expires_at,
                auth_method: entry.auth_method,
                has_profile_arn: entry.has_profile_arn,
            })
            .collect();

        // 按优先级排序（数字越小优先级越高）
        credentials.sort_by_key(|c| c.priority);

        CredentialsStatusResponse {
            total: snapshot.total,
            available: snapshot.available,
            current_id: snapshot.current_id,
            credentials,
        }
    }

    /// 设置凭证禁用状态
    pub fn set_disabled(&self, id: u64, disabled: bool) -> Result<(), AdminServiceError> {
        // 先获取当前凭证 ID，用于判断是否需要切换
        let snapshot = self.token_manager.snapshot();
        let current_id = snapshot.current_id;

        self.token_manager
            .set_disabled(id, disabled)
            .map_err(|e| self.classify_error(e, id))?;

        // 只有禁用的是当前凭证时才尝试切换到下一个
        if disabled && id == current_id {
            let _ = self.token_manager.switch_to_next();
        }
        Ok(())
    }

    /// 设置凭证优先级
    pub fn set_priority(&self, id: u64, priority: u32) -> Result<(), AdminServiceError> {
        self.token_manager
            .set_priority(id, priority)
            .map_err(|e| self.classify_error(e, id))
    }

    /// 重置失败计数并重新启用
    pub fn reset_and_enable(&self, id: u64) -> Result<(), AdminServiceError> {
        self.token_manager
            .reset_and_enable(id)
            .map_err(|e| self.classify_error(e, id))
    }

    /// 获取凭证余额
    pub async fn get_balance(&self, id: u64) -> Result<BalanceResponse, AdminServiceError> {
        let usage = self
            .token_manager
            .get_usage_limits_for(id)
            .await
            .map_err(|e| self.classify_balance_error(e, id))?;

        let current_usage = usage.current_usage();
        let usage_limit = usage.usage_limit();
        let remaining = (usage_limit - current_usage).max(0.0);
        let usage_percentage = if usage_limit > 0.0 {
            (current_usage / usage_limit * 100.0).min(100.0)
        } else {
            0.0
        };

        Ok(BalanceResponse {
            id,
            subscription_title: usage.subscription_title().map(|s| s.to_string()),
            current_usage,
            usage_limit,
            remaining,
            usage_percentage,
            next_reset_at: usage.next_date_reset,
        })
    }

    /// 添加新凭证
    ///
    /// 如果未指定优先级（默认为 0），则自动分配下一个可用优先级
    pub async fn add_credential(
        &self,
        req: AddCredentialRequest,
    ) -> Result<AddCredentialResponse, AdminServiceError> {
        // 如果优先级为 0，自动分配下一个优先级
        let priority = if req.priority == 0 {
            let snapshot = self.token_manager.snapshot();
            if snapshot.entries.is_empty() {
                // 没有现有凭证时，从 0 开始
                0
            } else {
                // 有现有凭证时，使用 max+1
                snapshot
                    .entries
                    .iter()
                    .map(|e| e.priority)
                    .max()
                    .unwrap_or(0)
                    + 1
            }
        } else {
            req.priority
        };

        // 构建凭证对象
        let new_cred = KiroCredentials {
            id: None,
            access_token: None,
            refresh_token: Some(req.refresh_token),
            profile_arn: None,
            expires_at: None,
            auth_method: Some(req.auth_method),
            client_id: req.client_id,
            client_secret: req.client_secret,
            priority,
        };

        // 调用 token_manager 添加凭证
        let credential_id = self
            .token_manager
            .add_credential(new_cred)
            .await
            .map_err(|e| self.classify_add_error(e))?;

        Ok(AddCredentialResponse {
            success: true,
            message: format!("凭证添加成功，ID: {}", credential_id),
            credential_id,
        })
    }

    /// 批量导入凭证
    ///
    /// 如果凭证未指定优先级（默认为 0），则自动按顺序分配优先级
    pub async fn import_credentials(
        &self,
        items: Vec<super::types::ImportCredentialItem>,
    ) -> Result<super::types::ImportCredentialsResponse, AdminServiceError> {
        let mut imported_ids = Vec::new();
        let mut skipped = 0;

        // 获取当前最大优先级，用于分配递增优先级
        let snapshot = self.token_manager.snapshot();
        let mut next_priority = if snapshot.entries.is_empty() {
            // 没有现有凭证时，从 0 开始
            0
        } else {
            // 有现有凭证时，从 max+1 开始
            snapshot
                .entries
                .iter()
                .map(|e| e.priority)
                .max()
                .unwrap_or(0)
                + 1
        };

        for item in items {
            // 如果优先级为 0（默认值），则自动分配递增优先级
            let priority = if item.priority == 0 {
                let assigned = next_priority;
                next_priority += 1;
                assigned
            } else {
                item.priority
            };

            // 构建凭证对象
            let new_cred = KiroCredentials {
                id: None,
                access_token: None,
                refresh_token: Some(item.refresh_token),
                profile_arn: None,
                expires_at: None,
                auth_method: Some(item.auth_method),
                client_id: item.client_id,
                client_secret: item.client_secret,
                priority: priority,
            };

            // 尝试添加凭证
            match self.token_manager.add_credential(new_cred).await {
                Ok(id) => {
                    imported_ids.push(id);
                }
                Err(e) => {
                    tracing::warn!("导入凭证失败，已跳过: {}", e);
                    skipped += 1;
                }
            }
        }

        let imported_count = imported_ids.len();
        let message = if skipped > 0 {
            format!("成功导入 {} 个凭证，跳过 {} 个无效凭证", imported_count, skipped)
        } else {
            format!("成功导入 {} 个凭证", imported_count)
        };

        Ok(super::types::ImportCredentialsResponse {
            success: true,
            message,
            imported_count,
            skipped_count: skipped,
            credential_ids: imported_ids,
        })
    }

    /// 删除凭证
    pub fn delete_credential(&self, id: u64) -> Result<(), AdminServiceError> {
        self.token_manager
            .delete_credential(id)
            .map_err(|e| self.classify_delete_error(e, id))
    }

    /// 分类简单操作错误（set_disabled, set_priority, reset_and_enable）
    fn classify_error(&self, e: anyhow::Error, id: u64) -> AdminServiceError {
        let msg = e.to_string();
        if msg.contains("不存在") {
            AdminServiceError::NotFound { id }
        } else {
            AdminServiceError::InternalError(msg)
        }
    }

    /// 分类余额查询错误（可能涉及上游 API 调用）
    fn classify_balance_error(&self, e: anyhow::Error, id: u64) -> AdminServiceError {
        let msg = e.to_string();

        // 1. 凭证不存在
        if msg.contains("不存在") {
            return AdminServiceError::NotFound { id };
        }

        // 2. 上游服务错误特征：HTTP 响应错误或网络错误
        let is_upstream_error =
            // HTTP 响应错误（来自 refresh_*_token 的错误消息）
            msg.contains("凭证已过期或无效") ||
            msg.contains("权限不足") ||
            msg.contains("已被限流") ||
            msg.contains("服务器错误") ||
            msg.contains("Token 刷新失败") ||
            msg.contains("暂时不可用") ||
            // 网络错误（reqwest 错误）
            msg.contains("error trying to connect") ||
            msg.contains("connection") ||
            msg.contains("timeout") ||
            msg.contains("timed out");

        if is_upstream_error {
            AdminServiceError::UpstreamError(msg)
        } else {
            // 3. 默认归类为内部错误（本地验证失败、配置错误等）
            // 包括：缺少 refreshToken、refreshToken 已被截断、无法生成 machineId 等
            AdminServiceError::InternalError(msg)
        }
    }

    /// 分类添加凭证错误
    fn classify_add_error(&self, e: anyhow::Error) -> AdminServiceError {
        let msg = e.to_string();

        // 凭证验证失败（refreshToken 无效、格式错误等）
        let is_invalid_credential = msg.contains("缺少 refreshToken")
            || msg.contains("refreshToken 为空")
            || msg.contains("refreshToken 已被截断")
            || msg.contains("凭证已过期或无效")
            || msg.contains("权限不足")
            || msg.contains("已被限流");

        if is_invalid_credential {
            AdminServiceError::InvalidCredential(msg)
        } else if msg.contains("error trying to connect")
            || msg.contains("connection")
            || msg.contains("timeout")
        {
            AdminServiceError::UpstreamError(msg)
        } else {
            AdminServiceError::InternalError(msg)
        }
    }

    /// 分类删除凭证错误
    fn classify_delete_error(&self, e: anyhow::Error, id: u64) -> AdminServiceError {
        let msg = e.to_string();
        if msg.contains("不存在") {
            AdminServiceError::NotFound { id }
        } else if msg.contains("只能删除已禁用的凭证") {
            AdminServiceError::InvalidCredential(msg)
        } else {
            AdminServiceError::InternalError(msg)
        }
    }
}
