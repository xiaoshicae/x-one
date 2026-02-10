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

use std::sync::atomic::{AtomicBool, Ordering};

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// 注册缓存初始化和关闭 Hook（幂等，多次调用只注册一次）
pub fn register_hook() {
    if REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    crate::before_start!(
        init::init_xcache,
        crate::xhook::HookOptions::new().order(60)
    );

    crate::before_stop!(
        init::shutdown_xcache,
        crate::xhook::HookOptions::new().order(i32::MAX - 60)
    );
}
