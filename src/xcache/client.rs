//! 缓存客户端 API
//!
//! 提供缓存实例获取、缓存值读写等对外功能。

use super::cache::Cache;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// 默认实例名
pub const DEFAULT_CACHE_NAME: &str = "_default_";

/// 全局缓存实例存储
static CACHE_STORE: OnceLock<RwLock<HashMap<String, Arc<Cache>>>> = OnceLock::new();

pub(crate) fn cache_store() -> &'static RwLock<HashMap<String, Arc<Cache>>> {
    CACHE_STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 重置缓存存储（仅测试用）
#[doc(hidden)]
pub fn reset_cache_store() {
    cache_store().write().clear();
}

/// 获取指定名称的缓存实例
pub fn c(name: &str) -> Option<Arc<Cache>> {
    let store = cache_store().read();
    store.get(name).cloned()
}

/// 获取默认缓存实例
pub fn default() -> Option<Arc<Cache>> {
    c(DEFAULT_CACHE_NAME)
}

/// 获取所有缓存实例名称
pub fn get_cache_names() -> Vec<String> {
    let store = cache_store().read();
    store.keys().cloned().collect()
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
