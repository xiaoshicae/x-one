//! xhttp 对外 API
//!
//! 提供全局 HTTP 客户端和便捷请求方法。
//!
//! ```ignore
//! // 使用便捷方法
//! let resp = x_one::xhttp::get("https://api.example.com/users")
//!     .header("X-Token", "abc")
//!     .send()
//!     .await?;
//!
//! // 获取底层 client 做更复杂操作
//! let client = x_one::xhttp::c();
//! ```

use super::config::XHttpConfig;
use crate::xutil;
use std::sync::OnceLock;
use std::time::Duration;

/// 全局 HTTP 客户端
pub(crate) static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 获取全局 HTTP 客户端引用
///
/// 框架初始化后返回按配置构建的 client；
/// 初始化前访问会使用默认配置并输出警告。
pub fn c() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        xutil::warn_if_enable_debug(
            "XHttp client accessed before init, using default configuration",
        );
        reqwest::Client::new()
    })
}

/// 根据配置构建 `reqwest::Client`
///
/// 内部初始化使用，支持 timeout、connect_timeout、keep_alive、连接池等参数。
pub fn build_client(config: &XHttpConfig) -> Result<reqwest::Client, crate::error::XOneError> {
    let timeout = duration_or(&config.timeout, Duration::from_secs(30));
    let connect_timeout = duration_or(&config.dial_timeout, Duration::from_secs(10));
    let keep_alive = duration_or(&config.dial_keep_alive, Duration::from_secs(30));

    let builder = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(connect_timeout)
        .pool_max_idle_per_host(config.pool_max_idle_per_host)
        .tcp_keepalive(keep_alive);

    builder
        .build()
        .map_err(|e| crate::error::XOneError::Other(format!("XHttp build client failed: {e}")))
}

fn duration_or(value: &str, default: Duration) -> Duration {
    xutil::to_duration(value).unwrap_or(default)
}

// ---- 便捷方法，直接使用全局 client 发起请求 ----

/// 发起 GET 请求
pub fn get(url: &str) -> reqwest::RequestBuilder {
    c().get(url)
}

/// 发起 POST 请求
pub fn post(url: &str) -> reqwest::RequestBuilder {
    c().post(url)
}

/// 发起 PUT 请求
pub fn put(url: &str) -> reqwest::RequestBuilder {
    c().put(url)
}

/// 发起 PATCH 请求
pub fn patch(url: &str) -> reqwest::RequestBuilder {
    c().patch(url)
}

/// 发起 DELETE 请求
pub fn delete(url: &str) -> reqwest::RequestBuilder {
    c().delete(url)
}

/// 发起 HEAD 请求
pub fn head(url: &str) -> reqwest::RequestBuilder {
    c().head(url)
}
