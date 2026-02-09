//! xlog - 日志模块
//!
//! 基于 `tracing` + `tracing-subscriber` 实现，
//! 提供 JSON 格式文件日志、控制台彩色输出、异步写入等功能。

pub mod client;
pub mod config;
pub mod console;
pub mod init;
pub mod kv_layer;
pub mod otel_fmt;

pub use client::{get_config, xlog_level};
pub use config::{LogLevel, XLOG_CONFIG_KEY, XLogConfig};
pub use kv_layer::SpanKvFields;

/// 注册日志 Hook
pub fn register_hook() {
    crate::before_start!(init::init_xlog, crate::xhook::HookOptions::new().order(2));
    crate::before_stop!(
        init::shutdown_xlog,
        crate::xhook::HookOptions::new().order(i32::MAX)
    );
}

// ============ KV 注入宏 ============

/// 创建携带 KV 字段的 Span，作用域内所有日志自动携带这些 KV
///
/// 返回一个 `Entered` guard，guard 存活期间所有日志自动包含指定的 KV 字段。
///
/// # Examples
///
/// ```
/// use x_one::{xlog_kv, xlog_info};
///
/// let _guard = xlog_kv!(user_id = "123", request_id = "abc");
/// // 此后的日志自动携带 user_id 和 request_id
/// ```
#[macro_export]
macro_rules! xlog_kv {
    ($($arg:tt)*) => {
        tracing::info_span!("", $($arg)*).entered()
    };
}

// ============ 日志宏 ============

/// INFO 级别日志
#[macro_export]
macro_rules! xlog_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
}

/// ERROR 级别日志
#[macro_export]
macro_rules! xlog_error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*)
    };
}

/// WARN 级别日志
#[macro_export]
macro_rules! xlog_warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*)
    };
}

/// DEBUG 级别日志
#[macro_export]
macro_rules! xlog_debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*)
    };
}
