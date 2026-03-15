//! HTTP 请求指标中间件
//!
//! 自动记录 HTTP 请求的 Counter 和 Histogram 指标。

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::time::Instant;

/// HTTP 请求指标中间件
///
/// 为每个请求记录：
/// - `http_requests_total` Counter（method, path, status）
/// - `http_request_duration_ms` Histogram（method, path, status）
///
/// # Examples
///
/// ```ignore
/// use axum::{Router, middleware};
/// use x_one::xaxum::middleware::metric_middleware;
///
/// let app = Router::new()
///     .layer(middleware::from_fn(metric_middleware));
/// ```
pub async fn metric_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status().as_u16().to_string();
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let labels = [
        ("method", method.as_str()),
        ("path", path.as_str()),
        ("status", status.as_str()),
    ];

    crate::xmetric::counter_inc("http_requests_total", &labels);
    crate::xmetric::histogram_observe("http_request_duration_ms", duration_ms, &labels);

    response
}
