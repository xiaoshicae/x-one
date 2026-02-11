//! Trace 中间件
//!
//! 为 axum 请求自动注入 OpenTelemetry trace 上下文，
//! 支持从入站 HTTP header 提取 W3C traceparent。

use axum::{extract::Request, middleware::Next, response::Response};
use opentelemetry::{
    Context, KeyValue, global,
    propagation::Extractor,
    trace::{SpanKind, Status, TraceContextExt, Tracer},
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

/// HTTP header 提取器，用于从请求头中提取 trace 上下文
struct HeaderExtractor<'a>(&'a axum::http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// 包装 Future，在每次 poll 时自动 attach OpenTelemetry 上下文
///
/// 解决 `ContextGuard` 不可 `Send` 的问题：不跨 await 持有 guard，
/// 而是在每次 poll 时重新 attach，确保 handler 中 `Context::current()` 始终有效。
struct OtelContextFuture<F> {
    inner: Pin<Box<F>>,
    otel_cx: Context,
}

impl<F: Future> Future for OtelContextFuture<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let _guard = self.otel_cx.clone().attach();
        self.inner.as_mut().poll(cx)
    }
}

/// Trace 中间件
///
/// 从 HTTP 请求头中提取 W3C `traceparent`，创建 server span，
/// 将 trace 上下文传递给下游 handler。handler 中通过
/// `opentelemetry::Context::current()` 可获取当前 trace_id。
///
/// 当 xtrace 未启用时直接透传请求，不产生额外开销。
pub async fn trace_middleware(req: Request, next: Next) -> Response {
    if !crate::xtrace::is_trace_enabled() {
        return next.run(req).await;
    }

    // 从请求头提取父级 trace 上下文
    let parent_cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });

    let tracer = global::tracer("x-one-http-server");
    // clone 替代 to_string：Method 是小枚举（栈拷贝），Uri 是引用计数（原子 +1）
    let method = req.method().clone();
    let uri = req.uri().clone();

    let span = tracer
        .span_builder(format!("{} {}", method.as_str(), uri.path()))
        .with_kind(SpanKind::Server)
        // 数组替代 vec!：避免每请求的 Vec 堆分配
        .with_attributes([
            KeyValue::new("http.method", method.as_str().to_owned()),
            KeyValue::new("http.target", uri.path().to_owned()),
        ])
        .start_with_context(&tracer, &parent_cx);

    let cx = Context::current_with_span(span);

    // 使用 OtelContextFuture 确保每次 poll 时 context 都正确绑定到当前线程
    let response = OtelContextFuture {
        inner: Box::pin(next.run(req)),
        otel_cx: cx.clone(),
    }
    .await;

    // 记录响应状态码（从之前捕获的 cx 中获取 span，此时 guard 已释放）
    let status = response.status().as_u16();
    let span_ref = cx.span();
    span_ref.set_attribute(KeyValue::new("http.status_code", i64::from(status)));
    if status >= 500 {
        span_ref.set_status(Status::error(format!("HTTP {status}")));
    }

    response
}
