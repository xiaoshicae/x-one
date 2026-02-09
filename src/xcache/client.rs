//! 缓存客户端 API

use super::cache::Cache;
use super::init::{DEFAULT_CACHE_NAME, cache_store};
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

/// 获取指定名称的缓存实例
pub fn c(name: &str) -> Option<Arc<Cache>> {
    let store = cache_store().read();
    store.get(name).cloned()
}

/// 获取默认缓存实例
pub fn default() -> Option<Arc<Cache>> {
    c(DEFAULT_CACHE_NAME)
}

/// 设置缓存值（使用默认缓存实例和默认 TTL）
pub fn set<V: Any + Send + Sync + 'static>(key: &str, value: V) {
    if let Some(c) = default() {
        c.set(key, value);
    }
}

/// 设置缓存值（使用默认缓存实例和指定 TTL）
pub fn set_with_ttl<V: Any + Send + Sync + 'static>(key: &str, value: V, ttl: Duration) {
    if let Some(c) = default() {
        c.set_with_ttl(key, value, ttl);
    }
}

/// 获取缓存值（从默认缓存实例）
pub fn get<T: Clone + 'static + Send + Sync>(key: &str) -> Option<T> {
    default().and_then(|c| c.get(key))
}

/// 删除缓存条目（从默认缓存实例）
pub fn del(key: &str) {
    if let Some(c) = default() {
        c.del(key);
    }
}
