//! 日志配置结构体

use serde::{Deserialize, Serialize};

/// 日志配置 key
pub const XLOG_CONFIG_KEY: &str = "XLog";

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct XLogConfig {
    /// 日志级别（默认 "info"）
    #[serde(rename = "Level")]
    pub level: LogLevel,

    /// 日志文件名称（默认 "app"）
    #[serde(rename = "Name")]
    pub name: String,

    /// 日志文件夹路径（默认 "./log"）
    #[serde(rename = "Path")]
    pub path: String,

    /// 日志内容是否在控制台打印（默认 false）
    #[serde(rename = "Console")]
    pub console: bool,

    /// 控制台打印格式是否为原始 JSON 格式（默认 false）
    #[serde(rename = "ConsoleFormatIsRaw")]
    pub console_format_is_raw: bool,

    /// 日志保存最大时间（默认 "7d"）
    #[serde(rename = "MaxAge")]
    pub max_age: String,

    /// 日志切割时长（默认 "1d"）
    #[serde(rename = "RotateTime")]
    pub rotate_time: String,

    /// 日志时间的时区（默认 "Asia/Shanghai"）
    #[serde(rename = "Timezone")]
    pub timezone: String,
}

impl Default for XLogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            name: "app".to_string(),
            path: "./log".to_string(),
            console: false,
            console_format_is_raw: false,
            max_age: "7d".to_string(),
            rotate_time: "1d".to_string(),
            timezone: "Asia/Shanghai".to_string(),
        }
    }
}


