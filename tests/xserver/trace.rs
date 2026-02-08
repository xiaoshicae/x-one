use axum::{Router, body::Body, routing::get};
use opentelemetry::{trace::TraceContextExt, Context};
use serial_test::serial;
use tower::ServiceExt;

/// 初始化测试用 tracer 并启用 trace
fn setup_trace() {
    // 设置 config 使 init_xtrace 能正常初始化
    x_one::xconfig::set_config(
        serde_yaml::from_str("XTrace:\n  Enable: true").unwrap(),
    );
    x_one::xtrace::init::init_xtrace().ok();
}

fn build_app() -> Router {
    Router::new()
        .route(
            "/test",
            get(|| async {
                let cx = Context::current();
                let sc = cx.span().span_context().clone();
                sc.trace_id().to_string()
            }),
        )
        .layer(axum::middleware::from_fn::<_, (axum::extract::Request,)>(
            x_one::xserver::trace::trace_middleware,
        ))
}

#[tokio::test]
#[serial]
async fn test_trace_middleware_creates_span() {
    setup_trace();

    let app = build_app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let trace_id = String::from_utf8(body.to_vec()).unwrap();

    // trace_id 应该是有效的（非全零）
    assert_ne!(
        trace_id, "00000000000000000000000000000000",
        "应该创建有效的 span"
    );
    assert_eq!(trace_id.len(), 32, "trace_id 应为 32 字符的十六进制字符串");
}

#[tokio::test]
#[serial]
async fn test_trace_middleware_extracts_traceparent() {
    setup_trace();

    let app = build_app();

    let expected_trace_id = "4bf92f3577b34da6a3ce929d0e0e4736";
    let traceparent = format!("00-{expected_trace_id}-00f067aa0ba902b7-01");

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/test")
                .header("traceparent", &traceparent)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let trace_id = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(
        trace_id, expected_trace_id,
        "handler 中的 trace_id 应与 traceparent 一致"
    );
}

#[tokio::test]
async fn test_trace_middleware_disabled_passthrough() {
    // 不初始化 xtrace，中间件应直接透传
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .layer(axum::middleware::from_fn::<_, (axum::extract::Request,)>(
            x_one::xserver::trace::trace_middleware,
        ));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "pong");
}
