//! xhttp 初始化逻辑

use crate::xconfig;
use crate::xutil;
use std::time::Duration;

use super::config::{XHTTP_CONFIG_KEY, XHttpConfig};
use super::client::HTTP_CLIENT;

/// 初始化 HTTP 客户端
pub fn init_xhttp() -> Result<(), crate::error::XOneError> {
    if !xconfig::contain_key(XHTTP_CONFIG_KEY) {
        xutil::info_if_enable_debug("XHttp config not found, skip init");
        return Ok(());
    }

    let config = load_config();
    let http_client = build_client(&config)?;

    // OnceLock::set 如果已初始化则忽略
    let _ = HTTP_CLIENT.set(http_client);

    xutil::info_if_enable_debug(&format!(
        "XHttp init success, timeout=[{}], retry_count=[{}]",
        config.timeout, config.retry_count
    ));
    Ok(())
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

fn duration_or(value: &str, default: Duration) -> Duration {
    xutil::to_duration(value).unwrap_or(default)
}

/// 加载 XHttp 配置
pub fn load_config() -> XHttpConfig {
    xconfig::parse_config::<XHttpConfig>(XHTTP_CONFIG_KEY).unwrap_or_default()
}