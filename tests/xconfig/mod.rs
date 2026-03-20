use serial_test::serial;
use x_one::xconfig::*;

#[test]
#[serial]
fn test_get_string_no_config() {
    reset_config();
    assert_eq!(get_string("Server.Name"), "");
}

#[test]
#[serial]
fn test_get_float64_existing_key_returns_value() {
    reset_config();
    let yaml = "metrics:\n  threshold: 3.14\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let v = get_float64("metrics.threshold");
    assert!((v - 3.14).abs() < f64::EPSILON, "应返回 3.14，实际: {v}");
    reset_config();
}

#[test]
#[serial]
fn test_get_float64_missing_key_returns_zero() {
    reset_config();
    assert!((get_float64("nonexistent.key") - 0.0).abs() < f64::EPSILON);
}

#[test]
#[serial]
fn test_get_float64_non_float_value_returns_zero() {
    reset_config();
    let yaml = "key: not_a_number\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert!((get_float64("key") - 0.0).abs() < f64::EPSILON);
    reset_config();
}

#[test]
#[serial]
fn test_get_value_existing_key_returns_value() {
    reset_config();
    let yaml = "Server:\n  Name: test-app\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let v = get_value("Server.Name");
    assert!(v.is_some());
    assert_eq!(v.unwrap().as_str().unwrap(), "test-app");
    reset_config();
}

#[test]
#[serial]
fn test_get_value_missing_key_returns_none() {
    reset_config();
    let yaml = "Server:\n  Name: test\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert!(get_value("Server.Port").is_none());
    assert!(get_value("NonExistent").is_none());
    reset_config();
}

#[test]
#[serial]
fn test_get_value_no_config_returns_none() {
    reset_config();
    assert!(get_value("any.key").is_none());
}

#[test]
#[serial]
fn test_parse_config_missing_key_returns_error() {
    reset_config();
    let yaml = "Server:\n  Name: test\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let result = parse_config::<x_one::xaxum::AxumConfig>("NonExistent");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "错误消息应包含 'not found'，实际: {err_msg}"
    );
    reset_config();
}

#[test]
#[serial]
fn test_parse_config_invalid_value_returns_error() {
    reset_config();
    let yaml = "XAxum: not_a_map\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let result = parse_config::<x_one::xaxum::AxumConfig>("XAxum");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("parse config"),
        "错误消息应包含 'parse config'，实际: {err_msg}"
    );
    reset_config();
}

#[test]
#[serial]
fn test_get_server_name_default() {
    reset_config();
    assert_eq!(get_server_name(), DEFAULT_SERVER_NAME);
}

#[test]
#[serial]
fn test_get_server_name_with_config() {
    reset_config();
    let yaml = "Server:
  Name: my-app
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert_eq!(get_server_name(), "my-app");
    reset_config();
}

#[test]
#[serial]
fn test_get_server_version_default() {
    reset_config();
    assert_eq!(get_server_version(), DEFAULT_SERVER_VERSION);
}

#[test]
#[serial]
fn test_get_int() {
    reset_config();
    let yaml = "XAxum:
  Port: 9090
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert_eq!(get_int("XAxum.Port"), 9090);
    reset_config();
}

#[test]
#[serial]
fn test_get_bool() {
    reset_config();
    let yaml = "XAxum:
  UseHttp2: true
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert!(get_bool("XAxum.UseHttp2"));
    reset_config();
}

#[test]
#[serial]
fn test_contain_key() {
    reset_config();
    let yaml = "Server:
  Name: test
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert!(contain_key("Server.Name"));
    assert!(!contain_key("Server.NonExistent"));
    reset_config();
}

#[test]
#[serial]
fn test_get_string_slice() {
    reset_config();
    let yaml = "items:
  - a
  - b
  - c
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert_eq!(get_string_slice("items"), vec!["a", "b", "c"]);
    reset_config();
}

#[test]
#[serial]
fn test_parse_config() {
    reset_config();
    let yaml = "XAxum:
  Host: '127.0.0.1'
  Port: 3000
";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let cfg: x_one::xaxum::AxumConfig = parse_config("XAxum").unwrap();
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.port, 3000);
    reset_config();
}

#[test]
#[serial]
fn test_get_float64_integer_value_returns_as_float() {
    reset_config();
    let yaml = "metrics:\n  count: 42\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    // YAML 整数也能通过 as_f64 获取
    let v = get_float64("metrics.count");
    assert!(
        (v - 42.0).abs() < f64::EPSILON,
        "整数应可转为浮点，实际: {v}"
    );
    reset_config();
}

#[test]
#[serial]
fn test_get_float64_negative_value() {
    reset_config();
    let yaml = "temp:\n  value: -1.5\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let v = get_float64("temp.value");
    assert!((v - (-1.5)).abs() < f64::EPSILON);
    reset_config();
}

#[test]
#[serial]
fn test_get_value_nested_key_returns_subtree() {
    reset_config();
    let yaml = "a:\n  b:\n    c: deep\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);

    // 中间节点返回子树
    let v = get_value("a.b");
    assert!(v.is_some());
    let subtree = v.unwrap();
    assert!(subtree.get("c").is_some());

    // 叶子节点返回值
    let v = get_value("a.b.c");
    assert!(v.is_some());
    assert_eq!(v.unwrap().as_str().unwrap(), "deep");
    reset_config();
}

#[test]
#[serial]
fn test_parse_config_deserialize_error_contains_key_and_failed() {
    reset_config();
    // 值存在但类型完全不匹配（字符串无法解析为 struct）
    let yaml = "XAxum: 12345\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    let result = parse_config::<x_one::xaxum::AxumConfig>("XAxum");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("XAxum"),
        "错误消息应包含 key 名称，实际: {err_msg}"
    );
    assert!(
        err_msg.contains("failed"),
        "错误消息应包含 'failed'，实际: {err_msg}"
    );
    reset_config();
}

#[test]
#[serial]
fn test_get_raw_server_name_empty_when_no_config() {
    reset_config();
    assert_eq!(get_raw_server_name(), "");
}

#[test]
#[serial]
fn test_get_raw_server_name_with_config() {
    reset_config();
    let yaml = "Server:\n  Name: my-service\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert_eq!(get_raw_server_name(), "my-service");
    reset_config();
}

#[test]
#[serial]
fn test_get_server_version_with_config() {
    reset_config();
    let yaml = "Server:\n  Version: v1.2.3\n";
    let config: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    set_config(config);
    assert_eq!(get_server_version(), "v1.2.3");
    reset_config();
}

#[test]
#[serial]
fn test_get_int_missing_key_returns_zero() {
    reset_config();
    assert_eq!(get_int("nonexistent"), 0);
}

#[test]
#[serial]
fn test_get_bool_missing_key_returns_false() {
    reset_config();
    assert!(!get_bool("nonexistent"));
}

#[test]
#[serial]
fn test_get_string_slice_missing_key_returns_empty() {
    reset_config();
    assert!(get_string_slice("nonexistent").is_empty());
}
