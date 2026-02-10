use axum::body::Body;
use axum::routing::get;
use axum::{Router, http::StatusCode};
use serial_test::serial;
use std::net::SocketAddr;
use tower::ServiceExt;
use x_one::Server;
use x_one::xaxum::*;

// ============================================================
// XAxum builder 测试
// ============================================================

#[test]
#[serial]
fn test_xaxum_builder_default() {
    x_one::xconfig::reset_config();
    let server = XAxum::new().build();
    // 无 xconfig 时，使用默认 0.0.0.0:8000
    assert_eq!(server.addr(), SocketAddr::from(([0, 0, 0, 0], 8000)));
    x_one::xconfig::reset_config();
}

#[test]
fn test_xaxum_from_router() {
    let router = Router::new().route("/health", get(|| async { "ok" }));
    let server = XAxum::from_router(router).addr("127.0.0.1:0").build();
    assert_eq!(server.addr(), "127.0.0.1:0".parse::<SocketAddr>().unwrap());
}

#[test]
fn test_xaxum_custom_addr() {
    let server = XAxum::new().addr("127.0.0.1:9000").build();
    assert_eq!(
        server.addr(),
        "127.0.0.1:9000".parse::<SocketAddr>().unwrap()
    );
}

#[test]
fn test_xaxum_invalid_addr_fallback() {
    let server = XAxum::new().addr("invalid").build();
    assert_eq!(server.addr(), SocketAddr::from(([0, 0, 0, 0], 8000)));
}

#[test]
fn test_xaxum_with_route_register() {
    let server = XAxum::new()
        .with_route_register(|r| r.route("/ping", get(|| async { "pong" })))
        .addr("127.0.0.1:0")
        .build();
    // 验证构建成功
    assert_eq!(server.addr(), "127.0.0.1:0".parse::<SocketAddr>().unwrap());
}

#[test]
fn test_xaxum_with_multiple_route_registers() {
    let server = XAxum::new()
        .with_route_register(|r| r.route("/api/v1", get(|| async { "v1" })))
        .with_route_register(|r| r.route("/api/v2", get(|| async { "v2" })))
        .addr("127.0.0.1:0")
        .build();
    assert_eq!(server.addr(), "127.0.0.1:0".parse::<SocketAddr>().unwrap());
}

#[test]
fn test_xaxum_with_middleware() {
    let server = XAxum::new()
        .with_route_register(|r| r.route("/test", get(|| async { "ok" })))
        .with_middleware(|r| r) // no-op 中间件
        .addr("127.0.0.1:0")
        .build();
    assert_eq!(server.addr(), "127.0.0.1:0".parse::<SocketAddr>().unwrap());
}

#[test]
fn test_xaxum_disable_all_middleware() {
    let server = XAxum::new()
        .enable_log_middleware(false)
        .enable_trace_middleware(false)
        .addr("127.0.0.1:0")
        .build();
    assert_eq!(server.addr(), "127.0.0.1:0".parse::<SocketAddr>().unwrap());
}

#[tokio::test]
async fn test_xaxum_into_router_serves_requests() {
    // 构建后提取 router，用 tower::ServiceExt 测试路由可用性
    let router = XAxum::new()
        .with_route_register(|r| r.route("/ping", get(|| async { "pong" })))
        .enable_log_middleware(false)
        .enable_trace_middleware(false)
        .addr("127.0.0.1:0")
        .build()
        .into_router();

    let response = router
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
    assert_eq!(body, "pong");
}

#[tokio::test]
async fn test_xaxum_server_trait() {
    // XAxumServer 实现 Server trait，可通过 stop() 优雅关闭
    let server = XAxum::new()
        .with_route_register(|r| r.route("/health", get(|| async { "ok" })))
        .addr("127.0.0.1:0")
        .build();

    // 验证 stop 不报错
    let result = server.stop().await;
    assert!(result.is_ok());
}

// ============================================================
// 配置相关测试
// ============================================================

#[test]
fn test_axum_config_default_values() {
    let c = AxumConfig::default();
    assert_eq!(c.host, "0.0.0.0");
    assert_eq!(c.port, 8000);
    assert!(!c.use_http2);
    assert!(c.enable_banner);
    assert!(c.swagger.is_none());
}

#[test]
fn test_axum_swagger_config_default_schemes() {
    let c = AxumSwaggerConfig::default();
    assert_eq!(c.schemes, vec!["https", "http"]);
}

#[test]
#[serial]
fn test_parse_axum_config_default() {
    x_one::xconfig::reset_config();
    let cfg: AxumConfig = x_one::xconfig::parse_config("XAxum").unwrap_or_default();
    assert_eq!(cfg.host, "0.0.0.0");
    assert_eq!(cfg.port, 8000);
    assert!(!cfg.use_http2);
}

#[test]
#[serial]
fn test_parse_axum_config_with_yaml() {
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Host: '127.0.0.1'
  Port: 3000
  UseHttp2: true
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);
    let cfg: AxumConfig = x_one::xconfig::parse_config("XAxum").unwrap();
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.port, 3000);
    assert!(cfg.use_http2);
    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_xaxum_config_from_yaml() {
    // 验证 builder 从 YAML 配置读取地址
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Host: '10.0.0.1'
  Port: 5000
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);

    let server = XAxum::new().build();
    assert_eq!(
        server.addr(),
        "10.0.0.1:5000".parse::<SocketAddr>().unwrap()
    );

    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_xaxum_builder_addr_overrides_config() {
    // builder 显式设置的地址优先于 YAML 配置
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Host: '10.0.0.1'
  Port: 5000
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);

    let server = XAxum::new().addr("127.0.0.1:9000").build();
    assert_eq!(
        server.addr(),
        "127.0.0.1:9000".parse::<SocketAddr>().unwrap()
    );

    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_enable_banner_from_config() {
    // 配置文件中关闭 banner
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  EnableBanner: false
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);

    let cfg: AxumConfig = x_one::xconfig::parse_config("XAxum").unwrap();
    assert!(!cfg.enable_banner);

    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_enable_banner_default_true_in_config() {
    // 默认配置中 enable_banner 为 true
    x_one::xconfig::reset_config();
    let cfg = AxumConfig::default();
    assert!(cfg.enable_banner);
    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_parse_swagger_config_default() {
    x_one::xconfig::reset_config();
    let swagger: AxumSwaggerConfig =
        x_one::xconfig::parse_config("XAxum.Swagger").unwrap_or_default();
    assert_eq!(swagger.schemes, vec!["https", "http"]);
    assert!(swagger.host.is_empty());
}

#[test]
#[serial]
fn test_xaxum_config_invalid_host_fallback() {
    // 配置中设置无效 host，构建器应回退到 0.0.0.0:port
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Host: 'invalid host'
  Port: 9000
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);

    let server = XAxum::new().build();
    assert_eq!(server.addr(), SocketAddr::from(([0, 0, 0, 0], 9000)));

    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_parse_swagger_config_with_yaml() {
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Swagger:
    Host: 'api.example.com'
    BasePath: '/v1'
    Title: 'My API'
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);
    let swagger: AxumSwaggerConfig = x_one::xconfig::parse_config("XAxum.Swagger").unwrap();
    assert_eq!(swagger.host, "api.example.com");
    assert_eq!(swagger.base_path, "/v1");
    assert_eq!(swagger.title, "My API");
    x_one::xconfig::reset_config();
}

// ============================================================
// h2c (HTTP/2 明文) 测试
// ============================================================

#[test]
fn test_use_http2_builder_default_false() {
    // 默认不启用 h2c
    let server = XAxum::new().addr("127.0.0.1:0").build();
    assert!(!server.use_http2());
}

#[test]
fn test_use_http2_builder_enable() {
    // builder 显式启用 h2c
    let server = XAxum::new().use_http2(true).addr("127.0.0.1:0").build();
    assert!(server.use_http2());
}

#[test]
fn test_use_http2_builder_disable() {
    // builder 显式禁用 h2c
    let server = XAxum::new().use_http2(false).addr("127.0.0.1:0").build();
    assert!(!server.use_http2());
}

#[test]
#[serial]
fn test_use_http2_from_config() {
    // 从 YAML 配置读取 UseHttp2
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  UseHttp2: true
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);

    let server = XAxum::new().addr("127.0.0.1:0").build();
    assert!(server.use_http2());

    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_use_http2_builder_overrides_config() {
    // builder 设置优先于配置文件
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  UseHttp2: true
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);

    let server = XAxum::new().use_http2(false).addr("127.0.0.1:0").build();
    assert!(!server.use_http2());

    x_one::xconfig::reset_config();
}

#[tokio::test]
async fn test_h2c_server_accepts_http1_request() {
    // h2c 服务器同时支持 HTTP/1.1 请求（auto 模式自动检测）
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // 先绑定随机端口获取地址
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let server = Arc::new(
        XAxum::new()
            .with_route_register(|r| r.route("/ping", get(|| async { "pong" })))
            .use_http2(true)
            .enable_banner(false)
            .enable_log_middleware(false)
            .enable_trace_middleware(false)
            .addr(&addr.to_string())
            .build(),
    );

    // 后台运行服务器
    let server_clone = Arc::clone(&server);
    let server_handle = tokio::spawn(async move {
        server_clone.run().await.unwrap();
    });

    // 等待服务器就绪
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 用原始 TCP 发送 HTTP/1.1 请求
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();

    assert!(
        response.contains("HTTP/1.1 200"),
        "expected 200 OK, got: {response}"
    );
    assert!(
        response.contains("pong"),
        "expected body 'pong', got: {response}"
    );

    // 优雅关闭
    server.stop().await.unwrap();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), server_handle).await;
}

#[tokio::test]
async fn test_h2c_server_graceful_shutdown() {
    // 验证 h2c 模式下 stop() 能正常关闭服务器
    let server = XAxum::new()
        .with_route_register(|r| r.route("/health", get(|| async { "ok" })))
        .use_http2(true)
        .enable_banner(false)
        .enable_log_middleware(false)
        .enable_trace_middleware(false)
        .addr("127.0.0.1:0")
        .build();

    // stop 在 run 之前调用不报错
    let result = server.stop().await;
    assert!(result.is_ok());
}
