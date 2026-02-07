use x_one::xcache::config::*;

#[test]
fn test_default_config() {
    let config = XCacheConfig::default();
    assert_eq!(config.max_capacity, 100_000);
    assert_eq!(config.default_ttl, "5m");
    assert!(config.name.is_empty());
}

#[test]
fn test_deserialize_full_yaml() {
    let yaml = r#"
MaxCapacity: 200000
DefaultTTL: "10m"
Name: "primary"
"#;
    let config: XCacheConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.max_capacity, 200_000);
    assert_eq!(config.default_ttl, "10m");
    assert_eq!(config.name, "primary");
}

#[test]
fn test_deserialize_minimal_yaml() {
    let yaml = "{}";
    let config: XCacheConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.max_capacity, 100_000);
    assert_eq!(config.default_ttl, "5m");
}
