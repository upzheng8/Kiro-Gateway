use serde::{Deserialize, Serialize, Deserializer};
use std::fs;
use std::path::Path;

/// 机器码备份信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineIdBackup {
    pub machine_id: String,
    pub backup_time: String,
}

// 自定义反序列化：支持旧的字符串格式和新的结构体格式
impl<'de> Deserialize<'de> for MachineIdBackup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct MachineIdBackupVisitor;

        impl<'de> Visitor<'de> for MachineIdBackupVisitor {
            type Value = MachineIdBackup;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a struct with machineId and backupTime")
            }

            // 旧格式：直接是字符串
            fn visit_str<E>(self, value: &str) -> Result<MachineIdBackup, E>
            where
                E: de::Error,
            {
                Ok(MachineIdBackup {
                    machine_id: value.to_string(),
                    backup_time: "未知".to_string(),
                })
            }

            // 新格式：结构体
            fn visit_map<M>(self, mut map: M) -> Result<MachineIdBackup, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut machine_id = None;
                let mut backup_time = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "machineId" => machine_id = Some(map.next_value()?),
                        "backupTime" => backup_time = Some(map.next_value()?),
                        _ => { let _ = map.next_value::<serde_json::Value>(); }
                    }
                }

                Ok(MachineIdBackup {
                    machine_id: machine_id.ok_or_else(|| de::Error::missing_field("machineId"))?,
                    backup_time: backup_time.unwrap_or_else(|| "未知".to_string()),
                })
            }
        }

        deserializer.deserialize_any(MachineIdBackupVisitor)
    }
}

/// KNA 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    /// 反代服务独立端口（可选，默认 8991）
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,

    #[serde(default = "default_region")]
    pub region: String,

    #[serde(default = "default_kiro_version")]
    pub kiro_version: String,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default = "default_system_version")]
    pub system_version: String,

    #[serde(default = "default_node_version")]
    pub node_version: String,

    /// 锁定的模型名称（可选，仅影响客户端操作）
    #[serde(default)]
    pub locked_model: Option<String>,

    /// 机器码备份（可选，用于恢复）
    #[serde(default)]
    pub machine_id_backup: Option<MachineIdBackup>,

    /// 分组列表（id -> 名称映射）
    #[serde(default = "default_groups")]
    pub groups: Vec<GroupConfig>,

    /// 反代使用的分组 ID（为空表示使用所有分组）
    #[serde(default)]
    pub active_group_id: Option<String>,

    /// 反代服务是否自动启动
    #[serde(default)]
    pub proxy_auto_start: bool,

    /// 是否启用自动刷新 Token
    #[serde(default)]
    pub auto_refresh_enabled: bool,

    /// 自动刷新间隔（分钟），默认 10 分钟
    #[serde(default = "default_auto_refresh_interval")]
    pub auto_refresh_interval_minutes: u32,
}

/// 分组配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupConfig {
    pub id: String,
    pub name: String,
}

fn default_groups() -> Vec<GroupConfig> {
    vec![GroupConfig {
        id: "default".to_string(),
        name: "默认分组".to_string(),
    }]
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8990
}

fn default_proxy_port() -> u16 {
    8991
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_kiro_version() -> String {
    "0.8.0".to_string()
}

fn default_system_version() -> String {
    const SYSTEM_VERSIONS: &[&str] = &["darwin#24.6.0", "win32#10.0.22631"];
    SYSTEM_VERSIONS[fastrand::usize(..SYSTEM_VERSIONS.len())].to_string()
}

fn default_node_version() -> String {
    "22.21.1".to_string()
}

fn default_auto_refresh_interval() -> u32 {
    10 // 默认 10 分钟
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            proxy_port: default_proxy_port(),
            region: default_region(),
            kiro_version: default_kiro_version(),
            api_key: None,
            system_version: default_system_version(),
            node_version: default_node_version(),
            locked_model: None,
            machine_id_backup: None,
            groups: default_groups(),
            active_group_id: None,
            proxy_auto_start: false,
            auto_refresh_enabled: false,
            auto_refresh_interval_minutes: default_auto_refresh_interval(),
        }
    }
}

impl Config {
    /// 获取默认配置文件路径
    pub fn default_config_path() -> &'static str {
        "config.json"
    }

    /// 从文件加载配置
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            // 配置文件不存在，返回默认配置
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 从文件加载配置，如果不存在则创建默认配置文件
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let default_config = Self::default();
            default_config.save(path)?;
            tracing::info!("已创建默认配置文件: {:?}", path);
            return Ok(default_config);
        }
        Self::load(path)
    }

    /// 保存配置到文件
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
