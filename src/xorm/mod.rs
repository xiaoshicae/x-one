//! xorm - 数据库连接管理模块
//!
//! 基于 sqlx 封装，提供数据库连接池管理，
//! 支持 PostgreSQL/MySQL，单/多实例模式。

pub mod client;
pub mod config;
pub mod init;

pub use client::{
    DEFAULT_POOL_NAME, PoolEntry, get_driver, get_dsn, get_pool_config, get_pool_names,
    pool_configs_store,
};
pub use config::{Driver, XORM_CONFIG_KEY, XOrmConfig};

use std::sync::atomic::{AtomicBool, Ordering};

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// 注册 xorm 的 before_start 和 before_stop hooks
///
/// 多次调用只注册一次（幂等）。
pub fn register_hook() {
    if REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    crate::before_start!(init::init_xorm, crate::xhook::HookOptions::with_order(5));

    crate::before_stop!(
        init::shutdown_xorm,
        crate::xhook::HookOptions::with_order(3)
    );
}
