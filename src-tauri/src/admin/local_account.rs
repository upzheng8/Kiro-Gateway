//! 本地账号读取模块
//! 
//! 从 Kiro 客户端本地凭证文件读取 Token

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 本地 Kiro 凭证结构
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalKiroCredential {
    /// Access Token
    pub access_token: Option<String>,
    /// Refresh Token
    pub refresh_token: Option<String>,
    /// Profile ARN
    pub profile_arn: Option<String>,
    /// 过期时间
    pub expires_at: Option<String>,
    /// 认证方式
    pub auth_method: Option<String>,
    /// 提供者 (Google 等)
    pub provider: Option<String>,
}

/// 获取本地 Kiro 凭证文件路径
/// Windows: %USERPROFILE%\.aws\sso\cache\kiro-auth-token.json
/// macOS/Linux: ~/.aws/sso/cache/kiro-auth-token.json
pub fn get_local_credential_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        home.join(".aws")
            .join("sso")
            .join("cache")
            .join("kiro-auth-token.json")
    })
}

/// 读取本地 Kiro 凭证
pub fn read_local_credential() -> anyhow::Result<LocalKiroCredential> {
    let path = get_local_credential_path()
        .ok_or_else(|| anyhow::anyhow!("无法获取用户目录"))?;
    
    if !path.exists() {
        return Err(anyhow::anyhow!("本地凭证文件不存在: {:?}", path));
    }
    
    let content = std::fs::read_to_string(&path)?;
    let credential: LocalKiroCredential = serde_json::from_str(&content)?;
    
    if credential.refresh_token.is_none() {
        return Err(anyhow::anyhow!("本地凭证文件中没有 refreshToken"));
    }
    
    Ok(credential)
}

/// 写入本地 Kiro 凭证（用于切换账号）
pub fn write_local_credential(credential: &LocalKiroCredential) -> anyhow::Result<()> {
    let path = get_local_credential_path()
        .ok_or_else(|| anyhow::anyhow!("无法获取用户目录"))?;
    
    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(credential)?;
    std::fs::write(&path, content)?;
    
    Ok(())
}
