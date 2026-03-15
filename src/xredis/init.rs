//! xredis 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;

use super::client::{DEFAULT_CLIENT_NAME, client_store};
use super::config::{XREDIS_CONFIG_KEY, XRedisConfig, sanitize_for_log};

/// 初始化 XRedis（根据配置创建连接管理器）
///
/// 使用 `ConnectionManager` 自动管理连接，支持自动重连。
pub fn init_xredis() -> Result<(), crate::error::XOneError> {
    if !xconfig::contain_key(XREDIS_CONFIG_KEY) {
        xutil::info_if_enable_debug("XRedis config not found, skip init");
        return Ok(());
    }

    let configs = super::config::load_configs();

    if configs.is_empty() {
        xutil::info_if_enable_debug("XRedis config empty, skip init");
        return Ok(());
    }

    // 使用 tokio block_in_place 来在同步 hook 中执行异步连接
    let rt = tokio::runtime::Handle::try_current().map_err(|e| {
        crate::error::XOneError::Other(format!("XRedis init requires tokio runtime: {e}"))
    })?;

    let mut store = client_store().write();

    for (i, config) in configs.iter().enumerate() {
        if config.addr.is_empty() {
            xutil::warn_if_enable_debug(&format!(
                "XRedis config name=[{}] has empty addr, skip",
                config.name
            ));
            continue;
        }

        let sanitized = sanitize_for_log(config);
        xutil::info_if_enable_debug(&format!(
            "XRedis connecting name=[{}], config={:?}",
            config.name, sanitized
        ));

        let client = build_client(config)?;
        let conn = rt
            .block_on(async { redis::aio::ConnectionManager::new(client).await })
            .map_err(|e| {
                crate::error::XOneError::Other(format!(
                    "XRedis create connection manager failed for [{}]: {e}",
                    config.name
                ))
            })?;

        let name = xutil::default_if_empty(config.name.as_str(), DEFAULT_CLIENT_NAME).to_string();
        store.insert(name.clone(), conn.clone());

        // 第一个实例同时设为默认
        if i == 0 && !config.name.is_empty() {
            store.insert(DEFAULT_CLIENT_NAME.to_string(), conn);
        }
    }

    xutil::info_if_enable_debug(&format!(
        "XRedis init success, client_count=[{}]",
        store.len()
    ));
    Ok(())
}

/// 根据配置构建 Redis Client
fn build_client(config: &XRedisConfig) -> Result<redis::Client, crate::error::XOneError> {
    let url = build_url(config);
    redis::Client::open(url.as_str())
        .map_err(|e| crate::error::XOneError::Other(format!("XRedis open client failed: {e}")))
}

/// 拼装 Redis URL
///
/// 如果 addr 已经是 redis:// 开头则直接使用，否则拼装。
fn build_url(config: &XRedisConfig) -> String {
    if config.addr.starts_with("redis://") || config.addr.starts_with("rediss://") {
        return config.addr.clone();
    }

    let mut url = String::from("redis://");
    if !config.username.is_empty() || !config.password.is_empty() {
        url.push_str(&config.username);
        url.push(':');
        url.push_str(&config.password);
        url.push('@');
    }
    url.push_str(&config.addr);
    url.push('/');
    url.push_str(&config.db.to_string());
    url
}

/// 关闭所有 Redis 连接
///
/// 清空全局 store，ConnectionManager drop 时自动清理。
pub fn shutdown_xredis() -> Result<(), crate::error::XOneError> {
    let mut store = client_store().write();
    let count = store.len();
    store.clear();
    xutil::info_if_enable_debug(&format!("XRedis shutdown, cleared {count} connections"));
    Ok(())
}
