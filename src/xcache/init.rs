//! xcache 初始化和关闭逻辑

use crate::xconfig;
use crate::xutil;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use super::cache::Cache;
use super::config::{XCACHE_CONFIG_KEY, XCacheConfig};

/// 默认实例名
pub const DEFAULT_CACHE_NAME: &str = "_default_";

/// 全局缓存实例存储
static CACHE_STORE: OnceLock<RwLock<HashMap<String, Arc<Cache>>>> = OnceLock::new();

pub fn cache_store() -> &'static RwLock<HashMap<String, Arc<Cache>>> {
    CACHE_STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 初始化 xcache
pub fn init_xcache() -> Result<(), crate::error::XOneError> {
    let configs = load_configs();

    if configs.is_empty() {
        xutil::info_if_enable_debug("XCache no config found, using default");
        // 创建默认缓存实例
        let default_config = XCacheConfig::default();
        create_cache_instance(&default_config)?;
        return Ok(());
    }

    for config in &configs {
        create_cache_instance(config)?;
    }

    let store = cache_store().read();
    xutil::info_if_enable_debug(&format!(
        "XCache init success, cache_count=[{}]",
        store.len()
    ));
    Ok(())
}

/// 创建缓存实例
pub fn create_cache_instance(config: &XCacheConfig) -> Result<(), crate::error::XOneError> {
    let name = if config.name.is_empty() {
        DEFAULT_CACHE_NAME.to_string()
    } else {
        config.name.clone()
    };

    let max_capacity = if config.max_capacity > 0 {
        config.max_capacity
    } else {
        100_000
    };

    let default_ttl = xutil::to_duration(&config.default_ttl)
        .unwrap_or_else(|_| std::time::Duration::from_secs(300));

    let cache_instance = Arc::new(Cache::new(max_capacity, default_ttl));

    let mut store = cache_store().write();
    store.insert(name.clone(), cache_instance);

    xutil::info_if_enable_debug(&format!(
        "XCache created instance name=[{}], max_capacity=[{}], default_ttl=[{:?}]",
        name, max_capacity, default_ttl
    ));

    Ok(())
}

/// 关闭 xcache
pub fn shutdown_xcache() -> Result<(), crate::error::XOneError> {
    let mut store = cache_store().write();
    let count = store.len();
    store.clear();
    xutil::info_if_enable_debug(&format!("XCache shutdown, cleared {count} cache instances"));
    Ok(())
}

/// 获取所有缓存实例名称
pub fn get_cache_names() -> Vec<String> {
    let store = cache_store().read();
    store.keys().cloned().collect()
}

/// 加载 XCache 配置
pub fn load_configs() -> Vec<XCacheConfig> {
    // 先尝试加载为单个配置
    if let Ok(config) = xconfig::parse_config::<XCacheConfig>(XCACHE_CONFIG_KEY) {
        return vec![config];
    }

    // 再尝试加载为配置数组
    if let Ok(configs) = xconfig::parse_config::<Vec<XCacheConfig>>(XCACHE_CONFIG_KEY) {
        return configs;
    }

    Vec::new()
}
