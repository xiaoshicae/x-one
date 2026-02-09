use x_one::xconfig::*;

#[test]
fn test_get_string_no_config() {
    reset_config();
    assert_eq!(get_string("Server.Name"), "");
}

#[test]
fn test_get_server_name_default() {
    reset_config();
    assert_eq!(get_server_name(), DEFAULT_SERVER_NAME);
}

#[test]
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
fn test_get_server_version_default() {
    reset_config();
    assert_eq!(get_server_version(), DEFAULT_SERVER_VERSION);
}

#[test]
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
