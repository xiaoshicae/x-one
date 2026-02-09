use serial_test::serial;
use std::net::SocketAddr;
use x_one::Server;
use x_one::xaxum::*;

#[test]
fn test_axum_server_with_addr() {
    let router = axum::Router::new();
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = AxumServer::with_addr(router, addr);
    assert_eq!(server.addr(), addr);
}

#[tokio::test]
async fn test_axum_tls_server_not_implemented() {
    let router = axum::Router::new();
    let server = AxumTlsServer::new(router, "cert.pem", "key.pem");
    let result = server.run().await;
    assert!(result.is_err());
}

#[test]
fn test_axum_config_default_values() {
    let c = AxumConfig::default();
    assert_eq!(c.host, "0.0.0.0");
    assert_eq!(c.port, 8000);
    assert!(!c.use_http2);
    assert!(c.swagger.is_none());
}

#[test]
fn test_axum_swagger_config_default_schemes() {
    let c = AxumSwaggerConfig::default();
    assert_eq!(c.schemes, vec!["https", "http"]);
}

#[test]
#[serial]
fn test_load_config_default() {
    x_one::xconfig::reset_config();
    let cfg = load_config();
    assert_eq!(cfg.host, "0.0.0.0");
    assert_eq!(cfg.port, 8000);
    assert!(!cfg.use_http2);
}

#[test]
#[serial]
fn test_load_config_with_yaml() {
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Host: '127.0.0.1'
  Port: 3000
  UseHttp2: true
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);
    let cfg = load_config();
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.port, 3000);
    assert!(cfg.use_http2);
    x_one::xconfig::reset_config();
}

#[test]
#[serial]
fn test_load_swagger_config_default() {
    x_one::xconfig::reset_config();
    let swagger = load_swagger_config();
    assert_eq!(swagger.schemes, vec!["https", "http"]);
    assert!(swagger.host.is_empty());
}

#[test]
#[serial]
fn test_load_swagger_config_with_yaml() {
    x_one::xconfig::reset_config();
    let yaml = "XAxum:
  Swagger:
    Host: 'api.example.com'
    BasePath: '/v1'
    Title: 'My API'
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    x_one::xconfig::set_config(config);
    let swagger = load_swagger_config();
    assert_eq!(swagger.host, "api.example.com");
    assert_eq!(swagger.base_path, "/v1");
    assert_eq!(swagger.title, "My API");
    x_one::xconfig::reset_config();
}
