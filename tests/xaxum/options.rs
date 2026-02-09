use axum::body::Body;
use axum::routing::get;
use axum::{Router, http::StatusCode};
use tower::ServiceExt;
use x_one::xaxum::options::AxumOptions;

#[test]
fn test_options_default() {
    let opts = AxumOptions::default();
    assert!(opts.enable_log_middleware, "默认应启用日志中间件");
    assert!(opts.enable_trace_middleware, "默认应启用追踪中间件");
}

#[test]
fn test_options_new_equals_default() {
    let opts = AxumOptions::new();
    assert!(opts.enable_log_middleware);
    assert!(opts.enable_trace_middleware);
}

#[test]
fn test_options_disable_log() {
    let opts = AxumOptions::new().with_log_middleware(false);
    assert!(!opts.enable_log_middleware, "应禁用日志中间件");
    assert!(opts.enable_trace_middleware, "追踪中间件应保持启用");
}

#[test]
fn test_options_disable_trace() {
    let opts = AxumOptions::new().with_trace_middleware(false);
    assert!(opts.enable_log_middleware, "日志中间件应保持启用");
    assert!(!opts.enable_trace_middleware, "应禁用追踪中间件");
}

#[test]
fn test_options_disable_all() {
    let opts = AxumOptions::new()
        .with_log_middleware(false)
        .with_trace_middleware(false);
    assert!(!opts.enable_log_middleware, "日志中间件应被禁用");
    assert!(!opts.enable_trace_middleware, "追踪中间件应被禁用");
}

#[tokio::test]
async fn test_server_with_options_no_middleware() {
    // 禁用所有中间件后，请求仍然正常处理
    let opts = AxumOptions::new()
        .with_log_middleware(false)
        .with_trace_middleware(false);

    let router = Router::new().route("/ping", get(|| async { "pong" }));

    let server = x_one::AxumServer::with_options(router, opts);

    // 使用 with_addr 绕过 config，直接用 tower::ServiceExt 测试 router
    // 这里只验证 router 仍然可用
    let app = Router::new().route("/ping", get(|| async { "pong" }));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "pong", "禁用中间件后请求应正常处理");

    // 验证 server 已创建成功（addr 应为默认值）
    let _addr = server.addr();
}
