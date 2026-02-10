//! XAxum HTTP 服务器实现
//!
//! 提供基于 Axum 的 HTTP 服务器，实现 Server trait。
//! 支持 HTTP/1.1（默认）和 h2c（HTTP/2 明文）两种模式。

use super::{banner, config};
use crate::error::XOneError;
use crate::xserver::Server;
use crate::xutil;
use axum::Router;
use axum::serve::ListenerExt;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::watch;

/// Axum HTTP 服务器
///
/// 通过 [`super::builder::XAxum`] 构建，实现 [`Server`] trait。
///
/// 配置项（addr、banner、http2）在 `run()` 阶段解析，
/// 确保 `run_server()` 中的 `init()` 先加载配置。
///
/// # Examples
///
/// ```
/// use x_one::xaxum::builder::XAxum;
///
/// let server = XAxum::new().addr("127.0.0.1:8080").build();
/// assert_eq!(server.addr().port(), 8080);
/// ```
pub struct XAxumServer {
    router: Router,
    /// 用户显式设置的地址（None 表示从配置读取）
    addr: Option<SocketAddr>,
    /// 用户显式设置的 banner 开关（None 表示从配置读取）
    enable_banner: Option<bool>,
    /// 用户显式设置的 h2c 开关（None 表示从配置读取）
    use_http2: Option<bool>,
    shutdown_tx: watch::Sender<bool>,
}

impl XAxumServer {
    /// 创建新的 XAxumServer（仅供 builder 内部调用）
    pub(crate) fn new(
        router: Router,
        addr: Option<SocketAddr>,
        enable_banner: Option<bool>,
        use_http2: Option<bool>,
    ) -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            router,
            addr,
            enable_banner,
            use_http2,
            shutdown_tx,
        }
    }

    /// 获取监听地址
    ///
    /// 优先返回 builder 显式设置的地址，否则从配置读取。
    pub fn addr(&self) -> SocketAddr {
        self.addr.unwrap_or_else(|| resolve_config().0)
    }

    /// 是否启用 h2c（HTTP/2 明文）
    ///
    /// 优先返回 builder 显式设置的值，否则从配置读取。
    pub fn use_http2(&self) -> bool {
        self.use_http2.unwrap_or_else(|| resolve_config().2)
    }

    /// 消费 server，返回内部 Router
    pub fn into_router(self) -> Router {
        self.router
    }

    /// HTTP/1.1 模式：使用 axum::serve
    async fn run_http1(
        &self,
        listener: tokio::net::TcpListener,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<(), XOneError> {
        let listener = listener.tap_io(|tcp_stream| {
            let _ = tcp_stream.set_nodelay(true);
        });
        axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.changed().await;
            })
            .await
            .map_err(|e| XOneError::Server(format!("server error: {e}")))?;
        Ok(())
    }

    /// h2c 模式：使用 hyper-util auto::Builder，支持 HTTP/1 + HTTP/2 明文自动检测
    async fn run_h2c(
        &self,
        listener: tokio::net::TcpListener,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<(), XOneError> {
        use hyper::body::Incoming;
        use hyper_util::rt::{TokioExecutor, TokioIo};
        use hyper_util::server::conn::auto;
        use hyper_util::server::graceful::GracefulShutdown;
        use tower_service::Service;

        let graceful = GracefulShutdown::new();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    let (socket, _remote_addr) = result
                        .map_err(|e| XOneError::Server(format!("accept failed: {e}")))?;
                    let _ = socket.set_nodelay(true);

                    let tower_service = self.router.clone();

                    let hyper_service = hyper::service::service_fn(
                        move |request: axum::extract::Request<Incoming>| {
                            tower_service.clone().call(request)
                        },
                    );

                    let builder = auto::Builder::new(TokioExecutor::new());
                    let conn = builder
                        .serve_connection_with_upgrades(
                            TokioIo::new(socket),
                            hyper_service,
                        );

                    let conn = graceful.watch(conn.into_owned());
                    tokio::spawn(async move {
                        if let Err(e) = conn.await {
                            xutil::warn_if_enable_debug(
                                &format!("h2c connection error: {e}"),
                            );
                        }
                    });
                }
                _ = shutdown_rx.changed() => break,
            }
        }

        // 等待活跃连接完成（超时 10 秒）
        tokio::time::timeout(Duration::from_secs(10), graceful.shutdown())
            .await
            .ok();

        Ok(())
    }
}

impl Server for XAxumServer {
    async fn run(&self) -> Result<(), XOneError> {
        // 在 run 阶段解析配置（init() 已在 run_server 中调用）
        let (addr, enable_banner, use_http2) = resolve_config();
        let addr = self.addr.unwrap_or(addr);
        let enable_banner = self.enable_banner.unwrap_or(enable_banner);
        let use_http2 = self.use_http2.unwrap_or(use_http2);

        if enable_banner {
            banner::print_banner();
        }

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| XOneError::Server(format!("bind {addr} failed: {e}")))?;

        xutil::info_if_enable_debug(&format!("axum server listening on {addr}"));

        let shutdown_rx = self.shutdown_tx.subscribe();

        if use_http2 {
            self.run_h2c(listener, shutdown_rx).await
        } else {
            self.run_http1(listener, shutdown_rx).await
        }
    }

    async fn stop(&self) -> Result<(), XOneError> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

/// 从配置解析 (addr, enable_banner, use_http2)
fn resolve_config() -> (SocketAddr, bool, bool) {
    let c = config::load_config();
    let addr = parse_addr(&c.host, c.port);
    (addr, c.enable_banner, c.use_http2)
}

/// 解析 host:port 为 SocketAddr，失败时回退到 0.0.0.0:port
fn parse_addr(host: &str, port: u16) -> SocketAddr {
    format!("{host}:{port}").parse().unwrap_or_else(|e| {
        xutil::warn_if_enable_debug(&format!(
            "parse addr [{host}:{port}] failed, fallback to 0.0.0.0:{port}, err=[{e}]"
        ));
        SocketAddr::from(([0, 0, 0, 0], port))
    })
}
