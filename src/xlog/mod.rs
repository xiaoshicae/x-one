//! xlog - 日志模块
//!
//! 基于 `tracing` + `tracing-subscriber` 实现，
//! 提供 JSON 格式文件日志、控制台彩色输出、异步写入等功能。

pub mod async_writer;
pub mod config;
pub mod console;
pub mod init;

pub use config::{LogLevel, XLOG_CONFIG_KEY, XLogConfig};

/// 注册日志初始化 Hook
pub fn register_hook() {
    crate::xhook::before_start(
        "xlog::init",
        init::init_xlog,
        crate::xhook::HookOptions::with_order(100), // 默认顺序，在 xconfig 之后
    );
}

/// 获取当前日志级别
pub fn xlog_level() -> String {
    crate::xutil::take_or_default(
        crate::xconfig::get_string(&format!("{XLOG_CONFIG_KEY}.Level")),
        "info",
    )
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
