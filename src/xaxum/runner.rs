//! Axum 服务器运行入口
//!
//! 提供 HTTP/HTTPS 服务器的启动函数。

use super::options::AxumOptions;
#[allow(deprecated)]
use super::server::{AxumServer, AxumTlsServer};
use crate::error::Result;
use crate::xserver;

/// 以 Axum HTTP 服务器运行
///
/// 初始化所有模块后，创建 AxumServer 并以异步方式运行，
/// 阻塞等待退出信号（SIGINT/SIGTERM）。
pub async fn run_axum(router: axum::Router) -> Result<()> {
    let server = AxumServer::new(router);
    xserver::run_server(&server).await
}

/// 以 Axum HTTP 服务器运行（自定义选项）
///
/// 通过 `AxumOptions` 控制中间件启用/禁用。
pub async fn run_axum_with_options(router: axum::Router, opts: AxumOptions) -> Result<()> {
    let server = AxumServer::with_options(router, opts);
    xserver::run_server(&server).await
}

/// 以 Axum HTTPS 服务器运行
///
/// **注意**：TLS 尚未实现，调用会立即返回错误。
#[deprecated(note = "TLS 尚未实现，请勿在生产环境使用")]
#[allow(deprecated)]
pub async fn run_axum_tls(router: axum::Router, cert_file: &str, key_file: &str) -> Result<()> {
    let server = AxumTlsServer::new(router, cert_file, key_file);
    xserver::run_server(&server).await
}
