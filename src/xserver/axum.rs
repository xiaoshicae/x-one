//! Axum HTTP 服务器
//!
//! 对应 Go 版的 Gin 服务器实现（配置名使用 Axum）。

use crate::error::XOneError;
use super::Server;
use crate::xconfig;
use crate::xutil;
use std::net::SocketAddr;

/// Axum HTTP 服务器
pub struct AxumServer {
    router: axum::Router,
    addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl AxumServer {
    /// 创建新的 Axum HTTP 服务器
    pub fn new(router: axum::Router) -> Self {
        let axum_config = xconfig::get_gin_config();

        if axum_config.use_http2 {
            xutil::info_if_enable_debug("axum server use http2");
        }

        let addr: SocketAddr = format!("{}:{}", axum_config.host, axum_config.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], axum_config.port)));

        xutil::info_if_enable_debug(&format!("axum server listen at: {addr}"));

        let (shutdown_tx, _) = tokio::sync::watch::channel(false);

        Self {
            router,
            addr,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// 使用自定义地址创建 Axum HTTP 服务器
    pub fn with_addr(router: axum::Router, addr: SocketAddr) -> Self {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        Self {
            router,
            addr,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// 获取监听地址
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Server for AxumServer {
    async fn run(&self) -> Result<(), XOneError> {
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| XOneError::Server(format!("bind failed: {e}")))?;

        xutil::info_if_enable_debug(&format!("axum server listening on {}", self.addr));

        let mut shutdown_rx = self
            .shutdown_tx
            .as_ref()
            .map(|tx| tx.subscribe())
            .ok_or_else(|| XOneError::Server("shutdown channel not available".to_string()))?;

        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.changed().await;
            })
            .await
            .map_err(|e| XOneError::Server(format!("server error: {e}")))?;

        Ok(())
    }

    async fn stop(&self) -> Result<(), XOneError> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        Ok(())
    }
}

/// Axum TLS (HTTPS) 服务器
///
/// TLS 完整实现将在 Phase 2 提供。
#[allow(dead_code)]
pub struct AxumTlsServer {
    router: axum::Router,
    addr: SocketAddr,
    cert_file: String,
    key_file: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl AxumTlsServer {
    /// 创建新的 Axum HTTPS 服务器
    pub fn new(router: axum::Router, cert_file: &str, key_file: &str) -> Self {
        let axum_config = xconfig::get_gin_config();

        let addr: SocketAddr = format!("{}:{}", axum_config.host, axum_config.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], axum_config.port)));

        xutil::info_if_enable_debug(&format!("axum server listen at: {addr} (TLS)"));

        let (shutdown_tx, _) = tokio::sync::watch::channel(false);

        Self {
            router,
            addr,
            cert_file: cert_file.to_string(),
            key_file: key_file.to_string(),
            shutdown_tx: Some(shutdown_tx),
        }
    }
}

impl Server for AxumTlsServer {
    async fn run(&self) -> Result<(), XOneError> {
        // TLS 支持需要额外的 crate（如 axum-server 或 rustls）
        // 这里提供基础框架，完整 TLS 实现在 Phase 2
        Err(XOneError::Server(
            "TLS server not yet implemented, will be available in Phase 2".to_string(),
        ))
    }

    async fn stop(&self) -> Result<(), XOneError> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        Ok(())
    }
}

