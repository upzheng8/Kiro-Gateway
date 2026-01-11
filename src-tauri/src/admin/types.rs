//! Admin API 类型定义

use serde::{Deserialize, Serialize};

// ============ 凭证状态 ============

/// 所有凭证状态响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatusResponse {
    /// 凭证总数
    pub total: usize,
    /// 可用凭证数量（未禁用）
    pub available: usize,
    /// 当前活跃凭证 ID
    pub current_id: u64,
    /// 各凭证状态列表
    pub credentials: Vec<CredentialStatusItem>,
}

/// 单个凭证的状态信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatusItem {
    /// 凭证唯一 ID
    pub id: u64,
    /// 优先级（数字越小优先级越高）
    pub priority: u32,
    /// 是否被禁用
    pub disabled: bool,
    /// 连续失败次数
    pub failure_count: u32,
    /// 是否为当前活跃凭证
    pub is_current: bool,
    /// Token 过期时间（RFC3339 格式）
    pub expires_at: Option<String>,
    /// 认证方式
    pub auth_method: Option<String>,
    /// 是否有 Profile ARN
    pub has_profile_arn: bool,
    /// 用户邮箱
    pub email: Option<String>,
    /// 订阅类型
    pub subscription_title: Option<String>,
    /// 当前使用量
    pub current_usage: Option<f64>,
    /// 使用限额
    pub usage_limit: Option<f64>,
    /// 剩余额度
    pub remaining: Option<f64>,
    /// 下次重置时间
    pub next_reset_at: Option<f64>,
    /// Refresh Token
    pub refresh_token: Option<String>,
    /// Access Token
    pub access_token: Option<String>,
    /// Profile ARN
    pub profile_arn: Option<String>,
    /// 凭证状态：normal(正常), invalid(无效/封禁), expired(过期)
    pub status: String,
    /// 分组 ID
    pub group_id: String,
}

// ============ 刷新凭证响应 ============

/// 刷新单个凭证响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshCredentialResponse {
    pub id: u64,
    pub success: bool,
    pub email: Option<String>,
    pub subscription_title: Option<String>,
    pub remaining: f64,
    pub message: String,
}

/// 批量刷新结果项
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResultItem {
    pub id: u64,
    pub success: bool,
    pub email: Option<String>,
    pub remaining: Option<f64>,
    pub error: Option<String>,
}

/// 批量刷新凭证响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshAllResponse {
    pub success_count: u32,
    pub fail_count: u32,
    pub total: u32,
    pub results: Vec<RefreshResultItem>,
}

/// 批量刷新凭证请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshBatchRequest {
    /// 要刷新的凭证 ID 列表，为空则刷新所有活跃凭证
    pub ids: Option<Vec<u64>>,
}

// ============ 操作请求 ============

/// 启用/禁用凭证请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDisabledRequest {
    /// 是否禁用
    pub disabled: bool,
}

/// 修改优先级请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPriorityRequest {
    /// 新优先级值
    pub priority: u32,
}

/// 添加凭证请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCredentialRequest {
    /// 刷新令牌（必填）
    pub refresh_token: String,

    /// 认证方式（可选，默认 social）
    #[serde(default = "default_auth_method")]
    pub auth_method: String,

    /// OIDC Client ID（IdC 认证需要）
    pub client_id: Option<String>,

    /// OIDC Client Secret（IdC 认证需要）
    pub client_secret: Option<String>,

    /// 优先级（可选，默认 0）
    #[serde(default)]
    pub priority: u32,
}

fn default_auth_method() -> String {
    "social".to_string()
}

/// 添加凭证成功响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCredentialResponse {
    pub success: bool,
    pub message: String,
    /// 新添加的凭证 ID
    pub credential_id: u64,
}

/// 批量导入凭证请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCredentialsRequest {
    /// 要导入的凭证列表
    pub credentials: Vec<ImportCredentialItem>,
}

/// 单个导入凭证项
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCredentialItem {
    /// 刷新令牌（必填）
    pub refresh_token: String,
    /// 认证方式（可选，默认 social）
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    /// OIDC Client ID（IdC 认证需要）
    pub client_id: Option<String>,
    /// OIDC Client Secret（IdC 认证需要）
    pub client_secret: Option<String>,
    /// 优先级（可选，默认 0）
    #[serde(default)]
    pub priority: u32,
    /// 分组 ID（可选，默认 "default"）
    #[serde(default = "default_group_id")]
    pub group_id: String,
}

fn default_group_id() -> String {
    "default".to_string()
}

/// 批量导入凭证响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCredentialsResponse {
    pub success: bool,
    pub message: String,
    /// 成功导入的数量
    pub imported_count: usize,
    /// 跳过的数量（无效凭证）
    pub skipped_count: usize,
    /// 新添加的凭证 ID 列表
    pub credential_ids: Vec<u64>,
}

// ============ 余额查询 ============

/// 余额查询响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    /// 凭证 ID
    pub id: u64,
    /// 用户邮箱
    pub email: Option<String>,
    /// 订阅类型
    pub subscription_title: Option<String>,
    /// 当前使用量
    pub current_usage: f64,
    /// 使用限额
    pub usage_limit: f64,
    /// 剩余额度
    pub remaining: f64,
    /// 使用百分比
    pub usage_percentage: f64,
    /// 下次重置时间（Unix 时间戳）
    pub next_reset_at: Option<f64>,
    
    // === 凭证详情 ===
    /// 认证方式
    pub auth_method: Option<String>,
    /// Access Token
    pub access_token: Option<String>,
    /// Refresh Token
    pub refresh_token: Option<String>,
    /// Profile ARN
    pub profile_arn: Option<String>,
    /// Token 过期时间
    pub expires_at: Option<String>,
}

// ============ 通用响应 ============

/// 操作成功响应
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
}

/// 错误响应
#[derive(Debug, Serialize)]
pub struct AdminErrorResponse {
    pub error: AdminError,
}

#[derive(Debug, Serialize)]
pub struct AdminError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

impl AdminErrorResponse {
    pub fn new(error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: AdminError {
                error_type: error_type.into(),
                message: message.into(),
            },
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new("invalid_request", message)
    }

    pub fn authentication_error() -> Self {
        Self::new("authentication_error", "Invalid or missing admin API key")
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub fn api_error(message: impl Into<String>) -> Self {
        Self::new("api_error", message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("internal_error", message)
    }
}

// ============ 配置 API ============

/// 获取配置响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConfigResponse {
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// API 密钥
    pub api_key: Option<String>,
    /// AWS 区域
    pub region: String,
}

/// 更新配置请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfigRequest {
    /// 监听地址（可选）
    pub host: Option<String>,
    /// 监听端口（可选）
    pub port: Option<u16>,
    /// API 密钥（可选）
    pub api_key: Option<String>,
    /// AWS 区域（可选）
    pub region: Option<String>,
}

// ============ 批量操作 ============

/// 批量删除请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteRequest {
    /// 要删除的凭证 ID 列表
    pub ids: Vec<u64>,
}

/// 导出凭证请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCredentialsRequest {
    /// 要导出的凭证 ID 列表（空则导出全部）
    #[serde(default)]
    pub ids: Vec<u64>,
    /// 导出类型：full（完整数据）或 tokens_only（仅 token）
    pub export_type: Option<String>,
}

// ============ 模型锁定 ============

/// 设置锁定模型请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLockedModelRequest {
    /// 要锁定的模型名称（null 或空表示取消锁定）
    pub model: Option<String>,
}

// ============ 分组管理 ============

/// 分组信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    /// 该分组下的凭证数量
    pub credential_count: u32,
}

/// 分组列表响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupsResponse {
    pub groups: Vec<GroupInfo>,
    /// 当前反代使用的分组 ID（null 表示使用所有分组）
    pub active_group_id: Option<String>,
}

/// 添加分组请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddGroupRequest {
    pub name: String,
}

/// 删除分组请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGroupRequest {
    pub id: String,
}

/// 设置活跃分组请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetActiveGroupRequest {
    /// 要设置为活跃的分组 ID（null 表示使用所有分组）
    pub group_id: Option<String>,
}

/// 修改凭证分组请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCredentialGroupRequest {
    pub group_id: String,
}

/// 重命名分组请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameGroupRequest {
    pub name: String,
}

/// 代理服务状态响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatusResponse {
    /// 是否正在运行
    pub running: bool,
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// 使用的分组 ID（null 表示全部）
    pub active_group_id: Option<String>,
}

/// 启动/停止代理请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProxyEnabledRequest {
    pub enabled: bool,
}
