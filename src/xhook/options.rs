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
    /// 创建默认选项（order=100, must_invoke_success=true, timeout=5s）
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置执行顺序（数字越小越先执行）
    pub fn order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    /// 设置失败时是否必须返回错误
    pub fn must_success(mut self, must: bool) -> Self {
        self.must_invoke_success = must;
        self
    }

    /// 设置执行超时时间
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
