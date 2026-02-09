use x_one::xhttp::config::*;

#[test]
fn test_default_config() {
    let config = XHttpConfig::default();
    assert_eq!(config.timeout, "30s");
    assert_eq!(config.dial_timeout, "10s");
    assert_eq!(config.dial_keep_alive, "30s");
    assert_eq!(config.pool_max_idle_per_host, 10);
}

#[test]
fn test_deserialize_full_yaml() {
    let yaml = r#"
Timeout: "60s"
DialTimeout: "5s"
DialKeepAlive: "60s"
PoolMaxIdlePerHost: 20
"#;
    let config: XHttpConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.timeout, "60s");
    assert_eq!(config.dial_timeout, "5s");
    assert_eq!(config.pool_max_idle_per_host, 20);
}

#[test]
fn test_deserialize_minimal_yaml() {
    let yaml = "{}";
    let config: XHttpConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.timeout, "30s");
}
