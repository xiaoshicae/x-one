//! xorm - 数据库连接管理模块
//!
//! 基于 sqlx 封装，提供数据库连接池管理，
//! 支持 PostgreSQL/MySQL，单/多实例模式。

pub mod config;
pub mod init;

pub use config::{Driver, XOrmConfig, XORM_CONFIG_KEY};
pub use init::{get_driver, get_dsn, get_pool_config, get_pool_names};

use crate::xhook;
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

    xhook::before_start(
        "xorm::init",
        init::init_xorm,
        xhook::HookOptions::with_order(5),
    );

    xhook::before_stop(
        "xorm::shutdown",
        init::shutdown_xorm,
        xhook::HookOptions::with_order(3),
    );
}
