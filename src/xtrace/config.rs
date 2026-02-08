//! xtrace 配置结构体

use serde::{Deserialize, Serialize};

/// XTrace 配置 key
pub const XTRACE_CONFIG_KEY: &str = "XTrace";

/// XTrace 配置
///
/// # 配置示例
/// ```yaml
/// XTrace:
///   Enable: true
///   Console: false
/// ```
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct XTraceConfig {
    /// 是否启用链路追踪（默认 true，None 视为 true）
    #[serde(rename = "Enable")]
    pub enable: Option<bool>,

    /// 是否输出到控制台（默认 false）
    #[serde(rename = "Console", default)]
    pub console: bool,
}

impl XTraceConfig {
    /// 判断是否启用 trace（None 视为 true）
    pub fn is_enabled(&self) -> bool {
        self.enable.unwrap_or(true)
    }
}
