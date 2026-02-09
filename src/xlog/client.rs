//! xlog 对外 API
//!
//! 提供日志级别查询和配置获取功能。

use super::config::{XLOG_CONFIG_KEY, XLogConfig};

/// 获取当前日志级别
pub fn xlog_level() -> String {
    crate::xutil::take_or_default(
        crate::xconfig::get_string(&format!("{XLOG_CONFIG_KEY}.Level")),
        "info",
    )
}

/// 从配置中获取日志配置
pub fn get_config() -> XLogConfig {
    crate::xconfig::parse_config::<XLogConfig>(XLOG_CONFIG_KEY).unwrap_or_default()
}
