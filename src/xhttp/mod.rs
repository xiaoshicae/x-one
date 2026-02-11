//! xhttp - HTTP 客户端模块
//!
//! 基于 reqwest 封装，提供全局 HTTP 客户端和便捷请求方法。

pub mod client;
pub mod config;
pub mod init;

pub use client::{build_client, c, delete, get, head, patch, post, put};
pub use config::XHttpConfig;

use std::sync::atomic::{AtomicBool, Ordering};

/// 幂等注册标志
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// 注册 HTTP 客户端 Hook（幂等，多次调用只注册一次）
///
/// xhttp 仅注册 before_start，无需 before_stop（reqwest Client 可直接 drop）。
pub fn register_hook() {
    if REGISTERED
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    crate::before_start!(init::init_xhttp, crate::xhook::HookOptions::new().order(40));
}
