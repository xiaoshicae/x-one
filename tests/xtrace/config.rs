use x_one::xtrace::config::*;

#[test]
fn test_default_config() {
    let config = XTraceConfig::default();
    assert!(config.enable.is_none());
    assert!(!config.console);
}

#[test]
fn test_is_enabled_none_returns_true() {
    let config = XTraceConfig {
        enable: None,
        console: false,
    };
    assert!(config.is_enabled());
}

#[test]
fn test_is_enabled_true() {
    let config = XTraceConfig {
        enable: Some(true),
        console: false,
    };
    assert!(config.is_enabled());
}

#[test]
fn test_is_enabled_false() {
    let config = XTraceConfig {
        enable: Some(false),
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
    assert_eq!(config.enable, Some(true));
    assert!(config.console);
}

#[test]
fn test_deserialize_minimal_yaml() {
    let yaml = "{}";
    let config: XTraceConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.enable.is_none());
    assert!(!config.console);
}
