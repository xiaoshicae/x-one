//! xtrace - 链路追踪模块
//!
//! 基于 OpenTelemetry 实现，提供 TracerProvider 初始化、
//! Tracer 获取、Trace 启用判断等功能。

pub mod config;
pub mod init;

pub use config::XTraceConfig;
pub use init::{is_trace_enabled, get_tracer};

use crate::xhook;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 默认 shutdown 超时时间（5 秒）
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// 自定义 shutdown 超时
static CUSTOM_SHUTDOWN_TIMEOUT: std::sync::OnceLock<Mutex<Duration>> = std::sync::OnceLock::new();

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

fn shutdown_timeout_store() -> &'static Mutex<Duration> {
    CUSTOM_SHUTDOWN_TIMEOUT
        .get_or_init(|| Mutex::new(Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS)))
}

/// 注册 xtrace 的 before_start 和 before_stop hooks
///
/// 多次调用只注册一次（幂等）。
pub fn register_hook() {
    if REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    crate::before_start!(init::init_xtrace, xhook::HookOptions::with_order(3));

    crate::before_stop!(init::shutdown_xtrace, xhook::HookOptions::with_order(1));
}

/// 设置 shutdown 超时时间
pub fn set_shutdown_timeout(timeout: Duration) {
    if timeout > Duration::ZERO {
        let mut store = shutdown_timeout_store().lock();
        *store = timeout;
    }
}

/// 获取 shutdown 超时时间
pub fn get_shutdown_timeout() -> Duration {
    *shutdown_timeout_store().lock()
}

