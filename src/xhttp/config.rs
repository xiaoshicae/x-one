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
///   MaxIdleConnsPerHost: 100
///   PoolMaxIdlePerHost: 10
///   RetryCount: 3
///   RetryWaitTime: "1s"
///   RetryMaxWaitTime: "10s"
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

    /// 每个主机最大空闲连接数（默认 100）
    #[serde(rename = "MaxIdleConnsPerHost", default = "default_max_idle_conns")]
    pub max_idle_conns_per_host: usize,

    /// 连接池每个主机最大空闲数（默认 10）
    #[serde(rename = "PoolMaxIdlePerHost", default = "default_pool_max_idle")]
    pub pool_max_idle_per_host: usize,

    /// 重试次数（默认 0，不重试）
    #[serde(rename = "RetryCount", default)]
    pub retry_count: usize,

    /// 重试等待时间（duration 字符串，默认 "1s"）
    #[serde(rename = "RetryWaitTime", default = "default_retry_wait_time")]
    pub retry_wait_time: String,

    /// 重试最大等待时间（duration 字符串，默认 "10s"）
    #[serde(rename = "RetryMaxWaitTime", default = "default_retry_max_wait_time")]
    pub retry_max_wait_time: String,
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
fn default_max_idle_conns() -> usize {
    100
}
fn default_pool_max_idle() -> usize {
    10
}
fn default_retry_wait_time() -> String {
    "1s".to_string()
}
fn default_retry_max_wait_time() -> String {
    "10s".to_string()
}

impl Default for XHttpConfig {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            dial_timeout: default_dial_timeout(),
            dial_keep_alive: default_dial_keep_alive(),
            max_idle_conns_per_host: default_max_idle_conns(),
            pool_max_idle_per_host: default_pool_max_idle(),
            retry_count: 0,
            retry_wait_time: default_retry_wait_time(),
            retry_max_wait_time: default_retry_max_wait_time(),
        }
    }
}


