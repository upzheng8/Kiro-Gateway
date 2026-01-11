//! Admin API 模块
//!
//! 提供凭证管理和监控功能的 HTTP API
//!
//! # 功能
//! - 查询所有凭证状态
//! - 启用/禁用凭证
//! - 修改凭证优先级
//! - 重置失败计数
//! - 查询凭证余额
//!
//! # 使用
//! ```ignore
//! let admin_service = AdminService::new(token_manager.clone());
//! let admin_state = AdminState::new(admin_api_key, admin_service);
//! let admin_router = create_admin_router(admin_state);
//! ```

mod error;
mod handlers;
pub mod local_account;
mod middleware;
mod router;
mod service;
pub mod types;

pub use middleware::AdminState;
pub use router::create_admin_router;
pub use service::AdminService;
