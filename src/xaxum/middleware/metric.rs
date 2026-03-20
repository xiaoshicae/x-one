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
    // 用 &'static str 避免每请求的 method String 分配
    let method = static_method_str(req.method());
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    let status_code = response.status().as_u16();
    // 栈上缓冲区，避免 to_string() 堆分配
    let mut status_buf = [0u8; 3];
    let status = format_status_code(status_code, &mut status_buf);
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let labels = [
        ("method", method),
        ("path", path.as_str()),
        ("status", status),
    ];

    crate::xmetric::counter_inc("http_requests_total", &labels);
    crate::xmetric::histogram_observe("http_request_duration_ms", duration_ms, &labels);

    response
}

/// HTTP Method 转 &'static str，避免每请求的 String 堆分配
fn static_method_str(method: &axum::http::Method) -> &'static str {
    match *method {
        axum::http::Method::GET => "GET",
        axum::http::Method::POST => "POST",
        axum::http::Method::PUT => "PUT",
        axum::http::Method::DELETE => "DELETE",
        axum::http::Method::PATCH => "PATCH",
        axum::http::Method::HEAD => "HEAD",
        axum::http::Method::OPTIONS => "OPTIONS",
        axum::http::Method::CONNECT => "CONNECT",
        axum::http::Method::TRACE => "TRACE",
        _ => "UNKNOWN",
    }
}

/// 将 HTTP 状态码写入栈缓冲区，避免 to_string() 堆分配
fn format_status_code(code: u16, buf: &mut [u8; 3]) -> &str {
    buf[0] = b'0' + (code / 100 % 10) as u8;
    buf[1] = b'0' + (code / 10 % 10) as u8;
    buf[2] = b'0' + (code % 10) as u8;
    // 安全：buf 仅含 ASCII 数字
    std::str::from_utf8(buf).unwrap_or("000")
}
