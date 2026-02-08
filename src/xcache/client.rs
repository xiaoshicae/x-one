use super::cache::Cache;
use super::init::{DEFAULT_CACHE_NAME, cache_store};
use std::sync::Arc;

/// 获取指定名称的缓存实例
pub fn c(name: &str) -> Option<Arc<Cache>> {
    let store = cache_store().read();
    store.get(name).cloned()
}

/// 获取默认缓存实例
pub fn default() -> Option<Arc<Cache>> {
    c(DEFAULT_CACHE_NAME)
}
