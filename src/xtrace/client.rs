//! xtrace 对外 API
//!
//! 提供 Trace 启用判断和 Tracer 获取功能。

use std::sync::atomic::{AtomicBool, Ordering};

/// 全局 trace 启用标志
pub(crate) static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

/// 判断 trace 是否启用
pub fn is_trace_enabled() -> bool {
    TRACE_ENABLED.load(Ordering::Acquire)
}
