//! xcache - 本地缓存模块
//!
//! 基于 moka 封装，提供泛型缓存 API，支持多实例和 TTL。

pub mod cache;
pub mod client;
pub mod config;
pub mod init;

pub use client::{
    DEFAULT_CACHE_NAME, c, default, del, get, get_cache_names, reset_cache_store, set, set_with_ttl,
};
pub use config::XCacheConfig;

/// 注册缓存初始化和关闭 Hook
pub fn register_hook() {
    crate::before_start!(init::init_xcache, crate::xhook::HookOptions::with_order(6));

    crate::before_stop!(
        init::shutdown_xcache,
        crate::xhook::HookOptions::with_order(2)
    );
}
