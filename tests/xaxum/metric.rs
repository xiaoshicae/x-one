use axum::body::Body;
use axum::routing::get;
use axum::{Router, http::StatusCode};
use serial_test::serial;
use tower::ServiceExt;
use x_one::xaxum::middleware::metric::metric_middleware;

fn build_app_with_metric() -> Router {
    Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(metric_middleware))
}

#[tokio::test]
#[serial]
async fn test_metric_middleware_records_metrics() {
    x_one::xmetric::reset_metrics();

    let app = build_app_with_metric();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证指标已记录：encode registry 并检查输出
    let mut output = String::new();
    prometheus_client::encoding::text::encode(&mut output, &x_one::xmetric::registry().read())
        .unwrap();
    assert!(
        output.contains("http_requests_total"),
        "应包含请求计数指标: {output}"
    );
    assert!(
        output.contains("http_request_duration_ms"),
        "应包含请求耗时指标: {output}"
    );
}
