//! xhttp 配置结构体
//!
//! 对应 `application.yml` 中的 `XHttp` 节点。
//!
//! ```yaml
//! XHttp:
//!   Timeout: "30s"
//!   DialTimeout: "10s"
//!   PoolMaxIdlePerHost: 10
//! ```

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
#[serde(default)]
pub struct XHttpConfig {
    /// 请求超时（duration 字符串，默认 "30s"）
    #[serde(rename = "Timeout")]
    pub timeout: String,

    /// 连接超时（duration 字符串，默认 "10s"）
    #[serde(rename = "DialTimeout")]
    pub dial_timeout: String,

    /// Keep-alive 时间（duration 字符串，默认 "30s"）
    #[serde(rename = "DialKeepAlive")]
    pub dial_keep_alive: String,

    /// 连接池每个主机最大空闲数（默认 10）
    #[serde(rename = "PoolMaxIdlePerHost")]
    pub pool_max_idle_per_host: usize,
}

impl Default for XHttpConfig {
    fn default() -> Self {
        Self {
            timeout: "30s".into(),
            dial_timeout: "10s".into(),
            dial_keep_alive: "30s".into(),
            pool_max_idle_per_host: 10,
        }
    }
}

/// 加载 XHttp 配置
pub(crate) fn load_config() -> XHttpConfig {
    crate::xconfig::parse_config::<XHttpConfig>(XHTTP_CONFIG_KEY).unwrap_or_default()
}
