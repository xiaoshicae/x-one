//! 统一错误类型

/// x-one 框架统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum XOneError {
    /// Hook 执行错误
    #[error("hook error: {0}")]
    Hook(String),

    /// 配置错误
    #[error("config error: {0}")]
    Config(String),

    /// 日志初始化错误
    #[error("log error: {0}")]
    Log(String),

    /// 服务器运行错误
    #[error("server error: {0}")]
    Server(String),

    /// IO 错误
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// 多个错误合并
    #[error("multiple errors: {}", .0.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))]
    Multi(Vec<XOneError>),

    /// 其他错误
    #[error("{0}")]
    Other(String),
}

impl From<String> for XOneError {
    fn from(s: String) -> Self {
        XOneError::Other(s)
    }
}

/// 便捷的 Result 类型别名
pub type Result<T> = std::result::Result<T, XOneError>;


