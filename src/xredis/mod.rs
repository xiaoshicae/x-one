//! xredis - Redis 连接管理模块
//!
//! 基于 redis-rs 封装，提供 Redis 连接管理，
//! 支持单/多实例模式，自动重连。

pub mod client;
pub mod config;
pub mod init;

pub use client::{
    DEFAULT_CLIENT_NAME, get_client_names, redis_client, redis_client_with_name, reset_clients,
    set_client,
};
pub use config::{XREDIS_CONFIG_KEY, XRedisConfig};

use std::sync::atomic::{AtomicBool, Ordering};

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// 注册 xredis 的 before_start 和 before_stop hooks
///
/// 多次调用只注册一次（幂等）。
pub fn register_hook() {
    if REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    crate::before_start!(
        init::init_xredis,
        crate::xhook::HookOptions::new().order(55)
    );

    crate::before_stop!(
        init::shutdown_xredis,
        crate::xhook::HookOptions::new().order(i32::MAX - 55)
    );
}
