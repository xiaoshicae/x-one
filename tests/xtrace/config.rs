use x_one::xtrace::config::*;

#[test]
fn test_default_config() {
    let config = XTraceConfig::default();
    assert!(config.enable);
    assert!(!config.console);
}

#[test]
fn test_is_enabled_true() {
    let config = XTraceConfig {
        enable: true,
        console: false,
    };
    assert!(config.is_enabled());
}

#[test]
fn test_is_enabled_false() {
    let config = XTraceConfig {
        enable: false,
        console: false,
    };
    assert!(!config.is_enabled());
}

#[test]
fn test_deserialize_from_yaml() {
    let yaml = r#"
Enable: true
Console: true
"#;
    let config: XTraceConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.enable);
    assert!(config.console);
}

#[test]
fn test_deserialize_minimal_yaml() {
    let yaml = "{}";
    let config: XTraceConfig = serde_yaml::from_str(yaml).unwrap();
    // 未配置时默认启用
    assert!(config.enable);
    assert!(!config.console);
}
