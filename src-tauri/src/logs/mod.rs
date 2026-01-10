//! 日志收集模块
//! 
//! 用于收集应用日志并通过 API 提供给 Admin UI

use std::sync::{Arc, RwLock};
use std::collections::VecDeque;
use chrono::Local;
use serde::Serialize;

/// 单条日志记录
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// 时间戳 (HH:MM:SS)
    pub timestamp: String,
    /// 日志级别
    pub level: String,
    /// 日志消息
    pub message: String,
    /// 请求详情（可选）
    pub request: Option<RequestInfo>,
    /// 响应详情（可选）
    pub response: Option<ResponseInfo>,
}

/// 请求信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestInfo {
    pub model: String,
    pub max_tokens: i32,
    pub stream: bool,
    pub message_count: usize,
    pub system_preview: String,
    pub user_message_preview: String,
}

/// 响应信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseInfo {
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub stop_reason: String,
    pub has_tool_use: bool,
    pub response_preview: String,
}

/// 日志收集器
pub struct LogCollector {
    logs: RwLock<VecDeque<LogEntry>>,
    max_size: usize,
}

impl LogCollector {
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    /// 添加日志
    pub fn add_log(&self, level: &str, message: &str) {
        let entry = LogEntry {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
            request: None,
            response: None,
        };
        self.push_entry(entry);
    }

    /// 添加请求日志
    pub fn add_request_log(&self, request: RequestInfo) {
        let entry = LogEntry {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            level: "INFO".to_string(),
            message: format!("📨 收到请求: {} ({}条消息)", request.model, request.message_count),
            request: Some(request),
            response: None,
        };
        self.push_entry(entry);
    }

    /// 添加响应日志
    pub fn add_response_log(&self, response: ResponseInfo, is_stream: bool) {
        let entry = LogEntry {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            level: "INFO".to_string(),
            message: format!("📤 {}响应完成: {} (输入:{}, 输出:{})", 
                if is_stream { "流式" } else { "同步" },
                response.model,
                response.input_tokens,
                response.output_tokens
            ),
            request: None,
            response: Some(response),
        };
        self.push_entry(entry);
    }

    fn push_entry(&self, entry: LogEntry) {
        let mut logs = self.logs.write().unwrap();
        if logs.len() >= self.max_size {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    /// 获取所有日志
    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.read().unwrap().iter().cloned().collect()
    }

    /// 获取指定索引之后的日志
    pub fn get_logs_since(&self, since_index: usize) -> Vec<LogEntry> {
        let logs = self.logs.read().unwrap();
        if since_index >= logs.len() {
            return Vec::new();
        }
        logs.iter().skip(since_index).cloned().collect()
    }

    /// 获取日志总数
    pub fn len(&self) -> usize {
        self.logs.read().unwrap().len()
    }

    /// 清空日志
    pub fn clear(&self) {
        self.logs.write().unwrap().clear();
    }
}

// 全局日志收集器
lazy_static::lazy_static! {
    pub static ref LOG_COLLECTOR: Arc<LogCollector> = Arc::new(LogCollector::new(500));
}

/// 安全截取字符串
pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
