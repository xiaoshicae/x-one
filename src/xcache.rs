pub mod cache;
pub mod client;
pub mod config;
pub mod init;

use std::any::Any;

pub use client::{c, default};
pub use config::XCacheConfig;
pub use init::{
    cache_store, create_cache_instance, get_cache_names, init_xcache, load_configs, shutdown_xcache,
};

// 兼容旧 API
pub fn set<V: Any + Send + Sync + 'static>(key: &str, value: V) {
    if let Some(c) = default() {
        c.set(key, value);
    }
}

pub fn set_with_ttl<V: Any + Send + Sync + 'static>(key: &str, value: V, ttl: std::time::Duration) {
    if let Some(c) = default() {
        c.set_with_ttl(key, value, ttl);
    }
}

pub fn get<T: Clone + 'static + Send + Sync>(key: &str) -> Option<T> {
    default().and_then(|c| c.get(key))
}

pub fn del(key: &str) {
    if let Some(c) = default() {
        c.del(key);
    }
}

pub fn register_hook() {
    crate::before_start!(init::init_xcache, crate::xhook::HookOptions::with_order(6));

    crate::before_stop!(
        init::shutdown_xcache,
        crate::xhook::HookOptions::with_order(2)
    );
}
