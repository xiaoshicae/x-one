//! HTTP 访问日志中间件
//!
//! 记录请求/响应的关键信息（method、path、status、headers、body、耗时），
//! 敏感 header 自动脱敏，二进制/大体积 body 自动跳过。

use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use std::fmt::Write;
use std::time::Instant;

/// Body 日志显示截断阈值（4KB）
const MAX_BODY_DISPLAY: usize = 4096;

/// Body 缓冲上限（256KB），超出此大小的 body 不做缓冲
const MAX_BODY_BUFFER: usize = 256 * 1024;

/// 敏感 header 列表（小写）
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-csrf-token",
];

/// 二进制 Content-Type 前缀
const BINARY_CT_PREFIXES: &[&str] = &["image/", "audio/", "video/", "font/"];

/// 二进制 Content-Type 完整匹配
const BINARY_CT_EXACT: &[&str] = &[
    "application/octet-stream",
    "application/zip",
    "application/gzip",
    "application/pdf",
    "multipart/form-data",
];

/// 判断是否为敏感 header（大小写不敏感）
///
/// 使用 `eq_ignore_ascii_case` 避免分配临时 String。
#[doc(hidden)]
pub fn is_sensitive_header(name: &str) -> bool {
    SENSITIVE_HEADERS
        .iter()
        .any(|s| name.eq_ignore_ascii_case(s))
}

/// 将 header 转为 JSON 字符串，敏感值脱敏为 `***`
///
/// 手动拼接 JSON，单次堆分配；正确转义引号和反斜杠。
#[doc(hidden)]
pub fn format_headers(headers: &HeaderMap) -> String {
    let mut buf = String::with_capacity(256);
    buf.push('{');
    for (i, (name, value)) in headers.iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        buf.push('"');
        buf.push_str(name.as_str());
        buf.push_str("\":\"");
        if is_sensitive_header(name.as_str()) {
            buf.push_str("***");
        } else {
            let v = value.to_str().unwrap_or("<non-utf8>");
            for c in v.chars() {
                match c {
                    '"' => buf.push_str("\\\""),
                    '\\' => buf.push_str("\\\\"),
                    '\n' => buf.push_str("\\n"),
                    '\r' => buf.push_str("\\r"),
                    '\t' => buf.push_str("\\t"),
                    c if c < '\x20' => {
                        // JSON 要求控制字符用 \u00XX 转义，直接写入 buf 避免临时 String
                        let _ = write!(buf, "\\u{:04x}", c as u32);
                    }
                    _ => buf.push(c),
                }
            }
        }
        buf.push('"');
    }
    buf.push('}');
    buf
}

/// 判断 Content-Type 是否为二进制类型
#[doc(hidden)]
pub fn is_binary_content_type(headers: &HeaderMap) -> bool {
    let ct = match headers.get("content-type") {
        Some(v) => match v.to_str() {
            Ok(s) => s,
            Err(_) => return false,
        },
        None => return false,
    };

    // 取分号前的主类型部分（去除 charset 等参数）
    let main_type = ct.split(';').next().unwrap_or(ct).trim();

    // 使用 ASCII 大小写不敏感比较，避免 to_ascii_lowercase() 堆分配
    for prefix in BINARY_CT_PREFIXES {
        if main_type.len() >= prefix.len() && main_type[..prefix.len()].eq_ignore_ascii_case(prefix)
        {
            return true;
        }
    }

    BINARY_CT_EXACT
        .iter()
        .any(|exact| main_type.eq_ignore_ascii_case(exact))
}

/// 将 body 字节转为可记录字符串
///
/// - 二进制内容返回 `<binary>`
/// - 超过 4KB 截断并标注 `...(truncated)`
/// - 空 body 返回空字符串
#[doc(hidden)]
pub fn body_to_string(bytes: &[u8], headers: &HeaderMap) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if is_binary_content_type(headers) {
        return "<binary>".to_owned();
    }

    body_display_string(bytes)
}

/// HTTP 访问日志中间件
///
/// 记录每个请求的 method、path、query、headers、body、响应状态码和耗时。
/// 应注册在 trace 中间件外层，以便日志自动携带 trace context。
///
/// # 性能特性
///
/// - INFO 级别未启用时直接透传，零开销
/// - 二进制 body 不缓冲，直接透传
/// - 请求 body 仅在 Content-Length 已知且 ≤ 256KB 时缓冲
/// - 响应 body 不缓冲，直接透传（仅记录 status + headers）
/// - 日志文本截断为 4KB 显示（UTF-8 char boundary 安全）
/// - headers 手动拼接 JSON，单次堆分配
pub async fn log_middleware(req: Request, next: Next) -> Response {
    // 日志级别未启用时直接透传，跳过所有 header/body 构造开销
    if !tracing::enabled!(tracing::Level::INFO) {
        return next.run(req).await;
    }

    let start = Instant::now();

    // 在消耗 body 之前就地格式化 headers，避免 HeaderMap clone
    // Method clone（小结构体）+ Uri clone（引用计数）代替多次 String 堆分配
    let method = req.method().clone();
    let uri = req.uri().clone();
    let req_headers_str = format_headers(req.headers());

    // 请求 body：仅在 Content-Length 已知且在安全范围内时缓冲
    // 无 Content-Length 时不消耗 body，避免大流式请求数据丢失
    let should_buffer_req = !is_binary_content_type(req.headers())
        && content_length(req.headers()).is_some_and(|len| len <= MAX_BODY_BUFFER);

    let (req, req_body_str) = if should_buffer_req {
        let (parts, body) = req.into_parts();
        match axum::body::to_bytes(body, MAX_BODY_BUFFER).await {
            Ok(bytes) => {
                let s = body_display_string(&bytes);
                (Request::from_parts(parts, Body::from(bytes)), s)
            }
            Err(e) => {
                tracing::warn!("log middleware: buffer request body failed: {e}");
                (Request::from_parts(parts, Body::empty()), String::new())
            }
        }
    } else {
        (req, String::new())
    };

    // 调用下一个中间件/handler
    let response = next.run(req).await;

    let status = response.status().as_u16();
    let resp_headers_str = format_headers(response.headers());
    let elapsed = start.elapsed();

    tracing::info!(
        http.method = %method,
        http.path = %uri.path(),
        http.query = %uri.query().unwrap_or_default(),
        http.status = status,
        http.elapsed = %format_elapsed(elapsed),
        http.request_headers = %req_headers_str,
        http.request_body = %req_body_str,
        http.response_headers = %resp_headers_str,
        "HTTP request completed"
    );

    response
}

/// 从 headers 获取 Content-Length
fn content_length(headers: &HeaderMap) -> Option<usize> {
    headers.get("content-length")?.to_str().ok()?.parse().ok()
}

/// 将已确认为文本的 body 字节转为可显示字符串（内部使用）
///
/// 截断时按 char boundary 对齐，避免多字节 UTF-8 字符被截断导致 panic。
fn body_display_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let text = String::from_utf8_lossy(bytes);
    if text.len() > MAX_BODY_DISPLAY {
        // 从 MAX_BODY_DISPLAY 向前找到最近的 char boundary
        let mut end = MAX_BODY_DISPLAY;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...(truncated)", &text[..end])
    } else {
        text.into_owned()
    }
}

/// 格式化耗时为人类可读字符串
///
/// - < 1µs → `"850ns"`
/// - < 1ms → `"123.4µs"`
/// - < 1s  → `"12.34ms"`
/// - ≥ 1s  → `"1.50s"`
fn format_elapsed(elapsed: std::time::Duration) -> String {
    let nanos = elapsed.as_nanos();
    if nanos < 1_000 {
        format!("{nanos}ns")
    } else if nanos < 1_000_000 {
        format!("{:.1}µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", elapsed.as_secs_f64())
    }
}
