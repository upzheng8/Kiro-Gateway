//! Token 管理模块
//!
//! 负责 Token 过期检测和刷新，支持 Social 和 IdC 认证方式
//! 支持单凭证 (TokenManager) 和多凭证 (MultiTokenManager) 管理

use anyhow::bail;
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::Mutex as TokioMutex;

use std::path::PathBuf;

use crate::http_client::{ProxyConfig, build_client};
use crate::kiro::machine_id;
use crate::kiro::model::credentials::KiroCredentials;
use crate::kiro::model::token_refresh::{
    IdcRefreshRequest, IdcRefreshResponse, RefreshRequest, RefreshResponse,
};
use crate::kiro::model::usage_limits::UsageLimitsResponse;
use crate::model::config::Config;

/// Token 管理器
///
/// 负责管理凭证和 Token 的自动刷新
pub struct TokenManager {
    config: Config,
    credentials: KiroCredentials,
    proxy: Option<ProxyConfig>,
}

impl TokenManager {
    /// 创建新的 TokenManager 实例
    pub fn new(config: Config, credentials: KiroCredentials, proxy: Option<ProxyConfig>) -> Self {
        Self {
            config,
            credentials,
            proxy,
        }
    }

    /// 获取凭证的引用
    pub fn credentials(&self) -> &KiroCredentials {
        &self.credentials
    }

    /// 获取配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 确保获取有效的访问 Token
    ///
    /// 如果 Token 过期或即将过期，会自动刷新
    pub async fn ensure_valid_token(&mut self) -> anyhow::Result<String> {
        if is_token_expired(&self.credentials) || is_token_expiring_soon(&self.credentials) {
            self.credentials =
                refresh_token(&self.credentials, &self.config, self.proxy.as_ref()).await?;

            // 刷新后再次检查 token 时间有效性
            if is_token_expired(&self.credentials) {
                anyhow::bail!("刷新后的 Token 仍然无效或已过期");
            }
        }

        self.credentials
            .access_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("没有可用的 accessToken"))
    }

    /// 获取使用额度信息
    ///
    /// 调用 getUsageLimits API 查询当前账户的使用额度
    pub async fn get_usage_limits(&mut self) -> anyhow::Result<UsageLimitsResponse> {
        let token = self.ensure_valid_token().await?;
        get_usage_limits(&self.credentials, &self.config, &token, self.proxy.as_ref()).await
    }
}

/// 检查 Token 是否在指定时间内过期
pub(crate) fn is_token_expiring_within(
    credentials: &KiroCredentials,
    minutes: i64,
) -> Option<bool> {
    credentials
        .expires_at
        .as_ref()
        .and_then(|expires_at| DateTime::parse_from_rfc3339(expires_at).ok())
        .map(|expires| expires <= Utc::now() + Duration::minutes(minutes))
}

/// 检查 Token 是否已过期（提前 5 分钟判断）
pub(crate) fn is_token_expired(credentials: &KiroCredentials) -> bool {
    is_token_expiring_within(credentials, 5).unwrap_or(true)
}

/// 检查 Token 是否即将过期（10分钟内）
pub(crate) fn is_token_expiring_soon(credentials: &KiroCredentials) -> bool {
    is_token_expiring_within(credentials, 10).unwrap_or(false)
}

/// 验证 refreshToken 的基本有效性
pub(crate) fn validate_refresh_token(credentials: &KiroCredentials) -> anyhow::Result<()> {
    let refresh_token = credentials
        .refresh_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少 refreshToken"))?;

    if refresh_token.is_empty() {
        bail!("refreshToken 为空");
    }

    if refresh_token.len() < 100 || refresh_token.ends_with("...") || refresh_token.contains("...")
    {
        bail!(
            "refreshToken 已被截断（长度: {} 字符）。\n\
             这通常是 Kiro IDE 为了防止凭证被第三方工具使用而故意截断的。",
            refresh_token.len()
        );
    }

    Ok(())
}

/// 刷新 Token
pub(crate) async fn refresh_token(
    credentials: &KiroCredentials,
    config: &Config,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<KiroCredentials> {
    validate_refresh_token(credentials)?;

    // 根据 auth_method 选择刷新方式
    let auth_method = credentials.auth_method.as_deref().unwrap_or("social");

    match auth_method.to_lowercase().as_str() {
        "idc" | "builder-id" => refresh_idc_token(credentials, config, proxy).await,
        _ => refresh_social_token(credentials, config, proxy).await,
    }
}

/// 刷新 Social Token
async fn refresh_social_token(
    credentials: &KiroCredentials,
    config: &Config,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<KiroCredentials> {
    tracing::info!("正在刷新 Social Token...");

    let refresh_token = credentials.refresh_token.as_ref().unwrap();
    let region = &config.region;

    let refresh_url = format!("https://prod.{}.auth.desktop.kiro.dev/refreshToken", region);
    let refresh_domain = format!("prod.{}.auth.desktop.kiro.dev", region);
    let machine_id = machine_id::generate_from_credentials(credentials)
        .ok_or_else(|| anyhow::anyhow!("无法生成 machineId"))?;
    let kiro_version = &config.kiro_version;

    let client = build_client(proxy, 60)?;
    let body = RefreshRequest {
        refresh_token: refresh_token.to_string(),
    };

    let response = client
        .post(&refresh_url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            format!("KiroIDE-{}-{}", kiro_version, machine_id),
        )
        .header("Accept-Encoding", "gzip, compress, deflate, br")
        .header("host", &refresh_domain)
        .header("Connection", "close")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = match status.as_u16() {
            401 => "OAuth 凭证已过期或无效，需要重新认证",
            403 => "权限不足，无法刷新 Token",
            429 => "请求过于频繁，已被限流",
            500..=599 => "服务器错误，AWS OAuth 服务暂时不可用",
            _ => "Token 刷新失败",
        };
        bail!("{}: {} {}", error_msg, status, body_text);
    }

    let data: RefreshResponse = response.json().await?;

    let mut new_credentials = credentials.clone();
    new_credentials.access_token = Some(data.access_token);

    if let Some(new_refresh_token) = data.refresh_token {
        new_credentials.refresh_token = Some(new_refresh_token);
    }

    if let Some(profile_arn) = data.profile_arn {
        new_credentials.profile_arn = Some(profile_arn);
    }

    if let Some(expires_in) = data.expires_in {
        let expires_at = Utc::now() + Duration::seconds(expires_in);
        new_credentials.expires_at = Some(expires_at.to_rfc3339());
    }

    Ok(new_credentials)
}

/// IdC Token 刷新所需的 x-amz-user-agent header
const IDC_AMZ_USER_AGENT: &str = "aws-sdk-js/3.738.0 ua/2.1 os/other lang/js md/browser#unknown_unknown api/sso-oidc#3.738.0 m/E KiroIDE";

/// 刷新 IdC Token (AWS SSO OIDC)
async fn refresh_idc_token(
    credentials: &KiroCredentials,
    config: &Config,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<KiroCredentials> {
    tracing::info!("正在刷新 IdC Token...");

    let refresh_token = credentials.refresh_token.as_ref().unwrap();
    let client_id = credentials
        .client_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("IdC 刷新需要 clientId"))?;
    let client_secret = credentials
        .client_secret
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("IdC 刷新需要 clientSecret"))?;

    let region = &config.region;
    let refresh_url = format!("https://oidc.{}.amazonaws.com/token", region);

    let client = build_client(proxy, 60)?;
    let body = IdcRefreshRequest {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        refresh_token: refresh_token.to_string(),
        grant_type: "refresh_token".to_string(),
    };

    let response = client
        .post(&refresh_url)
        .header("Content-Type", "application/json")
        .header("Host", format!("oidc.{}.amazonaws.com", region))
        .header("Connection", "keep-alive")
        .header("x-amz-user-agent", IDC_AMZ_USER_AGENT)
        .header("Accept", "*/*")
        .header("Accept-Language", "*")
        .header("sec-fetch-mode", "cors")
        .header("User-Agent", "node")
        .header("Accept-Encoding", "br, gzip, deflate")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = match status.as_u16() {
            401 => "IdC 凭证已过期或无效，需要重新认证",
            403 => "权限不足，无法刷新 Token",
            429 => "请求过于频繁，已被限流",
            500..=599 => "服务器错误，AWS OIDC 服务暂时不可用",
            _ => "IdC Token 刷新失败",
        };
        bail!("{}: {} {}", error_msg, status, body_text);
    }

    let data: IdcRefreshResponse = response.json().await?;

    let mut new_credentials = credentials.clone();
    new_credentials.access_token = Some(data.access_token);

    if let Some(new_refresh_token) = data.refresh_token {
        new_credentials.refresh_token = Some(new_refresh_token);
    }

    if let Some(expires_in) = data.expires_in {
        let expires_at = Utc::now() + Duration::seconds(expires_in);
        new_credentials.expires_at = Some(expires_at.to_rfc3339());
    }

    Ok(new_credentials)
}

/// getUsageLimits API 所需的 x-amz-user-agent header 前缀
const USAGE_LIMITS_AMZ_USER_AGENT_PREFIX: &str = "aws-sdk-js/1.0.0";

/// 获取使用额度信息
pub(crate) async fn get_usage_limits(
    credentials: &KiroCredentials,
    config: &Config,
    token: &str,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<UsageLimitsResponse> {
    tracing::debug!("正在获取使用额度信息...");

    let region = &config.region;
    let host = format!("q.{}.amazonaws.com", region);
    let machine_id = machine_id::generate_from_credentials(credentials)
        .ok_or_else(|| anyhow::anyhow!("无法生成 machineId"))?;
    let kiro_version = &config.kiro_version;

    // 构建 URL
    let mut url = format!(
        "https://{}/getUsageLimits?isEmailRequired=true&origin=AI_EDITOR&resourceType=AGENTIC_REQUEST",
        host
    );

    // profileArn 是可选的
    if let Some(profile_arn) = &credentials.profile_arn {
        url.push_str(&format!("&profileArn={}", urlencoding::encode(profile_arn)));
    }

    // 构建 User-Agent headers
    let user_agent = format!(
        "aws-sdk-js/1.0.0 ua/2.1 os/darwin#24.6.0 lang/js md/nodejs#22.21.1 \
         api/codewhispererruntime#1.0.0 m/N,E KiroIDE-{}-{}",
        kiro_version, machine_id
    );
    let amz_user_agent = format!(
        "{} KiroIDE-{}-{}",
        USAGE_LIMITS_AMZ_USER_AGENT_PREFIX, kiro_version, machine_id
    );

    let client = build_client(proxy, 60)?;

    let response = client
        .get(&url)
        .header("x-amz-user-agent", &amz_user_agent)
        .header("User-Agent", &user_agent)
        .header("host", &host)
        .header("amz-sdk-invocation-id", uuid::Uuid::new_v4().to_string())
        .header("amz-sdk-request", "attempt=1; max=1")
        .header("Authorization", format!("Bearer {}", token))
        .header("Connection", "close")
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = match status.as_u16() {
            401 => "认证失败，Token 无效或已过期",
            403 => "权限不足，无法获取使用额度",
            429 => "请求过于频繁，已被限流",
            500..=599 => "服务器错误，AWS 服务暂时不可用",
            _ => "获取使用额度失败",
        };
        bail!("{}: {} {}", error_msg, status, body_text);
    }

    let data: UsageLimitsResponse = response.json().await?;
    Ok(data)
}

// ============================================================================
// 多凭证 Token 管理器
// ============================================================================

/// 单个凭证条目的状态
struct CredentialEntry {
    /// 凭证唯一 ID
    id: u64,
    /// 凭证信息
    credentials: KiroCredentials,
    /// API 调用连续失败次数
    failure_count: u32,
    /// 是否已禁用
    disabled: bool,
    /// 禁用原因（用于区分手动禁用 vs 自动禁用，便于自愈）
    disabled_reason: Option<DisabledReason>,
}

impl CredentialEntry {
    /// 检查凭证是否可用于反代
    /// 
    /// 同时检查以下条件：
    /// - disabled 为 false
    /// - status 不是 "invalid"
    fn is_available(&self) -> bool {
        !self.disabled && self.credentials.status != "invalid"
    }
}

/// 禁用原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisabledReason {
    /// Admin API 手动禁用
    Manual,
    /// 连续失败达到阈值后自动禁用
    TooManyFailures,
    /// 账户被暂停（TEMPORARILY_SUSPENDED 或类似 403/401 错误）
    Suspended,
}

/// 检查错误是否表示凭证被暂停/无效（需要禁用凭证）
/// 
/// 只有在确定凭证本身无效时才返回 true，临时性错误（如限流、服务器错误）不会触发禁用
fn is_credential_invalid_error(error_msg: &str) -> bool {
    // 1. 检测 AWS 账户暂停 (最明确的信号)
    if error_msg.contains("TEMPORARILY_SUSPENDED") ||
       error_msg.contains("temporarily is suspended") ||
       error_msg.contains("temporarily suspended") {
        return true;
    }
    
    // 2. 检测凭证过期/无效（刷新 Token 失败）
    if error_msg.contains("凭证已过期或无效") ||
       error_msg.contains("OAuth 凭证已过期或无效") ||
       error_msg.contains("IdC 凭证已过期或无效") {
        return true;
    }
    
    // 3. 检测 401 认证失败（Token 无效）
    // 注意：401 通常表示 Token 失效，需要重新认证
    if error_msg.contains("401") && 
       (error_msg.contains("Unauthorized") || error_msg.contains("认证失败")) {
        return true;
    }
    
    // 4. 检测 403 权限不足 + 特定错误消息
    // 注意：不是所有 403 都表示凭证无效，只有以下情况才禁用：
    //   - 含有 suspended 相关信息（上面已检测）
    //   - 含有 "无效" 或 "revoked" 等关键词
    if error_msg.contains("403") && error_msg.contains("Forbidden") {
        // 需要进一步检查是否真的是凭证问题
        if error_msg.contains("User ID") || 
           error_msg.contains("revoked") ||
           error_msg.contains("invalid") ||
           error_msg.contains("locked") {
            return true;
        }
    }
    
    false
}

// ============================================================================
// Admin API 公开结构
// ============================================================================

/// 凭证条目快照（用于 Admin API 读取）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialEntrySnapshot {
    /// 凭证唯一 ID
    pub id: u64,
    /// 是否被禁用
    pub disabled: bool,
    /// 连续失败次数
    pub failure_count: u32,
    /// 认证方式
    pub auth_method: Option<String>,
    /// 是否有 Profile ARN
    pub has_profile_arn: bool,
    /// Token 过期时间
    pub expires_at: Option<String>,
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

/// 凭证管理器状态快照
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerSnapshot {
    /// 凭证条目列表
    pub entries: Vec<CredentialEntrySnapshot>,
    /// 当前活跃凭证 ID
    pub current_id: u64,
    /// 总凭证数量
    pub total: usize,
    /// 可用凭证数量
    pub available: usize,
}

/// 多凭证 Token 管理器
///
/// 支持多个凭证的管理，实现固定优先级 + 故障转移策略
/// 故障统计基于 API 调用结果，而非 Token 刷新结果
pub struct MultiTokenManager {
    config: Config,
    proxy: Option<ProxyConfig>,
    /// 凭证条目列表
    entries: Mutex<Vec<CredentialEntry>>,
    /// 当前活动凭证 ID
    current_id: Mutex<u64>,
    /// Token 刷新锁，确保同一时间只有一个刷新操作
    refresh_lock: TokioMutex<()>,
    /// 凭证文件路径（用于回写）
    credentials_path: Option<PathBuf>,
    /// 是否为多凭证格式（数组格式才回写）
    is_multiple_format: bool,
    /// 活跃分组 ID（反代使用，None 表示使用所有分组）
    active_group_id: Mutex<Option<String>>,
}

/// 每个凭证最大 API 调用失败次数
const MAX_FAILURES_PER_CREDENTIAL: u32 = 3;

/// API 调用上下文
///
/// 绑定特定凭证的调用上下文，确保 token、credentials 和 id 的一致性
/// 用于解决并发调用时 current_id 竞态问题
#[derive(Clone)]
pub struct CallContext {
    /// 凭证 ID（用于 report_success/report_failure）
    pub id: u64,
    /// 凭证信息（用于构建请求头）
    pub credentials: KiroCredentials,
    /// 访问 Token
    pub token: String,
}

impl MultiTokenManager {
    /// 创建多凭证 Token 管理器
    ///
    /// # Arguments
    /// * `config` - 应用配置
    /// * `credentials` - 凭证列表
    /// * `proxy` - 可选的代理配置
    /// * `credentials_path` - 凭证文件路径（用于回写）
    /// * `is_multiple_format` - 是否为多凭证格式（数组格式才回写）
    pub fn new(
        config: Config,
        credentials: Vec<KiroCredentials>,
        proxy: Option<ProxyConfig>,
        credentials_path: Option<PathBuf>,
        is_multiple_format: bool,
    ) -> anyhow::Result<Self> {
        // 计算当前最大 ID，为没有 ID 的凭证分配新 ID
        let max_existing_id = credentials.iter().filter_map(|c| c.id).max().unwrap_or(0);
        let mut next_id = max_existing_id + 1;
        let mut has_new_ids = false;

        let entries: Vec<CredentialEntry> = credentials
            .into_iter()
            .map(|mut cred| {
                let id = cred.id.unwrap_or_else(|| {
                    let id = next_id;
                    next_id += 1;
                    cred.id = Some(id);
                    has_new_ids = true;
                    id
                });
                // 根据 status 字段初始化 disabled 状态
                // 这样 invalid 状态的凭证在重启后仍然被禁用
                let (disabled, disabled_reason) = if cred.status == "invalid" {
                    tracing::warn!("凭证 #{} 状态为 invalid，已自动禁用", id);
                    (true, Some(DisabledReason::Suspended))
                } else {
                    (false, None)
                };
                CredentialEntry {
                    id,
                    credentials: cred,
                    failure_count: 0,
                    disabled,
                    disabled_reason,
                }
            })
            .collect();

        // 检测重复 ID
        let mut seen_ids = std::collections::HashSet::new();
        let mut duplicate_ids = Vec::new();
        for entry in &entries {
            if !seen_ids.insert(entry.id) {
                duplicate_ids.push(entry.id);
            }
        }
        if !duplicate_ids.is_empty() {
            anyhow::bail!("检测到重复的凭证 ID: {:?}", duplicate_ids);
        }

        // 选择初始凭证：ID 最小的可用凭证，无可用凭证时为 0
        let initial_id = entries
            .iter()
            .filter(|e| e.is_available())
            .min_by_key(|e| e.id)
            .map(|e| e.id)
            .unwrap_or(0);

        let manager = Self {
            config,
            proxy,
            entries: Mutex::new(entries),
            current_id: Mutex::new(initial_id),
            refresh_lock: TokioMutex::new(()),
            credentials_path,
            is_multiple_format,
            active_group_id: Mutex::new(None),
        };

        // 如果有新分配的 ID，立即持久化到配置文件
        if has_new_ids {
            if let Err(e) = manager.persist_credentials() {
                tracing::warn!("新分配 ID 后持久化失败: {}", e);
            } else {
                tracing::info!("已为凭证分配新 ID 并写回配置文件");
            }
        }

        Ok(manager)
    }

    /// 获取配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取当前活动凭证的克隆
    pub fn credentials(&self) -> KiroCredentials {
        let entries = self.entries.lock();
        let current_id = *self.current_id.lock();
        entries
            .iter()
            .find(|e| e.id == current_id)
            .map(|e| e.credentials.clone())
            .unwrap_or_default()
    }

    /// 获取凭证总数
    pub fn total_count(&self) -> usize {
        self.entries.lock().len()
    }

    /// 获取可用凭证数量
    pub fn available_count(&self) -> usize {
        self.entries.lock().iter().filter(|e| e.is_available()).count()
    }

    /// 获取当前使用的凭证 ID
    pub fn current_id(&self) -> u64 {
        *self.current_id.lock()
    }

    /// 设置活跃分组（反代使用）
    pub fn set_active_group(&self, group_id: Option<String>) {
        {
            let mut active = self.active_group_id.lock();
            *active = group_id;
        }
        // 切换分组后重新选择凭证（锁已释放）
        self.select_smallest_id_in_group();
    }

    /// 获取当前活跃分组
    pub fn get_active_group(&self) -> Option<String> {
        self.active_group_id.lock().clone()
    }

    /// 刷新凭证选择（重新选择当前分组内 ID 最小的凭证）
    pub fn refresh_credential_selection(&self) {
        self.select_smallest_id_in_group();
    }

    /// 检查凭证是否在活跃分组内
    fn is_in_active_group(&self, credentials: &KiroCredentials) -> bool {
        let active_group = self.active_group_id.lock();
        match active_group.as_ref() {
            None => true, // 无分组限制，所有凭证可用
            Some(group_id) => &credentials.group_id == group_id,
        }
    }

    /// 选择活跃分组内 ID 最小的凭证
    fn select_smallest_id_in_group(&self) {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();
        let active_group = self.active_group_id.lock();

        // 选择活跃分组内 ID 最小的可用凭证
        let best = entries
            .iter()
            .filter(|e| {
                if !e.is_available() {
                    return false;
                }
                match active_group.as_ref() {
                    None => true,
                    Some(group_id) => &e.credentials.group_id == group_id,
                }
            })
            .min_by_key(|e| e.id);

        match best {
            Some(entry) => {
                if entry.id != *current_id {
                    tracing::info!(
                        "分组切换后选择凭证: #{} -> #{}",
                        *current_id,
                        entry.id
                    );
                    *current_id = entry.id;
                } else {
                    tracing::info!("分组内当前凭证有效: #{}", entry.id);
                }
            }
            None => {
                // 分组内没有可用凭证，将 current_id 设为 0
                tracing::warn!("活跃分组内没有可用凭证，current_id 设为 0");
                *current_id = 0;
            }
        }
    }

    /// 获取用于导出的凭证数据
    /// 
    /// # Arguments
    /// * `ids` - 要导出的凭证 ID 列表，为空则导出全部
    ///
    /// # Returns
    /// 凭证列表（包含完整数据）
    pub fn get_credentials_for_export(&self, ids: &[u64]) -> Vec<KiroCredentials> {
        let entries = self.entries.lock();
        let id_set: std::collections::HashSet<u64> = ids.iter().cloned().collect();
        
        entries
            .iter()
            .filter(|e| id_set.is_empty() || id_set.contains(&e.id))
            .map(|e| e.credentials.clone())
            .collect()
    }

    /// 获取 API 调用上下文
    ///
    /// 返回绑定了 id、credentials 和 token 的调用上下文
    /// 确保整个 API 调用过程中使用一致的凭证信息
    ///
    /// 如果 Token 过期或即将过期，会自动刷新
    /// Token 刷新失败时会尝试下一个可用凭证（不计入失败次数）
    pub async fn acquire_context(&self) -> anyhow::Result<CallContext> {
        let total = self.total_count();
        let mut tried_count = 0;

        loop {
            if tried_count >= total {
                anyhow::bail!(
                    "所有凭证均无法获取有效 Token（可用: {}/{}）",
                    self.available_count(),
                    total
                );
            }

            let (id, credentials) = {
                let mut entries = self.entries.lock();
                let current_id = *self.current_id.lock();
                let active_group = self.active_group_id.lock();

                // 分组过滤闭包
                let in_group = |cred: &KiroCredentials| -> bool {
                    match active_group.as_ref() {
                        None => true,
                        Some(group_id) => &cred.group_id == group_id,
                    }
                };

                // 找到当前凭证（需要在分组内且可用）
                if let Some(entry) = entries.iter().find(|e| {
                    e.id == current_id && e.is_available() && in_group(&e.credentials)
                }) {
                    (entry.id, entry.credentials.clone())
                } else {
                    // 当前凭证不可用，选择分组内 ID 最小的可用凭证
                    let mut best = entries
                        .iter()
                        .filter(|e| e.is_available() && in_group(&e.credentials))
                        .min_by_key(|e| e.id);

                    // 没有可用凭证：如果是"自动禁用导致全灭"，做一次类似重启的自愈
                    if best.is_none()
                        && entries.iter().any(|e| {
                            e.disabled && e.disabled_reason == Some(DisabledReason::TooManyFailures)
                        })
                    {
                        tracing::warn!(
                            "所有凭证均已被自动禁用，执行自愈：重置失败计数并重新启用（等价于重启）"
                        );
                        for e in entries.iter_mut() {
                            if e.disabled_reason == Some(DisabledReason::TooManyFailures) {
                                e.disabled = false;
                                e.disabled_reason = None;
                                e.failure_count = 0;
                            }
                        }
                        best = entries
                            .iter()
                            .filter(|e| e.is_available() && in_group(&e.credentials))
                            .min_by_key(|e| e.id);
                    }

                    if let Some(entry) = best {
                        // 先提取数据
                        let new_id = entry.id;
                        let new_creds = entry.credentials.clone();
                        drop(active_group);
                        drop(entries);
                        // 更新 current_id
                        let mut current_id = self.current_id.lock();
                        *current_id = new_id;
                        (new_id, new_creds)
                    } else {
                        // 注意：必须在 bail! 之前计算 available_count，
                        // 因为 available_count() 会尝试获取 entries 锁，
                        // 而此时我们已经持有该锁，会导致死锁
                        let available = entries.iter().filter(|e| !e.disabled).count();
                        let group_info = match active_group.as_ref() {
                            Some(g) => format!("分组 '{}' 内", g),
                            None => "全部".to_string(),
                        };
                        anyhow::bail!("{}凭证均已禁用或无可用凭证（{}/{}）", group_info, available, total);
                    }
                }
            };

            // 尝试获取/刷新 Token
            match self.try_ensure_token(id, &credentials).await {
                Ok(ctx) => {
                    return Ok(ctx);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::warn!("凭证 #{} Token 刷新失败，尝试下一个凭证: {}", id, error_msg);

                    // 检测是否为凭证无效/被暂停的错误
                    if is_credential_invalid_error(&error_msg) {
                        let mut entries = self.entries.lock();
                        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                            entry.disabled = true;
                            entry.disabled_reason = Some(DisabledReason::Suspended);
                            entry.credentials.status = "invalid".to_string();
                            tracing::error!(
                                "凭证 #{} 已被自动禁用（账户暂停/无效）: {}",
                                id,
                                error_msg
                            );
                        }
                        drop(entries);
                        // 持久化更改
                        if let Err(persist_err) = self.persist_credentials() {
                            tracing::warn!("凭证禁用后持久化失败: {}", persist_err);
                        }
                    }

                    // Token 刷新失败，切换到下一个优先级的凭证（不计入失败次数）
                    self.switch_to_next_by_id();
                    tried_count += 1;
                }
            }
        }
    }

    /// 切换到下一个 ID 最小的可用凭证（内部方法）
    fn switch_to_next_by_id(&self) {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();

        // 选择 ID 最小的未禁用凭证（排除当前凭证）
        if let Some(entry) = entries
            .iter()
            .filter(|e| !e.disabled && e.id != *current_id)
            .min_by_key(|e| e.id)
        {
            *current_id = entry.id;
            tracing::info!(
                "已切换到凭证 #{}",
                entry.id
            );
        }
    }

    /// 选择 ID 最小的未禁用凭证作为当前凭证（内部方法）
    ///
    /// 不排除当前凭证，纯粹按 ID 选择
    fn select_smallest_id(&self) {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();

        // 选择 ID 最小的未禁用凭证（不排除当前凭证）
        if let Some(best) = entries
            .iter()
            .filter(|e| !e.disabled)
            .min_by_key(|e| e.id)
        {
            if best.id != *current_id {
                tracing::info!(
                    "切换凭证: #{} -> #{}",
                    *current_id,
                    best.id
                );
                *current_id = best.id;
            }
        }
    }

    /// 尝试使用指定凭证获取有效 Token
    ///
    /// 使用双重检查锁定模式，确保同一时间只有一个刷新操作
    ///
    /// # Arguments
    /// * `id` - 凭证 ID，用于更新正确的条目
    /// * `credentials` - 凭证信息
    async fn try_ensure_token(
        &self,
        id: u64,
        credentials: &KiroCredentials,
    ) -> anyhow::Result<CallContext> {
        // 第一次检查（无锁）：快速判断是否需要刷新
        let needs_refresh = is_token_expired(credentials) || is_token_expiring_soon(credentials);

        let creds = if needs_refresh {
            // 获取刷新锁，确保同一时间只有一个刷新操作
            let _guard = self.refresh_lock.lock().await;

            // 第二次检查：获取锁后重新读取凭证，因为其他请求可能已经完成刷新
            let current_creds = {
                let entries = self.entries.lock();
                entries
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.credentials.clone())
                    .ok_or_else(|| anyhow::anyhow!("凭证 #{} 不存在", id))?
            };

            if is_token_expired(&current_creds) || is_token_expiring_soon(&current_creds) {
                // 确实需要刷新
                let new_creds =
                    refresh_token(&current_creds, &self.config, self.proxy.as_ref()).await?;

                if is_token_expired(&new_creds) {
                    anyhow::bail!("刷新后的 Token 仍然无效或已过期");
                }

                // 更新凭证
                {
                    let mut entries = self.entries.lock();
                    if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                        entry.credentials = new_creds.clone();
                    }
                }

                // 回写凭证到文件（仅多凭证格式），失败只记录警告
                if let Err(e) = self.persist_credentials() {
                    tracing::warn!("Token 刷新后持久化失败（不影响本次请求）: {}", e);
                }

                new_creds
            } else {
                // 其他请求已经完成刷新，直接使用新凭证
                tracing::debug!("Token 已被其他请求刷新，跳过刷新");
                current_creds
            }
        } else {
            credentials.clone()
        };

        let token = creds
            .access_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("没有可用的 accessToken"))?;

        Ok(CallContext {
            id,
            credentials: creds,
            token,
        })
    }

    /// 将凭证列表回写到源文件
    ///
    /// 仅在以下条件满足时回写：
    /// - 源文件是多凭证格式（数组）
    /// - credentials_path 已设置
    ///
    /// # Returns
    /// - `Ok(true)` - 成功写入文件
    /// - `Ok(false)` - 跳过写入（非多凭证格式或无路径配置）
    /// - `Err(_)` - 写入失败
    fn persist_credentials(&self) -> anyhow::Result<bool> {
        use anyhow::Context;

        // 仅多凭证格式才回写
        if !self.is_multiple_format {
            return Ok(false);
        }

        let path = match &self.credentials_path {
            Some(p) => p,
            None => return Ok(false),
        };

        // 收集所有凭证
        let credentials: Vec<KiroCredentials> = {
            let entries = self.entries.lock();
            entries.iter().map(|e| e.credentials.clone()).collect()
        };

        // 序列化为 pretty JSON
        let json = serde_json::to_string_pretty(&credentials).context("序列化凭证失败")?;

        // 写入文件（在 Tokio runtime 内使用 block_in_place 避免阻塞 worker）
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| std::fs::write(path, &json))
                .with_context(|| format!("回写凭证文件失败: {:?}", path))?;
        } else {
            std::fs::write(path, &json).with_context(|| format!("回写凭证文件失败: {:?}", path))?;
        }

        tracing::debug!("已回写凭证到文件: {:?}", path);
        Ok(true)
    }

    /// 报告指定凭证 API 调用成功
    ///
    /// 重置该凭证的失败计数
    ///
    /// # Arguments
    /// * `id` - 凭证 ID（来自 CallContext）
    pub fn report_success(&self, id: u64) {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.failure_count = 0;
            tracing::debug!("凭证 #{} API 调用成功", id);
        }
    }

    /// 设置凭证分组（Admin API）
    pub fn set_group(&self, id: u64, group_id: &str) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?;
            entry.credentials.group_id = group_id.to_string();
        }
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 报告指定凭证 API 调用失败
    ///
    /// 增加失败计数，达到阈值时禁用凭证并切换到优先级最高的可用凭证
    /// 返回是否还有可用凭证可以重试
    ///
    /// # Arguments
    /// * `id` - 凭证 ID（来自 CallContext）
    pub fn report_failure(&self, id: u64) -> bool {
        let mut entries = self.entries.lock();
        let mut current_id = self.current_id.lock();

        let entry = match entries.iter_mut().find(|e| e.id == id) {
            Some(e) => e,
            None => return entries.iter().any(|e| !e.disabled),
        };

        entry.failure_count += 1;
        let failure_count = entry.failure_count;

        tracing::warn!(
            "凭证 #{} API 调用失败（{}/{}）",
            id,
            failure_count,
            MAX_FAILURES_PER_CREDENTIAL
        );

        if failure_count >= MAX_FAILURES_PER_CREDENTIAL {
            entry.disabled = true;
            entry.disabled_reason = Some(DisabledReason::TooManyFailures);
            tracing::error!("凭证 #{} 已连续失败 {} 次，已被禁用", id, failure_count);

            // 切换到 ID 最小的可用凭证
            if let Some(next) = entries
                .iter()
                .filter(|e| e.is_available())
                .min_by_key(|e| e.id)
            {
                *current_id = next.id;
                tracing::info!(
                    "已切换到凭证 #{}",
                    next.id
                );
            } else {
                tracing::error!("所有凭证均已禁用！");
                return false;
            }
        }

        // 检查是否还有可用凭证
        entries.iter().any(|e| e.is_available())
    }

    /// 报告指定凭证 API 调用失败（带错误消息）
    ///
    /// 与 report_failure 类似，但会检测错误消息：
    /// - 如果是账户暂停/凭证无效错误，立即禁用凭证
    /// - 否则按普通失败处理（累计失败次数）
    ///
    /// # Arguments
    /// * `id` - 凭证 ID
    /// * `error_msg` - 错误响应消息
    ///
    /// # Returns
    /// 是否还有可用凭证
    pub fn report_failure_with_error(&self, id: u64, error_msg: &str) -> bool {
        // 检测是否为凭证无效/被暂停的错误
        if is_credential_invalid_error(error_msg) {
            let mut entries = self.entries.lock();
            let mut current_id = self.current_id.lock();
            
            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                entry.disabled = true;
                entry.disabled_reason = Some(DisabledReason::Suspended);
                entry.credentials.status = "invalid".to_string();
                tracing::error!(
                    "凭证 #{} 已被自动禁用（账户暂停/无效）",
                    id
                );
                
                // 切换到 ID 最小的可用凭证
                if let Some(next) = entries.iter().filter(|e| e.is_available()).min_by_key(|e| e.id) {
                    *current_id = next.id;
                    tracing::info!("已切换到凭证 #{}", next.id);
                } else {
                    tracing::error!("所有凭证均已禁用！");
                }
                
                // 释放锁后持久化
                drop(current_id);
                drop(entries);
                if let Err(e) = self.persist_credentials() {
                    tracing::warn!("凭证禁用后持久化失败: {}", e);
                }
            }
            
            return self.entries.lock().iter().any(|e| e.is_available());
        }
        
        // 普通失败处理
        self.report_failure(id)
    }

    /// 切换到下一个可用凭证（按列表顺序轮询）
    ///
    /// 返回是否成功切换
    pub fn switch_to_next(&self) -> bool {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();
        let active_group = self.active_group_id.lock();

        // 分组过滤闭包
        let in_group = |cred: &KiroCredentials| -> bool {
            match active_group.as_ref() {
                None => true,
                Some(group_id) => &cred.group_id == group_id,
            }
        };

        // 收集分组内可用的凭证
        let available: Vec<_> = entries
            .iter()
            .filter(|e| e.is_available() && in_group(&e.credentials))
            .collect();
        
        if available.is_empty() {
            tracing::warn!("没有可用的凭证");
            return false;
        }
        
        if available.len() == 1 {
            // 只有一个凭证，无法切换
            tracing::info!("只有一个可用凭证，无法切换");
            return false;
        }

        // 找到当前凭证在可用列表中的位置，然后选择下一个
        let current_available_pos = available.iter().position(|e| e.id == *current_id);
        
        let next = if let Some(pos) = current_available_pos {
            // 循环到下一个
            let next_pos = (pos + 1) % available.len();
            available[next_pos]
        } else {
            // 当前凭证不在可用列表中，选择第一个
            available[0]
        };

        *current_id = next.id;
        tracing::info!(
            "已切换到凭证 #{}（顺序轮询）",
            next.id,
        );
        true
    }

    /// 获取使用额度信息
    pub async fn get_usage_limits(&self) -> anyhow::Result<UsageLimitsResponse> {
        let ctx = self.acquire_context().await?;
        get_usage_limits(
            &ctx.credentials,
            &self.config,
            &ctx.token,
            self.proxy.as_ref(),
        )
        .await
    }

    /// 刷新所有凭证的 Token
    /// 
    /// 返回成功刷新的凭证数量（10 并发）
    pub async fn refresh_all_credentials(&self) -> anyhow::Result<usize> {
        use futures::stream::{self, StreamExt};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        
        let credentials_to_refresh: Vec<(u64, KiroCredentials)> = {
            let entries = self.entries.lock();
            entries
                .iter()
                .filter(|e| !e.disabled)
                .map(|e| (e.id, e.credentials.clone()))
                .collect()
        };

        if credentials_to_refresh.is_empty() {
            return Ok(0);
        }

        let refreshed_count = Arc::new(AtomicUsize::new(0));
        let config = self.config.clone();
        let proxy = self.proxy.clone();
        let entries_ref = &self.entries;
        
        // 10 并发刷新
        stream::iter(credentials_to_refresh)
            .for_each_concurrent(10, |(id, credentials)| {
                let config = config.clone();
                let proxy = proxy.clone();
                let refreshed_count = refreshed_count.clone();
                
                async move {
                    match refresh_token(&credentials, &config, proxy.as_ref()).await {
                        Ok(new_creds) => {
                            let mut entries = entries_ref.lock();
                            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                                entry.credentials = new_creds;
                                refreshed_count.fetch_add(1, Ordering::SeqCst);
                                tracing::debug!("凭证 #{} Token 已刷新", id);
                            }
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            tracing::warn!("凭证 #{} Token 刷新失败: {}", id, error_msg);
                            
                            // 检测是否为凭证无效/被暂停的错误
                            if is_credential_invalid_error(&error_msg) {
                                let mut entries = entries_ref.lock();
                                if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                                    entry.disabled = true;
                                    entry.disabled_reason = Some(DisabledReason::Suspended);
                                    entry.credentials.status = "invalid".to_string();
                                    tracing::error!(
                                        "凭证 #{} 已被自动禁用（账户暂停/无效）: {}",
                                        id,
                                        error_msg
                                    );
                                }
                            }
                        }
                    }
                }
            })
            .await;

        let count = refreshed_count.load(Ordering::SeqCst);
        
        // 持久化凭证（无论成功还是有凭证被禁用都需要持久化）
        if let Err(e) = self.persist_credentials() {
            tracing::warn!("刷新后持久化失败: {}", e);
        }

        Ok(count)
    }

    // ========================================================================
    // Admin API 方法
    // ========================================================================

    /// 获取管理器状态快照（用于 Admin API）
    pub fn snapshot(&self) -> ManagerSnapshot {
        let entries = self.entries.lock();
        let current_id = *self.current_id.lock();
        let available = entries.iter().filter(|e| e.is_available()).count();

        ManagerSnapshot {
            entries: entries
                .iter()
                .map(|e| CredentialEntrySnapshot {
                    id: e.id,
                    disabled: e.disabled,
                    failure_count: e.failure_count,
                    auth_method: e.credentials.auth_method.clone(),
                    has_profile_arn: e.credentials.profile_arn.is_some(),
                    expires_at: e.credentials.expires_at.clone(),
                    email: e.credentials.email.clone(),
                    subscription_title: e.credentials.subscription_title.clone(),
                    current_usage: e.credentials.current_usage,
                    usage_limit: e.credentials.usage_limit,
                    remaining: e.credentials.remaining,
                    next_reset_at: e.credentials.next_reset_at,
                    refresh_token: e.credentials.refresh_token.clone(),
                    access_token: e.credentials.access_token.clone(),
                    profile_arn: e.credentials.profile_arn.clone(),
                    status: e.credentials.status.clone(),
                    group_id: e.credentials.group_id.clone(),
                })
                .collect(),
            current_id,
            total: entries.len(),
            available,
        }
    }

    /// 设置凭证禁用状态（Admin API）
    pub fn set_disabled(&self, id: u64, disabled: bool) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?;
            entry.disabled = disabled;
            if !disabled {
                // 启用时重置失败计数
                entry.failure_count = 0;
                entry.disabled_reason = None;
            } else {
                entry.disabled_reason = Some(DisabledReason::Manual);
            }
        }
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 标记凭证为暂停/无效状态
    /// 
    /// 用于自动检测到凭证无效（如 TEMPORARILY_SUSPENDED）时禁用凭证
    pub fn mark_as_suspended(&self, id: u64) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?;
            entry.disabled = true;
            entry.disabled_reason = Some(DisabledReason::Suspended);
            entry.credentials.status = "invalid".to_string();
            tracing::error!("凭证 #{} 已被标记为暂停/无效", id);
        }
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 重置凭证失败计数并重新启用（Admin API）
    pub fn reset_and_enable(&self, id: u64) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?;
            entry.failure_count = 0;
            entry.disabled = false;
            entry.disabled_reason = None;
            // 如果凭证状态是 invalid（被暂停导致），恢复为 normal
            if entry.credentials.status == "invalid" {
                entry.credentials.status = "normal".to_string();
            }
        }
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 更新凭证状态（Admin API）
    pub fn update_status(&self, id: u64, status: &str) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?;
            entry.credentials.status = status.to_string();
        }
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 刷新指定凭证的 Token（Admin API）
    pub async fn refresh_token_for(&self, id: u64) -> anyhow::Result<()> {
        let credentials = {
            let entries = self.entries.lock();
            entries
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.credentials.clone())
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?
        };

        // 刷新 Token
        let new_credentials = refresh_token(&credentials, &self.config, self.proxy.as_ref()).await?;

        // 更新凭证（刷新成功，状态设为 normal）
        {
            let mut entries = self.entries.lock();
            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                entry.credentials.access_token = new_credentials.access_token;
                entry.credentials.expires_at = new_credentials.expires_at;
                entry.credentials.profile_arn = new_credentials.profile_arn.or(entry.credentials.profile_arn.clone());
                entry.credentials.status = "normal".to_string();
            }
        }

        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 获取指定凭证的使用额度（Admin API）
    pub async fn get_usage_limits_for(&self, id: u64) -> anyhow::Result<UsageLimitsResponse> {
        let credentials = {
            let entries = self.entries.lock();
            entries
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.credentials.clone())
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?
        };

        // 检查是否需要刷新 token
        let needs_refresh = is_token_expired(&credentials) || is_token_expiring_soon(&credentials);

        let token = if needs_refresh {
            let _guard = self.refresh_lock.lock().await;
            let current_creds = {
                let entries = self.entries.lock();
                entries
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.credentials.clone())
                    .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?
            };

            if is_token_expired(&current_creds) || is_token_expiring_soon(&current_creds) {
                match refresh_token(&current_creds, &self.config, self.proxy.as_ref()).await {
                    Ok(new_creds) => {
                        {
                            let mut entries = self.entries.lock();
                            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                                entry.credentials = new_creds.clone();
                            }
                        }
                        // 持久化失败只记录警告，不影响本次请求
                        if let Err(e) = self.persist_credentials() {
                            tracing::warn!("Token 刷新后持久化失败（不影响本次请求）: {}", e);
                        }
                        new_creds
                            .access_token
                            .ok_or_else(|| anyhow::anyhow!("刷新后无 access_token"))?
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        // 检测是否为凭证无效/被暂停的错误
                        if is_credential_invalid_error(&error_msg) {
                            let mut entries = self.entries.lock();
                            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                                entry.disabled = true;
                                entry.disabled_reason = Some(DisabledReason::Suspended);
                                entry.credentials.status = "invalid".to_string();
                                tracing::error!(
                                    "凭证 #{} 已被自动禁用（账户暂停/无效）: {}",
                                    id,
                                    error_msg
                                );
                            }
                            drop(entries);
                            let _ = self.persist_credentials();
                        }
                        return Err(e);
                    }
                }
            } else {
                current_creds
                    .access_token
                    .ok_or_else(|| anyhow::anyhow!("凭证无 access_token"))?
            }
        } else {
            credentials
                .access_token
                .ok_or_else(|| anyhow::anyhow!("凭证无 access_token"))?
        };

        let credentials = {
            let entries = self.entries.lock();
            entries
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.credentials.clone())
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?
        };

        let usage = match get_usage_limits(&credentials, &self.config, &token, self.proxy.as_ref()).await {
            Ok(u) => u,
            Err(e) => {
                let error_msg = e.to_string();
                // 检测是否为凭证无效/被暂停的错误
                if is_credential_invalid_error(&error_msg) {
                    let mut entries = self.entries.lock();
                    if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                        entry.disabled = true;
                        entry.disabled_reason = Some(DisabledReason::Suspended);
                        entry.credentials.status = "invalid".to_string();
                        tracing::error!(
                            "凭证 #{} 已被自动禁用（账户暂停/无效）: {}",
                            id,
                            error_msg
                        );
                    }
                    drop(entries);
                    let _ = self.persist_credentials();
                }
                return Err(e);
            }
        };
        
        // 更新凭证的缓存信息（email、subscription、余额）
        let email = usage.email().map(|s| s.to_string());
        let subscription_title = usage.subscription_title().map(|s| s.to_string());
        let current_usage = usage.current_usage();
        let usage_limit_val = usage.usage_limit();
        let remaining = (usage_limit_val - current_usage).max(0.0);
        let next_reset_at = usage.next_date_reset;
        
        {
            let mut entries = self.entries.lock();
            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                let mut changed = false;
                if email.is_some() && entry.credentials.email != email {
                    entry.credentials.email = email;
                    changed = true;
                }
                if subscription_title.is_some() && entry.credentials.subscription_title != subscription_title {
                    entry.credentials.subscription_title = subscription_title;
                    changed = true;
                }
                // 更新余额信息
                entry.credentials.current_usage = Some(current_usage);
                entry.credentials.usage_limit = Some(usage_limit_val);
                entry.credentials.remaining = Some(remaining);
                entry.credentials.next_reset_at = next_reset_at;
                changed = true;
                
                if changed {
                    drop(entries);
                    if let Err(e) = self.persist_credentials() {
                        tracing::warn!("更新缓存信息后持久化失败: {}", e);
                    }
                }
            }
        }
        
        Ok(usage)
    }

    /// 添加新凭证（Admin API）
    ///
    /// # 流程
    /// 1. 验证凭证基本字段（refresh_token 不为空）
    /// 2. 检查重复（refresh_token 已存在则跳过）
    /// 3. 尝试刷新 Token 验证凭证有效性
    /// 4. 分配新 ID（当前最大 ID + 1）
    /// 5. 添加到 entries 列表
    /// 6. 持久化到配置文件
    ///
    /// # 返回
    /// - `Ok(u64)` - 新凭证 ID
    /// - `Err(_)` - 验证失败或添加失败
    pub async fn add_credential(&self, new_cred: KiroCredentials) -> anyhow::Result<u64> {
        // 1. 基本验证
        validate_refresh_token(&new_cred)?;

        // 2. 检查重复（基于 refresh_token 前 50 字符）
        let new_refresh_token = new_cred.refresh_token.as_ref().unwrap();
        let new_token_prefix: String = new_refresh_token.chars().take(50).collect();
        {
            let entries = self.entries.lock();
            for entry in entries.iter() {
                if let Some(existing_token) = &entry.credentials.refresh_token {
                    let existing_prefix: String = existing_token.chars().take(50).collect();
                    if existing_prefix == new_token_prefix {
                        anyhow::bail!("凭证已存在（与凭证 #{} 重复）", entry.id);
                    }
                }
            }
        }

        // 3. 尝试刷新 Token 验证凭证有效性
        let mut validated_cred =
            refresh_token(&new_cred, &self.config, self.proxy.as_ref()).await?;


        // 4. 分配新 ID（找最小可用 ID，从 1 开始，复用已删除的 ID）
        let new_id = {
            let entries = self.entries.lock();
            let used_ids: std::collections::HashSet<u64> = entries.iter().map(|e| e.id).collect();
            // 从 1 开始找第一个未使用的 ID
            let mut id = 1u64;
            while used_ids.contains(&id) {
                id += 1;
            }
            id
        };

        // 5. 设置 ID 并保留用户输入的元数据
        validated_cred.id = Some(new_id);
        validated_cred.auth_method = new_cred.auth_method;
        validated_cred.client_id = new_cred.client_id;
        validated_cred.client_secret = new_cred.client_secret;

        {
            let mut entries = self.entries.lock();
            entries.push(CredentialEntry {
                id: new_id,
                credentials: validated_cred,
                failure_count: 0,
                disabled: false,
                disabled_reason: None,
            });
        }

        // 6. 持久化
        self.persist_credentials()?;

        tracing::info!("成功添加凭证 #{}", new_id);

        // 7. 获取余额信息（异步，不影响添加结果）
        // 这会在后台更新 email、subscription、balance 等信息
        if let Err(e) = self.get_usage_limits_for(new_id).await {
            tracing::warn!("添加凭证 #{} 后获取余额失败: {}", new_id, e);
        }

        Ok(new_id)
    }

    /// 删除凭证（Admin API）
    ///
    /// # 行为
    /// 1. 验证凭证存在
    /// 2. 从 entries 移除
    /// 3. 如果删除的是当前凭证，切换到优先级最高的可用凭证
    /// 4. 如果删除后没有凭证，将 current_id 重置为 0
    /// 5. 持久化到文件
    ///
    /// # 返回
    /// - `Ok(())` - 删除成功
    /// - `Err(_)` - 凭证不存在或持久化失败
    pub fn delete_credential(&self, id: u64) -> anyhow::Result<()> {
        let was_current = {
            let mut entries = self.entries.lock();

            // 查找凭证
            let _entry = entries
                .iter()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭证不存在: {}", id))?;

            // 记录是否是当前凭证
            let current_id = *self.current_id.lock();
            let was_current = current_id == id;

            // 删除凭证
            entries.retain(|e| e.id != id);

            was_current
        };

        // 如果删除的是当前凭证，切换到优先级最高的可用凭证
        if was_current {
            self.select_smallest_id();
        }

        // 如果删除后没有任何凭证，将 current_id 重置为 0（与初始化行为保持一致）
        {
            let entries = self.entries.lock();
            if entries.is_empty() {
                let mut current_id = self.current_id.lock();
                *current_id = 0;
                tracing::info!("所有凭证已删除，current_id 已重置为 0");
            }
        }

        // 持久化更改
        self.persist_credentials()?;

        tracing::info!("已删除凭证 #{}", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_manager_new() {
        let config = Config::default();
        let credentials = KiroCredentials::default();
        let tm = TokenManager::new(config, credentials, None);
        assert!(tm.credentials().access_token.is_none());
    }

    #[test]
    fn test_is_token_expired_with_expired_token() {
        let mut credentials = KiroCredentials::default();
        credentials.expires_at = Some("2020-01-01T00:00:00Z".to_string());
        assert!(is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expired_with_valid_token() {
        let mut credentials = KiroCredentials::default();
        let future = Utc::now() + Duration::hours(1);
        credentials.expires_at = Some(future.to_rfc3339());
        assert!(!is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expired_within_5_minutes() {
        let mut credentials = KiroCredentials::default();
        let expires = Utc::now() + Duration::minutes(3);
        credentials.expires_at = Some(expires.to_rfc3339());
        assert!(is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expired_no_expires_at() {
        let credentials = KiroCredentials::default();
        assert!(is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expiring_soon_within_10_minutes() {
        let mut credentials = KiroCredentials::default();
        let expires = Utc::now() + Duration::minutes(8);
        credentials.expires_at = Some(expires.to_rfc3339());
        assert!(is_token_expiring_soon(&credentials));
    }

    #[test]
    fn test_is_token_expiring_soon_beyond_10_minutes() {
        let mut credentials = KiroCredentials::default();
        let expires = Utc::now() + Duration::minutes(15);
        credentials.expires_at = Some(expires.to_rfc3339());
        assert!(!is_token_expiring_soon(&credentials));
    }

    #[test]
    fn test_validate_refresh_token_missing() {
        let credentials = KiroCredentials::default();
        let result = validate_refresh_token(&credentials);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_refresh_token_valid() {
        let mut credentials = KiroCredentials::default();
        credentials.refresh_token = Some("a".repeat(150));
        let result = validate_refresh_token(&credentials);
        assert!(result.is_ok());
    }

    // MultiTokenManager 测试

    #[test]
    fn test_multi_token_manager_new() {
        let config = Config::default();
        let cred1 = KiroCredentials::default();
        let cred2 = KiroCredentials::default();

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None, false).unwrap();
        assert_eq!(manager.total_count(), 2);
        assert_eq!(manager.available_count(), 2);
    }

    #[test]
    fn test_multi_token_manager_empty_credentials() {
        let config = Config::default();
        let result = MultiTokenManager::new(config, vec![], None, None, false);
        // 支持 0 个凭证启动（可通过管理面板添加）
        assert!(result.is_ok());
        let manager = result.unwrap();
        assert_eq!(manager.total_count(), 0);
        assert_eq!(manager.available_count(), 0);
    }

    #[test]
    fn test_multi_token_manager_duplicate_ids() {
        let config = Config::default();
        let mut cred1 = KiroCredentials::default();
        cred1.id = Some(1);
        let mut cred2 = KiroCredentials::default();
        cred2.id = Some(1); // 重复 ID

        let result = MultiTokenManager::new(config, vec![cred1, cred2], None, None, false);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("重复的凭证 ID"),
            "错误消息应包含 '重复的凭证 ID'，实际: {}",
            err_msg
        );
    }

    #[test]
    fn test_multi_token_manager_report_failure() {
        let config = Config::default();
        let cred1 = KiroCredentials::default();
        let cred2 = KiroCredentials::default();

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None, false).unwrap();

        // 凭证会自动分配 ID（从 1 开始）
        // 前两次失败不会禁用（使用 ID 1）
        assert!(manager.report_failure(1));
        assert!(manager.report_failure(1));
        assert_eq!(manager.available_count(), 2);

        // 第三次失败会禁用第一个凭证
        assert!(manager.report_failure(1));
        assert_eq!(manager.available_count(), 1);

        // 继续失败第二个凭证（使用 ID 2）
        assert!(manager.report_failure(2));
        assert!(manager.report_failure(2));
        assert!(!manager.report_failure(2)); // 所有凭证都禁用了
        assert_eq!(manager.available_count(), 0);
    }

    #[test]
    fn test_multi_token_manager_report_success() {
        let config = Config::default();
        let cred = KiroCredentials::default();

        let manager = MultiTokenManager::new(config, vec![cred], None, None, false).unwrap();

        // 失败两次（使用 ID 1）
        manager.report_failure(1);
        manager.report_failure(1);

        // 成功后重置计数（使用 ID 1）
        manager.report_success(1);

        // 再失败两次不会禁用
        manager.report_failure(1);
        manager.report_failure(1);
        assert_eq!(manager.available_count(), 1);
    }

    #[test]
    fn test_multi_token_manager_switch_to_next() {
        let config = Config::default();
        let mut cred1 = KiroCredentials::default();
        cred1.refresh_token = Some("token1".to_string());
        let mut cred2 = KiroCredentials::default();
        cred2.refresh_token = Some("token2".to_string());

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None, false).unwrap();

        // 初始是第一个凭证
        assert_eq!(
            manager.credentials().refresh_token,
            Some("token1".to_string())
        );

        // 切换到下一个
        assert!(manager.switch_to_next());
        assert_eq!(
            manager.credentials().refresh_token,
            Some("token2".to_string())
        );
    }
}
