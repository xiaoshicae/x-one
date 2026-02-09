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
pub use xaxum::{AxumServer, AxumTlsServer};
pub use xserver::Server;
pub use xserver::blocking::BlockingServer;

use std::sync::OnceLock;

static AUTO_INIT: OnceLock<()> = OnceLock::new();

fn ensure_init() {
    AUTO_INIT.get_or_init(init_all_inner);
}

/// 初始化所有模块
///
/// 自动注册模块并执行 before_start hooks：
/// 1. xconfig（order=1）：加载配置文件
/// 2. xlog（order=2）：设置日志系统（配置存在才执行）
/// 3. xtrace（order=3）：链路追踪（配置存在才执行）
/// 4. xhttp（order=4）：HTTP 客户端（配置存在才执行）
/// 5. xorm（order=5）：数据库连接池（配置存在才执行）
/// 6. xcache（order=6）：本地缓存（配置存在才执行）
/// 7. 用户注册的自定义 hooks
pub fn init_all() {
    ensure_init();
}

fn init_all_inner() {
    xconfig::register_hook();
    xlog::register_hook();
    xtrace::register_hook();
    xhttp::register_hook();
    xorm::register_hook();
    xcache::register_hook();

    if let Err(e) = xhook::invoke_before_start_hooks() {
        xutil::error_if_enable_debug(&format!("XOne init_all hook error: {}", e));
    }
}

#[ctor::ctor]
fn auto_init() {
    ensure_init();
}

/// 以 Axum HTTP 服务器运行
///
/// 初始化所有模块后，创建 AxumServer 并以异步方式运行，
/// 阻塞等待退出信号（SIGINT/SIGTERM）。
pub async fn run_axum(router: axum::Router) -> Result<()> {
    ensure_init();
    let server = AxumServer::new(router);
    xserver::run_with_server(&server).await
}

/// 以 Axum HTTPS 服务器运行
pub async fn run_axum_tls(router: axum::Router, cert_file: &str, key_file: &str) -> Result<()> {
    ensure_init();
    let server = AxumTlsServer::new(router, cert_file, key_file);
    xserver::run_with_server(&server).await
}

/// 以阻塞式服务器运行
///
/// 适用于 consumer/job 等无 HTTP 接口的服务。
pub async fn run_blocking_server() -> Result<()> {
    ensure_init();
    let server = BlockingServer::new();
    xserver::run_with_server(&server).await
}
