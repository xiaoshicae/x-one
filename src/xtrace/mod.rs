//! xtrace - 链路追踪模块
//!
//! 基于 OpenTelemetry 实现，提供 TracerProvider 初始化、
//! Tracer 获取、Trace 启用判断等功能。

pub mod config;
pub mod init;

pub use init::is_trace_enabled;
pub use config::XTraceConfig;

use crate::xhook;
use std::sync::atomic::{AtomicBool, Ordering};

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

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

    crate::before_start!(init::init_xtrace, xhook::HookOptions::new().order(3));

    crate::before_stop!(init::shutdown_xtrace, xhook::HookOptions::new().order(1));
}
