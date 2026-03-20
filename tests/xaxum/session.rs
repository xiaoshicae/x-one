use axum::body::Body;
use axum::routing::get;
use axum::{Extension, Router, http::StatusCode};
use tower::ServiceExt;
use x_one::xaxum::middleware::session::{SessionContext, session_middleware};

fn build_app_with_session() -> Router {
    Router::new()
        .route(
            "/ctx",
            get(|Extension(ctx): Extension<SessionContext>| async move {
                ctx.set("visited", "true");
                let v = ctx.get("visited").unwrap_or_default();
                format!("visited={v}")
            }),
        )
        .layer(axum::middleware::from_fn(session_middleware))
}

#[tokio::test]
async fn test_session_middleware_injects_context() {
    let app = build_app_with_session();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/ctx")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, "visited=true");
}

#[test]
fn test_session_context_new() {
    let ctx = SessionContext::new();
    assert!(ctx.get("any").is_none());
}

#[test]
fn test_session_context_set_get() {
    let ctx = SessionContext::new();
    ctx.set("key", "value");
    assert_eq!(ctx.get("key"), Some("value".to_string()));
    assert!(ctx.get("missing").is_none());
}

#[test]
fn test_session_context_clone() {
    let ctx = SessionContext::new();
    ctx.set("a", "1");
    let cloned = ctx.clone();
    // Arc 共享，修改通过 clone 可见
    assert_eq!(cloned.get("a"), Some("1".to_string()));
}

#[test]
fn test_session_context_debug() {
    let ctx = SessionContext::new();
    let debug = format!("{ctx:?}");
    assert!(debug.contains("SessionContext"));
}
