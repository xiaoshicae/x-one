//! xmetric - Prometheus 指标采集模块
//!
//! 基于 prometheus-client 封装，提供 Counter/Gauge/Histogram 便捷 API，
//! 支持命名空间和全局常量标签。
//!
//! # Examples
//!
//! ```ignore
//! use x_one::xmetric;
//!
//! // 递增计数器
//! xmetric::counter_inc("http_requests_total", &[("method", "GET"), ("path", "/api")]);
//!
//! // 设置仪表盘
//! xmetric::gauge_set("active_connections", 42.0, &[]);
//!
//! // 观测直方图
//! xmetric::histogram_observe("request_duration_ms", 123.5, &[("method", "POST")]);
//! ```

pub mod client;
pub mod config;
pub mod init;

pub use client::{
    counter_add, counter_inc, gauge_dec, gauge_inc, gauge_set, histogram_observe, registry,
    reset_metrics,
};
pub use config::{XMETRIC_CONFIG_KEY, XMetricConfig};

use std::sync::atomic::{AtomicBool, Ordering};

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// 注册 xmetric 的 before_start 和 before_stop hooks
///
/// 多次调用只注册一次（幂等）。
pub fn register_hook() {
    if REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    crate::before_start!(
        init::init_xmetric,
        crate::xhook::HookOptions::new().order(45)
    );

    crate::before_stop!(
        init::shutdown_xmetric,
        crate::xhook::HookOptions::new().order(i32::MAX - 45)
    );
}
