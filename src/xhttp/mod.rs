//! xhttp - HTTP 客户端模块
//!
//! 基于 reqwest 封装，提供全局 HTTP 客户端和便捷请求方法。

pub mod client;
pub mod config;
pub mod init;

pub use client::{build_client, c, delete, get, head, load_config, patch, post, put};
pub use config::XHttpConfig;

pub fn register_hook() {
    crate::before_start!(init::init_xhttp, crate::xhook::HookOptions::new().order(4));
}
