//! API 网关模块
//!
//! 复用现有 `8766` HTTP 服务，以 `/gateway` 前缀承载多协议入站、协议转换、渠道路由
//! 与用量统计。

pub mod affinity;
pub mod canonical;
pub mod commands;
pub mod config;
pub mod executor;
pub mod server;
pub mod translate;
pub mod usage;
