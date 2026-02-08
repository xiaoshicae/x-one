//! Hook 选项配置

use std::time::Duration;

/// 默认超时时间（5 秒）
const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// Hook 选项
#[derive(Debug, Clone)]
pub struct HookOptions {
    /// 执行顺序，数字越小越先执行（默认 100）
    pub order: i32,
    /// 执行失败时是否必须返回错误（默认 true）
    pub must_invoke_success: bool,
    /// 执行超时时间（默认 5 秒）
    pub timeout: Duration,
}

impl Default for HookOptions {
    fn default() -> Self {
        Self {
            order: 100,
            must_invoke_success: true,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

impl HookOptions {
    /// 便捷构造：仅设置执行顺序
    pub fn with_order(order: i32) -> Self {
        Self {
            order,
            ..Default::default()
        }
    }

    /// 便捷构造：设置执行顺序和失败是否必须返回错误
    pub fn with_order_and_must(order: i32, must_invoke_success: bool) -> Self {
        Self {
            order,
            must_invoke_success,
            ..Default::default()
        }
    }
}
