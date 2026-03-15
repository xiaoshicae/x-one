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

#[test]
fn test_deserialize_config_all_fields() {
    let yaml = r#"
Addr: "redis://custom:6380"
Password: "pass123"
DB: 5
Username: "admin"
DialTimeout: "1s"
ReadTimeout: "2s"
WriteTimeout: "3s"
MaxRetries: 5
Name: "session"
"#;
    let config: XRedisConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.addr, "redis://custom:6380");
    assert_eq!(config.password, "pass123");
    assert_eq!(config.db, 5);
    assert_eq!(config.username, "admin");
    assert_eq!(config.dial_timeout, "1s");
    assert_eq!(config.read_timeout, "2s");
    assert_eq!(config.write_timeout, "3s");
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.name, "session");
}

#[test]
fn test_deserialize_config_empty_yaml_uses_defaults() {
    // 空映射：所有字段使用 serde(default)
    let yaml = "{}";
    let config: XRedisConfig = serde_yaml::from_str(yaml).unwrap();
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
fn test_deserialize_config_partial_fields() {
    // 只设置部分字段，其余使用默认值
    let yaml = r#"
Addr: "redis://partial:6381"
DB: 3
"#;
    let config: XRedisConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.addr, "redis://partial:6381");
    assert_eq!(config.db, 3);
    // 其余应为默认值
    assert_eq!(config.password, "");
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.dial_timeout, "500ms");
}

#[test]
fn test_deserialize_config_list() {
    let yaml = r#"
- Addr: "redis://host1:6379"
  Name: "cache"
- Addr: "redis://host2:6379"
  Name: "session"
"#;
    let configs: Vec<XRedisConfig> = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].name, "cache");
    assert_eq!(configs[1].name, "session");
}
