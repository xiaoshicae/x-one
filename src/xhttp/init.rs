//! xhttp 初始化逻辑

use crate::xconfig;
use crate::xutil;

use super::client::{HTTP_CLIENT, build_client, load_config};
use super::config::XHTTP_CONFIG_KEY;

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

    xutil::info_if_enable_debug(&format!("XHttp init success, timeout=[{}]", config.timeout));
    Ok(())
}
