use x_one::xredis::{XREDIS_CONFIG_KEY, XRedisConfig};

#[test]
fn test_config_key() {
    assert_eq!(XREDIS_CONFIG_KEY, "XRedis");
}

#[test]
fn test_default_config() {
    let config = XRedisConfig::default();
    assert_eq!(config.addr, "redis://localhost:6379");
    assert_eq!(config.password, "");
    assert_eq!(config.db, 0);
    assert_eq!(config.username, "");
    assert_eq!(config.dial_timeout, "500ms");
    assert_eq!(config.read_timeout, "500ms");
    assert_eq!(config.write_timeout, "500ms");
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.name, "");
}

#[test]
fn test_deserialize_config() {
    let yaml = r#"
Addr: "redis://myhost:6380"
Password: "secret"
DB: 2
Name: "cache"
"#;
    let config: XRedisConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.addr, "redis://myhost:6380");
    assert_eq!(config.password, "secret");
    assert_eq!(config.db, 2);
    assert_eq!(config.name, "cache");
}
