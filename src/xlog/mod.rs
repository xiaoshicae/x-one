//! xlog - 日志模块
//!
//! 基于 `tracing` + `tracing-subscriber` 实现，
//! 提供 JSON 格式文件日志、控制台彩色输出、异步写入等功能。

pub mod async_writer;
pub mod client;
pub mod config;
pub mod console;
pub mod init;
pub mod otel_fmt;

pub use client::{get_config, xlog_level};
pub use config::{LogLevel, XLOG_CONFIG_KEY, XLogConfig};

/// 注册日志初始化 Hook
pub fn register_hook() {
    crate::before_start!(init::init_xlog, crate::xhook::HookOptions::with_order(2));
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
