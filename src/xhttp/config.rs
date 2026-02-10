//! xhttp 配置结构体

use serde::Deserialize;

/// XHttp 配置 key
pub const XHTTP_CONFIG_KEY: &str = "XHttp";

/// XHttp 配置
///
/// # 配置示例
/// ```yaml
/// XHttp:
///   Timeout: "30s"
///   DialTimeout: "10s"
///   DialKeepAlive: "30s"
///   PoolMaxIdlePerHost: 10
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct XHttpConfig {
    /// 请求超时（duration 字符串，默认 "30s"）
    #[serde(rename = "Timeout", default = "default_timeout")]
    pub timeout: String,

    /// 连接超时（duration 字符串，默认 "10s"）
    #[serde(rename = "DialTimeout", default = "default_dial_timeout")]
    pub dial_timeout: String,

    /// Keep-alive 时间（duration 字符串，默认 "30s"）
    #[serde(rename = "DialKeepAlive", default = "default_dial_keep_alive")]
    pub dial_keep_alive: String,

    /// 连接池每个主机最大空闲数（默认 10）
    #[serde(rename = "PoolMaxIdlePerHost", default = "default_pool_max_idle")]
    pub pool_max_idle_per_host: usize,
}

fn default_timeout() -> String {
    "30s".to_string()
}
fn default_dial_timeout() -> String {
    "10s".to_string()
}
fn default_dial_keep_alive() -> String {
    "30s".to_string()
}
fn default_pool_max_idle() -> usize {
    10
}

/// 加载 XHttp 配置
pub(crate) fn load_config() -> XHttpConfig {
    crate::xconfig::parse_config::<XHttpConfig>(XHTTP_CONFIG_KEY).unwrap_or_default()
}

impl Default for XHttpConfig {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            dial_timeout: default_dial_timeout(),
            dial_keep_alive: default_dial_keep_alive(),
            pool_max_idle_per_host: default_pool_max_idle(),
        }
    }
}
