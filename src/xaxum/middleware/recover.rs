//! Panic 恢复中间件
//!
//! 捕获 handler panic，返回 500 而不是连接断开。

use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};

/// Panic 恢复中间件
///
/// 捕获下游 handler 中的 panic，返回 500 Internal Server Error。
/// 防止 panic 导致连接中断。
///
/// # Examples
///
/// ```ignore
/// use axum::{Router, middleware};
/// use x_one::xaxum::middleware::recover_middleware;
///
/// let app = Router::new()
///     .layer(middleware::from_fn(recover_middleware));
/// ```
pub async fn recover_middleware(req: Request<Body>, next: Next) -> Response {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| next.run(req))) {
        Ok(future) => future.await,
        Err(_) => {
            tracing::error!("handler panicked, returning 500");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal Server Error"))
                .unwrap_or_default()
        }
    }
}
