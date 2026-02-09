use axum::body::Body;
use axum::routing::{get, post};
use axum::{Router, http::StatusCode};
use tower::ServiceExt;

fn build_app_with_log() -> Router {
    Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/echo", post(|body: String| async move { body }))
        .layer(axum::middleware::from_fn::<_, (axum::extract::Request,)>(
            x_one::xaxum::middleware::log::log_middleware,
        ))
}

#[tokio::test]
async fn test_log_middleware_basic() {
    let app = build_app_with_log();

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
    assert_eq!(body, "pong", "log 中间件不应影响请求/响应内容");
}

#[tokio::test]
async fn test_log_middleware_sensitive_headers_redacted() {
    // 验证辅助函数的脱敏逻辑
    use x_one::xaxum::middleware::log::{format_headers, is_sensitive_header};

    assert!(is_sensitive_header("Authorization"));
    assert!(is_sensitive_header("cookie"));
    assert!(is_sensitive_header("X-Api-Key"));
    assert!(is_sensitive_header("SET-COOKIE"));
    assert!(!is_sensitive_header("Content-Type"));
    assert!(!is_sensitive_header("Accept"));

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", "Bearer secret-token".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("x-api-key", "my-secret-key".parse().unwrap());

    let formatted = format_headers(&headers);
    assert!(
        formatted.contains("\"authorization\":\"***\""),
        "authorization 应被脱敏: {formatted}"
    );
    assert!(
        formatted.contains("\"x-api-key\":\"***\""),
        "x-api-key 应被脱敏: {formatted}"
    );
    assert!(
        formatted.contains("\"content-type\":\"application/json\""),
        "content-type 不应被脱敏: {formatted}"
    );
    assert!(
        !formatted.contains("secret"),
        "脱敏后不应包含敏感值: {formatted}"
    );
}

#[tokio::test]
async fn test_log_middleware_binary_body_skipped() {
    use x_one::xaxum::middleware::log::{body_to_string, is_binary_content_type};

    // 验证二进制 Content-Type 判断
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("content-type", "image/png".parse().unwrap());
    assert!(is_binary_content_type(&headers));

    headers.insert("content-type", "application/octet-stream".parse().unwrap());
    assert!(is_binary_content_type(&headers));

    headers.insert("content-type", "application/json".parse().unwrap());
    assert!(!is_binary_content_type(&headers));

    headers.insert("content-type", "text/plain".parse().unwrap());
    assert!(!is_binary_content_type(&headers));

    // 验证二进制 body 转字符串
    let mut bin_headers = axum::http::HeaderMap::new();
    bin_headers.insert("content-type", "image/jpeg".parse().unwrap());
    let result = body_to_string(b"\x89PNG\r\n", &bin_headers);
    assert_eq!(result, "<binary>", "二进制 body 应标记为 <binary>");

    // 验证空 body
    let empty_headers = axum::http::HeaderMap::new();
    let result = body_to_string(b"", &empty_headers);
    assert!(result.is_empty(), "空 body 应返回空字符串");

    // 验证超长文本 body 截断
    let mut text_headers = axum::http::HeaderMap::new();
    text_headers.insert("content-type", "text/plain".parse().unwrap());
    let long_body = "a".repeat(5000);
    let result = body_to_string(long_body.as_bytes(), &text_headers);
    assert!(
        result.ends_with("...(truncated)"),
        "超长 body 应被截断: 长度={}",
        result.len()
    );
    assert!(result.len() < 5000, "截断后长度应小于原始长度");
}

#[tokio::test]
async fn test_log_middleware_text_body_recorded() {
    let app = build_app_with_log();

    let request_body = r#"{"name":"test","value":42}"#;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/echo")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, request_body, "log 中间件应透传 body，不改变内容");
}
