use x_one::xhttp::config::*;

#[test]
fn test_default_config() {
    let config = XHttpConfig::default();
    assert_eq!(config.timeout, "30s");
    assert_eq!(config.dial_timeout, "10s");
    assert_eq!(config.dial_keep_alive, "30s");
    assert_eq!(config.max_idle_conns_per_host, 100);
    assert_eq!(config.pool_max_idle_per_host, 10);
    assert_eq!(config.retry_count, 0);
    assert_eq!(config.retry_wait_time, "1s");
    assert_eq!(config.retry_max_wait_time, "10s");
}

#[test]
fn test_deserialize_full_yaml() {
    let yaml = r#"
Timeout: "60s"
DialTimeout: "5s"
DialKeepAlive: "60s"
MaxIdleConnsPerHost: 200
PoolMaxIdlePerHost: 20
RetryCount: 3
RetryWaitTime: "2s"
RetryMaxWaitTime: "30s"
"#;
    let config: XHttpConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.timeout, "60s");
    assert_eq!(config.dial_timeout, "5s");
    assert_eq!(config.max_idle_conns_per_host, 200);
    assert_eq!(config.retry_count, 3);
}

#[test]
fn test_deserialize_minimal_yaml() {
    let yaml = "{}";
    let config: XHttpConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.timeout, "30s");
    assert_eq!(config.retry_count, 0);
}
