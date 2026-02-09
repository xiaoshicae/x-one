//! x-one: Rust 微服务框架
//!
//! 从 Go 版 xone 框架移植而来，提供配置管理、日志、Hook 生命周期、
//! HTTP 服务、链路追踪、数据库连接管理、本地缓存等功能。

pub mod error;
pub mod xaxum;
pub mod xcache;
pub mod xconfig;
pub mod xhook;
pub mod xhttp;
pub mod xlog;
pub mod xorm;
pub mod xserver;
pub mod xtrace;
pub mod xutil;

pub use error::{Result, XOneError};
#[allow(deprecated)]
pub use xaxum::{AxumOptions, AxumServer, AxumTlsServer};
#[allow(deprecated)]
pub use xaxum::{run_axum, run_axum_tls, run_axum_with_options};
pub use xserver::Server;
pub use xserver::blocking::BlockingServer;
pub use xserver::{run_blocking_server, run_server};

/// 初始化所有模块
///
/// 注册内置模块 hook 并执行 before_start hooks。
/// 通常由框架自动调用，也可手动调用确保初始化完成。
pub fn init_all() -> Result<()> {
    xserver::ensure_init()
}

#[ctor::ctor]
fn auto_init() {
    if let Err(e) = xserver::ensure_init() {
        eprintln!("x-one auto init failed: {e}");
    }
}
