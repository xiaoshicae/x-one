use axum::body::Body;
use axum::routing::get;
use axum::{Router, http::StatusCode};
use tower::ServiceExt;

fn build_app_with_recover() -> Router {
    Router::new()
        .route("/ok", get(|| async { "ok" }))
        .route(
            "/panic",
            get(|| async {
                if true {
                    panic!("handler panic");
                }
                "unreachable"
            }),
        )
        .layer(axum::middleware::from_fn(
            x_one::xaxum::middleware::recover::recover_middleware,
        ))
}

#[tokio::test]
async fn test_recover_middleware_normal_request() {
    let app = build_app_with_recover();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/ok")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_recover_middleware_catches_panic() {
    let app = build_app_with_recover();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/panic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "Internal Server Error");
}
