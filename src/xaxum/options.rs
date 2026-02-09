//! Axum 服务器选项
//!
//! 通过 `AxumOptions` 控制中间件的启用/禁用。

/// Axum 服务器选项
///
/// 控制 HTTP 服务器中间件的开关，默认全部启用。
///
/// # Examples
///
/// ```
/// use x_one::xaxum::options::AxumOptions;
///
/// // 默认：所有中间件启用
/// let opts = AxumOptions::default();
/// assert!(opts.enable_log_middleware);
/// assert!(opts.enable_trace_middleware);
///
/// // 禁用日志中间件
/// let opts = AxumOptions::new().with_log_middleware(false);
/// assert!(!opts.enable_log_middleware);
/// assert!(opts.enable_trace_middleware);
/// ```
#[derive(Debug, Clone)]
pub struct AxumOptions {
    /// 是否启用日志中间件（默认 true）
    pub enable_log_middleware: bool,
    /// 是否启用链路追踪中间件（默认 true）
    pub enable_trace_middleware: bool,
}

impl Default for AxumOptions {
    fn default() -> Self {
        Self {
            enable_log_middleware: true,
            enable_trace_middleware: true,
        }
    }
}

impl AxumOptions {
    /// 创建默认选项（所有中间件启用）
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否启用日志中间件
    pub fn with_log_middleware(mut self, enable: bool) -> Self {
        self.enable_log_middleware = enable;
        self
    }

    /// 设置是否启用链路追踪中间件
    pub fn with_trace_middleware(mut self, enable: bool) -> Self {
        self.enable_trace_middleware = enable;
        self
    }
}
