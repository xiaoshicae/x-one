use x_one::xmetric::{XMETRIC_CONFIG_KEY, XMetricConfig};

#[test]
fn test_config_key() {
    assert_eq!(XMETRIC_CONFIG_KEY, "XMetric");
}

#[test]
fn test_default_config() {
    let config = XMetricConfig::default();
    assert!(config.namespace.is_empty());
    assert!(config.const_labels.is_empty());
    assert!(!config.http_duration_buckets.is_empty());
    assert!(!config.histogram_buckets.is_empty());
}

#[test]
fn test_deserialize_config() {
    let yaml = r#"
Namespace: "myapp"
ConstLabels:
  env: "prod"
  cluster: "cn-east"
"#;
    let config: XMetricConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.namespace, "myapp");
    assert_eq!(config.const_labels.get("env").unwrap(), "prod");
    assert_eq!(config.const_labels.get("cluster").unwrap(), "cn-east");
}
