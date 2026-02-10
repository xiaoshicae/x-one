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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct XTraceConfig {
    /// 是否启用链路追踪（默认 true）
    #[serde(rename = "Enable", default = "default_enable")]
    pub enable: bool,

    /// 是否输出到控制台（默认 false）
    #[serde(rename = "Console", default)]
    pub console: bool,
}

fn default_enable() -> bool {
    true
}

impl Default for XTraceConfig {
    fn default() -> Self {
        Self {
            enable: default_enable(),
            console: false,
        }
    }
}

impl XTraceConfig {
    /// 判断是否启用 trace
    pub fn is_enabled(&self) -> bool {
        self.enable
    }
}

/// 加载 XTrace 配置
pub(crate) fn load_config() -> XTraceConfig {
    crate::xconfig::parse_config::<XTraceConfig>(XTRACE_CONFIG_KEY).unwrap_or_default()
}
