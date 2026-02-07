//! Cache 封装结构体

use moka::sync::Cache as MokaCache;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

/// 缓存值类型（类型擦除，Arc 实现 Clone）
type CacheValue = Arc<dyn Any + Send + Sync>;

/// Cache 封装
///
/// 包装 moka::sync::Cache，提供泛型 API。
pub struct Cache {
    inner: MokaCache<String, CacheValue>,
    default_ttl: Duration,
}

impl Cache {
    /// 创建新的缓存实例
    pub fn new(max_capacity: u64, default_ttl: Duration) -> Self {
        let inner = MokaCache::builder()
            .max_capacity(max_capacity)
            .time_to_live(default_ttl)
            .build();

        Self { inner, default_ttl }
    }

    /// 获取缓存值（泛型，通过 downcast 类型转换）
    pub fn get<V: Any + Clone + Send + Sync>(&self, key: &str) -> Option<V> {
        self.inner
            .get(&key.to_string())
            .and_then(|v| v.downcast_ref::<V>().cloned())
    }

    /// 设置缓存值（使用默认 TTL）
    pub fn set<V: Any + Send + Sync + 'static>(&self, key: &str, value: V) {
        self.inner
            .insert(key.to_string(), Arc::new(value) as CacheValue);
    }

    /// 设置缓存值（指定 TTL）
    ///
    /// 注意：moka 0.12 的 per-entry TTL 需要通过 Expiry trait 实现，
    /// 当前简化为使用全局 TTL。后续可按需扩展。
    pub fn set_with_ttl<V: Any + Send + Sync + 'static>(
        &self,
        key: &str,
        value: V,
        _ttl: Duration,
    ) {
        // TODO: 支持 per-entry TTL
        self.inner
            .insert(key.to_string(), Arc::new(value) as CacheValue);
    }

    /// 删除缓存条目
    pub fn del(&self, key: &str) {
        self.inner.invalidate(&key.to_string());
    }

    /// 获取缓存条目数
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// 获取默认 TTL
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("entry_count", &self.inner.entry_count())
            .field("default_ttl", &self.default_ttl)
            .finish()
    }
}
