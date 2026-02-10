//! 本地缓存封装
//!
//! 基于 moka 的类型安全缓存，支持泛型存取和 per-entry TTL。

use crate::xutil;
use moka::Expiry;
use moka::sync::Cache as MokaCache;
use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 缓存条目（值 + 可选 per-entry TTL）
#[derive(Clone)]
struct CacheEntry {
    value: Arc<dyn Any + Send + Sync>,
    /// 自定义 TTL，None 时使用全局 time_to_live
    custom_ttl: Option<Duration>,
}

/// Per-entry 过期策略
///
/// 有自定义 TTL 时使用它，否则返回 None 使用全局 time_to_live。
struct PerEntryExpiry;

impl Expiry<String, CacheEntry> for PerEntryExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &CacheEntry,
        _current_time: Instant,
    ) -> Option<Duration> {
        value.custom_ttl
    }

    fn expire_after_update(
        &self,
        _key: &String,
        value: &CacheEntry,
        _updated_at: Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        value.custom_ttl
    }
}

/// 本地缓存实例
///
/// 基于 moka::sync::Cache 封装，通过类型擦除支持任意类型的存取。
///
/// ```ignore
/// use x_one::xcache;
///
/// // 通过全局便捷 API 操作默认缓存
/// xcache::set("user:123", "Alice".to_string());
/// let name: Option<String> = xcache::get("user:123");
///
/// // 获取命名缓存实例做更精细操作
/// if let Some(cache) = xcache::c("session") {
///     cache.set_with_ttl("token", "abc", Duration::from_secs(300));
/// }
/// ```
pub struct Cache {
    inner: MokaCache<String, CacheEntry>,
    default_ttl: Duration,
}

impl Cache {
    /// 创建新的缓存实例
    pub fn new(max_capacity: u64, default_ttl: Duration) -> Self {
        let inner = MokaCache::builder()
            .max_capacity(max_capacity)
            .time_to_live(default_ttl)
            .expire_after(PerEntryExpiry)
            .build();

        Self { inner, default_ttl }
    }

    /// 获取缓存值（泛型，通过 downcast 类型转换）
    ///
    /// 如果 key 存在但类型不匹配，返回 None。
    pub fn get<V: Any + Clone + Send + Sync>(&self, key: &str) -> Option<V> {
        let entry = self.inner.get(key)?;
        let result = entry.value.downcast_ref::<V>().cloned();
        if result.is_none() {
            xutil::warn_if_enable_debug(&format!(
                "xcache: type mismatch for key=[{key}], expected=[{}]",
                std::any::type_name::<V>()
            ));
        }
        result
    }

    /// 设置缓存值（使用默认 TTL）
    pub fn set<V: Any + Send + Sync + 'static>(&self, key: &str, value: V) {
        self.inner.insert(
            key.to_string(),
            CacheEntry {
                value: Arc::new(value),
                custom_ttl: None,
            },
        );
    }

    /// 设置缓存值（指定 TTL）
    ///
    /// 通过 moka Expiry trait 实现 per-entry TTL，
    /// 该条目会在指定 TTL 后过期，而非使用全局默认 TTL。
    pub fn set_with_ttl<V: Any + Send + Sync + 'static>(&self, key: &str, value: V, ttl: Duration) {
        self.inner.insert(
            key.to_string(),
            CacheEntry {
                value: Arc::new(value),
                custom_ttl: Some(ttl),
            },
        );
    }

    /// 删除缓存条目
    pub fn del(&self, key: &str) {
        self.inner.invalidate(key);
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
