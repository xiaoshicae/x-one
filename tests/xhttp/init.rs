use x_one::xhttp::client::*;
use x_one::xhttp::*;

#[test]
fn test_build_client_default_config() {
    let config = XHttpConfig::default();
    let result = build_client(&config);
    assert!(result.is_ok());
}

#[test]
fn test_build_client_custom_config() {
    let config = XHttpConfig {
        timeout: "60s".to_string(),
        dial_timeout: "5s".to_string(),
        dial_keep_alive: "120s".to_string(),
        pool_max_idle_per_host: 50,
        ..XHttpConfig::default()
    };
    let result = build_client(&config);
    assert!(result.is_ok());
}

#[test]
fn test_build_client_with_invalid_duration_uses_default() {
    // to_duration 无效时回退默认值，不会导致构建失败
    let config = XHttpConfig {
        timeout: "invalid".to_string(),
        ..XHttpConfig::default()
    };
    let result = build_client(&config);
    assert!(result.is_ok());
}

#[test]
fn test_client_returns_instance() {
    let c = c();
    // 能获取到即可
    let _ = c;
}

#[test]
fn test_config_default() {
    let config = XHttpConfig::default();
    assert_eq!(config.timeout, "30s");
    assert_eq!(config.dial_timeout, "10s");
    assert_eq!(config.dial_keep_alive, "30s");
    assert_eq!(config.pool_max_idle_per_host, 10);
}
