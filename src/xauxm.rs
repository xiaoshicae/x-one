//! Auxm HTTP 服务器
//!
//! 对应 Go 版的 HTTP 服务器实现。

pub mod trace;

use crate::error::XOneError;
use crate::xconfig;
use crate::xserver::Server;
use crate::xutil;
use std::net::SocketAddr;

/// Auxm HTTP 服务器（基于 Axum）
pub struct AuxmServer {
    router: axum::Router,
    addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl AuxmServer {
    /// 创建新的 Auxm HTTP 服务器
    pub fn new(router: axum::Router) -> Self {
        let auxm_config = xconfig::get_auxm_config();

        if auxm_config.use_http2 {
            xutil::info_if_enable_debug("auxm server use http2");
        }

        let addr: SocketAddr = format!("{}:{}", auxm_config.host, auxm_config.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], auxm_config.port)));

        xutil::info_if_enable_debug(&format!("auxm server listen at: {addr}"));

        let (shutdown_tx, _) = tokio::sync::watch::channel(false);

        // 自动注入 trace 中间件
        let router = router.layer(axum::middleware::from_fn::<_, (axum::extract::Request,)>(
            trace::trace_middleware,
        ));

        Self {
            router,
            addr,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// 使用自定义地址创建 Auxm HTTP 服务器
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

impl Server for AuxmServer {
    async fn run(&self) -> Result<(), XOneError> {
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| XOneError::Server(format!("bind failed: {e}")))?;

        xutil::info_if_enable_debug(&format!("auxm server listening on {}", self.addr));

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

/// Auxm TLS (HTTPS) 服务器
///
/// TLS 完整实现将在 Phase 2 提供。
#[allow(dead_code)]
pub struct AuxmTlsServer {
    router: axum::Router,
    addr: SocketAddr,
    cert_file: String,
    key_file: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl AuxmTlsServer {
    /// 创建新的 Auxm HTTPS 服务器
    pub fn new(router: axum::Router, cert_file: &str, key_file: &str) -> Self {
        let auxm_config = xconfig::get_auxm_config();

        let addr: SocketAddr = format!("{}:{}", auxm_config.host, auxm_config.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], auxm_config.port)));

        xutil::info_if_enable_debug(&format!("auxm server listen at: {addr} (TLS)"));

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

impl Server for AuxmTlsServer {
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
