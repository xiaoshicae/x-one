//! xhttp 对外 API
//!
//! 提供全局 HTTP 客户端和便捷请求方法。

use super::config::{XHTTP_CONFIG_KEY, XHttpConfig};
use crate::xutil;
use std::sync::OnceLock;
use std::time::Duration;

/// 全局 HTTP 客户端
pub(crate) static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 获取全局 HTTP 客户端
pub fn c() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        xutil::warn_if_enable_debug(
            "XHttp client accessed before init, using default configuration",
        );
        reqwest::Client::new()
    })
}

/// 根据配置构建 reqwest::Client
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

/// 加载 XHttp 配置
pub fn load_config() -> XHttpConfig {
    crate::xconfig::parse_config::<XHttpConfig>(XHTTP_CONFIG_KEY).unwrap_or_default()
}

fn duration_or(value: &str, default: Duration) -> Duration {
    xutil::to_duration(value).unwrap_or(default)
}

// 便捷方法，直接使用全局 client 发起请求

pub fn get(url: &str) -> reqwest::RequestBuilder {
    c().get(url)
}

pub fn post(url: &str) -> reqwest::RequestBuilder {
    c().post(url)
}

pub fn put(url: &str) -> reqwest::RequestBuilder {
    c().put(url)
}

pub fn patch(url: &str) -> reqwest::RequestBuilder {
    c().patch(url)
}

pub fn delete(url: &str) -> reqwest::RequestBuilder {
    c().delete(url)
}

pub fn head(url: &str) -> reqwest::RequestBuilder {
    c().head(url)
}
